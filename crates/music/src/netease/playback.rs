//! Playback for NetEase Cloud Music: what [`crate::engine`] needs that is NetEase's own.
//!
//! Very little. NetEase hands out a plain MP3 or FLAC url for each song the account may hear in
//! full, so the stream takes the bytes as they come and rodio decodes them. The threads, the
//! queue, the preload and the gapless join are the engine's. Nothing is kept on disk: a song
//! is fetched again every time it plays.

use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;

use crate::engine::{self, Fetch};
use crate::netease::client::NetEaseClient;
use crate::stream::{Plain, Reader, Stream};
use crate::{Encoding, PlaybackConfig, PlaybackEvents, PlaybackFactory, Player, Quality};

/// A song downloading, and how NetEase says it is encoded.
#[derive(Clone)]
pub struct Loaded {
    stream: Stream,
    encoding: Option<Encoding>,
}

pub struct Factory {
    client: NetEaseClient,
}

impl Factory {
    pub fn new(client: NetEaseClient) -> Self {
        Self { client }
    }
}

impl PlaybackFactory for Factory {
    fn start(&self, config: PlaybackConfig) -> (Box<dyn Player>, Box<dyn PlaybackEvents>) {
        engine::start(
            NetEase {
                client: self.client.clone(),
                quality: config.quality,
            },
            config,
        )
    }
}

struct NetEase {
    client: NetEaseClient,
    /// The level every song is asked for at.
    quality: Quality,
}

#[async_trait]
impl Fetch for NetEase {
    type Loaded = Loaded;
    type Source = rodio::Decoder<Reader>;

    fn name(&self) -> &'static str {
        "netease"
    }

    /// Asks for the song's stream at the chosen level, or the best below it the account may
    /// hear, and starts downloading it. A song the account may only preview, or not hear at
    /// all, fails here and plays as unavailable.
    async fn load(&self, id: &str) -> Result<Loaded> {
        let resolved = self.client.resolve(id, self.quality).await?;
        log::debug!("netease: {id} streams at {} kbps", resolved.bitrate / 1000);
        let response = self.client.open(&resolved).await?;
        let total = response.content_length();
        let stream = Stream::pulling(response, total, Plain).primed().await?;
        Ok(Loaded {
            stream,
            encoding: resolved.encoding,
        })
    }

    fn encoding(&self, loaded: &Loaded) -> Option<Encoding> {
        loaded.encoding
    }

    fn gated(&self, error: &anyhow::Error) -> bool {
        NetEaseClient::signed_out(error)
    }

    async fn downloaded(&self, loaded: &Loaded) {
        loaded.stream.finished().await;
    }

    fn progress(&self, loaded: &Loaded) -> Option<f32> {
        let total = loaded.stream.total().filter(|total| *total > 0)?;
        Some(loaded.stream.arrived() as f32 / total as f32)
    }

    /// Builds a decoder over a stream and places it at `at`. The bytes past the preroll are
    /// still arriving, so this only reads the header.
    fn open(&self, id: &str, loaded: &Loaded, at: Duration) -> Option<Self::Source> {
        let total = loaded.stream.total();
        let mut builder = rodio::Decoder::builder()
            .with_data(loaded.stream.reader())
            .with_seekable(total.is_some());
        if let Some(total) = total {
            builder = builder.with_byte_len(total);
        }
        let mut decoder = match builder.build() {
            Ok(decoder) => decoder,
            Err(error) => {
                log::warn!("playback: cannot decode the netease track {id}: {error}");
                return None;
            }
        };
        if !at.is_zero()
            && let Err(error) = rodio::Source::try_seek(&mut decoder, at)
        {
            log::warn!(
                "playback: cannot start the netease track {id} at {}s: {error}",
                at.as_secs()
            );
        }
        Some(decoder)
    }
}
