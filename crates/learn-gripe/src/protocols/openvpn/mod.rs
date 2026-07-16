//! OpenVPN outbound data plane (TCP transport + AEAD data channel).
//!
//! Like WireGuard, OpenVPN is an L3 encrypted tunnel, not a per-target stream
//! proxy: to relay a TCP connection we run a userspace smoltcp TCP/IP stack
//! (already vendored for the TUN inbound) bound to the address the server pushes
//! us, and each relayed connection is a smoltcp socket whose inner IP packets
//! are sealed into OpenVPN `P_DATA_V2` packets and carried over the control
//! connection's TCP transport.
//!
//! We own the protocol implementation here (framing, the reliable/acked control
//! channel, key-method-2 + the OpenVPN PRF, and the AEAD data channel), and
//! delegate the two "do not hand-roll" surfaces to vetted crates: the
//! control-channel TLS handshake runs on our vendored `rustls` fork (tunnelled
//! over `P_CONTROL_V1` messages), and the userspace TCP/IP relay runs on
//! `smoltcp` — the same "delegate the wire codec, own the plumbing" split used
//! elsewhere in the kernel.
//!
//! Scope (this slice — deliberately a subset of OpenVPN, not full parity):
//! - **TCP and UDP transport** (`proto tcp` / `proto udp`). On UDP the reliable
//!   control channel retransmits unacked handshake packets on a timer; the
//!   AEAD data channel is unreliable (inner TCP recovers loss).
//! - **AEAD data ciphers only**: `AES-256-GCM` (default), `AES-128-GCM`,
//!   `CHACHA20-POLY1305`. CBC + HMAC ciphers are rejected.
//! - **TCP and UDP relay** through the tunnel (IPv4 inner targets only);
//!   tunnel-side DNS is not implemented (targets resolve via the host
//!   resolver).
//! - Server certificate pinned to the inline `ca`; optional client-certificate
//!   (`cert`/`key`) and/or username/password auth.
//! - **`tls-auth` / `tls-crypt` control-channel protection** from an inline
//!   "OpenVPN Static key V1" (mutually exclusive): `tls-auth` HMAC-wraps every
//!   control packet (`auth` digest SHA1/SHA256/SHA512, `key-direction` 0/1 or
//!   bidirectional); `tls-crypt` encrypts + authenticates them (AES-256-CTR +
//!   HMAC-SHA256, fixed client direction). See [`tlswrap`].
//! - No compression and no keepalive ping generation. Unsupported option
//!   combinations fail explicitly rather than silently degrading.

mod control;
mod data;
mod device;
mod device_loop;
mod keymethod;
mod netstack;
mod packet;
mod push;
mod stream;
mod tls;
mod tlswrap;

use std::net::SocketAddr;

use anyhow::{Context, Result, anyhow, bail};

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::outbound::BoxedStream;

use device::OpenVpnDevice;

pub use stream::OvpnUdpAssoc;

/// Default tunnel MTU (max inner IP packet).
const DEFAULT_MTU: u32 = 1500;
/// Default AEAD data cipher when the config omits `cipher`.
const DEFAULT_CIPHER: &str = "AES-256-GCM";

/// Parsed OpenVPN outbound configuration for the implemented subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenVpnOutboundConfig {
    pub server: String,
    pub port: u16,
    /// Inline CA certificate (PEM) the server cert is pinned to.
    ca_pem: String,
    /// Optional client certificate chain (PEM) for cert auth.
    client_cert_pem: Option<String>,
    /// Optional client private key (PEM) for cert auth.
    client_key_pem: Option<String>,
    username: Option<String>,
    password: Option<String>,
    /// Normalized AEAD cipher name (e.g. `AES-256-GCM`).
    cipher: String,
    /// Inline `tls-auth` static key (mutually exclusive with `tls_crypt`).
    tls_auth: Option<String>,
    /// Inline `tls-crypt` static key (mutually exclusive with `tls_auth`).
    tls_crypt: Option<String>,
    /// `key-direction` for `tls-auth` (`None` = bidirectional).
    key_direction: Option<u8>,
    /// Normalized `auth` digest name for the `tls-auth` HMAC (default SHA1).
    auth_digest: String,
    /// Whether the tunnel transport is UDP (`true`) or TCP (`false`).
    udp: bool,
    mtu: u32,
}

impl OpenVpnOutboundConfig {
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .ok_or_else(|| anyhow!("openvpn: missing `server`"))?;
        let port = opts.port.ok_or_else(|| anyhow!("openvpn: missing `port`"))?;

