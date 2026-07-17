//! Sudoku outbound (TCP-only data plane).
//!
//! [Sudoku](https://github.com/SUDOKU-ASCII/sudoku) is a self-designed tunnel
//! whose distinguishing feature is a deterministic byte-obfuscation layer: each
//! plaintext byte is expanded into four "hint" wire-bytes derived from a
//! key-seeded assignment of the 288 valid 4×4 Sudoku grids, optionally
//! interleaved with padding. Underneath that look the tunnel is a conventional
//! AEAD record layer wrapping a small control protocol (KIP) that performs an
//! X25519 handshake and then carries an `OpenTCP` request before relaying bytes
//! transparently.
//!
//! ## Layering (outermost → innermost)
//!
//! ```text
//! TCP  ──  obfuscation (4-hint expand)  ──  AEAD record  ──  KIP / payload
//! ```
//!
//! The client writes plaintext into the record layer, which frames + encrypts
//! it; the obfuscation layer then expands every record byte into hint bytes
//! before they reach the socket. Reads run the same stack in reverse.
//!
//! ## Scope (TCP baseline + UDP-over-TCP)
//!
//! This module implements the common single-table case with HTTP masking
//! disabled: transparent TCP relay via `OpenTCP` and UDP relay via
//! UDP-over-TCP (`StartUoT`, see [`uot`]). The uplink is always the pure
//! (one-byte → four-hint) codec; the downlink is either the pure codec
//! (`enable-pure-downlink: true`, the default) or the bandwidth-optimised
//! 6-bit *packed* codec (`enable-pure-downlink: false`). With `multiplex: on`
//! the TCP data plane multiplexes logical streams over one shared tunnel via
//! `StartMux` (see [`mux`]); UDP still rides its own `StartUoT` tunnel. Legacy
//! HTTP masking (a one-shot fake request-header prefix, see [`mask`]) is
//! supported; the CDN-friendly HTTP tunnel modes (`stream`/`poll`/`auto`/`ws`)
//! and the reverse channel are intentionally left to follow-up work and are
//! rejected up front rather than mis-handled.

mod grid;
mod kip;
mod layout;
mod mask;
mod mux;
mod obfs;
mod record;
mod rng;
mod rng_cooked;
mod table;
mod uot;

use anyhow::{Context, Result, bail};

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::outbound::BoxedStream;
use crate::transport::{self, Security, Transport};

use self::record::AeadMethod;

pub use self::uot::SudokuUdpAssoc;

/// Parsed configuration for a Sudoku outbound (TCP relay + UDP-over-TCP).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SudokuOutboundConfig {
    pub server: String,
    pub port: u16,
    /// Pre-shared key (`key`); seeds the obfuscation table and the PSK record
    /// bases, and is the handshake user identity.
    pub key: String,
    /// Negotiated AEAD record cipher.
    aead_method: AeadMethod,
    /// Obfuscation table layout (`table-type`), e.g. `prefer_entropy`.
    table_type: String,
    /// Optional 8-symbol custom table pattern (empty when unused).
    custom_pattern: String,
    /// Per-byte padding probability percentage range `[min, max]`.
    padding_min: u32,
    padding_max: u32,
    /// Downlink codec: the pure (one-byte → four-hint) codec when `true`
    /// (`enable-pure-downlink`, default), otherwise the 6-bit packed codec.
    pure_downlink: bool,
    /// Multiplex logical streams over one shared tunnel via `StartMux` when
    /// `true` (`multiplex: on`); otherwise one tunnel per connection.
    session_mux: bool,
    /// Legacy HTTP masking parameters when enabled (`httpmask` with mode
    /// empty/`legacy`); `None` disables masking. The CDN HTTP tunnel modes
    /// (`stream`/`poll`/`auto`/`ws`) are rejected at config time.
    http_mask: Option<mask::HttpMaskConfig>,
}

