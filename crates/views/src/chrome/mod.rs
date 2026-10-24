mod aside;
mod devices;
mod jam;
mod notice;
mod output_picker;
mod player_bar;
mod sidebar_left;
mod sidebar_right;
mod title_bar;
mod toasts;
mod toolbar;
pub(crate) mod tools;

pub(crate) use aside::{Aside, VerseLook, verse, verse_sizes};
pub(crate) use jam::{JamCode, JamInvite, JamJoin};
pub(crate) use notice::AppNotice;
pub(crate) use output_picker::OutputPicker;
pub(crate) use player_bar::PlayerBar;
pub(crate) use sidebar_left::SidebarLeft;
pub(crate) use sidebar_right::SidebarRight;
#[cfg(target_os = "macos")]
pub use title_bar::traffic_lights;
pub(crate) use title_bar::{Busy, TitleBar, TitleBarEvent, TitleBarOptions};
pub use toasts::ToastStack;
pub(crate) use toolbar::{Searchable, Toolbar, Tooled};

use gpui::prelude::*;
use gpui::{App, Div, Entity, Global, Pixels, Window, div, px};
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
use state::AppSettings;
use ui::{ActiveTheme as _, MIN_CONTENT, Room, eyebrow};

/// The width the content sheet's hairline rim takes, one pixel on either side.
const SHEET_RIMS: Pixels = px(2.);

/// The radius every element touching a corner of the window rounds that corner with, or `None`
/// under server-side decorations or `Rounding::Square`. It never exceeds half the title bar's
/// height, since GPUI clamps a quad's radius to half its shorter side and the title bar would
/// otherwise draw a tighter corner than the rest of the chrome.
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub(crate) fn window_radius(settings: &AppSettings, cx: &App, window: &Window) -> Option<Pixels> {
    if window.is_fullscreen() || settings.server_side_decorations() {
        return None;
    }
    match settings.window_rounding() {
        ui::Rounding::Square => None,
        rounding => Some(rounding.radius().min(cx.theme().metrics.title_bar / 2.)),
    }
}

pub(crate) fn section_label(key: &'static str, cx: &App) -> Div {
    div()
        .flex()
        .flex_none()
        .items_end()
        .h(cx.theme().metrics.list_row)
        .px_2()
        .pb_1()
        .child(eyebrow(i18n::lookup(key, None), cx))
}

pub(crate) fn cap(min: Pixels, max: Pixels, keep: Pixels, window: &Window) -> Pixels {
    let room = window.viewport_size().width - keep;
    max.min(room.max(min))
}

#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct Chrome {
    sidebar_left: Pixels,
    sidebar_right: Pixels,
    /// The width the left sidebar leaves free for an open right sidebar, counted even while
    /// the window is too narrow to show it.
    reserved_right: Pixels,
    /// The part of a page covered by the translucent player bar.
    player_bottom: Pixels,
    /// The widths the sidebars are sliding toward. Pages lay out against these rather than
    /// the widths of the frame, so a grid's column count or an artwork's size changes once
    /// as a sidebar starts to slide instead of stepping back and forth while it moves.
    resting_left: Pixels,
    resting_right: Pixels,
}

struct Installed(Entity<Chrome>);

impl Global for Installed {}

impl Chrome {
    pub fn entity(cx: &mut App) -> Entity<Chrome> {
        if cx.try_global::<Installed>().is_none() {
            let chrome = cx.new(|_| Chrome::default());
            cx.set_global(Installed(chrome));
        }
        cx.global::<Installed>().0.clone()
    }

    /// Publishes the sidebars' widths this frame and the widths they are sliding toward.
    pub(crate) fn publish(
        (left, right): (Pixels, Pixels),
        (resting_left, resting_right): (Pixels, Pixels),
        reserved_right: Pixels,
        cx: &mut App,
    ) {
        let next = Self {
            sidebar_left: left,
            sidebar_right: right,
            reserved_right,
            player_bottom: Self::get(cx).player_bottom,
            resting_left,
            resting_right,
        };
        let chrome = Self::entity(cx);
        chrome.update(cx, |chrome, cx| {
            if *chrome != next {
                *chrome = next;
                cx.notify();
            }
        });
    }