        // Transport: TCP or UDP (IPv4 client variants).
        let udp = match opts.proto.as_deref().map(|p| p.trim().to_ascii_lowercase()) {
            None => false,
            Some(proto) => match proto.as_str() {
                "tcp" | "tcp-client" | "tcp4" | "tcp4-client" => false,
                "udp" | "udp-client" | "udp4" | "udp4-client" => true,
                other => bail!("openvpn: unsupported `proto` {other:?} (only tcp and udp are implemented)"),
            },
        };

        // Device type: tun only (no L2 tap bridging).
        if let Some(dev) = opts.dev.as_deref() {
            let dev = dev.trim().to_ascii_lowercase();
            if !dev.is_empty() && dev != "tun" && !dev.starts_with("tun") {
                bail!("openvpn: unsupported `dev` {dev:?} (only tun is implemented)");
            }
        }

        // Data cipher: AEAD only.
        let cipher = normalize_cipher(opts.cipher.as_deref())?;

        // Compression is not implemented.
        if let Some(comp) = opts.comp_lzo.as_deref() {
            let comp = comp.trim().to_ascii_lowercase();
            if !comp.is_empty() && comp != "no" && comp != "false" {
                bail!("openvpn: `comp-lzo` compression is not supported");
            }
        }

        // Static control-channel protection: tls-auth XOR tls-crypt, each an
        // inline "OpenVPN Static key V1" that must parse at config time.
        let tls_auth = opts.tls_auth.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let tls_crypt = opts.tls_crypt.as_deref().map(str::trim).filter(|s| !s.is_empty());
        if tls_auth.is_some() && tls_crypt.is_some() {
            bail!("openvpn: `tls-auth` and `tls-crypt` are mutually exclusive");
        }
        if let Some(key) = tls_auth.or(tls_crypt) {
            tlswrap::parse_static_key(key)?;
        }
        let key_direction = opts.key_direction;
        tlswrap::KeyDirection::from_option(key_direction)?;
        if key_direction.is_some() && tls_auth.is_none() {
            bail!("openvpn: `key-direction` requires `tls-auth` (tls-crypt has a fixed direction)");
        }
        // `auth` names the tls-auth HMAC digest (the data channel is AEAD-only,
        // where the digest is unused beyond the options string).
        let auth_digest = match opts.auth.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(name) => tlswrap::AuthDigest::parse(name)?.name().to_string(),
            None => "SHA1".to_string(),
        };

        // The server cert is always pinned to the inline CA; verification
        // cannot be turned off for this outbound.
        if opts.skip_cert_verify == Some(true) {
            bail!("openvpn: `skip-cert-verify` is not supported (server cert is pinned to `ca`)");
        }

        let ca_pem = opts
            .ca
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("openvpn: missing inline `ca` certificate"))?;
        if !ca_pem.contains("BEGIN CERTIFICATE") {
            bail!("openvpn: `ca` must be an inline PEM certificate (file paths are not supported)");
        }

        // Client certificate auth: both `cert` and `key` or neither.
        let client_cert_pem = opts.cert.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let client_key_pem = opts.key.as_deref().map(str::trim).filter(|s| !s.is_empty());
        match (client_cert_pem, client_key_pem) {
            (Some(_), None) => bail!("openvpn: `cert` provided without `key`"),
            (None, Some(_)) => bail!("openvpn: `key` provided without `cert`"),
            _ => {}
        }
        let username = opts.username.clone().filter(|s| !s.is_empty());
        let password = opts.password.clone().filter(|s| !s.is_empty());
        if client_cert_pem.is_none() && username.is_none() {
            bail!("openvpn: authentication required (`cert`+`key` and/or `username`+`password`)");
        }

        let mtu = opts.mtu.filter(|m| *m >= 576).unwrap_or(DEFAULT_MTU);

        Ok(Self {
            server,
            port,
            ca_pem: ca_pem.to_string(),
            client_cert_pem: client_cert_pem.map(str::to_string),
            client_key_pem: client_key_pem.map(str::to_string),
            username,
            password,
            cipher,
            tls_auth: tls_auth.map(str::to_string),
            tls_crypt: tls_crypt.map(str::to_string),
            key_direction,
            auth_digest,
            udp,
            mtu,
        })
    }

    /// Stable identity for the device cache. Every field that changes the
    /// negotiated tunnel (endpoint, cipher, and *all* credential material) is
    /// folded in, so configs that differ only in password or client cert never
    /// alias onto the same shared device.
    fn registry_key(&self) -> String {
        let mut cred = crc32fast::Hasher::new();
        cred.update(self.ca_pem.as_bytes());
        cred.update(&[0]);
        cred.update(self.client_cert_pem.as_deref().unwrap_or_default().as_bytes());
        cred.update(&[0]);
        cred.update(self.client_key_pem.as_deref().unwrap_or_default().as_bytes());
        cred.update(&[0]);
        cred.update(self.password.as_deref().unwrap_or_default().as_bytes());
        cred.update(&[0]);
        cred.update(self.tls_auth.as_deref().unwrap_or_default().as_bytes());
        cred.update(&[0]);
        cred.update(self.tls_crypt.as_deref().unwrap_or_default().as_bytes());
        cred.update(&[0]);
        cred.update(&[self.key_direction.map_or(0xff, |d| d)]);
        cred.update(self.auth_digest.as_bytes());
        let cred_tag = cred.finalize();
        let proto = if self.udp { "udp" } else { "tcp" };
        format!(
            "{}:{}|{proto}|{}|{}|{cred_tag:08x}",
            self.server,
            self.port,
            self.cipher,
            self.username.as_deref().unwrap_or_default()
        )
    }

    /// Build the configured control-channel wrap (`tls-auth` / `tls-crypt`),
    /// or `None` when the control channel runs in cleartext.
    fn control_wrap(&self) -> Result<Option<tlswrap::ControlWrap>> {
        if let Some(key) = self.tls_crypt.as_deref() {
            return Ok(Some(tlswrap::ControlWrap::tls_crypt(key)?));
        }
        if let Some(key) = self.tls_auth.as_deref() {
            let direction = tlswrap::KeyDirection::from_option(self.key_direction)?;
            let digest = tlswrap::AuthDigest::parse(&self.auth_digest)?;
            return Ok(Some(tlswrap::ControlWrap::tls_auth(key, direction, digest)?));
        }
        Ok(None)
    }
}

