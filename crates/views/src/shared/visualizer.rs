use std::time::{Duration, Instant};

use gpui::Window;
use music::Spectrum;
use ui::Levels;

/// How long the drawn levels take to close 63% of the gap to the analyzer's. A time constant
/// rather than a share per frame, so the motion is the same on a 60 Hz and a 144 Hz display.
const SETTLE: Duration = Duration::from_millis(40);
/// A long gap means the view was away; adopt the new levels instead of easing from stale ones.
const STALL: Duration = Duration::from_millis(250);
/// Levels below this are visually silent and can stop requesting frames after a pause.
const SILENT: f32 = 0.001;

/// Smooths the spectrum while rendered. Frame demand ends when hidden or a paused tail settles.
#[derive(Default)]
pub struct VisualizerDrive {
    shown: Levels,
    beat: Option<Instant>,
}

impl VisualizerDrive {
    pub fn levels(&self) -> Levels {
        self.shown.clone()
    }

    /// Samples the current engine, or fades to silence while paused. Only a rendered view can
    /// request another frame, so leaving fullscreen cannot leave a background callback loop.
    pub fn show(&mut self, spectrum: Spectrum, playing: bool, window: &mut Window) {
        let now = Instant::now();
        let rate = match self.beat.replace(now) {
            Some(beat) if now.duration_since(beat) < STALL => {
                1. - (-now.duration_since(beat).as_secs_f32() / SETTLE.as_secs_f32()).exp()
            }
            _ => 1.,
        };
        match playing {
            true => {
                ease(&mut self.shown.left, spectrum.left(), rate);
                ease(&mut self.shown.right, spectrum.right(), rate);
                window.request_animation_frame();
            }
            false => {
                let mut settling = false;
                for level in self.shown.left.iter_mut().chain(&mut self.shown.right) {
                    *level *= 1. - rate;
                    match *level > SILENT {
                        true => settling = true,
                        false => *level = 0.,
                    }
                }
                if settling {
                    window.request_animation_frame();
                }
            }
        }
    }

    pub fn hide(&mut self) {
        self.beat = None;
        self.shown = Levels::default();
    }
}

/// Walks `shown` a share of the way towards `target`, adopting it outright when the band
/// count changes.
fn ease(shown: &mut Vec<f32>, target: Vec<f32>, rate: f32) {
    if shown.len() != target.len() {
        *shown = target;
        return;
    }
    for (shown, target) in shown.iter_mut().zip(&target) {
        *shown += (target - *shown) * rate;
    }
}
