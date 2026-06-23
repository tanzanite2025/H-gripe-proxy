use super::constants::IO_TIMEOUT_SECONDS;
use anyhow::{Result, bail};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{Duration, timeout},
};

pub(super) async fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut length = [0_u8; 4];
    timeout(Duration::from_secs(IO_TIMEOUT_SECONDS), stream.read_exact(&mut length)).await??;
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 || length > 4096 {
        bail!("encrypted protocol frame length is outside canary bounds");
    }
    let mut payload = vec![0_u8; length];
    timeout(Duration::from_secs(IO_TIMEOUT_SECONDS), stream.read_exact(&mut payload)).await??;
    Ok(payload)
}

pub(super) async fn write_frame(stream: &mut TcpStream, payload: &[u8]) -> Result<u64> {
    let length = u32::try_from(payload.len())?;
    timeout(
        Duration::from_secs(IO_TIMEOUT_SECONDS),
        stream.write_all(&length.to_be_bytes()),
    )
    .await??;
    timeout(Duration::from_secs(IO_TIMEOUT_SECONDS), stream.write_all(payload)).await??;
    Ok(u64::from(length) + 4)
}
