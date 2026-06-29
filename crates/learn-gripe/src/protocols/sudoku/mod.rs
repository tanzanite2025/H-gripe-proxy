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
//! ## Scope (TCP-only baseline)
//!
//! This module implements the common single-table, pure-uplink / pure-downlink
//! TCP case: HTTP masking disabled, the 6-bit *packed* downlink, multiplexing,
//! UDP-over-TCP and the reverse channel are intentionally left to follow-up
//! work and are rejected up front rather than mis-handled.

mod grid;
mod kip;
mod layout;
mod obfs;
mod record;
mod rng;
mod rng_cooked;
mod table;

use anyhow::{Context, Result, bail};

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::outbound::BoxedStream;
use crate::transport::{self, Security, Transport};

use self::record::AeadMethod;

/// Parsed configuration for a Sudoku outbound (TCP-only baseline).
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
}

impl SudokuOutboundConfig {
    /// Build an outbound config from a parsed `sudoku` proxy entry.
    ///
    /// Only the TCP-only baseline is accepted: HTTP masking must be disabled and
    /// the pure (one-byte → four-hint) downlink must be selected. The packed
    /// downlink, MUX, UoT and the reverse channel are deferred to follow-up work
    /// and are rejected here rather than silently mis-handled.
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

        // The kernel implements the pure downlink only; the 6-bit packed downlink
        // is out of scope for this baseline.
        if opts.enable_pure_downlink != Some(true) {
            bail!("sudoku: only the pure downlink is supported in this build; set enable-pure-downlink: true");
        }

        // HTTP masking (legacy header and CDN tunnel) is not implemented; only an
        // explicitly disabled mask is accepted.
        if let Some(mask) = &opts.httpmask
            && mask.disable != Some(true)
        {
            bail!("sudoku: HTTP masking is not supported yet; set httpmask.disable: true");
        }

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
        })
    }
}

/// Connect through the Sudoku server to `target` and return a relay-ready
/// stream. Dials TCP, layers the obfuscation + AEAD record stacks, runs the KIP
/// X25519 handshake (rekeying the record layer with the derived session keys),
/// writes the `OpenTCP` request, and hands back a transparent stream.
pub async fn connect(config: &SudokuOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let tables = table::new_directional_table(&config.key, &config.table_type, &config.custom_pattern)
        .context("sudoku: build obfuscation table")?;
    let table::DirectionalTable { uplink, downlink } = tables;

    let inner = transport::establish(&config.server, config.port, &Security::None, &Transport::Tcp)
        .await
        .context("sudoku: dial server")?;

    // Outermost on-wire layer: expand record bytes into Sudoku hint bytes.
    let obfs = obfs::ObfsStream::new(
        inner,
        uplink,
        downlink,
        config.padding_min as i32,
        config.padding_max as i32,
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

    kip::write_open_tcp(&mut record, target)
        .await
        .with_context(|| format!("sudoku: OpenTCP to {target}"))?;

    Ok(Box::new(record))
}

#[cfg(test)]
mod interop_tests;
