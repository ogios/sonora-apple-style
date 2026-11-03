use anyhow::{Context as _, Result, ensure};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};

/// The largest individual command, snapshot or page accepted from a local peer.
const MAX_FRAME: usize = 64 * 1024 * 1024;

/// Reads a length-prefixed JSON frame, rejecting oversized frames before allocating their body.
pub async fn read<T: DeserializeOwned>(stream: &mut (impl AsyncRead + Unpin)) -> Result<T> {
    let length = stream
        .read_u32()
        .await
        .context("cannot read the frame length")? as usize;
    ensure!(
        length <= MAX_FRAME,
        "the service frame exceeds the size limit"
    );
    let mut body = vec![0; length];
    stream
        .read_exact(&mut body)
        .await
        .context("cannot read the service frame")?;
    serde_json::from_slice(&body).context("cannot decode the service frame")
}

/// Writes one bounded JSON frame. A connection must serialize calls to this function.
pub async fn write<T: Serialize>(stream: &mut (impl AsyncWrite + Unpin), value: &T) -> Result<()> {
    let body = serde_json::to_vec(value).context("cannot encode the service frame")?;
    ensure!(
        body.len() <= MAX_FRAME,
        "the service frame exceeds the size limit"
    );
    stream
        .write_u32(body.len() as u32)
        .await
        .context("cannot write the frame length")?;
    stream
        .write_all(&body)
        .await
        .context("cannot write the service frame")?;
    stream
        .flush()
        .await
        .context("cannot flush the service frame")
}
