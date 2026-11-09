//! What the equalizer screen shows of the sound on its way out: the spectrum of the music ahead
//! of the equalizer, the signal path from the track to the device, and the output meter. Nothing
//! is analysed or read unless a screen holds a watch.

use std::sync::{Arc, Weak};
use std::time::Duration;

#[cfg(sonora_gui)]
use crate::runtime::prelude::*;
use crate::runtime::{Context, Entity, Subscription, Task};
use music::output::{DeviceInfo, Sample};
use music::{Level, Normalisation, TrackSignal};

use crate::{Equalizer, Outputs, Playback, PlaybackState};

/// How often the spectrum is read while watched, about sixty times a second. The analysis moves
/// on every 32nd of its window, about 11 ms at 48 kHz, so every read has something new.
const SPECTRUM_TICK: Duration = Duration::from_millis(16);
/// How many spectrum ticks pass between two reads of the output meter: every one, so the meter
/// moves as smoothly as the spectrum.
const METER_EVERY: u32 = 1;
/// How many bits of a sample the 32-bit floats Sonora works in carry.
pub const FLOAT_BITS: u8 = 24;

/// The spectrum heard ahead of the equalizer, the signal path and the output meter, read while a
/// screen holds the one watch `watch` returns.
pub struct Signal {
    playback: Entity<Playback>,
    outputs: Entity<Outputs>,
    equalizer: Entity<Equalizer>,
    /// The latest levels, `None` while nothing is loaded or before a window has filled.
    levels: Option<Vec<Level>>,
    path: Option<SignalPath>,
    /// Alive while a watch task or a `Following` is held, so the path is read only then. Every
    /// holder shares the one `Arc`, so the path stops being read only once the last lets go.
    watched: Weak<()>,
    _subscriptions: Vec<Subscription>,
    #[cfg(sonora_gui)]
    spectrum: music::Spectrum,
    #[cfg(sonora_gui)]
    spectrum_ready: bool,
}

/// Keeps the signal path read for as long as it is held, without the spectrum or the meter.
pub struct Following {
    _held: Arc<()>,
    _remote: Option<crate::client::Lease>,
}

/// What happens to the sound on its way out, in order.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SignalPath {
    /// What the engine knows about the track playing: how it is encoded, what it decodes to and
    /// how it is normalised. `None` until the engine has opened it.
    pub track: Option<TrackSignal>,
    /// The equalizer, while it is on.
    pub equalizer: Option<Equalizing>,
    /// The volume as the slider shows it, from 0 to 1.
    pub level: f32,
    /// The gain the volume applies, 1 at full volume.
    pub volume: f32,
    /// The stream Sonora plays into, which every track is resampled to, with what the sound
    /// server says of the sink it plays on. `None` while none is open.
    pub stream: Option<DeviceInfo>,
}

/// What the equalizer does while on.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Equalizing {
    pub preamp_db: f64,
    /// Whether its filters leave every sample untouched at the rate that plays.
    pub passes_through: bool,
}

/// Something Sonora does that changes the samples it decoded.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Change {
    /// The track carries more bits than the floats Sonora works in, so they are rounded.
    Rounded {
        from: u8,
    },
    /// A fixed normalisation gain, in decibels.
    Normalised(f32),
    /// librespot's normaliser, whose gain follows the music.
    Limited,
    /// Sonora's resampler, for a track at another rate than the stream.
    Resampled {
        from: u32,
        to: u32,
    },
    Equalized,
    /// The volume's gain as a factor.
    Volume(f32),
    Remixed {
        from: u16,
        to: u16,
    },
    /// The device takes fewer bits than the track carries, or integers for a track that has
    /// no bit depth of its own. `from` is the bits Sonora carries, `None` for such a track.
    Requantized {
        from: Option<u8>,
        to: u8,
    },
}

