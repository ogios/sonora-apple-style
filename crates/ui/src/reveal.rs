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
//!
//! A place is remembered by its id alone rather than by where it sits in the element tree,
//! since a page often lays its skeletons out in a plain grid and its content in a virtual
//! list, and the two have to find each other. Ids are app-wide, so each names its page. A
//! place not drawn for a while is forgotten.

use std::cell::RefCell;
use std::collections::HashMap;
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

/// How long a place that is no longer drawn is remembered.
const FORGET: Duration = Duration::from_secs(2);

thread_local! {
    /// Every place drawn lately, with the phase it is in and when it was last drawn.
    static PLACES: RefCell<Places> = RefCell::new(Places::default());
}

#[derive(Default)]
struct Places {
    phases: HashMap<ElementId, (Phase, Instant)>,
    swept: Option<Instant>,
}

impl Places {
    /// The phase of `id`, `fresh` for a place not drawn lately, marked as drawn now.
    fn phase(&mut self, id: &ElementId, fresh: Phase, now: Instant) -> &mut Phase {
        if self
            .swept
            .is_none_or(|swept| now.duration_since(swept) > FORGET)
        {
            self.phases
                .retain(|_, (_, seen)| now.duration_since(*seen) <= FORGET);
            self.swept = Some(now);
        }
        let entry = self.phases.entry(id.clone()).or_insert((fresh, now));
        entry.1 = now;
        &mut entry.0
    }
}

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
    /// The content came, but none for this place, so its skeleton fades out on its own from
    /// the moment given: one past the end of a list shorter than the skeletons guessed.
    Leaving(Instant),
}

/// Marks `id` as showing its skeleton, for a place that draws the hand-over itself rather than
/// through [`Reveal`]: rows a list positions on its own, say.
pub fn waiting(id: &ElementId, cx: &App) {
    if animates(cx) {
        PLACES.with_borrow_mut(|places| {
            *places.phase(id, Phase::Waiting, Instant::now()) = Phase::Waiting;
        });
    }
}

/// How far `id`'s content has crossed in from its skeleton, from 0 to 1, or `None` once it
/// shows on its own or never had a skeleton to cross from. `order` staggers its start. While
/// it crosses, the next frame is asked for.
pub fn crossing(id: &ElementId, order: usize, window: &mut Window, cx: &App) -> Option<f32> {
    if !animates(cx) {
        return None;
    }
    let now = Instant::now();
    let span = paced(SPAN);
    let progress = PLACES.with_borrow_mut(|places| {
        let phase = places.phase(id, Phase::Shown, now);
        if let Phase::Waiting = phase {
            let delay = paced(STAGGER) * order.min(STAGGER_CAP) as u32;
            *phase = Phase::Revealing(now + delay);
        }
        let Phase::Revealing(start) = *phase else {
            // Content for a place whose skeleton already left shows at once.
            *phase = Phase::Shown;
            return None;
        };
        let elapsed = now.saturating_duration_since(start);
        let progress = elapsed.as_secs_f32() / span.as_secs_f32().max(f32::EPSILON);
        if progress >= 1. {
            *phase = Phase::Shown;
            return None;
        }
        Some(progress.max(0.))
    });
    if progress.is_some() {
        window.request_animation_frame();
    }
    progress
}

/// How much of `id`'s skeleton still shows, from 1 down to 0, once the content around it came
/// but none took its place, or `None` once it has gone or if it never showed. Draw the
/// skeleton at this opacity where it stood, so a list that turns out shorter than its
/// skeletons lets the rest fade rather than vanish. While it fades, the next frame is asked
/// for.
pub fn leaving(id: &ElementId, window: &mut Window, cx: &App) -> Option<f32> {
    if !animates(cx) {
        return None;
    }
    let now = Instant::now();
    let span = paced(SPAN).mul_f32(SKELETON_FADE);
    let shows = PLACES.with_borrow_mut(|places| {
        let (phase, seen) = places.phases.get_mut(id)?;
        *seen = now;
        if let Phase::Waiting = phase {
            *phase = Phase::Leaving(now);
        }
        let Phase::Leaving(start) = *phase else {
            return None;
        };
        let progress =
            now.duration_since(start).as_secs_f32() / span.as_secs_f32().max(f32::EPSILON);
        if progress >= 1. {
            places.phases.remove(id);
            return None;
        }
        Some(1. - ease_out_cubic(progress))
    });
    if shows.is_some() {
        window.request_animation_frame();
    }
    shows
}

/// Every skeleton still leaving among `ids`, taken in order until the first that is not, with
/// how much of it shows: the tail of a list or a grid past its last item.
pub fn leavers(
    ids: impl IntoIterator<Item = ElementId>,
    window: &mut Window,
    cx: &App,
) -> Vec<(ElementId, f32)> {
    ids.into_iter()
        .map_while(|id| leaving(&id, window, cx).map(|shows| (id, shows)))
        .collect()
}

/// Styles content `progress` of the way into its place: faded and a little low at 0.
pub fn arriving<E: Styled>(element: E, progress: f32) -> E {
    let shown = ease_out_cubic(progress.clamp(0., 1.));
    element
        .opacity(shown)
        .layer_translate(point(Pixels::ZERO, RISE * (1. - shown)))
}

/// How much of the skeleton still shows `progress` of the way through the hand-over.
pub fn lingering(progress: f32) -> f32 {
    1. - ease_out_cubic((progress / SKELETON_FADE).clamp(0., 1.))
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
    /// while the skeleton shows or fades. `id` names the place across the whole app, and has
    /// to be the same for the skeleton and the content that replaces it.
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
            // Remember that this place showed a skeleton, so the content that replaces it
            // crosses in rather than cutting.
            waiting(&id, cx);
            return base.child(skeleton(window, cx));
        };
        // The content always sits in one wrapper of the same shape, fading or not, so the
        // frame its reveal ends on lays out exactly like the one before.
        let holder = div().w_full();
        match crossing(&id, order, window, cx) {
            None => base.child(holder.child(content)),
            Some(progress) => base.child(arriving(holder, progress).child(content)).child(
                div()
                    .absolute()
                    .inset_0()
                    .overflow_hidden()
                    .opacity(lingering(progress))
                    .child(skeleton(window, cx)),
            ),
        }
    }
}
