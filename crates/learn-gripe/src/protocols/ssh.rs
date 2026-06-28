//! SSH tunnel outbound (`type: ssh`).
//!
//! Forwards a connection through an SSH server by opening a `direct-tcpip`
//! channel (RFC 4254 §7.2) — the same mechanism as OpenSSH's `ssh -L` local
//! forwarding: the server dials the requested target on our behalf and relays
//! bytes over the encrypted channel. This is the SSH analogue of the
//! upstream-SOCKS5 / HTTP-`CONNECT` outbounds.
//!
//! The SSH transport itself (version exchange, key exchange, cipher/MAC
//! negotiation, channel windowing) is delegated to the vetted [`russh`] crate
//! with its pure-Rust `ring` backend (no aws-lc-rs / C toolchain), mirroring how
//! the kernel delegates QUIC to quinn and the userspace netstack to smoltcp; we
//! own only the orchestration (auth method selection, host-key pinning, channel
//! open, stream hand-off).
//!
//! Supported clash/mihomo knobs:
//! * `username` (required) and one of:
//!   * `password` → password authentication, or
//!   * `private-key` (+ optional `private-key-passphrase`) → public-key
//!     authentication. OpenSSH / PKCS#8 / PKCS#5 / PuTTY key formats are
//!     accepted (whatever [`russh::keys::decode_secret_key`] understands).
//! * `host-key` → a list of accepted server public keys in `authorized_keys`
//!   format; when set, a server presenting any other key is rejected (host-key
//!   pinning). When empty, the server key is accepted unverified.
//! * `host-key-algorithms` → when set, restricts the accepted server host-key
//!   algorithm names (e.g. `ssh-ed25519`).
//!
//! There is no UDP relay over an SSH `direct-tcpip` channel, so UDP
//! associations are refused up front (see
//! [`crate::outbound::supports_udp_associate`]).

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use anyhow::{Context as _, Result, anyhow, bail};
use russh::keys::{HashAlg, PrivateKeyWithHashAlg, PublicKey, decode_secret_key};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::outbound::BoxedStream;

/// How the client authenticates to the SSH server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshAuth {
    /// `password` authentication.
    Password(String),
    /// Public-key authentication with a private key (PEM/OpenSSH/PuTTY text)
    /// and an optional decryption passphrase.
    PrivateKey { pem: String, passphrase: Option<String> },
}

/// Fully-resolved SSH outbound parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshOutboundConfig {
    pub server: String,
    pub port: u16,
    pub username: String,
    pub auth: SshAuth,
    /// Accepted server host keys in `authorized_keys` text form. Empty means
    /// "accept any key" (no host-key verification).
    pub host_keys: Vec<String>,
    /// Accepted server host-key algorithm names. Empty means "no restriction".
    pub host_key_algorithms: Vec<String>,
}

impl SshOutboundConfig {
    /// Build an outbound config from a parsed `ssh` proxy entry.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("ssh: missing server")?;
        let port = opts.port.context("ssh: missing port")?;
        let username = opts
            .username
            .clone()
            .filter(|s| !s.is_empty())
            .context("ssh: missing username")?;

        // A private key takes precedence over a password when both are present,
        // matching mihomo. One of the two is required.
        let auth = if let Some(pem) = opts.private_key.as_deref().filter(|s| !s.is_empty()) {
            let passphrase = opts.private_key_passphrase.clone().filter(|s| !s.is_empty());
            // Validate the key parses now so a bad key is a config error rather
            // than a per-connection failure.
            decode_secret_key(pem, passphrase.as_deref()).context("ssh: invalid private-key")?;
            SshAuth::PrivateKey {
                pem: pem.to_string(),
                passphrase,
            }
        } else if let Some(password) = opts.password.as_deref() {
            SshAuth::Password(password.to_string())
        } else {
            bail!("ssh: requires a password or private-key");
        };

        let host_keys: Vec<String> = opts
            .host_key
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect();
        for hk in &host_keys {
            PublicKey::from_openssh(hk).with_context(|| format!("ssh: invalid host-key {hk:?}"))?;
        }

        let host_key_algorithms: Vec<String> = opts
            .host_key_algorithms
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect();

