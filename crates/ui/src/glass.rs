use gpui::prelude::*;
use gpui::{
    App, Axis, Bounds, BoxShadow, Canvas, Corners, Div, Hsla, Pixels, Window, canvas, div, fill,
    linear_color_stop, linear_gradient, px,
};

use crate::theme::ActiveTheme as _;

/// How much of the popover colour a glass control keeps over what it blurs.
pub const GLASS_FILL: f32 = 0.2;
/// How hard a glass control blurs what is behind it.
pub const GLASS_BLUR: Pixels = px(8.);
/// Raised surfaces retain contrast even over bright artwork, while a faint backdrop remains visible.
const POPOVER_FILL: f32 = 0.82;
/// How much of the popover colour a frosted bar keeps, and how far it blurs what passes under.
/// Both are thicker than a menu's, since a bar sits over the page for good and its text has to
/// read over whatever scrolls by.
const BAR_FILL: f32 = 0.86;
const BAR_BLUR: Pixels = px(18.);

/// How much of the popover colour a Liquid Glass panel keeps over its blur, on a dark theme
/// and a light one. Thinner than a bar, since the rim and the shading carry the edge and the
/// colour under the panel is what makes it read as glass.
const LIQUID_FILL_DARK: f32 = 0.42;
const LIQUID_FILL_LIGHT: f32 = 0.5;
/// What a Liquid Glass panel keeps of the popover colour with the blur turned off, where
/// nothing behind it is softened and the text needs the fill to stand on.
const LIQUID_FLAT: f32 = 0.9;
/// How hard a Liquid Glass panel blurs what passes under it.
const LIQUID_BLUR: Pixels = px(28.);
/// How far the corners lean from a circle's arc toward a continuous, squircle-like curve.
const LIQUID_SMOOTHING: f32 = 0.6;
/// How many times the theme radius a floating panel is rounded by, so the Corners setting
/// still scales it and Square keeps it square.
const LIQUID_ROUNDING: f32 = 2.2;
/// How much lighter the top of a panel's fill starts than its foot, the sheen a curved
/// surface catches from above.
const LIQUID_SHEEN: f32 = 0.06;
/// The white of the hairline rim around a panel, on a dark theme and a light one.
const RIM_DARK: f32 = 0.18;
const RIM_LIGHT: f32 = 0.7;
/// The specular line just inside the top edge, and the shade just inside the foot, which
/// together make a flat panel read as a slab with thickness.
const SPECULAR_DARK: f32 = 0.3;
const SPECULAR_LIGHT: f32 = 0.95;
const UNDERSIDE_DARK: f32 = 0.35;
const UNDERSIDE_LIGHT: f32 = 0.08;
/// The soft glow that fills a panel's inside edge, so the rim fades inward rather than
/// stopping at the hairline.
const GLOW: f32 = 0.05;
const GLOW_BLUR: Pixels = px(10.);
/// The drop shadow a floating panel casts on what is behind it.
const DROP_DARK: f32 = 0.45;
const DROP_LIGHT: f32 = 0.14;
const DROP_OFFSET: Pixels = px(10.);
const DROP_BLUR: Pixels = px(30.);
/// The hairline a lit edge is drawn with, inside the panel's bounds.
const EDGE: Pixels = px(1.);
/// How much of the page colour the floating content sheet keeps over the colour field, on a
/// dark theme and a light one: dense enough that pages read as one surface, as in Apple Music,
/// thin enough that the field still tints its edges.
const SHEET_DARK: f32 = 1.;
const SHEET_LIGHT: f32 = 1.;
/// The hairline rim around the content sheet, fainter than a glass panel's.
const SHEET_RIM_DARK: f32 = 0.08;
const SHEET_RIM_LIGHT: f32 = 0.5;
/// The fill of the glass pill that marks a selected entry, as a share of the foreground.
const PILL_FILL: f32 = 0.12;
const PILL_FOOT: f32 = 0.06;
/// The drop shadow under a pill, much closer than a panel's since it sits on the panel.
const PILL_DROP: f32 = 0.18;
const PILL_DROP_BLUR: Pixels = px(6.);
const PILL_DROP_OFFSET: Pixels = px(2.);
/// The colour field painted behind Liquid Glass chrome: how strong each blob's core is, how
/// wide a blob spans as a share of the window's longer side, how much of that span the sharp
/// circle under its shadow takes, and how far the shadow feathers as a share of the span.
const FIELD_DARK: f32 = 0.3;
const FIELD_LIGHT: f32 = 0.6;
const FIELD_SPAN: f32 = 0.5;
const FIELD_CORE: f32 = 0.6;
const FIELD_SOFT: f32 = 0.28;
/// Blob centres as shares of the window, and how far round the colour wheel from the lead
/// tint each one turns.
const FIELD_BLOBS: [(f32, f32, f32); 4] = [
    (0.1, 0.18, 0.),
    (0.88, 0.22, 0.12),
    (0.55, 0.95, -0.1),
    (0.2, 0.9, 0.2),
];
/// The saturation and lightness a field blob is pinned to, on a dark theme and a light one,
/// so a dull cover and the neutral fallback still glow.
const FIELD_SATURATION: f32 = 0.75;
const FIELD_LIGHTNESS_DARK: f32 = 0.3;
/// The lightness the accent text colour is held to, at least on a dark page and at most on a
/// light one, so it reads against either.
const ACCENT_TEXT_DARK: f32 = 0.7;
const ACCENT_TEXT_LIGHT: f32 = 0.42;
const FIELD_LIGHTNESS_LIGHT: f32 = 0.62;