impl SudokuOutboundConfig {
    /// Build an outbound config from a parsed `sudoku` proxy entry.
    ///
    /// The uplink is always the pure (one-byte → four-hint) codec; the downlink
    /// follows `enable-pure-downlink` (default `true` = pure, `false` = 6-bit
    /// packed). Legacy HTTP masking (mode empty/`legacy`) prefixes the stream
    /// with a fake request header; the CDN HTTP tunnel modes
    /// (`stream`/`poll`/`auto`/`ws`) are rejected here. The tunnel then carries
    /// both TCP (`OpenTCP`, or `StartMux` when `multiplex: on`) and UDP
    /// (`StartUoT`) traffic. The reverse channel is deferred to follow-up work.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("sudoku: missing server")?;
        let port = opts.port.context("sudoku: missing port")?;
        let key = opts
            .key
            .clone()
            .filter(|s| !s.is_empty())
            .context("sudoku: missing key")?;

        let aead_method = AeadMethod::parse(opts.aead_method.as_deref().unwrap_or("").trim())?;

        // Downlink codec follows `enable-pure-downlink` (default true = pure,
        // false = 6-bit packed); both are implemented.
        let pure_downlink = opts.enable_pure_downlink.unwrap_or(true);

        // Native session mux is enabled only by `multiplex: on` (matching
        // upstream `SessionMuxEnabled`); `off`/`auto`/absent keep one tunnel per
        // connection.
        let session_mux = opts
            .multiplex
            .as_deref()
            .map(|m| m.trim().eq_ignore_ascii_case("on"))
            .unwrap_or(false);

        // HTTP masking: the legacy one-shot fake-header prefix (mode empty or
        // `legacy`) is supported; the CDN HTTP tunnel modes are not yet.
        let http_mask = match &opts.httpmask {
            Some(m) if m.disable != Some(true) => {
                let mode = m.mode.as_deref().unwrap_or("").trim().to_ascii_lowercase();
                match mode.as_str() {
                    "stream" | "poll" | "auto" | "ws" => bail!(
                        "sudoku: HTTP tunnel mode {mode:?} is not supported yet; use legacy masking (omit httpmask.mode) or set httpmask.disable: true"
                    ),
                    "" | "legacy" => {
                        let host = m
                            .host
                            .clone()
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| format!("{server}:{port}"));
                        Some(mask::HttpMaskConfig {
                            host,
                            path_root: m.path_root.clone().unwrap_or_default(),
                        })
                    }
                    other => bail!("sudoku: unknown httpmask.mode {other:?}"),
                }
            }
            _ => None,
        };

        let table_type = opts.table_type.clone().unwrap_or_default();
        let custom_pattern = opts
            .custom_table
            .clone()
            .or_else(|| opts.custom_tables.as_ref().and_then(|v| v.first().cloned()))
            .unwrap_or_default();

        // Validate the table can actually be built (rejects an invalid table-type
        // or malformed custom pattern at config time rather than on first dial).
        table::new_directional_table(&key, &table_type, &custom_pattern)
            .context("sudoku: invalid table-type / custom-table")?;

        Ok(Self {
            server,
            port,
            key,
            aead_method,
            table_type,
            custom_pattern,
            padding_min: opts.padding_min.unwrap_or(0),
            padding_max: opts.padding_max.unwrap_or(0),
            pure_downlink,
            session_mux,
            http_mask,
        })
    }
}