    pub fn get(cx: &App) -> Self {
        cx.try_global::<Installed>()
            .map(|installed| *installed.0.read(cx))
            .unwrap_or_default()
    }

    /// The width the left sidebar takes this frame, as it slides.
    pub fn left(cx: &App) -> Pixels {
        Self::get(cx).sidebar_left
    }

    /// The width the right sidebar takes this frame, as it slides.
    pub fn right(cx: &App) -> Pixels {
        Self::get(cx).sidebar_right
    }

    pub fn reserved_right(cx: &App) -> Pixels {
        Self::get(cx).reserved_right
    }

    /// The extra scroll space needed to bring a page's last item above the player bar.
    pub fn bottom(cx: &App) -> Pixels {
        Self::get(cx).player_bottom
    }

    /// How far a page that runs up under the title bar reaches above where pages start: under
    /// Liquid Glass, from the strip the history capsule and the page's tools stand in up to the
    /// content sheet's top edge, and nothing otherwise. A page with a picture at its head, as
    /// an artist's has, starts this far up and pads its content back down by as much.
    pub fn under_bar(window: &Window, cx: &App) -> Pixels {
        match ui::liquid(cx) {
            true => ui::snapped(cx.theme().metrics.title_bar, window) - ui::liquid_gap(cx),
            false => Pixels::ZERO,
        }
    }

    /// How far the sidebars keep their last row clear of the player bar: nothing under Liquid
    /// Glass, where the bar sits inside the content sheet and the sidebars run to the foot of
    /// the window beside it.
    pub fn beside(cx: &App) -> Pixels {
        match ui::liquid(cx) {
            true => Pixels::ZERO,
            false => Self::bottom(cx),
        }
    }

    /// Publishes the player bar's overlap after its responsive height has been resolved.
    pub(crate) fn set_bottom(bottom: Pixels, cx: &mut App) {
        Self::entity(cx).update(cx, |chrome, cx| {
            if chrome.player_bottom != bottom {
                chrome.player_bottom = bottom;
                cx.notify();
                cx.refresh_windows();
            }
        });
    }

    /// The width pages lay out in: what the sidebars leave once they come to rest, so a page
    /// settles into its final layout as soon as a sidebar starts to slide.
    pub fn content(window: &Window, cx: &App) -> Pixels {
        let chrome = Self::get(cx);
        // Under Liquid Glass the pages sit on a sheet inset by a gap on either side, inside a
        // hairline rim.
        let sheet = match ui::liquid(cx) {
            true => ui::liquid_gap(cx) * 2. + SHEET_RIMS,
            false => Pixels::ZERO,
        };
        (window.viewport_size().width - chrome.resting_left - chrome.resting_right - sheet)
            .max(MIN_CONTENT)
    }

    pub fn room(window: &Window, cx: &App) -> Room {
        Room::of(Self::content(window, cx))
    }

    /// Classifies chrome that spans the whole window, including the space under sidebars.
    pub fn window_room(window: &Window) -> Room {
        Room::of(window.viewport_size().width)
    }
}

/// How far a Liquid Glass page's top fade reaches, and its bottom fade past the player bar so
/// content dissolves before it slides under it; and the whole reach of a side panel's fades.
const FADE_TAIL: Pixels = px(48.);

/// Under Liquid Glass, fades a page's content out at its top edge and above the player bar
/// while there is more of it to scroll to that way, as Settings does. A page whose table pins
/// its head to the top keeps that edge sharp, since the fade would dim the head itself.
pub(crate) fn edge_fade(scroller: ui::Scroller, pinned: bool, cx: &App) -> ui::Scroller {
    let top = match pinned {
        true => Pixels::ZERO,
        false => FADE_TAIL,
    };
    match ui::liquid(cx) && ui::effects() {
        true => scroller.edge_fade(top, Chrome::bottom(cx) + FADE_TAIL),
        false => scroller,
    }
}

/// Under Liquid Glass, fades a side panel's content out towards its top and bottom edges while
/// there is more of it to scroll to that way.
pub(crate) fn panel_fade(scroller: ui::Scroller, cx: &App) -> ui::Scroller {
    match ui::liquid(cx) && ui::effects() {
        true => scroller.edge_fade(FADE_TAIL, FADE_TAIL),
        false => scroller,
    }
}