impl SignalPath {
    /// Everything that keeps what leaves Sonora from being the decoded track bit for bit, in
    /// the order it happens. Empty means bit-perfect from Sonora.
    pub fn changes(&self) -> Vec<Change> {
        let mut changes = Vec::new();
        let bits = self
            .track
            .as_ref()
            .and_then(|track| track.encoding?.depth());
        if let Some(from) = bits.filter(|bits| *bits > FLOAT_BITS) {
            changes.push(Change::Rounded { from });
        }
        match self.track.as_ref().map(|track| track.normalisation) {
            Some(Normalisation::Gain(db)) if db != 0. => changes.push(Change::Normalised(db)),
            Some(Normalisation::Limited) => changes.push(Change::Limited),
            _ => {}
        }
        if let Some((from, to)) = self.resampled() {
            changes.push(Change::Resampled { from, to });
        }
        if self
            .equalizer
            .is_some_and(|eq| eq.preamp_db != 0. || !eq.passes_through)
        {
            changes.push(Change::Equalized);
        }
        if self.volume != 1. {
            changes.push(Change::Volume(self.volume));
        }
        if let (Some(track), Some(stream)) = (&self.track, &self.stream) {
            if track.channels != stream.format.channels {
                changes.push(Change::Remixed {
                    from: track.channels,
                    to: stream.format.channels,
                });
            }
            if let Sample::Integer(to) = stream.format.sample
                && bits.is_none_or(|bits| bits.min(FLOAT_BITS) > to)
            {
                changes.push(Change::Requantized {
                    from: bits.map(|bits| bits.min(FLOAT_BITS)),
                    to,
                });
            }
        }
        changes
    }

    /// The track's rate and the stream's, when Sonora resamples one to the other.
    pub fn resampled(&self) -> Option<(u32, u32)> {
        let (from, to) = (self.track.as_ref()?.rate, self.stream.as_ref()?.format.rate);
        (from != to).then_some((from, to))
    }

    /// The rate the sound server runs the sink at when it differs from the stream's, so it
    /// resamples again. `None` when it is the same or nothing says.
    pub fn server_rate(&self) -> Option<u32> {
        let stream = self.stream.as_ref()?;
        let sink = stream.sink.as_ref()?.format?;
        (sink.rate != stream.format.rate).then_some(sink.rate)
    }
}

impl crate::client::Model for Signal {
    #[cfg(sonora_gui)]
    const KIND: &'static str = "signal";
    #[cfg(sonora_gui)]
    const DYNAMIC: bool = false;

    #[cfg(not(sonora_gui))]
    fn remote_snapshot(&self, cx: &crate::runtime::App) -> serde_json::Value {
        let spectrum = self
            .playback
            .read(cx)
            .spectrum(cx)
            .map(|spectrum| (spectrum.left(), spectrum.right()));
        serde_json::json!({"levels": self.levels, "path": self.path, "spectrum": spectrum})
    }

    #[cfg(sonora_gui)]
    fn apply_remote(
        &mut self,
        data: &serde_json::Value,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        let levels = serde_json::from_value(data["levels"].clone())?;
        let path = serde_json::from_value(data["path"].clone())?;
        let spectrum: Option<(Vec<f32>, Vec<f32>)> =
            serde_json::from_value(data["spectrum"].clone())?;
        self.levels = levels;
        self.path = path;
        self.spectrum_ready = spectrum.is_some();
        if let Some((left, right)) = spectrum {
            self.spectrum.publish(&left, &right);
        }
        cx.notify();
        Ok(())
    }
}

impl Signal {
    /// Reads the signal path again whenever playback, the outputs or the equalizer change while
    /// a watch is held.
    pub fn new(
        playback: Entity<Playback>,
        outputs: Entity<Outputs>,
        equalizer: Entity<Equalizer>,
        cx: &mut Context<Self>,
    ) -> Self {
        #[cfg(sonora_gui)]
        if crate::client::register::<Self>(cx) {
            return Self {
                playback,
                outputs,
                equalizer,
                levels: None,
                path: None,
                watched: Weak::new(),
                _subscriptions: Vec::new(),
                #[cfg(sonora_gui)]
                spectrum: music::Spectrum::new(),
                #[cfg(sonora_gui)]
                spectrum_ready: false,
            };
        }
        let subscriptions = [
            cx.observe(&playback, |this, _, cx| this.follow_path(cx)),
            cx.observe(&outputs, |this, _, cx| this.follow_path(cx)),
            cx.observe(&equalizer, |this, _, cx| this.follow_path(cx)),
        ];
        Self {
            playback,
            outputs,
            equalizer,
            levels: None,
            path: None,
            watched: Weak::new(),
            _subscriptions: subscriptions.into(),
            #[cfg(sonora_gui)]
            spectrum: music::Spectrum::new(),
            #[cfg(sonora_gui)]
            spectrum_ready: false,
        }
    }

    /// The signal path as last read, while a screen watches it and a track is loaded.
    pub fn path(&self) -> Option<&SignalPath> {
        self.path.as_ref()
    }