/// Whether a surface that only frosts for the look of it should frost at all. A backdrop blur
/// is the priciest thing the renderer does per frame, so the setting turns those off and each
/// one falls back to a flat fill. The settings header and the fullscreen chrome frost whatever
/// it answers: their haze is what keeps text readable over the content passing under it.
pub fn blurring(cx: &App) -> bool {
    cx.theme().blur
}

/// Styles an element as frosted glass: a faint popover fill over a hard blur of
/// whatever it floats on. Over a flat colour the blur shows nothing, so the fill is all
/// that separates the control from the page there. A site the blur setting reaches asks
/// `blurring` before calling this, since the thin fill needs the wash to read.
pub fn glass<E: Styled>(element: E, cx: &App) -> E {
    let theme = cx.theme();

    element
        .bg(theme.popover.opacity(GLASS_FILL))
        .backdrop_blur(GLASS_BLUR)
}

/// The fill shared by context menus and raised translucent panels. With blur disabled it
/// becomes opaque, since there is no wash behind the text to preserve its contrast.
pub fn popover_fill(cx: &App) -> Hsla {
    let theme = cx.theme();
    match blurring(cx) {
        true => theme.popover.opacity(POPOVER_FILL),
        false => theme.popover,
    }
}

/// Gives a raised panel the same fill, foreground and backdrop blur as a context menu.
pub fn popover_surface<E: Styled>(element: E, cx: &App) -> E {
    let element = element
        .bg(popover_fill(cx))
        .text_color(cx.theme().popover_foreground);
    match blurring(cx) {
        true => frost(element),
        false => element,
    }
}

/// Styles chrome that stays over scrolling content, the player bar and a table's head: a thick
/// popover fill over a wide blur. The caller asks `blurring` first, since without the blur the
/// thin fill would leave the text on the bare page.
pub fn bar_surface<E: Styled>(element: E, cx: &App) -> E {
    element
        .bg(cx.theme().popover.opacity(BAR_FILL))
        .backdrop_blur(BAR_BLUR)
}

/// Blurs what is behind an element by the glass width and nothing more, for a hover fill
/// that is already translucent. Only hovers that float over real content go through it;
/// on flat paint the blur shows nothing, so those stay plain fills. The caller asks
/// `blurring` first, since this one has no theme to read.
pub fn frost<E: Styled>(element: E) -> E {
    element.backdrop_blur(GLASS_BLUR)
}

/// Whether the chrome is drawn as Liquid Glass: floating panels inset from the window edges,
/// deeply rounded, with a lit rim and a shaded foot. The Liquid Glass appearance setting
/// decides it, and every site that floats or rims for the look asks this first.
pub fn liquid(cx: &App) -> bool {
    cx.theme().liquid
}

