use std::time::Duration;

use gpui::prelude::*;
use gpui::{App, Entity, SharedString, div};
use i18n::t;
use state::{Playback, Queue, Repeat, Sonora, Steered};
use ui::{ActiveTheme as _, Button, Text};

use crate::shared::ambient;

pub(crate) const NOTCH: f32 = 0.05;
const STEP: f32 = 0.004;
/// The roomy transport under Liquid Glass: how much taller than a standard control each button
/// stands, how much larger the play button is again, and how large their glyphs are against
/// the body text.
const ROOMY_GROW: f32 = 1.15;
const ROOMY_PLAY: f32 = 1.3;
const ROOMY_GLYPH: f32 = 1.45;
const ROOMY_PLAY_GLYPH: f32 = 1.85;
/// How much larger again the roomy transport stands in the Liquid Glass fullscreen player.
const FULLSCREEN_GROW: f32 = 1.3;

/// The other device playback is on, which the player shows instead of this app's own.
pub(crate) fn steered(cx: &App) -> Option<Steered> {
    Sonora::global(cx).devices.read(cx).steered()
}

/// How far through `total` `elapsed` is, from 0 to 1.
pub(crate) fn fraction(elapsed: Duration, total: Duration) -> f32 {
    match total.is_zero() {
        true => 0.,
        false => (elapsed.as_secs_f32() / total.as_secs_f32()).clamp(0., 1.),
    }
}

pub(crate) fn volume_icon(level: f32) -> &'static str {
    match level {
        level if level <= 0.0001 => "icons/volume-x.svg",
        level if level < 0.25 => "icons/volume.svg",
        level if level < 0.5 => "icons/volume-1.svg",
        _ => "icons/volume-2.svg",
    }
}

pub(crate) fn percent(fraction: f32) -> SharedString {
    t!("player-percent", value = (fraction * 100.).round())
}

/// The heart that saves or drops `track`. A click toggles the track `playback` holds by then,
/// which is the one shown, so callers only lend the track to read its state.
pub(crate) fn like(track: Option<&music::Track>, playback: &Entity<Playback>, cx: &App) -> Button {
    let theme = *cx.theme();
    let library = Sonora::global(cx).library.clone();
    let playback = playback.clone();
    let id = track.and_then(|track| track.id.as_deref());
    let saved = id.is_some_and(|id| library.read(cx).saved(id));

    Button::new("toggle-liked-track")
        .ghost()
        .backgroundless()
        .small()
        .icon(match saved {
            true => "icons/heart-filled.svg",
            false => "icons/heart.svg",
        })
        .tooltip_above(match saved {
            true => "menu-remove-from-library",
            false => "menu-add-to-library",
        })
        .tint(match saved {
            true => theme.primary,
            false => ui::liquid_secondary(cx),
        })
        .disabled(id.is_none())
        .on_click(move |_, _, cx| {
            if let Some(track) = playback.read(cx).track().cloned() {
                library.update(cx, |library, cx| library.toggle(track, cx));
            }
        })
}

/// `big` is only true in fullscreen, where these buttons float over the ambient
/// background and so blur their hover fills. Without that background, and on
/// the flat player bar, the blur would show nothing.
pub(crate) fn transport(
    playback: &Entity<Playback>,
    queue: &Entity<Queue>,
    big: bool,
    cx: &App,
) -> impl IntoElement {
    let frosted = big && ambient::shown(cx);
    if big && ui::liquid(cx) {
        return roomy(playback, queue, frosted, FULLSCREEN_GROW, cx).into_any_element();
    }

    div()
        .flex()
        .items_center()
        .gap_2()
        .child(shuffle(playback, frosted, cx))
        .child(previous(playback, frosted, cx))
        .child(toggle(playback, big, frosted, cx))
        .child(next(playback, queue, frosted, cx))
        .child(repeat(playback, frosted, cx))
        .into_any_element()
}

/// The transport the Liquid Glass player bar carries: the same five buttons, round and large
/// enough to hit without aiming, the play button the largest of them, as Cider's are.
pub(crate) fn roomy_transport(
    playback: &Entity<Playback>,
    queue: &Entity<Queue>,
    cx: &App,
) -> impl IntoElement {
    roomy(playback, queue, false, 1., cx)
}

