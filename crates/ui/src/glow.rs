//! Hover fills that fade in and out instead of snapping.

use gpui::{
    App, ElementId, Entity, EntityId, Fill, Hsla, Rgba, StatefulInteractiveElement,
    StyleRefinement, Window,
};

use crate::motion::{HoverFade, Motion, animates};

/// A hover listener, as an element takes one.
pub type Hover = Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>;

/// How lit an element's hover fill stands this frame, kept per element across frames. Build
/// one while rendering, paint `fill` as the element's background and `attach` it so the
/// pointer turns the fade. Without motion there is none, and the caller keeps a plain hover
/// style.
pub struct Glow {
    fade: Entity<HoverFade>,
    level: f32,
    view: EntityId,
}

impl Glow {
    pub fn new(id: ElementId, window: &mut Window, cx: &mut App) -> Option<Self> {
        if !animates(cx) {
            return None;
        }
        let fade =
            window.use_keyed_state((id, "glow"), cx, |_, _| HoverFade::new(Motion::Base.span()));
        let level = fade.update(cx, |fade, cx| fade.level(window, cx));
        Some(Self {
            fade,
            level,
            view: window.current_view(),
        })
    }

    /// Reads an existing fade, for an element that already keeps one for its own motion.
    pub fn of(fade: Entity<HoverFade>, level: f32, view: EntityId) -> Self {
        Self { fade, level, view }
    }

    pub fn level(&self) -> f32 {
        self.level
    }

    /// The background that shows `fill` at this frame's level laid over `under`, or nothing
    /// while the element rests unlit with no background of its own.
    pub fn fill(&self, under: Option<Hsla>, fill: Hsla) -> Option<Hsla> {
        if self.level <= 0. {
            return under;
        }
        let lit = fill.opacity(self.level);
        Some(match under {
            Some(under) if under.a >= 1. => under.blend(lit),
            Some(under) => toward(under, fill, self.level),
            None => lit,
        })
    }

    /// Lets the pointer coming and going turn the fade.
    pub fn attach<E: StatefulInteractiveElement>(self, element: E) -> E {
        element.on_hover(self.listener(None))
    }

    /// The hover listener that turns the fade, running `also` after it, for an element that
    /// listens to hover for something of its own: an element takes only one hover listener.
    pub fn listener(self, also: Option<Hover>) -> impl Fn(&bool, &mut Window, &mut App) + 'static {
        let Self { fade, view, .. } = self;
        move |hovered, window, cx| {
            fade.update(cx, |fade, _| fade.set(*hovered));
            cx.notify(view);
            if let Some(also) = also.as_ref() {
                also(hovered, window, cx);
            }
        }
    }
}

/// Splits a hover style into its solid background, which a `Glow` can fade, and the rest,
/// which still applies at once.
pub fn hover_fill(style: StyleRefinement) -> (Option<Hsla>, StyleRefinement) {
    let mut rest = style;
    let fill = match rest.background.take() {
        Some(Fill::Color(background)) => match background.as_solid() {
            Some(color) => Some(color),
            None => {
                rest.background = Some(Fill::Color(background));
                None
            }
        },
        None => None,
    };
    (fill, rest)
}

/// A translucent background turned `level` of the way to `fill`, colour and cover alike, so a
/// hover fill takes the background's place as it fades in rather than mixing into it and
/// keeping the background's cover.
fn toward(under: Hsla, fill: Hsla, level: f32) -> Hsla {
    let low = Rgba::from(under);
    let high = Rgba::from(fill);
    let mix = |low: f32, high: f32| low + (high - low) * level;
    Rgba {
        r: mix(low.r, high.r),
        g: mix(low.g, high.g),
        b: mix(low.b, high.b),
        a: mix(low.a, high.a),
    }
    .into()
}