/// The corner radius of a floating Liquid Glass panel, a multiple of the theme radius.
pub fn liquid_radius(cx: &App) -> Pixels {
    cx.theme().radius * LIQUID_ROUNDING
}

/// The bright hairline rim of a floating Liquid Glass panel, a menu or a dialog included.
pub fn liquid_rim(cx: &App) -> Hsla {
    let rim = match cx.theme().background.l < 0.5 {
        true => SHEET_RIM_DARK,
        false => SHEET_RIM_LIGHT,
    };
    gpui::white().opacity(rim)
}

/// How far a floating Liquid Glass panel stands off the window edges and its neighbours.
pub fn liquid_gap(cx: &App) -> Pixels {
    cx.theme().metrics.pad
}

/// Styles an element as a Liquid Glass panel: a thin tinted fill that lightens toward the top,
/// a hard blur of what is behind it, a hairline rim, a specular line inside the top edge, a
/// shade inside the foot, and a soft drop shadow. The fill is the popover colour, so it takes
/// the adaptive theme's cover tint with everything else. The caller sets the radius, and the
/// corners are smoothed toward a continuous curve. Refraction is not drawn: the renderer only
/// blurs what is behind an element.
pub fn liquid_glass<E: Styled>(element: E, cx: &App) -> E {
    let drop = match cx.theme().background.l < 0.5 {
        true => DROP_DARK,
        false => DROP_LIGHT,
    };
    material(
        element,
        BoxShadow::new(Pixels::ZERO, DROP_OFFSET, gpui::black().opacity(drop))
            .blur_radius(DROP_BLUR),
        cx,
    )
}

/// The large rounded sheet the pages sit on under Liquid Glass: the page colour, solid, so the
/// one framed panel stands clear of the dark window around it as Cider's does, with a faint
/// rim and the panels' drop shadow.
pub fn liquid_sheet<E: Styled>(element: E, cx: &App) -> E {
    let theme = cx.theme();
    let dark = theme.background.l < 0.5;
    let (fill, rim, drop) = match dark {
        true => (SHEET_DARK, SHEET_RIM_DARK, DROP_DARK),
        false => (SHEET_LIGHT, SHEET_RIM_LIGHT, DROP_LIGHT),
    };

    element
        .rounded(liquid_radius(cx))
        .corner_smoothing(LIQUID_SMOOTHING)
        .bg(theme.background.opacity(fill))
        .border_1()
        .border_color(gpui::white().opacity(rim))
        .shadow(vec![
            BoxShadow::new(Pixels::ZERO, DROP_OFFSET, gpui::black().opacity(drop))
                .blur_radius(DROP_BLUR),
        ])
}

/// A colour wash that holds until `hold`, then fades to clear along `axis`. Its visible
/// edges follow the sheet's rounded scroll bounds, including while the page scrolls.
pub fn liquid_wash(color: Hsla, hold: f32, axis: Axis, cx: &App) -> Canvas<()> {
    let radius = sheet_radius(cx);
    let hold = hold.clamp(0., 1. - f32::EPSILON);
    canvas(
        |_, _, _| (),
        move |bounds, (), window, _| {
            let clip = window.content_mask().bounds;
            let visible = bounds.intersect(&clip);
            if visible.is_empty() {
                return;
            }

            // Round the visible viewport, rather than the oversized wash: scrolling must not
            // move the corners away from the sheet. Keep the gradient in its original space.
            let (start, span, visible_start, visible_span, angle) = match axis {
                Axis::Vertical => (
                    bounds.top(),
                    bounds.size.height,
                    visible.top(),
                    visible.size.height,
                    180.,
                ),
                Axis::Horizontal => (
                    bounds.left(),
                    bounds.size.width,
                    visible.left(),
                    visible.size.width,
                    90.,
                ),
            };
            let end = start + span;
            let fade = start + span * hold;
            let alpha = |at: Pixels| ((end - at) / (end - fade)).clamp(0., 1.);
            let held = ((fade - visible_start) / visible_span).clamp(0., 1. - f32::EPSILON);
            window.paint_quad(
                fill(
                    visible,
                    linear_gradient(
                        angle,
                        linear_color_stop(color.opacity(alpha(visible_start)), held),
                        linear_color_stop(color.opacity(alpha(visible_start + visible_span)), 1.),
                    ),
                )
                .corner_radii(sheet_corners(visible, clip, radius))
                .corner_smoothing(LIQUID_SMOOTHING),
            );
        },
    )
}

