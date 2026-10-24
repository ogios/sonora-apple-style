use gpui::prelude::*;
use gpui::{App, Div, Pixels, Window, div, px};

use crate::lens::{Mark, mark};
use crate::motion::{Springs, Sprung, animates, css_ease};

/// How close to its target, in pixels, a folding height counts as arrived.
const EPSILON: f32 = 0.5;

/// A disclosure's content folding open and shut: its height eases between nothing and what the
/// content measures, clipped as it goes, and the content fades with it. At rest open the content
/// sits in the flow at its own height, so it can grow and shrink freely; only the fold itself
/// pins a height.
///
/// Keep one per disclosure in the view, `set` it as the disclosure opens or closes, and while
/// `drawn` says so give `render` the content.
pub struct Unfold {
    open: bool,
    height: Sprung,
    natural: Mark,
    /// Set until the content first measures, so a disclosure that starts open shows at once
    /// instead of unfolding as the window opens.
    snapping: bool,
}

impl Unfold {
    pub fn new(open: bool) -> Self {
        Self {
            open,
            height: Sprung::new(0., Springs::FOLD, EPSILON),
            natural: Mark::new(),
            snapping: open,
        }
    }

    /// Opens or closes the disclosure, from wherever its fold stands.
    pub fn set(&mut self, open: bool) {
        self.open = open;
        if !open {
            self.snapping = false;
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Whether the fold is moving, so whatever sits below it is moving too.
    pub fn folding(&self) -> bool {
        !self.height.resting()
    }

    /// Whether the content is still to be drawn: while open, and while it folds away.
    pub fn drawn(&self) -> bool {
        self.open || !self.height.resting() || self.height.target() > 0.
    }

    /// How open the disclosure stands, from 0 shut to 1 open, for a chevron that turns with it.
    pub fn openness(&self) -> f32 {
        let natural = self
            .natural
            .get()
            .map_or(0., |bounds| bounds.size.height / px(1.));
        match (self.open, self.height.resting()) {
            (open, true) => f32::from(u8::from(open)),
            (_, false) if natural > 0. => (self.height.position() / natural).clamp(0., 1.),
            (open, false) => f32::from(u8::from(open)),
        }
    }

    /// Lays out `content` at this frame's fold.
    pub fn render(&mut self, content: impl IntoElement, window: &mut Window, cx: &App) -> Div {
        let measured = self.natural.get().map(|bounds| bounds.size.height / px(1.));
        let target = match (self.open, measured) {
            (true, Some(natural)) => natural,
            (true, None) => 0.,
            (false, _) => 0.,
        };
        if self.snapping && measured.is_some() {
            self.snapping = false;
            self.height.snap(target);
        } else if !animates(cx) {
            self.height.snap(target);
        } else {
            self.height.aim(target, Springs::FOLD);
        }
        let height = self.height.value(window, cx);
        let settled_open =
            self.open && (self.snapping || (self.height.resting() && measured.is_some()));
        if !self.open && self.height.resting() {
            self.natural.clear();
        }
        let inner = div()
            .relative()
            .flex()
            .flex_col()
            .w_full()
            .child(content)
            .child(mark(&self.natural));
        match settled_open {
            true => div().w_full().child(inner),
            false => {
                let shown = match measured {
                    Some(natural) if natural > 0. => (height / natural).clamp(0., 1.),
                    _ => 0.,
                };
                div()
                    .relative()
                    .w_full()
                    .flex_none()
                    .h(px(height.max(0.)))
                    .overflow_hidden()
                    .child(
                        inner
                            .absolute()
                            .top_0()
                            .left_0()
                            .right_0()
                            .opacity(css_ease(shown)),
                    )
            }
        }
    }
}

/// How close to its target, in pixels, a sliding drawer's width counts as arrived.
const DRAWER_EPSILON: f32 = 0.5;

/// A side panel's width sliding between nothing and its full width as it opens and closes.
/// Only a change between open and shut slides: a resize, or the first frame, lands at once, so
/// dragging the panel's grip never lags behind the pointer.
#[derive(Default)]
pub struct Drawer {
    width: Option<Sprung>,
    open: bool,
}

impl Drawer {
    pub fn new() -> Self {
        Self::default()
    }

    /// The width the panel takes this frame, toward `full` while `open` and nothing while shut.
    pub fn width(&mut self, open: bool, full: Pixels, window: &mut Window, cx: &App) -> Pixels {
        let target = match open {
            true => full / px(1.),
            false => 0.,
        };
        let turned = open != self.open;
        self.open = open;
        let width = self
            .width
            .get_or_insert_with(|| Sprung::new(target, Springs::FOLD, DRAWER_EPSILON));
        match (turned && animates(cx), width.resting()) {
            (true, _) | (false, false) => width.aim(target, Springs::FOLD),
            (false, true) => width.snap(target),
        }
        px(width.value(window, cx).max(0.))
    }

    /// Whether the width is still on its way, so the panel has to be drawn even while shut.
    pub fn sliding(&self) -> bool {
        self.width.as_ref().is_some_and(|width| !width.resting())
    }
}
