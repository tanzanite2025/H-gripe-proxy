//! Sudoku UDP-over-TCP ([`SudokuUdpAssoc`]).
//!
//! UoT reuses the exact same base tunnel as the TCP data plane (obfuscation +
//! AEAD record + KIP handshake, see [`super::establish_session`]); the only
//! difference is the control message written after the handshake: a single
//! empty `StartUoT` (`0x12`) request instead of `OpenTCP`. From then on the
//! stream carries UDP datagrams, one per frame:
//!
//! ```text
//! addr_len(u16 BE) | payload_len(u16 BE) | address | payload
//! ```
//!
//! `address` is the SOCKS5-style encoding shared with `OpenTCP`
//! ([`kip::encode_address`]). Client → server frames name the datagram's
//! destination; server → client frames name its source, which the egress
//! discards (the association already knows the target). One frame maps to
//! exactly one datagram, so packet boundaries survive the reliable stream.
//!
//! The stream is split so `send` and `recv` can run concurrently in the egress
//! `select!`, mirroring the other UDP-over-TCP associations (e.g. Snell).

use anyhow::{Context, Result, bail};
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::sync::Mutex;

use crate::address::TargetAddr;
use crate::outbound::BoxedStream;

use super::kip::{self, KIP_TYPE_START_UOT};
use super::{SudokuOutboundConfig, establish_session};

/// Upper bound on a UoT frame's address / payload field. The u16 length header
/// caps each at 65535 bytes, which comfortably covers any UDP datagram.
const MAX_UOT_LEN: usize = u16::MAX as usize;

/// A Sudoku UDP-over-TCP association (one per destination, matching the other
/// UDP egresses' `connect` / `send` / `recv` shape).
pub struct SudokuUdpAssoc {
    /// The fixed destination named in every datagram sent on this association.
    target: TargetAddr,
    write: Mutex<WriteHalf<BoxedStream>>,
    read: Mutex<ReadHalf<BoxedStream>>,
}

impl SudokuUdpAssoc {
    /// Open a UoT association to `config.server` for datagrams destined to
    /// `target`: bring up the shared session and send the `StartUoT` preface.
    pub async fn connect(config: &SudokuOutboundConfig, target: &TargetAddr) -> Result<Self> {
        let mut stream = establish_session(config).await?;
        kip::write_message(&mut stream, KIP_TYPE_START_UOT, &[])
            .await
            .context("sudoku uot: write StartUoT")?;
        stream.flush().await.context("sudoku uot: flush StartUoT")?;

        let (reader, writer) = tokio::io::split(stream);
        Ok(Self {
            target: target.clone(),
            write: Mutex::new(writer),
            read: Mutex::new(reader),
        })
    }

    /// Frame `payload` as one UoT datagram to `self.target` and write it.
    pub async fn send(&self, payload: &[u8]) -> Result<()> {
        let addr = kip::encode_address(&self.target)?;
        if addr.len() > MAX_UOT_LEN {
            bail!("sudoku uot: address too long: {}", addr.len());
        }
        if payload.len() > MAX_UOT_LEN {
            bail!("sudoku uot: payload too large: {}", payload.len());
        }
        let mut frame = Vec::with_capacity(4 + addr.len() + payload.len());
        frame.extend_from_slice(&(addr.len() as u16).to_be_bytes());
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        frame.extend_from_slice(&addr);
        frame.extend_from_slice(payload);

        let mut w = self.write.lock().await;
        w.write_all(&frame).await.context("sudoku uot: write datagram")?;
        w.flush().await.context("sudoku uot: flush datagram")?;
        Ok(())
    }

    /// Read one reply UoT datagram, discard its source address, and return the
    /// application payload.
    pub async fn recv(&self) -> Result<Vec<u8>> {
        let mut r = self.read.lock().await;
        let mut header = [0u8; 4];
        r.read_exact(&mut header).await.context("sudoku uot: read header")?;
        let addr_len = u16::from_be_bytes([header[0], header[1]]) as usize;
        let payload_len = u16::from_be_bytes([header[2], header[3]]) as usize;
        if addr_len == 0 {
            bail!("sudoku uot: empty address in reply");
        }

        // The reply carries the datagram's source address; the egress already
        // knows the target, so read the address bytes and discard them.
        let mut addr = vec![0u8; addr_len];
        r.read_exact(&mut addr).await.context("sudoku uot: read address")?;
        let mut payload = vec![0u8; payload_len];
        r.read_exact(&mut payload).await.context("sudoku uot: read payload")?;
        Ok(payload)
    }
}