/// The sheet's inner radius, after its hairline border.
pub(crate) fn sheet_radius(cx: &App) -> Pixels {
    (liquid_radius(cx) - EDGE).max(Pixels::ZERO)
}

/// Rounds only corners shared with the viewport, keeping the rounding fixed as content moves.
pub(crate) fn sheet_corners(
    bounds: Bounds<Pixels>,
    viewport: Bounds<Pixels>,
    radius: Pixels,
) -> Corners<Pixels> {
    let corner = |touches| match touches {
        true => radius,
        false => Pixels::ZERO,
    };
    Corners {
        top_left: corner(bounds.top() == viewport.top() && bounds.left() == viewport.left()),
        top_right: corner(bounds.top() == viewport.top() && bounds.right() == viewport.right()),
        bottom_left: corner(
            bounds.bottom() == viewport.bottom() && bounds.left() == viewport.left(),
        ),
        bottom_right: corner(
            bounds.bottom() == viewport.bottom() && bounds.right() == viewport.right(),
        ),
    }
}

/// The Liquid Glass material at the size of a control: the same fill, blur, rim and shading
/// as `liquid_glass`, with the close shadow of something resting on the surface rather than
/// floating over it.
pub fn liquid_control<E: Styled>(element: E, cx: &App) -> E {
    material(
        element,
        BoxShadow::new(
            Pixels::ZERO,
            PILL_DROP_OFFSET,
            gpui::black().opacity(PILL_DROP),
        )
        .blur_radius(PILL_DROP_BLUR),
        cx,
    )
}

/// Styles an element as the glass pill that marks the selected entry in a Liquid Glass panel
/// or segmented bar: a capsule of the foreground colour, faint and lighter at the top, with
/// its own lit top edge and a close shadow, so it reads as a lens resting on the panel.
pub fn liquid_pill<E: Styled>(element: E, cx: &App) -> E {
    let theme = cx.theme();
    let dark = theme.background.l < 0.5;
    let white = gpui::white();
    let (specular, rim) = match dark {
        true => (SPECULAR_DARK, RIM_DARK),
        false => (SPECULAR_LIGHT, RIM_LIGHT),
    };

    element
        .rounded_full()
        .corner_smoothing(LIQUID_SMOOTHING)
        .bg(linear_gradient(
            180.,
            linear_color_stop(theme.foreground.opacity(PILL_FILL), 0.),
            linear_color_stop(theme.foreground.opacity(PILL_FOOT), 1.),
        ))
        .border_1()
        .border_color(white.opacity(rim))
        .shadow(vec![
            BoxShadow::new(
                Pixels::ZERO,
                PILL_DROP_OFFSET,
                gpui::black().opacity(PILL_DROP),
            )
            .blur_radius(PILL_DROP_BLUR),
            BoxShadow::new(Pixels::ZERO, EDGE, white.opacity(specular)).inset(),
        ])
}

/// Marks a selected entry: the glass pill under Liquid Glass, and a flat `fallback` fill
/// otherwise.
pub fn chosen<E: Styled>(element: E, fallback: Hsla, cx: &App) -> E {
    match liquid(cx) {
        true => liquid_pill(element, cx),
        false => element.bg(fallback),
    }
}

/// How much of its lightness the dark page colour keeps where it lies under the sidebars, so
/// the window around the sheet reads near black, the way Cider's does.
const GROUND_DARK: f32 = 0.35;

/// The colour of the window around the content sheet under Liquid Glass, which the sidebars
/// stand on without a panel of their own: the page colour taken most of the way to black in a
/// dark theme, and left as it is in a light one.
pub fn liquid_ground(cx: &App) -> Hsla {
    let background = cx.theme().background;
    match background.l < 0.5 {
        true => Hsla {
            l: background.l * GROUND_DARK,
            ..background
        },
        false => background,
    }
}

