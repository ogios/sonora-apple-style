use std::sync::Arc;
use std::time::Instant;

use gpui::{App, ElementId, RenderImage, Resource, SharedString, Window};

use crate::motion::{Motion, animates, ease_out_cubic};

/// A decoded cover, including whether it is the higher-resolution replacement.
#[derive(Clone)]
pub(crate) struct LoadedCover {
    pub key: (Resource, u32),
    pub image: Arc<RenderImage>,
    pub upgraded: bool,
}

/// The decoded pictures retained by one cover slot while its replacement loads and enters.
#[derive(Default)]
struct Crossing {
    cover: Option<SharedString>,
    shown: Option<LoadedCover>,
    previous: Option<LoadedCover>,
    since: Option<Instant>,
}

/// One frame of the cover's entrance. The previous picture stays opaque beneath the new one.
pub(crate) struct CoverFrame {
    pub shown: LoadedCover,
    pub previous: Option<LoadedCover>,
    pub progress: Option<f32>,
}

/// Keeps the last decoded picture until `ready` can replace it. Resolution upgrades share
/// the current entrance clock; consecutive replacements wait for that entrance to finish.
pub(crate) fn crossfade(
    id: ElementId,
    cover: SharedString,
    ready: Option<LoadedCover>,
    window: &mut Window,
    cx: &mut App,
) -> Option<CoverFrame> {
    let state = window.use_keyed_state(id, cx, |_, _| Crossing::default());
    let span = Motion::Slow.span().as_secs_f32();
    let moving = animates(cx);
    let frame = state.update(cx, |state, _| {
        let now = Instant::now();
        if !moving
            || state
                .since
                .is_some_and(|since| since.elapsed().as_secs_f32() >= span)
        {
            state.previous = None;
            state.since = None;
        }
        if let Some(ready) = ready {
            if state.cover.as_ref() == Some(&cover) {
                if ready.upgraded || !state.shown.as_ref().is_some_and(|shown| shown.upgraded) {
                    state.shown = Some(ready);
                }
            } else if state.since.is_none() {
                state.previous = state.shown.take();
                state.shown = Some(ready);
                state.cover = Some(cover);
                state.since = moving.then_some(now);
                if !moving {
                    state.previous = None;
                }
            }
        }
        Some(CoverFrame {
            shown: state.shown.as_ref()?.clone(),
            previous: state.previous.clone(),
            progress: state
                .since
                .map(|since| ease_out_cubic((since.elapsed().as_secs_f32() / span).min(1.))),
        })
    });
    if frame.as_ref().is_some_and(|frame| frame.progress.is_some()) {
        window.on_next_frame(|window, _| window.refresh());
    }
    frame
}