    /// The spectrum ahead of the equalizer, in dBFS, while a track is loaded and watched.
    pub fn levels(&self) -> Option<&[Level]> {
        self.levels.as_deref()
    }

    /// Analyses the music ahead of the equalizer, reads the output meter and follows the signal
    /// path for as long as the returned task is held. The spectrum and the meter start afresh,
    /// and everything stops with the task.
    pub fn watch(&mut self, cx: &mut Context<Self>) -> Task<()> {
        if crate::client::is_remote(cx) {
            let lease = crate::client::lease("watch", cx);
            return cx.spawn(async move |_, _| {
                let _lease = lease;
                std::future::pending::<()>().await;
            });
        }
        let watch = Arc::new(self.playback.read(cx).analyzer().watch());
        let held = self.hold();
        self.levels = None;
        self.equalizer
            .update(cx, |equalizer, cx| equalizer.reset_meter(cx));
        self.read_path(cx);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let _held = held;
            for tick in (1..=METER_EVERY).cycle() {
                cx.background_executor().timer(SPECTRUM_TICK).await;
                let reading = watch.clone();
                let levels = cx.background_spawn(async move { reading.read() }).await;
                let updated = this.update(cx, |this, cx| {
                    this.show(levels, cx);
                    if tick == METER_EVERY {
                        this.equalizer
                            .update(cx, |equalizer, cx| equalizer.read_meter(cx));
                    }
                });
                if updated.is_err() {
                    break;
                }
            }
        })
    }

    /// Follows the signal path alone for as long as the returned guard is held, for a screen that
    /// shows where the sound goes but not how loud it is.
    pub fn follow(&mut self, cx: &mut Context<Self>) -> Following {
        if crate::client::is_remote(cx) {
            return Following {
                _held: Arc::new(()),
                _remote: crate::client::lease("follow", cx),
            };
        }
        let held = self.hold();
        self.read_path(cx);
        Following {
            _held: held,
            _remote: None,
        }
    }

    /// Holds the server's output spectrum sampling only while the visualizer is visible.
    pub fn watch_visualizer(&mut self, cx: &mut Context<Self>) -> Task<()> {
        if !crate::client::is_remote(cx) {
            return Task::ready(());
        }
        let lease = crate::client::lease("visualizer", cx);
        cx.spawn(async move |_, _| {
            let _lease = lease;
            std::future::pending::<()>().await;
        })
    }

    /// The latest output spectrum mirrored from the server.
    #[cfg(sonora_gui)]
    pub(crate) fn spectrum(&self) -> Option<music::Spectrum> {
        self.spectrum_ready.then(|| self.spectrum.clone())
    }

    /// The one `Arc` every watch and follower holds, made afresh when none is held.
    fn hold(&mut self) -> Arc<()> {
        self.watched.upgrade().unwrap_or_else(|| {
            let held = Arc::new(());
            self.watched = Arc::downgrade(&held);
            held
        })
    }

    /// Reads the signal path after a change, while a watch is held.
    fn follow_path(&mut self, cx: &mut Context<Self>) {
        if self.watched.strong_count() > 0 {
            self.read_path(cx);
        }
    }

    fn read_path(&mut self, cx: &mut Context<Self>) {
        let playback = self.playback.read(cx);
        let outputs = self.outputs.read(cx);
        let eq = self.equalizer.read(cx);
        let path = playback.track_signal().map(|track| {
            let stream = outputs.current().cloned();
            let equalizer = eq.enabled().then(|| Equalizing {
                preamp_db: eq.preamp_db(),
                passes_through: stream
                    .as_ref()
                    .is_some_and(|stream| eq.played().passes_through(stream.format.rate)),
            });
            SignalPath {
                track: track.cloned(),
                equalizer,
                level: playback.volume(),
                volume: playback.volume_gain(),
                stream,
            }
        });
        if path != self.path {
            self.path = path;
            cx.notify();
        }
    }

    /// Takes in a read, cleared while nothing is loaded.
    fn show(&mut self, levels: Option<Vec<Level>>, cx: &mut Context<Self>) {
        let loaded = matches!(
            self.playback.read(cx).state(),
            PlaybackState::Playing | PlaybackState::Paused | PlaybackState::Loading
        );
        let levels = levels.filter(|_| loaded);
        if levels != self.levels {
            self.levels = levels;
            cx.notify();
        }
    }
}