/// The accent text is drawn in, such as the title of the song playing now: the cover tint, or
/// the theme's selection hue without one, held light or dark enough to read on the page.
pub fn accent_text(cx: &App) -> Hsla {
    let theme = cx.theme();
    let lead = theme.tint.unwrap_or(theme.selection);
    let l = match theme.background.l < 0.5 {
        true => lead.l.max(ACCENT_TEXT_DARK),
        false => lead.l.min(ACCENT_TEXT_LIGHT),
    };
    Hsla { l, a: 1., ..lead }
}

/// The colour field Liquid Glass chrome floats over: wide, feathered blobs of the cover tint
/// and hues near it over the page colour, so the panels have something to blur and their rims
/// something to catch. Without a tint it falls back to the theme's selection hue. It is
/// static, drawn as the blurred shadows of circles, and fills its `relative` parent.
pub fn liquid_field(window: &Window, cx: &App) -> Div {
    let theme = cx.theme();
    let dark = theme.background.l < 0.5;
    let lead = theme.tint.unwrap_or(theme.selection);
    let (strength, lightness) = match dark {
        true => (FIELD_DARK, FIELD_LIGHTNESS_DARK),
        false => (FIELD_LIGHT, FIELD_LIGHTNESS_LIGHT),
    };
    let size = window.viewport_size();
    let span = size.width.max(size.height) * FIELD_SPAN;
    let core = span * FIELD_CORE;
    // A drop shadow is cut away under its own element, so each circle stands off past the
    // field's left edge and throws its shadow back onto the blob's place.
    let away = size.width + span;

    div()
        .absolute()
        .inset_0()
        .overflow_hidden()
        .children(
            FIELD_BLOBS
                .into_iter()
                .enumerate()
                .map(|(index, (x, y, turn))| {
                    let hue = match (index, theme.tint_secondary) {
                        (1, Some(second)) => second.h,
                        _ => (lead.h + turn).rem_euclid(1.),
                    };
                    let color = Hsla {
                        h: hue,
                        s: FIELD_SATURATION,
                        l: lightness,
                        a: strength,
                    };
                    div()
                        .absolute()
                        .left(size.width * x - core / 2. - away)
                        .top(size.height * y - core / 2.)
                        .size(core)
                        .rounded_full()
                        .shadow(vec![
                            BoxShadow::new(away, Pixels::ZERO, color)
                                .blur_radius(span * FIELD_SOFT),
                        ])
                }),
        )
}

/// The Liquid Glass material behind `liquid_glass` and `liquid_control`, casting `drop`.
fn material<E: Styled>(element: E, drop: BoxShadow, cx: &App) -> E {
    let theme = cx.theme();
    let dark = theme.background.l < 0.5;
    let pick = |on_dark: f32, on_light: f32| match dark {
        true => on_dark,
        false => on_light,
    };
    let fill = match blurring(cx) {
        true => theme
            .popover
            .opacity(pick(LIQUID_FILL_DARK, LIQUID_FILL_LIGHT)),
        false => theme.popover.opacity(LIQUID_FLAT),
    };
    let lit = Hsla {
        l: (fill.l + LIQUID_SHEEN).min(1.),
        ..fill
    };
    let white = gpui::white();
    let black = gpui::black();
    let element = match blurring(cx) {
        true => element.backdrop_blur(LIQUID_BLUR),
        false => element,
    };

    element
        .corner_smoothing(LIQUID_SMOOTHING)
        .bg(linear_gradient(
            180.,
            linear_color_stop(lit, 0.),
            linear_color_stop(fill, 0.55),
        ))
        .border_1()
        .border_color(white.opacity(pick(RIM_DARK, RIM_LIGHT)))
        .shadow(vec![
            drop,
            BoxShadow::new(
                Pixels::ZERO,
                EDGE,
                white.opacity(pick(SPECULAR_DARK, SPECULAR_LIGHT)),
            )
            .inset(),
            BoxShadow::new(
                Pixels::ZERO,
                -EDGE,
                black.opacity(pick(UNDERSIDE_DARK, UNDERSIDE_LIGHT)),
            )
            .inset(),
            BoxShadow::new(Pixels::ZERO, Pixels::ZERO, white.opacity(GLOW))
                .blur_radius(GLOW_BLUR)
                .inset(),
        ])
}