        Ok(Self {
            server,
            port,
            username,
            auth,
            host_keys,
            host_key_algorithms,
        })
    }
}

/// SSH client handler: enforces host-key pinning / algorithm restrictions at the
/// `check_server_key` callback. All other callbacks keep their defaults.
struct ClientHandler {
    accepted_keys: Vec<PublicKey>,
    allowed_algorithms: Vec<String>,
}

impl russh::client::Handler for ClientHandler {
    type Error = anyhow::Error;

    async fn check_server_key(&mut self, server_public_key: &PublicKey) -> Result<bool, Self::Error> {
        if !self.allowed_algorithms.is_empty() {
            let alg = server_public_key.algorithm().as_str().to_string();
            if !self.allowed_algorithms.iter().any(|a| a.eq_ignore_ascii_case(&alg)) {
                return Ok(false);
            }
        }
        if self.accepted_keys.is_empty() {
            // No pinning configured: accept the key (transport stays encrypted).
            return Ok(true);
        }
        Ok(self
            .accepted_keys
            .iter()
            .any(|k| k.key_data() == server_public_key.key_data()))
    }
}

/// Relay-ready SSH `direct-tcpip` stream. Holds the session [`russh::client::Handle`]
/// alongside the channel stream so the background session task stays alive for
/// the lifetime of the relay.
struct SshStream {
    stream: russh::ChannelStream<russh::client::Msg>,
    _session: russh::client::Handle<ClientHandler>,
}

impl AsyncRead for SshStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_read(cx, buf)
    }
}

impl AsyncWrite for SshStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().stream).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_shutdown(cx)
    }
}

