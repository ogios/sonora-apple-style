use std::time::Instant;

use gpui::prelude::*;
use gpui::{AnyElement, App, ElementId, Window, div};

use crate::motion::{Motion, animates, css_ease};

/// The cover an element showed last and the one it is fading out from.
struct Crossing {
    shown: Option<String>,
    previous: Option<String>,
    since: Instant,
}

/// Draws `cover` through `draw`, and when it changes, as the track does, lays the cover it
/// replaces over it and fades that out, so one cover dissolves into the next rather than
/// cutting. `id` keeps the memory of the last cover, one per place a cover is shown.
pub fn crossfade(
    id: impl Into<ElementId>,
    cover: Option<String>,
    window: &mut Window,
    cx: &mut App,
    draw: impl Fn(Option<String>) -> AnyElement,
) -> AnyElement {
    if !animates(cx) {
        return draw(cover);
    }
    let state = window.use_keyed_state(id.into(), cx, |_, _| Crossing {
        shown: cover.clone(),
        previous: None,
        since: Instant::now(),
    });
    let span = Motion::Slow.span().as_secs_f32();
    let fading = state.update(cx, |state, _| {
        if state.shown != cover {
            state.previous = Some(std::mem::replace(&mut state.shown, cover.clone())).flatten();
            state.since = Instant::now();
        }
        let progress = (state.since.elapsed().as_secs_f32() / span).min(1.);
        if progress >= 1. {
            state.previous = None;
        }
        state.previous.clone().map(|previous| (previous, progress))
    });
    match fading {
        None => draw(cover),
        Some((previous, progress)) => {
            window.request_animation_frame();
            div()
                .relative()
                .child(draw(cover))
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .opacity(1. - css_ease(progress))
                        .child(draw(Some(previous))),
                )
                .into_any_element()
        }
    }
}
