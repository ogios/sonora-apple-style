use gpui::prelude::*;
use gpui::{AnyElement, App, Context, Div, Hsla, Pixels, Window, div, relative};
use i18n::t;
use state::PlayerBarLayout;
use ui::{ActiveTheme as _, OptionCard, Switch};

use super::rows::grid;
use super::{Group, Row, SettingsView, Slot};

impl SettingsView {
    /// The player bar's own section in Appearance: layout cards followed by its material.
    pub(super) fn player_bar_groups(&self) -> Vec<Group> {
        vec![
            Group::new("settings-player-bar-layout", "icons/layout-grid.svg")
                .cards(Slot::PlayerBarLayout),
            Group::new("settings-player-bar-surface", "icons/blend.svg")
                .rows([Slot::PlayerBarTransparent]),
        ]
    }

    /// Miniature layouts share the saved choice with the actual workspace player bar.
    pub(super) fn player_bar_cards(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let chosen = self.settings.read(cx).player_bar_layout();
        let transparent = self.settings.read(cx).player_bar_translucent();
        let height = cx.theme().metrics.list_row * 1.6;
        grid(PlayerBarLayout::ALL.len() as u16, window, cx)
            .children(PlayerBarLayout::ALL.map(|layout| {
                OptionCard::new(layout.id())
                    .thumb_height(height)
                    .thumb(sketch(layout, transparent, cx))
                    .label(i18n::lookup(layout.key(), None))
                    .selected(layout == chosen)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.settings.update(cx, |settings, cx| {
                            settings.set_player_bar_layout(layout, cx);
                        });
                    }))
            }))
            .into_any_element()
    }

    /// Explains that transparency also moves the page behind the player bar. The switch stands
    /// off and held while Blur UI is off, since the bar cannot be see-through without it, and
    /// on and held under Liquid Glass, whose bar is always glass with the pages under it.
    pub(super) fn player_bar_transparent_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let settings = self.settings.read(cx);
        let on = settings.player_bar_transparent();
        let blur = settings.blur();
        let liquid = ui::liquid(cx);
        Row::new(
            t!("settings-player-bar-transparent"),
            Switch::new("player-bar-transparent", liquid || (on && blur))
                .disabled(liquid || !blur)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.settings.update(cx, |settings, cx| {
                        settings.set_player_bar_transparent(!on, cx);
                    });
                }))
                .into_any_element(),
        )
        .detail(match (liquid, blur) {
            (true, _) => t!("settings-player-bar-transparent-liquid"),
            (false, true) => t!("settings-player-bar-transparent-detail"),
            (false, false) => t!("settings-player-bar-transparent-needs-blur"),
        })
        .into_any_element()
    }
}

/// A still miniature of a page and its player bar. Coordinates are proportions of the panel,
/// so each layout keeps its shape when the settings column changes width.
fn sketch(layout: PlayerBarLayout, transparent: bool, cx: &App) -> Div {
    let theme = *cx.theme();
    let radius = theme.radius * 0.25;
    let ink = theme.foreground.opacity(0.6);
    let muted = theme.muted_foreground.opacity(0.4);
    let height = match layout {
        PlayerBarLayout::Classic => 0.44,
        PlayerBarLayout::Compact => 0.26,
        PlayerBarLayout::Centered => 0.44,
    };
    let (cover, title, artist, controls, seek, sound) = match layout {
        PlayerBarLayout::Classic => (
            [0.04, 0.15, 0.70],
            [0.18, 0.28, 0.17, 0.08],
            [0.18, 0.48, 0.12, 0.06],
            [0.46, 0.20, 0.21, 0.34],
            [0.41, 0.77, 0.32, 0.05],
            [0.82, 0.48, 0.13, 0.05],
        ),
        PlayerBarLayout::Compact => (
            [0.04, 0.15, 0.70],
            [0.13, 0.25, 0.12, 0.10],
            [0.13, 0.56, 0.09, 0.08],
            [0.33, 0.26, 0.17, 0.46],
            [0.55, 0.48, 0.22, 0.08],
            [0.82, 0.48, 0.13, 0.08],
        ),
        PlayerBarLayout::Centered => (
            [0.04, 0.15, 0.70],
            [0.40, 0.29, 0.20, 0.08],
            [0.43, 0.44, 0.14, 0.06],
            [0.16, 0.36, 0.12, 0.28],
            [0.38, 0.65, 0.24, 0.05],
            [0.82, 0.48, 0.13, 0.05],
        ),
    };
    let panel = div()
        .absolute()
        .bottom_0()
        .w_full()
        .h(relative(height))
        .border_t_1()
        .border_color(theme.border)
        .map(|panel| match transparent {
            true => ui::popover_surface(panel, cx),
            false => panel.bg(theme.secondary),
        })
        .child(artwork(cover, theme.primary.opacity(0.6), radius))
        .child(tile(title, ink, radius))
        .child(tile(artist, muted, radius))
        .child(transport(controls, cx))
        .child(tile(seek, muted, radius))
        .child(tile(
            [seek[0], seek[1], seek[2] * 0.4, seek[3]],
            ink,
            radius,
        ))
        .child(tile(sound, muted, radius));

    div()
        .relative()
        .size_full()
        .bg(theme.background)
        .child(tile([0.05, 0.10, 0.35, 0.05], muted, radius))
        .children((0..4).map(|index| {
            tile(
                [0.05 + index as f32 * 0.24, 0.28, 0.18, 0.46],
                theme.primary.opacity(0.15),
                radius,
            )
        }))
        .child(panel)
}

/// One simplified piece of artwork, text or progress in a miniature.
fn tile([left, top, width, height]: [f32; 4], fill: Hsla, radius: Pixels) -> Div {
    div()
        .absolute()
        .left(relative(left))
        .top(relative(top))
        .w(relative(width))
        .h(relative(height))
        .rounded(radius)
        .bg(fill)
}

/// The cover in a miniature. It is square whatever the panel's proportions, so only its left
/// edge, top edge and height are given.
fn artwork([left, top, height]: [f32; 3], fill: Hsla, radius: Pixels) -> Div {
    div()
        .absolute()
        .left(relative(left))
        .top(relative(top))
        .h(relative(height))
        .aspect_square()
        .rounded(radius)
        .bg(fill)
}

/// Muted abstract button marks suggest the controls without reproducing detailed live icons.
/// They sit close together in the middle of their slot, as the real buttons do.
fn transport([left, top, width, height]: [f32; 4], cx: &App) -> Div {
    let theme = cx.theme();
    let mark = theme.metrics.control_small * 0.22;
    div()
        .absolute()
        .left(relative(left))
        .top(relative(top))
        .w(relative(width))
        .h(relative(height))
        .flex()
        .items_center()
        .justify_center()
        .gap(mark * 0.8)
        .children((0..3).map(|_| {
            div()
                .size(mark)
                .rounded(theme.radius * 0.15)
                .bg(theme.muted_foreground.opacity(0.45))
        }))
}