/// Normalize + validate a data-channel cipher name, defaulting to AES-256-GCM.
fn normalize_cipher(cipher: Option<&str>) -> Result<String> {
    let Some(raw) = cipher.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(DEFAULT_CIPHER.to_string());
    };
    let upper = raw.to_ascii_uppercase();
    match upper.as_str() {
        "AES-128-GCM" | "AES-256-GCM" | "CHACHA20-POLY1305" => Ok(upper),
        other => bail!(
            "openvpn: unsupported `cipher` {other:?} (AEAD ciphers only: AES-128-GCM, AES-256-GCM, CHACHA20-POLY1305)"
        ),
    }
}

/// Connect a relayed TCP stream to `target` through the OpenVPN tunnel, reusing
/// (or lazily building) the per-config device.
pub async fn connect(config: &OpenVpnOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let device = OpenVpnDevice::get_or_create(config).await?;
    let dst = resolve_target(target).await?;
    let stream = device.open_tcp(dst).await?;
    Ok(Box::new(stream) as BoxedStream)
}

/// Open a relayed UDP association to `target` through the OpenVPN tunnel,
/// reusing (or lazily building) the per-config device. Each association is a
/// userspace smoltcp UDP socket; datagrams to the resolved destination are
/// sealed into `P_DATA_V2` like the TCP flows. The inner stack is IPv4-only, so
/// IPv6 destinations are rejected.
pub async fn connect_udp(config: &OpenVpnOutboundConfig, target: &TargetAddr) -> Result<OvpnUdpAssoc> {
    let device = OpenVpnDevice::get_or_create(config).await?;
    let dst = resolve_target(target).await?;
    if !dst.is_ipv4() {
        bail!("openvpn: inner relay is IPv4-only, cannot reach {dst}");
    }
    device.open_udp(dst).await
}

