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
//! - **TCP relay only** through the tunnel (IPv4 inner targets); UDP relay and
//!   tunnel-side DNS are not implemented (targets resolve via the host
//!   resolver).
//! - Server certificate pinned to the inline `ca`; optional client-certificate
//!   (`cert`/`key`) and/or username/password auth.
//! - No `tls-auth` / `tls-crypt` control-channel wrapping, no compression, no
//!   keepalive ping generation. Those are rejected or documented as absent, and
//!   any unsupported option combination fails explicitly rather than silently
//!   degrading.

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

use std::net::SocketAddr;

use anyhow::{Context, Result, anyhow, bail};

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::outbound::BoxedStream;

use device::OpenVpnDevice;

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

        // Static control-channel protection is not implemented.
        if opts.tls_crypt.as_deref().is_some_and(|v| !v.trim().is_empty()) {
            bail!("openvpn: `tls-crypt` is not supported");
        }
        if opts.tls_auth.as_deref().is_some_and(|v| !v.trim().is_empty()) {
            bail!("openvpn: `tls-auth` is not supported");
        }

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

    #[test]
    fn rejects_tls_crypt() {
        let err = OpenVpnOutboundConfig::from_proxy(&entry(&format!(
            "name: o\ntype: openvpn\nserver: vpn.example\nport: 1194\nusername: u\npassword: p\nca: \"{}\"\ntls-crypt: \"secret\"\n",
            CA.replace('\n', "\\n")
        )))
        .unwrap_err();
        assert!(err.to_string().contains("tls-crypt"), "{err}");
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
