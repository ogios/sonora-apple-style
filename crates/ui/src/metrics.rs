use gpui::{App, FontWeight, Pixels, SharedString, Styled, TextRun, Window, px};

use crate::glass::liquid;
use crate::theme::ActiveTheme as _;
use i18n::t;

const HEADER: f32 = 0.75;
/// How much taller a table row and a control stand under Liquid Glass, and the page margin
/// there.
const ROOMY_ROW: Pixels = px(12.);
const ROOMY_THUMB: Pixels = px(8.);
const ROOMY_CONTROL: Pixels = px(2.);
const ROOMY_CONTROL_SMALL: Pixels = px(4.);
const ROOMY_INSET: Pixels = px(32.);
/// How much more room a button keeps around its label and icon under Liquid Glass.
const ROOMY_BUTTON: f32 = 1.5;
const ROOMY_BUTTON_GAP: f32 = 1.34;
/// Under Liquid Glass the title bar's controls stand inside the top of the content sheet: the
/// bar keeps this much above a control, the sheet's gap from the window's top plus the room
/// inside its edge, and this much below it.
pub const BAR_ABOVE: Pixels = px(22.);
pub const BAR_BELOW: Pixels = px(8.);

pub const LEADING: f32 = 1.25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rounding {
    Square,
    Subtle,
    Rounded,
    Round,
}

impl Rounding {
    pub const ALL: [Self; 4] = [Self::Square, Self::Subtle, Self::Rounded, Self::Round];

    pub fn id(self) -> &'static str {
        match self {
            Self::Square => "square",
            Self::Subtle => "subtle",
            Self::Rounded => "rounded",
            Self::Round => "round",
        }
    }

    pub fn label(self) -> SharedString {
        match self {
            Self::Square => t!("corners-square"),
            Self::Subtle => t!("corners-subtle"),
            Self::Rounded => t!("corners-rounded"),
            Self::Round => t!("corners-round"),
        }
    }

    pub fn from_id(id: &str) -> Self {
        match id {
            "square" => Self::Square,
            "rounded" => Self::Rounded,
            "round" => Self::Round,
            _ => Self::Subtle,
        }
    }

    pub fn radius(self) -> Pixels {
        match self {
            Self::Square => px(0.),
            Self::Subtle => px(6.),
            Self::Rounded => px(10.),
            Self::Round => px(20.),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Metrics {
    pub row: Pixels,
    pub header: Pixels,
    pub pad: Pixels,
    pub inset: Pixels,
    pub control: Pixels,
    pub control_small: Pixels,
    pub field: Pixels,
    pub title_bar: Pixels,
    pub player_bar: Pixels,
    pub list_row: Pixels,
    pub thumb: Pixels,
    pub cover: Pixels,
    /// The room a button keeps between its border and its label or icon, side to side, for a
    /// regular and a small button, and between its icon and label.
    pub button_inset: Pixels,
    pub button_inset_small: Pixels,
    pub button_gap: Pixels,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new(px(14.))
    }
}

impl Metrics {
    pub fn new(base: Pixels) -> Self {
        let text = base / px(1.);
        let roomy = |step: f32, lines: f32| px(step.max((text * lines).round()));
        let row = roomy(42., 2.4);

        Self {
            row,
            header: px((row / px(1.) * HEADER).round()),
            pad: px(8.),
            inset: px(24.),
            control: roomy(32., 2.),
            control_small: roomy(26., 1.7),
            field: roomy(40., 2.6),
            title_bar: roomy(36., 2.4),
            player_bar: roomy(76., 5.),
            list_row: roomy(52., 3.2),
            thumb: px(34.),
            cover: px(140.),
            button_inset: px(12.),
            button_inset_small: px(8.),
            button_gap: px(6.),
        }
    }

    /// The roomier measures Liquid Glass lays the app out in, Cider's: taller table rows with
    /// larger artwork, taller controls, and wider page margins, so nothing feels cramped.
    pub fn roomy(self) -> Self {
        let row = self.row + ROOMY_ROW;

        Self {
            row,
            header: px((row / px(1.) * HEADER).round()),
            inset: self.inset.max(ROOMY_INSET),
            control: self.control + ROOMY_CONTROL,
            control_small: self.control_small + ROOMY_CONTROL_SMALL,
            thumb: self.thumb + ROOMY_THUMB,
            title_bar: self.control + ROOMY_CONTROL + BAR_ABOVE + BAR_BELOW,
            button_inset: self.button_inset * ROOMY_BUTTON,
            button_inset_small: self.button_inset_small * ROOMY_BUTTON,
            button_gap: self.button_gap * ROOMY_BUTTON_GAP,
            ..self
        }
    }
}

#[derive(Clone, Copy)]
pub enum Text {
    Tiny,
    Small,
    Label,
    Body,
    Large,
    Title,
    Display,
}

impl Text {
    pub(crate) fn ratio(self) -> f32 {
        match self {
            Self::Tiny => 0.77,
            Self::Small => 0.85,
            Self::Label => 0.92,
            Self::Body => 1.,
            Self::Large => 1.38,
            Self::Title => 1.69,
            Self::Display => 2.15,
        }
    }
}

/// The size Material 3 sets its body-medium role in, which the type scale is anchored to: that
/// role takes the user's font size and every other role scales with it.
const TYPE_ANCHOR: f32 = 14.;

/// Material 3's type scale: fifteen roles in five families, each with its own size, line height
/// and weight, laid out at the user's font size where body-medium stands at 14 / 20. Sizes and
/// line heights keep Material's proportions. Weights keep Cider's: display and headline roles
/// stand bold where Material sets them regular, so page and section titles read as Cider's do.
/// Material's letter spacing is left out, as text here cannot be tracked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    DisplayLarge,
    DisplayMedium,
    DisplaySmall,
    HeadlineLarge,
    HeadlineMedium,
    HeadlineSmall,
    TitleLarge,
    TitleMedium,
    TitleSmall,
    BodyLarge,
    BodyMedium,
    BodySmall,
    LabelLarge,
    LabelMedium,
    LabelSmall,
}