/// Resolve a relayed target to a literal socket address via the host resolver
/// (tunnel-side DNS is not implemented in this slice).
async fn resolve_target(target: &TargetAddr) -> Result<SocketAddr> {
    match target {
        TargetAddr::Ip(addr) => Ok(*addr),
        TargetAddr::Domain(host, port) => tokio::net::lookup_host((host.as_str(), *port))
            .await
            .with_context(|| format!("openvpn: resolve {host}:{port}"))?
            .next()
            .ok_or_else(|| anyhow!("openvpn: no addresses for {host}:{port}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::outbound_opts::ProxyEntry;

    fn entry(yaml: &str) -> ProxyEntry {
        serde_yaml_ng::from_str(yaml).expect("parse proxy entry")
    }

    const CA: &str = "-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n";

    #[test]
    fn requires_auth() {
        let err = OpenVpnOutboundConfig::from_proxy(&entry(&format!(
            "name: o\ntype: openvpn\nserver: vpn.example\nport: 1194\nca: \"{}\"\n",
            CA.replace('\n', "\\n")
        )))
        .unwrap_err();
        assert!(err.to_string().contains("authentication required"), "{err}");
    }

    #[test]
    fn accepts_udp_proto() {
        let cfg = OpenVpnOutboundConfig::from_proxy(&entry(&format!(
            "name: o\ntype: openvpn\nserver: vpn.example\nport: 1194\nproto: udp\nusername: u\npassword: p\nca: \"{}\"\n",
            CA.replace('\n', "\\n")
        )))
        .unwrap();
        assert!(cfg.udp);
    }

    #[test]
    fn rejects_unknown_proto() {
        let err = OpenVpnOutboundConfig::from_proxy(&entry(&format!(
            "name: o\ntype: openvpn\nserver: vpn.example\nport: 1194\nproto: sctp\nusername: u\npassword: p\nca: \"{}\"\n",
            CA.replace('\n', "\\n")
        )))
        .unwrap_err();
        assert!(err.to_string().contains("proto"), "{err}");
    }

    #[test]
    fn defaults_to_tcp_proto() {
        let cfg = OpenVpnOutboundConfig::from_proxy(&entry(&format!(
            "name: o\ntype: openvpn\nserver: vpn.example\nport: 1194\nusername: u\npassword: p\nca: \"{}\"\n",
            CA.replace('\n', "\\n")
        )))
        .unwrap();
        assert!(!cfg.udp);
    }

    #[test]
    fn rejects_cbc_cipher() {
        let err = OpenVpnOutboundConfig::from_proxy(&entry(&format!(
            "name: o\ntype: openvpn\nserver: vpn.example\nport: 1194\ncipher: AES-256-CBC\nusername: u\npassword: p\nca: \"{}\"\n",
            CA.replace('\n', "\\n")
        )))
        .unwrap_err();
        assert!(err.to_string().contains("cipher"), "{err}");
    }

    fn static_key() -> String {
        let body: String = (0..256).map(|i| format!("{:02x}", i as u8)).collect();
        format!("-----BEGIN OpenVPN Static key V1-----\\n{body}\\n-----END OpenVPN Static key V1-----\\n")
    }

    #[test]
    fn accepts_tls_auth_and_tls_crypt_but_not_both() {
        let base = format!(
            "name: o\ntype: openvpn\nserver: vpn.example\nport: 1194\nusername: u\npassword: p\nca: \"{}\"\n",
            CA.replace('\n', "\\n")
        );
        let key = static_key();

        let cfg = OpenVpnOutboundConfig::from_proxy(&entry(&format!("{base}tls-auth: \"{key}\"\nkey-direction: 1\n")))
            .unwrap();
        assert!(cfg.tls_auth.is_some());
        assert_eq!(cfg.key_direction, Some(1));
        assert!(cfg.control_wrap().unwrap().is_some());

        let cfg = OpenVpnOutboundConfig::from_proxy(&entry(&format!("{base}tls-crypt: \"{key}\"\n"))).unwrap();
        assert!(cfg.tls_crypt.is_some());
        assert!(cfg.control_wrap().unwrap().is_some());

        let err =
            OpenVpnOutboundConfig::from_proxy(&entry(&format!("{base}tls-auth: \"{key}\"\ntls-crypt: \"{key}\"\n")))
                .unwrap_err();
        assert!(err.to_string().contains("mutually exclusive"), "{err}");
    }

    #[test]
    fn rejects_malformed_static_key_and_stray_key_direction() {
        let base = format!(
            "name: o\ntype: openvpn\nserver: vpn.example\nport: 1194\nusername: u\npassword: p\nca: \"{}\"\n",
            CA.replace('\n', "\\n")
        );
        let err = OpenVpnOutboundConfig::from_proxy(&entry(&format!("{base}tls-crypt: \"secret\"\n"))).unwrap_err();
        assert!(err.to_string().contains("Static key"), "{err}");

        let err = OpenVpnOutboundConfig::from_proxy(&entry(&format!("{base}key-direction: 1\n"))).unwrap_err();
        assert!(err.to_string().contains("key-direction"), "{err}");

        let key = static_key();
        let err =
            OpenVpnOutboundConfig::from_proxy(&entry(&format!("{base}tls-auth: \"{key}\"\nauth: MD5\n"))).unwrap_err();
        assert!(err.to_string().contains("auth"), "{err}");
    }

    #[test]
    fn defaults_cipher_and_mtu() {
        let cfg = OpenVpnOutboundConfig::from_proxy(&entry(&format!(
            "name: o\ntype: openvpn\nserver: vpn.example\nport: 1194\nusername: u\npassword: p\nca: \"{}\"\n",
            CA.replace('\n', "\\n")
        )))
        .unwrap();
        assert_eq!(cfg.cipher, "AES-256-GCM");
        assert_eq!(cfg.mtu, DEFAULT_MTU);
    }
}
