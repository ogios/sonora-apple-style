use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{App, Bounds, Div, Pixels, Window, canvas, div, px};

use crate::glass::{liquid_pill, liquid_radius};
use crate::motion::{HoverFade, Motion, Springs, Sprung};

/// How close to its target, in pixels, an edge of the lens counts as arrived.
const EPSILON: f32 = 0.25;

/// Where an element was laid out on the last frame, in window coordinates. The render that
/// reads it and the `mark` canvas that records it share one cell.
#[derive(Clone, Default)]
pub struct Mark(Rc<Cell<Option<Bounds<Pixels>>>>);

impl Mark {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self) -> Option<Bounds<Pixels>> {
        self.0.get()
    }

    /// Forgets the bounds, for an entry that is no longer drawn.
    pub fn clear(&self) {
        self.0.set(None);
    }
}

/// An invisible element that fills its `relative` parent and records the parent's bounds into
/// `mark` each time it is laid out. A change asks for one more frame, so whatever was drawn
/// from the old bounds catches up on the next.
pub fn mark(mark: &Mark) -> impl IntoElement {
    let slot = mark.0.clone();
    canvas(
        move |bounds, window, _| {
            if slot.replace(Some(bounds)) != Some(bounds) {
                window.request_animation_frame();
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .inset_0()
}

/// The glass lens that marks the selected entry of a list or a segmented bar and slides from
/// one entry to the next. Each edge rides its own spring: the edge facing the new entry leads
/// on a quick one and the far edge trails on a springier one, so the lens stretches toward
/// where it is going, travels long, and settles with one soft wobble as the far edge lands.
///
/// Keep one per list in the view, `follow` the selected entry's `Mark` and the list's own,
/// and draw what `render` returns after the entries, so their glyphs pass through the lens.
/// It redraws only while its edges or visibility are changing.
pub struct Lens {
    left: Sprung,
    top: Sprung,
    right: Sprung,
    bottom: Sprung,
    shown: bool,
    /// Visibility fades independently of geometry, retaining the last position while hidden.
    visible: bool,
    visibility: HoverFade,
    corner: Corner,
}

/// The corners a lens takes, so it matches the shape of what it slides inside.
#[derive(Clone, Copy, Default, PartialEq)]
pub enum Corner {
    /// As round as a capsule of the lens's height allows, up to the panel radius: the
    /// sidebar's entries.
    #[default]
    Panel,
    /// Always a capsule, inside a capsule track.
    Capsule,
    /// This radius, concentric with a rounded-rectangle track.
    Fixed(Pixels),
}

impl Default for Lens {
    fn default() -> Self {
        let edge = || Sprung::new(0., Springs::LENS_LEAD, EPSILON);
        Self {
            left: edge(),
            top: edge(),
            right: edge(),
            bottom: edge(),
            shown: false,
            visible: false,
            visibility: HoverFade::new(Motion::Base.span()),
            corner: Corner::default(),
        }
    }
}

impl Lens {
    /// Aims the lens at `entry` as laid out inside `frame`, the element the lens is drawn in.
    /// Without either it hides, and the next entry it is sent to it lands on at once.
    pub fn follow(&mut self, entry: &Mark, frame: &Mark) {
        match (entry.get(), frame.get()) {
            (Some(entry), Some(frame)) => self.aim(Bounds {
                origin: entry.origin - frame.origin,
                size: entry.size,
            }),
            _ => self.hide(),
        }
    }

    /// Sends the lens to `target`, given relative to the element it is drawn in. The first
    /// placement lands at once; every later one stretches and slides.
    pub fn aim(&mut self, target: Bounds<Pixels>) {
        self.visible = true;
        self.visibility.set(true);
        let [left, top, right, bottom] =
            [target.left(), target.top(), target.right(), target.bottom()]
                .map(|edge| edge / px(1.));
        if !self.shown {
            self.left.snap(left);
            self.top.snap(top);
            self.right.snap(right);
            self.bottom.snap(bottom);
            self.shown = true;
            return;
        }
        // The lens is aimed on every frame, so an axis is planned again only when its target
        // moved, or the trailing edge would lose its slower spring halfway.
        if (top, bottom) != (self.top.target(), self.bottom.target()) {
            let (near, far) = lead(top, self.top.target());
            self.top.aim(top, near);
            self.bottom.aim(bottom, far);
        }
        if (left, right) != (self.left.target(), self.right.target()) {
            let (near, far) = lead(left, self.left.target());
            self.left.aim(left, near);
            self.right.aim(right, far);
        }
    }

    /// Lands the lens on its targets at once, for an entry carried along by its list, as when
    /// a fold above it opens, rather than one the selection moved to.
    pub fn ride(&mut self) {
        for edge in [
            &mut self.left,
            &mut self.top,
            &mut self.right,
            &mut self.bottom,
        ] {
            let target = edge.target();
            edge.snap(target);
        }
    }

    /// Whether the selection has reached its target, before a layout change carries it along.
    pub fn resting(&self) -> bool {
        self.left.resting() && self.top.resting() && self.right.resting() && self.bottom.resting()
    }

    /// Sets the corners the lens is drawn with.
    pub fn corner(&mut self, corner: Corner) {
        self.corner = corner;
    }

    /// Fades the lens away while keeping its geometry for a continuous return.
    pub fn hide(&mut self) {
        self.visible = false;
        self.visibility.set(false);
    }

    /// The lens as it stands this frame, placed absolutely in its parent, or nothing while
    /// hidden. Its corners stay as round as a capsule of the entry's height allows, up to the
    /// panel radius, so a stretched lens reads as one drop of glass.
    pub fn render(&mut self, window: &mut Window, cx: &App) -> Option<Div> {
        if !self.shown {
            return None;
        }
        let opacity = self.visibility.level(window, cx);
        if opacity <= 0. && !self.visible {
            return None;
        }
        let left = self.left.value(window, cx);
        let top = self.top.value(window, cx);
        let right = self.right.value(window, cx).max(left);
        let bottom = self.bottom.value(window, cx).max(top);
        let height = px(bottom - top);
        let radius = match self.corner {
            Corner::Panel => lens_radius(height, cx),
            Corner::Capsule => height / 2.,
            Corner::Fixed(radius) => radius.min(height / 2.),
        };

        Some(
            liquid_pill(
                div()
                    .absolute()
                    .left(px(left))
                    .top(px(top))
                    .w(px(right - left))
                    .h(height),
                cx,
            )
            .rounded(radius)
            .opacity(opacity),
        )
    }
}

/// The corner radius of a lens `height` high: as round as a capsule of that height allows, up
/// to the panel radius. An entry's hover fill takes it too, so hovering and choosing an entry
/// light the same shape.
pub fn lens_radius(height: Pixels, cx: &App) -> Pixels {
    (height / 2.).min(liquid_radius(cx))
}

/// Which springs the near and far edge of an axis ride for a move from `from` to `to`: the
/// edge the lens moves toward leads, the other trails. A lens that stays put on the axis keeps
/// both quick.
fn lead(to: f32, from: f32) -> (gpui::SpringConfig, gpui::SpringConfig) {
    match to.partial_cmp(&from) {
        // moving toward the far edge: the far edge leads
        Some(std::cmp::Ordering::Greater) => (Springs::LENS_TRAIL, Springs::LENS_LEAD),
        Some(std::cmp::Ordering::Less) => (Springs::LENS_LEAD, Springs::LENS_TRAIL),
        _ => (Springs::LENS_LEAD, Springs::LENS_LEAD),
    }
}

/// The least room art keeps between itself and the edge of a rounded container it sits in.
pub const NEST_INSET: Pixels = px(6.);
/// How round nested art may get, as a share of its side, so a cover in a capsule stays a
/// rounded square rather than turning into a circle.
const NEST_ROUND: f32 = 0.3;

/// Where art sits inside a rounded container so its corners run along the container's curve
/// instead of poking into it: the art's corner radius is the container's less the room around
/// it, as nested rounded rectangles are drawn concentric, and it stands in from the container's
/// side by `lead`, far enough that its corner keeps that room all the way round the curve even
/// where its own radius had to be capped.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Nest {
    /// The art's corner radius.
    pub radius: Pixels,
    /// The room above and below the art.
    pub inset: Pixels,
    /// The room between the container's side and the art.
    pub lead: Pixels,
}

/// Nests art of side `art` in a container rounded by `outer` with at least `inset` around it.
/// Round art, such as an avatar, passes `round` and keeps its circle.
pub fn nest(outer: Pixels, inset: Pixels, art: Pixels, round: bool) -> Nest {
    let inset = inset.max(NEST_INSET);
    let radius = match round {
        true => art / 2.,
        false => (outer - inset).clamp(Pixels::ZERO, art * NEST_ROUND),
    };
    Nest {
        radius,
        inset,
        lead: (outer - radius).max(inset),
    }
}
