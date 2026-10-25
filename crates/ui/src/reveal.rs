//! The hand-over from a skeleton to what it stood in for. Where a placeholder shows while
//! something loads, [`Reveal`] draws it, and once the content is there crossfades the two in
//! place: the skeleton fades out over the content as the content fades in and settles up a
//! few pixels into its place. A list or a grid hands each of its items over on its own, a
//! little after the one before it, so the content washes in across them.
//!
//! The reveal happens once, as the content replaces the skeleton on screen. Content that is
//! already there the first frame a place is drawn, from a cache or as a row scrolls back into
//! view, shows at once, and so does everything while motion is reduced. The content sits in
//! the same wrapper in every phase, so the reveal never moves the layout around it.

use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::{AnyElement, App, Div, ElementId, Pixels, StyleRefinement, Window, div, point, px};

use crate::motion::{animates, ease_out_cubic, paced};

/// How long one item takes to cross from its skeleton to its content.
const SPAN: Duration = Duration::from_millis(280);
/// How much later each item in a list or a grid starts than the one before it.
const STAGGER: Duration = Duration::from_millis(20);
/// How many items the stagger counts before the rest start together, so a long list arrives
/// as quickly as a short one: the last item is in place by `SPAN + STAGGER * STAGGER_CAP`.
const STAGGER_CAP: usize = 6;
/// How far below its place the content starts as it fades in.
const RISE: Pixels = px(6.);
/// How much of the span the skeleton takes to fade out, so it has mostly gone by the time
/// the content is half in and the two never read as one muddy layer.
const SKELETON_FADE: f32 = 0.6;

/// Where a place stands in the hand-over.
#[derive(Clone, Copy)]
enum Phase {
    /// The skeleton shows.
    Waiting,
    /// The content arrived, and crosses in from the moment given, which the stagger may put a
    /// little ahead.
    Revealing(Instant),
    /// The content shows on its own.
    Shown,
}

/// A skeleton that hands over to its content. Style it as the item it stands for, so it
/// takes the same place in its parent's layout, the way `w_full` or `flex_1` would on the
/// item itself.
#[derive(IntoElement)]
pub struct Reveal {
    id: ElementId,
    base: Div,
    order: usize,
    content: Option<AnyElement>,
    skeleton: Box<dyn FnOnce(&mut Window, &mut App) -> AnyElement>,
}

impl Reveal {
    /// `content` is `None` while it is on its way. `skeleton` draws the placeholder, only
    /// while the skeleton shows or fades. `id` names the place, unique among its siblings,
    /// and has to be the same for the skeleton and the content that replaces it.
    pub fn new(
        id: impl Into<ElementId>,
        content: Option<AnyElement>,
        skeleton: impl FnOnce(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            base: div(),
            order: 0,
            content,
            skeleton: Box::new(skeleton),
        }
    }

    /// Where the item stands in its list or grid, which staggers its start.
    pub fn order(mut self, order: usize) -> Self {
        self.order = order;
        self
    }
}

impl Styled for Reveal {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Reveal {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            id,
            base,
            order,
            content,
            skeleton,
        } = self;
        let base = base.relative();
        let Some(content) = content else {
            if animates(cx) {
                // Remember that this place showed a skeleton, so the content that replaces it
                // crosses in rather than cutting.
                let state = window.use_keyed_state(id, cx, |_, _| Phase::Waiting);
                state.update(cx, |phase, _| *phase = Phase::Waiting);
            }
            return base.child(skeleton(window, cx));
        };
        // The content always sits in one wrapper of the same shape, fading or not, so the
        // frame its reveal ends on lays out exactly like the one before.
        let holder = || div().w_full();
        if !animates(cx) {
            return base.child(holder().child(content));
        }

        let state = window.use_keyed_state(id, cx, |_, _| Phase::Shown);
        let span = paced(SPAN);
        let progress = state.update(cx, |phase, _| {
            if let Phase::Waiting = phase {
                let delay = paced(STAGGER) * order.min(STAGGER_CAP) as u32;
                *phase = Phase::Revealing(Instant::now() + delay);
            }
            let Phase::Revealing(start) = *phase else {
                return None;
            };
            let elapsed = Instant::now().saturating_duration_since(start);
            let progress = elapsed.as_secs_f32() / span.as_secs_f32().max(f32::EPSILON);
            if progress >= 1. {
                *phase = Phase::Shown;
                return None;
            }
            Some(progress)
        });
        let Some(progress) = progress else {
            return base.child(holder().child(content));
        };

        window.request_animation_frame();
        let shown = ease_out_cubic(progress.clamp(0., 1.));
        let fading = ease_out_cubic((progress / SKELETON_FADE).clamp(0., 1.));
        base.child(
            holder()
                .opacity(shown)
                .layer_translate(point(Pixels::ZERO, RISE * (1. - shown)))
                .child(content),
        )
        .child(
            div()
                .absolute()
                .inset_0()
                .overflow_hidden()
                .opacity(1. - fading)
                .child(skeleton(window, cx)),
        )
    }
}