/// Dial the Sudoku server and bring the tunnel up to the point where it is
/// ready to carry a control request: TCP + obfuscation + AEAD record stacks,
/// the KIP X25519 handshake, and the post-handshake record rekey. The returned
/// stream is shared by the TCP ([`connect`]) and UDP-over-TCP
/// ([`uot`](self::uot)) data planes, which differ only in the control message
/// they write next (`OpenTCP` vs `StartUoT`).
async fn establish_session(config: &SudokuOutboundConfig) -> Result<BoxedStream> {
    let tables = table::new_directional_table(&config.key, &config.table_type, &config.custom_pattern)
        .context("sudoku: build obfuscation table")?;
    let table::DirectionalTable { uplink, downlink } = tables;

    let mut inner = transport::establish(&config.server, config.port, &Security::None, &Transport::Tcp)
        .await
        .context("sudoku: dial server")?;

    // Legacy HTTP masking: write one fake request header before the raw stream.
    if let Some(mask_cfg) = &config.http_mask {
        mask::write_request_header(&mut inner, mask_cfg)
            .await
            .context("sudoku: write HTTP mask header")?;
    }

    // Outermost on-wire layer: expand record bytes into Sudoku hint bytes. The
    // client always writes the pure uplink; its downlink read path switches to
    // the packed decoder when `enable-pure-downlink` is disabled.
    let obfs = obfs::ObfsStream::new(
        inner,
        uplink,
        downlink,
        config.padding_min as i32,
        config.padding_max as i32,
        false,
        !config.pure_downlink,
    );

    // AEAD record layer, keyed initially from the PSK directional bases.
    let (psk_c2s, psk_s2c) = kip::derive_psk_bases(&config.key);
    let mut record =
        record::RecordStream::new(obfs, config.aead_method, &psk_c2s, &psk_s2c).context("sudoku: init record layer")?;

    // KIP X25519 handshake; rekey the record layer with the session bases. A
    // single configured table sends no table hint (matching `pickTable`).
    let outcome = kip::client_handshake(&mut record, &config.key, &[], None)
        .await
        .context("sudoku: KIP handshake")?;
    record
        .rekey(&outcome.session_c2s, &outcome.session_s2c)
        .context("sudoku: rekey after handshake")?;

    Ok(Box::new(record))
}

/// Connect through the Sudoku server to `target` and return a relay-ready
/// stream. Brings up the shared session (see [`establish_session`]), writes the
/// `OpenTCP` request, and hands back a transparent stream.
pub async fn connect(config: &SudokuOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    if config.session_mux {
        return mux::connect(config, target)
            .await
            .with_context(|| format!("sudoku: mux open to {target}"));
    }
    let mut stream = establish_session(config).await?;
    kip::write_open_tcp(&mut stream, target)
        .await
        .with_context(|| format!("sudoku: OpenTCP to {target}"))?;
    Ok(stream)
}

#[cfg(test)]
mod config_tests {
    use super::*;

    fn parse(yaml: &str) -> Result<SudokuOutboundConfig> {
        let entry: ProxyEntry = serde_yaml_ng::from_str(yaml).expect("valid proxy entry");
        SudokuOutboundConfig::from_proxy(&entry)
    }

    const BASE: &str = "name: s\ntype: sudoku\nserver: example.com\nport: 443\nkey: secret\n";

    #[test]
    fn absent_or_disabled_mask_leaves_no_masking() {
        assert!(parse(BASE).expect("base").http_mask.is_none());
        let disabled = parse(&format!("{BASE}httpmask:\n  disable: true\n")).expect("disabled");
        assert!(disabled.http_mask.is_none());
    }

    #[test]
    fn legacy_mask_is_enabled_with_defaults_and_overrides() {
        let default = parse(&format!("{BASE}httpmask:\n  disable: false\n")).expect("legacy");
        let mask = default.http_mask.expect("mask present");
        assert_eq!(mask.host, "example.com:443");
        assert_eq!(mask.path_root, "");

        let overridden = parse(&format!(
            "{BASE}httpmask:\n  mode: legacy\n  host: cdn.example.net\n  path-root: aabbcc\n"
        ))
        .expect("legacy override");
        let mask = overridden.http_mask.expect("mask present");
        assert_eq!(mask.host, "cdn.example.net");
        assert_eq!(mask.path_root, "aabbcc");
    }

    #[test]
    fn cdn_tunnel_modes_are_rejected() {
        for mode in ["stream", "poll", "auto", "ws"] {
            let err = parse(&format!("{BASE}httpmask:\n  mode: {mode}\n")).unwrap_err();
            assert!(err.to_string().contains("not supported yet"), "{mode}: {err}");
        }
    }

    #[test]
    fn unknown_mask_mode_is_rejected() {
        let err = parse(&format!("{BASE}httpmask:\n  mode: bogus\n")).unwrap_err();
        assert!(err.to_string().contains("unknown httpmask.mode"), "{err}");
    }
}

#[cfg(test)]
mod interop_tests;