/// The roomy transport, `grow` times its player-bar size. Over the fullscreen ambient field,
/// when `frosted`, its buttons are bare glyphs that only glow under the pointer, since the
/// glass bar they sit on is already the material.
fn roomy(
    playback: &Entity<Playback>,
    queue: &Entity<Queue>,
    frosted: bool,
    grow: f32,
    cx: &App,
) -> gpui::Div {
    let theme = *cx.theme();
    let side = theme.metrics.control * ROOMY_GROW * grow;
    let glyph = theme.text(Text::Body) * ROOMY_GLYPH * grow;
    let bare = |button: Button| button.when(frosted, Button::bare);
    let sized = |button: Button| bare(button).size(side).px_0().glyph(glyph);

    div()
        .flex()
        .items_center()
        .gap_2()
        .child(sized(shuffle(playback, frosted, cx)))
        .child(sized(previous(playback, frosted, cx)))
        .child(
            bare(toggle(playback, false, frosted, cx))
                .size(side * ROOMY_PLAY)
                .px_0()
                .glyph(theme.text(Text::Body) * ROOMY_PLAY_GLYPH * grow),
        )
        .child(sized(next(playback, queue, frosted, cx)))
        .child(sized(repeat(playback, frosted, cx)))
}

pub(crate) fn toggle(playback: &Entity<Playback>, big: bool, frosted: bool, cx: &App) -> Button {
    let held = playback.read(cx);
    let steered = steered(cx);
    let playing = steered
        .as_ref()
        .map_or_else(|| held.wants_playing(), |steered| steered.playing);
    let idle = held.track().is_none() && steered.is_none();
    let playback = playback.clone();

    let (id, icon, tooltip) = match playing {
        true => ("play-pause", "icons/pause.svg", "play-pause"),
        false => ("play-pause", "icons/play.svg", "play-resume"),
    };

    Button::new(id)
        .ghost()
        .when(frosted, Button::frosted)
        .when(!big, Button::small)
        .icon(icon)
        .tooltip_above(tooltip)
        .disabled(idle)
        .on_click(move |_, _, cx| {
            playback.update(cx, |playback, cx| playback.toggle_play(cx));
        })
}

fn shuffle(playback: &Entity<Playback>, frosted: bool, cx: &App) -> Button {
    let theme = *cx.theme();
    let on = playback.read(cx).shown_shuffle(cx);
    let playback = playback.clone();

    Button::new("shuffle")
        .ghost()
        .when(frosted, Button::frosted)
        .small()
        .icon("icons/shuffle.svg")
        .indicator(on)
        .tooltip_above("player-shuffle")
        .tint(match on {
            true => theme.primary,
            false => ui::liquid_secondary(cx),
        })
        .on_click(move |_, _, cx| {
            playback.update(cx, |playback, cx| playback.toggle_shuffle(cx));
        })
}

fn repeat(playback: &Entity<Playback>, frosted: bool, cx: &App) -> Button {
    let theme = *cx.theme();
    let repeat = playback.read(cx).shown_repeat(cx);
    let playback = playback.clone();

    Button::new("repeat")
        .ghost()
        .when(frosted, Button::frosted)
        .small()
        .icon(match repeat {
            Repeat::One => "icons/repeat-one.svg",
            _ => "icons/repeat.svg",
        })
        .indicator(repeat != Repeat::Off)
        .tooltip_above(match repeat {
            Repeat::Off => "player-repeat",
            Repeat::All => "player-repeat-all",
            Repeat::One => "player-repeat-one",
        })
        .tint(match repeat {
            Repeat::Off => ui::liquid_secondary(cx),
            _ => theme.primary,
        })
        .on_click(move |_, _, cx| {
            playback.update(cx, |playback, cx| playback.cycle_repeat(cx));
        })
}

fn previous(playback: &Entity<Playback>, frosted: bool, cx: &App) -> Button {
    let enabled = steered(cx).is_some() || playback.read(cx).has_previous(cx);
    let playback = playback.clone();

    Button::new("previous")
        .ghost()
        .when(frosted, Button::frosted)
        .small()
        .icon("icons/skip-back.svg")
        .tooltip_above("player-previous")
        .disabled(!enabled)
        .on_click(move |_, _, cx| {
            playback.update(cx, |playback, cx| playback.previous(cx));
        })
}

fn next(playback: &Entity<Playback>, queue: &Entity<Queue>, frosted: bool, cx: &App) -> Button {
    let enabled = steered(cx).is_some() || queue.read(cx).has_next();
    let playback = playback.clone();

    Button::new("next")
        .ghost()
        .when(frosted, Button::frosted)
        .small()
        .icon("icons/skip-forward.svg")
        .tooltip_above("player-next")
        .disabled(!enabled)
        .on_click(move |_, _, cx| {
            playback.update(cx, |playback, cx| playback.next(cx));
        })
}

pub(crate) fn moved(before: Option<f32>, after: Option<f32>) -> bool {
    match (before, after) {
        (Some(before), Some(after)) => (before - after).abs() > STEP,
        (before, after) => before.is_some() != after.is_some(),
    }
}