impl Type {
    /// The role's size and line height at the anchor, in Material's own pixels.
    fn spec(self) -> (f32, f32) {
        match self {
            Self::DisplayLarge => (57., 64.),
            Self::DisplayMedium => (45., 52.),
            Self::DisplaySmall => (36., 44.),
            Self::HeadlineLarge => (32., 40.),
            Self::HeadlineMedium => (28., 36.),
            Self::HeadlineSmall => (24., 32.),
            Self::TitleLarge => (22., 28.),
            Self::TitleMedium => (16., 24.),
            Self::TitleSmall => (14., 20.),
            Self::BodyLarge => (16., 24.),
            Self::BodyMedium => (14., 20.),
            Self::BodySmall => (12., 16.),
            Self::LabelLarge => (14., 20.),
            Self::LabelMedium => (12., 16.),
            Self::LabelSmall => (11., 16.),
        }
    }

    /// The role's size at `base`, the user's font size.
    pub fn size(self, base: Pixels) -> Pixels {
        px((base / px(1.) * self.spec().0 / TYPE_ANCHOR).round())
    }

    /// The role's line height at `base`, the user's font size.
    pub fn line(self, base: Pixels) -> Pixels {
        px((base / px(1.) * self.spec().1 / TYPE_ANCHOR).round())
    }

    pub fn weight(self) -> FontWeight {
        match self {
            Self::DisplayLarge
            | Self::DisplayMedium
            | Self::DisplaySmall
            | Self::HeadlineLarge
            | Self::HeadlineMedium
            | Self::HeadlineSmall => FontWeight::BOLD,
            Self::TitleLarge => FontWeight::SEMIBOLD,
            Self::TitleMedium | Self::TitleSmall => FontWeight::MEDIUM,
            Self::BodyLarge | Self::BodyMedium | Self::BodySmall => FontWeight::NORMAL,
            Self::LabelLarge | Self::LabelMedium | Self::LabelSmall => FontWeight::MEDIUM,
        }
    }
}

/// Sets an element's text in one role of the [`Type`] scale. The scale is part of the Liquid
/// Glass look: with it off, text keeps the sizes it was given before, so the flat look stands
/// as it was.
pub trait Typeset: Styled + Sized {
    /// Sets this element's text size, line height and weight to `role`'s at the user's font
    /// size. A weight set after it wins, for a role that needs to stand out further.
    fn typeset(self, role: Type, cx: &App) -> Self {
        let base = cx.theme().font_size;
        match liquid(cx) {
            true => self
                .text_size(role.size(base))
                .line_height(role.line(base))
                .font_weight(role.weight()),
            false => self,
        }
    }

    /// Sets this element's text size and weight to `role`'s but leaves its line height, for
    /// text centred in a row of fixed height, as a table cell's is.
    fn typeset_inline(self, role: Type, cx: &App) -> Self {
        match liquid(cx) {
            true => self
                .text_size(role.size(cx.theme().font_size))
                .font_weight(role.weight()),
            false => self,
        }
    }
}

impl<E: Styled + Sized> Typeset for E {}

const INSET: f32 = 0.25;
const BORDER: Pixels = px(1.);

pub fn snapped(value: Pixels, window: &Window) -> Pixels {
    let scale = window.scale_factor();
    px((value / px(1.) * scale).round() / scale)
}

pub fn text_width(text: impl Into<SharedString>, window: &Window) -> Pixels {
    let text = text.into();
    if text.is_empty() {
        return Pixels::ZERO;
    }

    let style = window.text_style();
    let run = TextRun {
        len: text.len(),
        font: style.font(),
        color: style.color,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let size = style.font_size.to_pixels(window.rem_size());

    window
        .text_system()
        .shape_line(text, size, &[run], None)
        .width
}

pub fn tucked(radius: Pixels, window: &Window) -> Pixels {
    (radius - window.rem_size() * INSET - BORDER).max(Pixels::ZERO)
}