/// Connect to the SSH server, authenticate, and open a `direct-tcpip` channel to
/// `target`, returning a relay-ready stream.
pub async fn connect(config: &SshOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let accepted_keys = config
        .host_keys
        .iter()
        .map(|s| PublicKey::from_openssh(s).map_err(|e| anyhow!("ssh: invalid host-key {s:?}: {e}")))
        .collect::<Result<Vec<_>>>()?;
    let handler = ClientHandler {
        accepted_keys,
        allowed_algorithms: config.host_key_algorithms.clone(),
    };

    let ssh_config = Arc::new(russh::client::Config::default());
    let tcp = TcpStream::connect((config.server.as_str(), config.port))
        .await
        .with_context(|| format!("ssh: dial {}:{}", config.server, config.port))?;
    let mut handle = russh::client::connect_stream(ssh_config, tcp, handler)
        .await
        .context("ssh: transport handshake")?;

    let authenticated = match &config.auth {
        SshAuth::Password(password) => handle
            .authenticate_password(&config.username, password)
            .await
            .context("ssh: password authentication")?,
        SshAuth::PrivateKey { pem, passphrase } => {
            let key = decode_secret_key(pem, passphrase.as_deref()).context("ssh: parse private-key")?;
            // RSA keys must pick a signature hash; ask the server for its best
            // supported one (falls back to the library default otherwise).
            let hash_alg: Option<HashAlg> = if key.algorithm().is_rsa() {
                handle.best_supported_rsa_hash().await.ok().flatten().flatten()
            } else {
                None
            };
            handle
                .authenticate_publickey(&config.username, PrivateKeyWithHashAlg::new(Arc::new(key), hash_alg))
                .await
                .context("ssh: public-key authentication")?
        }
    };
    if !authenticated.success() {
        bail!("ssh: authentication failed for user {:?}", config.username);
    }

    let channel = handle
        .channel_open_direct_tcpip(target.host(), u32::from(target.port()), "127.0.0.1", 0)
        .await
        .with_context(|| format!("ssh: open direct-tcpip channel to {target}"))?;

    Ok(Box::new(SshStream {
        stream: channel.into_stream(),
        _session: handle,
    }) as BoxedStream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::outbound_opts::ProxyEntry;

    fn parse_entry(yaml: &str) -> ProxyEntry {
        serde_yaml_ng::from_str(yaml).expect("parse proxy entry")
    }

    // An unencrypted OpenSSH ed25519 private key (test fixture, not a real
    // credential), used to exercise private-key parsing.
    const TEST_ED25519_KEY: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\n\
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\n\
QyNTUxOQAAACA4MD4wt0Q5NvzM5mp5IfHTcJ9iZ1tJR4/ZC+qzadF+8gAAAJDyL3MH8i9z\n\
BwAAAAtzc2gtZWQyNTUxOQAAACA4MD4wt0Q5NvzM5mp5IfHTcJ9iZ1tJR4/ZC+qzadF+8g\n\
AAAEC4G4oQ5s4gnxzIQ4cm42yXSgkQvVOBzlHusfTW2MoH4zgwPjC3RDk2/Mzmankh8dNw\n\
n2JnW0lHj9kL6rNp0X7yAAAADGdyaXBlLWNsaWVudAE=\n\
-----END OPENSSH PRIVATE KEY-----\n";

    #[test]
    fn parses_password_auth() {
        let cfg = SshOutboundConfig::from_proxy(&parse_entry(
            "name: s\ntype: ssh\nserver: ssh.example\nport: 22\nusername: alice\npassword: hunter2\n",
        ))
        .unwrap();
        assert_eq!(cfg.server, "ssh.example");
        assert_eq!(cfg.port, 22);
        assert_eq!(cfg.username, "alice");
        assert_eq!(cfg.auth, SshAuth::Password("hunter2".to_string()));
        assert!(cfg.host_keys.is_empty());
    }

    #[test]
    fn private_key_takes_precedence_over_password() {
        let yaml = format!(
            "name: s\ntype: ssh\nserver: ssh.example\nport: 22\nusername: bob\npassword: pw\nprivate-key: |\n{}\n",
            TEST_ED25519_KEY
                .lines()
                .map(|l| format!("  {l}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let cfg = SshOutboundConfig::from_proxy(&parse_entry(&yaml)).unwrap();
        match cfg.auth {
            SshAuth::PrivateKey { passphrase, .. } => assert_eq!(passphrase, None),
            other => panic!("expected private-key auth, got {other:?}"),
        }
    }

    #[test]
    fn host_keys_and_algorithms_are_collected() {
        let cfg = SshOutboundConfig::from_proxy(&parse_entry(
            "name: s\ntype: ssh\nserver: ssh.example\nport: 22\nusername: alice\npassword: pw\n\
host-key:\n  - 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICJkDpE7R5jwMQXtWJFxW5/mM5/vmC7reVIOfXCMzQEf'\n\
host-key-algorithms:\n  - ssh-ed25519\n",
        ))
        .unwrap();
        assert_eq!(cfg.host_keys.len(), 1);
        assert_eq!(cfg.host_key_algorithms, vec!["ssh-ed25519".to_string()]);
    }

    #[test]
    fn missing_credentials_is_rejected() {
        let err = SshOutboundConfig::from_proxy(&parse_entry(
            "name: s\ntype: ssh\nserver: ssh.example\nport: 22\nusername: alice\n",
        ))
        .unwrap_err();
        assert!(err.to_string().contains("password or private-key"), "{err}");
    }

    #[test]
    fn missing_username_is_rejected() {
        let err = SshOutboundConfig::from_proxy(&parse_entry(
            "name: s\ntype: ssh\nserver: ssh.example\nport: 22\npassword: pw\n",
        ))
        .unwrap_err();
        assert!(err.to_string().contains("username"), "{err}");
    }

    #[test]
    fn invalid_private_key_is_rejected() {
        let err = SshOutboundConfig::from_proxy(&parse_entry(
            "name: s\ntype: ssh\nserver: ssh.example\nport: 22\nusername: alice\nprivate-key: not-a-key\n",
        ))
        .unwrap_err();
        assert!(err.to_string().contains("private-key"), "{err}");
    }

    #[test]
    fn invalid_host_key_is_rejected() {
        let err = SshOutboundConfig::from_proxy(&parse_entry(
            "name: s\ntype: ssh\nserver: ssh.example\nport: 22\nusername: alice\npassword: pw\nhost-key:\n  - 'garbage'\n",
        ))
        .unwrap_err();
        assert!(err.to_string().contains("host-key"), "{err}");
    }
}
