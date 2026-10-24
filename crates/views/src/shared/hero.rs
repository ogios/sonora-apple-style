use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Div, ElementId, Entity, FontWeight, MouseButton, MouseDownEvent, Pixels,
    SharedString, Window, div, linear_color_stop, linear_gradient, px, relative,
};
use i18n::t;
use music::{Album, Track};
use state::{Origin, Playback};
use ui::{
    ActiveTheme as _, Artwork, Button, ExplicitBadge, LEADING, Pin, Pinnable as _, Skeleton,
    TableState, Text, upper,
};

use crate::shared::tracks::{self, TrackSource};

/// How wide the skeleton bars of a pending hero reach, for the title, the meta line and the
/// row of actions.
const TITLE_SKELETON: Pixels = px(240.);
/// Under Liquid Glass the hero's artwork stands this many times the cover size, and a wash of
/// the cover's colour bleeds this far past the hero on every side, fading out below it, the way
/// Apple Music tints the top of an album page. The page's own clip trims the bleed.
const HERO_GROW: f32 = 1.35;
const HERO_BLEED: Pixels = px(480.);
const HERO_FADE: Pixels = px(160.);
/// How strong the cover's colour is at the top of the wash.
const HERO_WASH: f32 = 0.5;
/// Where down the wash, from its top far above the hero to its foot below it, the colour
/// starts to fade.
const HERO_WASH_HOLD: f32 = 0.4;
/// The fill, rim and vertical padding of a hero fact's pill under Liquid Glass.
const CHIP_FILL: f32 = 0.08;
const CHIP_RIM: f32 = 0.08;
const CHIP_PAD_Y: f32 = 2.;
/// Under Liquid Glass an artist hero stands at least this many times the grown artwork's
/// height, and as tall as this share of the window, as Cider's does. Its picture reaches up to
/// the top of the content sheet and `ARTIST_BLEED` past either side, stays solid down this much
/// of the hero and dissolves over `ARTIST_FADE` below it in this many bands.
const ARTIST_TALL: f32 = 1.6;
const ARTIST_SHARE: f32 = 0.5;
const ARTIST_BLEED: Pixels = px(160.);
const ARTIST_SOLID: f32 = 0.45;
const ARTIST_FADE: Pixels = px(160.);
const ARTIST_BANDS: usize = 24;
/// The picture fills the sheet's width and is cropped top and bottom like a cover, keeping
/// this share of what is cut from above, so a face in the upper part of a portrait stays in.
const ARTIST_FOCUS: f32 = 0.3;
/// How far the dissolving bands are blurred, so the picture turns to frost as it fades.
const ARTIST_FROST: Pixels = px(12.);
/// Below the solid part the sharp picture crossfades into the frosted one over this far, in
/// this many bands.
const ARTIST_SHARPEN: Pixels = px(120.);
const ARTIST_SHARP_BANDS: usize = 10;
/// The scrim under an artist's name: where down the hero it starts, how dark it is at the
/// name, and where along it that is. Below that it deepens to the page colour by the foot.
const ARTIST_SCRIM_FROM: f32 = 0.3;
const ARTIST_SCRIM: f32 = 0.5;
const ARTIST_SCRIM_AT: f32 = 0.55;
/// How much larger than the display size an artist's name is set over its picture.
const ARTIST_NAME: f32 = 1.45;
/// How much of a round play button its glyph spans.
const ROUND_GLYPH: f32 = 0.42;
const META_SKELETON: Pixels = px(180.);
const ACTION_SKELETON: Pixels = px(96.);

/// One copyright line, with a © in front unless the provider already led it with © or ℗.
pub(crate) fn copyright_notice(text: &str) -> SharedString {
    match text.starts_with(['©', '℗']) {
        true => SharedString::from(text.to_owned()),
        false => t!("song-copyright", notice = text),
    }
}

/// The notices an album page prints under its tracks: the provider's copyright lines, or the
/// label behind ℗ when the provider names only that. Empty when it names neither.
pub(crate) fn copyright_notices(album: &Album) -> Vec<SharedString> {
    if !album.copyrights.is_empty() {
        return album
            .copyrights
            .iter()
            .map(|line| copyright_notice(line))
            .collect();
    }
    match album.label.trim() {
        "" => Vec::new(),
        label => vec![t!("album-label", label = label)],
    }
}

pub(crate) fn release_date_label(value: &str) -> SharedString {
    let parts: Vec<_> = value.split('-').collect();
    if parts.len() != 3 {
        return SharedString::from(value.to_owned());
    }
    let month = match parts[1] {
        "01" => t!("month-1"),
        "02" => t!("month-2"),
        "03" => t!("month-3"),
        "04" => t!("month-4"),
        "05" => t!("month-5"),
        "06" => t!("month-6"),
        "07" => t!("month-7"),
        "08" => t!("month-8"),
        "09" => t!("month-9"),
        "10" => t!("month-10"),
        "11" => t!("month-11"),
        "12" => t!("month-12"),
        _ => return SharedString::from(value.to_owned()),
    };
    let day = parts[2].trim_start_matches('0');

    t!("date-full", month = &month, day = day, year = parts[0])
}

#[derive(IntoElement, Default)]
pub(crate) struct HeroMetaStrip {
    items: Vec<AnyElement>,
}

impl HeroMetaStrip {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn text(mut self, text: impl Into<SharedString>) -> Self {
        self.items
            .push(div().flex_none().child(text.into()).into_any_element());
        self
    }

    pub(crate) fn item(mut self, item: impl IntoElement) -> Self {
        self.items.push(item.into_any_element());
        self
    }
}

impl RenderOnce for HeroMetaStrip {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = *cx.theme();
        let mut strip = div()
            .flex()
            .flex_wrap()
            .items_center()
            .min_w_0()
            .gap_1()
            .text_size(theme.text(Text::Small))
            .text_color(theme.muted_foreground);

        // Under Liquid Glass each fact stands on its own small pill, the way Apple Music chips
        // an album's genre, year and length, rather than running in a bulleted line.
        if ui::liquid(cx) {
            let chip = theme.foreground.opacity(CHIP_FILL);
            let rim = theme.foreground.opacity(CHIP_RIM);
            return strip.gap_2().children(self.items.into_iter().map(|item| {
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .px(theme.metrics.pad)
                    .py(px(CHIP_PAD_Y))
                    .rounded_full()
                    .bg(chip)
                    .border_1()
                    .border_color(rim)
                    .child(item)
            }));
        }

        for (index, item) in self.items.into_iter().enumerate() {
            strip = strip.child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap_1()
                    .when(index > 0, |this| this.child("•"))
                    .child(item),
            );
        }

        strip
    }
}

enum Listing {
    Owned(Vec<Track>),
    Listed(Entity<TableState<TrackSource>>),
}

impl Listing {
    fn first(&self, cx: &App) -> Option<usize> {
        match self {
            Self::Owned(tracks) => tracks.iter().position(|track| track.playable),
            Self::Listed(table) => tracks::first_playable(table, cx),
        }
    }

    fn holds(&self, id: &str, cx: &App) -> bool {
        match self {
            Self::Owned(tracks) => tracks.iter().any(|track| track.id.as_deref() == Some(id)),
            Self::Listed(table) => tracks::holds(table, id, cx),
        }
    }

    fn queue(&self, cx: &App) -> Vec<Track> {
        match self {
            Self::Owned(tracks) => tracks.clone(),
            Self::Listed(table) => tracks::ordered(table, cx),
        }
    }

    fn whence(&self, cx: &App) -> Option<Origin> {
        match self {
            Self::Owned(_) => None,
            Self::Listed(table) => tracks::whence(table, cx),
        }
    }
}

#[derive(IntoElement)]
pub(crate) struct HeroPlayButton {
    id: ElementId,
    label: SharedString,
    listing: Listing,
    from: Option<Origin>,
    shuffled: bool,
    /// The side of a round, icon-only button, as the play button over an artist's picture is.
    round: Option<Pixels>,
    playback: Entity<Playback>,
}

impl HeroPlayButton {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        tracks: Vec<Track>,
        playback: Entity<Playback>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            listing: Listing::Owned(tracks),
            from: None,
            shuffled: false,
            round: None,
            playback,
        }
    }

    /// Draws the button as a round glyph `side` across, without its label.
    pub(crate) fn round(mut self, side: Pixels) -> Self {
        self.round = Some(side);
        self
    }

    pub(crate) fn from(mut self, origin: Option<Origin>) -> Self {
        self.from = origin;
        self
    }

    /// The Shuffle button beside Play: an icon-only outline button that turns shuffle on and
    /// starts the tracks from a random one, never mirroring the transport state.
    pub(crate) fn shuffle(
        id: impl Into<ElementId>,
        tracks: Vec<Track>,
        playback: Entity<Playback>,
    ) -> Self {
        Self {
            shuffled: true,
            ..Self::new(id, SharedString::default(), tracks, playback)
        }
    }

    /// `shuffle` over a table's rows, in their displayed order.
    pub(crate) fn shuffle_listed(
        id: impl Into<ElementId>,
        table: &Entity<TableState<TrackSource>>,
        playback: Entity<Playback>,
    ) -> Self {
        Self {
            shuffled: true,
            ..Self::listed(id, SharedString::default(), table, playback)
        }
    }

    pub(crate) fn listed(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        table: &Entity<TableState<TrackSource>>,
        playback: Entity<Playback>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            listing: Listing::Listed(table.clone()),
            from: None,
            shuffled: false,
            round: None,
            playback,
        }
    }

    fn shuffle_button(self, cx: &App) -> Button {
        let disabled = self.listing.first(cx).is_none();
        let listing = self.listing;
        let from = self.from;
        let playback = self.playback;

        Button::new(self.id)
            .icon("icons/shuffle.svg")
            .tooltip("play-shuffle")
            .outline()
            .disabled(disabled)
            .on_click(move |_, _, cx| {
                let queued = listing.queue(cx);
                let from = from.clone().or_else(|| listing.whence(cx));
                playback.update(cx, |playback, cx| playback.shuffle_any(queued, from, cx));
            })
    }
}

impl RenderOnce for HeroPlayButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.shuffled {
            return div().flex().child(self.shuffle_button(cx));
        }
        let playing = {
            let playback = self.playback.read(cx);
            let current = playback.track().and_then(|track| track.id.as_deref());
            current
                .filter(|current| self.listing.holds(current, cx))
                .and_then(|_| playback.control())
        };
        let (label, icon) = match playing {
            Some(true) => (t!("play-pause"), "icons/pause.svg"),
            Some(false) => (t!("play-resume"), "icons/play.svg"),
            None => (self.label, "icons/play.svg"),
        };
        let disabled = self.listing.first(cx).is_none();
        let listing = self.listing;
        let from = self.from;
        let playback = self.playback;

        let round = self.round;
        div().flex().child(
            Button::new(self.id)
                .map(|button| match round {
                    Some(side) => button
                        .size(side)
                        .px_0()
                        .glyph(side * ROUND_GLYPH)
                        .when_some(playing, |button, playing| {
                            button.tooltip(match playing {
                                true => "play-pause",
                                false => "play-resume",
                            })
                        }),
                    None => button.label(label),
                })
                .icon(icon)
                .primary()
                .disabled(disabled)
                .on_click(move |_, _, cx| {
                    playback.update(cx, |playback, cx| match playing {
                        Some(_) => playback.toggle_play(cx),
                        None => {
                            let queued = listing.queue(cx);
                            let from = from.clone().or_else(|| listing.whence(cx));
                            playback.start_any(queued, from, cx)
                        }
                    });
                }),
        )
    }
}

type DragStart = Box<dyn Fn(&MouseDownEvent, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub(crate) struct PageHero {
    id: ElementId,
    title: SharedString,
    cover: Option<String>,
    fallback: Option<SharedString>,
    accent: bool,
    eyebrow: Option<SharedString>,
    meta: Option<AnyElement>,
    actions: Option<AnyElement>,
    circle: bool,
    explicit: bool,
    drag_start: Option<DragStart>,
    pin: Option<Pin>,
    pending: bool,
}

impl PageHero {
    pub(crate) fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            cover: None,
            fallback: None,
            accent: false,
            eyebrow: None,
            meta: None,
            actions: None,
            circle: false,
            explicit: false,
            drag_start: None,
            pin: None,
            pending: false,
        }
    }

    pub(crate) fn pin(mut self, pin: Option<Pin>) -> Self {
        self.pin = pin;
        self
    }

    pub(crate) fn drag_start(
        mut self,
        handler: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.drag_start = Some(Box::new(handler));
        self
    }

    pub(crate) fn cover(mut self, cover: Option<String>) -> Self {
        self.cover = cover;
        self
    }

    pub(crate) fn fallback(mut self, icon: impl Into<SharedString>) -> Self {
        self.fallback = Some(icon.into());
        self
    }

    pub(crate) fn accent(mut self) -> Self {
        self.accent = true;
        self
    }

    pub(crate) fn eyebrow(mut self, eyebrow: impl Into<SharedString>) -> Self {
        self.eyebrow = Some(eyebrow.into());
        self
    }

    pub(crate) fn meta(mut self, meta: impl IntoElement) -> Self {
        self.meta = Some(meta.into_any_element());
        self
    }

    pub(crate) fn actions(mut self, actions: impl IntoElement) -> Self {
        self.actions = Some(actions.into_any_element());
        self
    }

    pub(crate) fn circle(mut self) -> Self {
        self.circle = true;
        self
    }

    pub(crate) fn explicit(mut self, explicit: bool) -> Self {
        self.explicit = explicit;
        self
    }

    /// Draws the cover, title, meta line and actions as skeletons while the page has nothing
    /// to fill them with yet. The eyebrow stays, since it names the kind of page.
    pub(crate) fn pending(mut self, pending: bool) -> Self {
        self.pending = pending;
        self
    }
}

impl RenderOnce for PageHero {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = *cx.theme();

        let art = theme.metrics.cover;
        let drag_start = self.drag_start.map(Rc::from);
        let starter = drag_start.clone();

        if self.pending {
            return div()
                .id(self.id)
                .flex()
                .flex_none()
                .items_end()
                .gap_5()
                .pb_6()
                .child(
                    Skeleton::new()
                        .flex_none()
                        .size(art)
                        .rounded(theme.radius * 1.5)
                        .when(self.circle, |skeleton| skeleton.rounded_full()),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .h(art)
                        .justify_end()
                        .gap_3()
                        .line_height(relative(LEADING))
                        .children(self.eyebrow.map(|eyebrow| eyebrow_label(eyebrow, cx)))
                        .child(
                            Skeleton::new()
                                .w(TITLE_SKELETON)
                                .h(theme.text(Text::Display)),
                        )
                        .child(Skeleton::new().w(META_SKELETON).h(theme.metrics.pad))
                        .child(Skeleton::new().w(ACTION_SKELETON).h(theme.metrics.control)),
                );
        }

        // The wash takes the cover's lead colour once the artwork cache has read it.
        let liquid = ui::liquid(cx);
        let wash = self
            .cover
            .as_deref()
            .filter(|_| liquid)
            .and_then(|cover| ui::cover_palette(cover, cx))
            .and_then(|palette| palette.primary.or(palette.secondary));
        let art = match liquid {
            true => art * HERO_GROW,
            false => art,
        };
        // Under Liquid Glass an artist's picture fills the top of the page edge to edge and
        // dissolves into it, as Apple Music's artist pages do, instead of sitting in a circle.
        let bleed = liquid && self.circle && self.cover.is_some();
        let rise = artist_rise(window, cx);
        let tall = match bleed {
            true => (art * ARTIST_TALL).max(window.viewport_size().height * ARTIST_SHARE - rise),
            false => art * ARTIST_TALL,
        };
        let backdrop = bleed.then(|| artist_backdrop(self.cover.clone(), tall, rise, window, cx));
        let wash = wash.filter(|_| !bleed);

        div()
            .id(self.id)
            .relative()
            .when(bleed, |hero| hero.min_h(tall))
            .children(backdrop)
            .when_some(wash, |hero, wash| {
                hero.child(
                    div()
                        .absolute()
                        .top(-HERO_BLEED)
                        .left(-HERO_BLEED)
                        .right(-HERO_BLEED)
                        .bottom(-HERO_FADE)
                        .bg(linear_gradient(
                            180.,
                            linear_color_stop(wash.opacity(HERO_WASH), HERO_WASH_HOLD),
                            linear_color_stop(wash.opacity(0.), 1.),
                        )),
                )
            })
            .when_some(self.pin, |hero, pin| hero.pin(pin))
            .flex()
            .flex_none()
            .items_end()
            .gap_5()
            .pb_6()
            .when(!bleed, |hero| {
                hero.child(
                    div()
                        .flex_none()
                        .when_some(starter, |this, drag_start: Rc<DragStart>| {
                            this.on_mouse_down(MouseButton::Right, move |event, window, cx| {
                                drag_start(event, window, cx)
                            })
                        })
                        .child(
                            Artwork::new(self.cover)
                                .size(art)
                                .corner_radius(theme.radius * 1.5)
                                .when(self.circle, Artwork::circle)
                                .when_some(self.fallback, Artwork::fallback)
                                .when(self.accent, Artwork::accent),
                        ),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    // At least the artwork's height, and taller when a large font needs it,
                    // so the eyebrow never rises out of the hero.
                    .when(!bleed, |this| this.min_h(art))
                    .justify_end()
                    .gap_2()
                    .line_height(relative(LEADING))
                    .children(
                        self.eyebrow
                            .filter(|_| !bleed)
                            .map(|eyebrow| eyebrow_label(eyebrow, cx)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .min_w_0()
                            .map(|this| match bleed {
                                true => this
                                    .text_size(theme.text(Text::Display) * ARTIST_NAME)
                                    .font_weight(FontWeight::EXTRA_BOLD),
                                false => this
                                    .text_size(theme.text(Text::Display))
                                    .font_weight(FontWeight::BOLD),
                            })
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .when_some(drag_start, |this, drag_start: Rc<DragStart>| {
                                        this.on_mouse_down(
                                            MouseButton::Right,
                                            move |event, window, cx| drag_start(event, window, cx),
                                        )
                                    })
                                    .child(self.title),
                            )
                            .when(self.explicit, |this| {
                                this.child(div().flex_none().child(ExplicitBadge::new()))
                            }),
                    )
                    .children(self.meta)
                    .children(self.actions.map(|actions| div().pt_1().child(actions))),
            )
    }
}

/// How far above an artist hero its picture starts: up past the page's top padding and the
/// strip the title bar stands in, to the top edge of the content sheet.
fn artist_rise(window: &Window, cx: &App) -> Pixels {
    crate::chrome::Chrome::under_bar(window, cx) + cx.theme().metrics.inset
}

/// The full-bleed artist picture behind a Liquid Glass artist hero `tall` high, reaching `rise`
/// above it to the top of the sheet. The picture is laid like a cover over the sheet's width,
/// cropped top and bottom with the cut weighted below the subject, and solid down the upper
/// part of the hero. Below that it frosts over, crossfading into a blurred copy, and the blurred
/// copy dissolves in bands of falling opacity, since a picture cannot be masked by a gradient.
/// A scrim deepens to the page colour by the foot, keeping the name legible over a bright
/// picture and meeting the page without an edge.
fn artist_backdrop(
    cover: Option<String>,
    tall: Pixels,
    rise: Pixels,
    window: &Window,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let height = rise + tall + ARTIST_FADE;
    let edge = crate::chrome::Chrome::content(window, cx).max(height);
    let solid = rise + tall * ARTIST_SOLID;
    let picture = ui::snapped(-(edge - height) * ARTIST_FOCUS, window);
    let band = (height - solid) / ARTIST_BANDS as f32;
    // A strip of the picture at `top` within a box that starts `base` down the backdrop.
    let slice = |base: Pixels, top: Pixels, high: Pixels, opacity: f32| {
        div()
            .absolute()
            .top(top)
            .left_0()
            .right_0()
            .h(high)
            .overflow_hidden()
            .flex()
            .flex_col()
            .items_center()
            .opacity(opacity)
            .child(
                div()
                    .flex_none()
                    .mt(picture - base - top)
                    .child(Artwork::new(cover.clone()).size(edge)),
            )
    };

    // The frosted copy starts a little inside the solid part, so the blur has picture rather
    // than nothing to draw from at its top, where the sharp picture still covers it.
    let reach = ui::snapped(solid - ARTIST_FROST * 2., window);
    let frost = div()
        .absolute()
        .top(reach)
        .left_0()
        .right_0()
        .h(height - reach)
        .blur(ARTIST_FROST)
        .child(slice(reach, Pixels::ZERO, solid - reach, 1.))
        .children((0..ARTIST_BANDS).map(|index| {
            let fade = smooth((index as f32 + 0.5) / ARTIST_BANDS as f32);
            let top = ui::snapped(solid + band * index as f32, window);
            let foot = ui::snapped(solid + band * (index + 1) as f32, window);
            slice(reach, top - reach, foot - top, 1. - fade)
        }));
    // Over it the sharp picture runs on below the solid part and fades out in bands of its
    // own, so the picture frosts over gradually rather than at a line.
    let step = ARTIST_SHARPEN / ARTIST_SHARP_BANDS as f32;
    let sharpening = (0..ARTIST_SHARP_BANDS).map(|index| {
        let fade = smooth((index as f32 + 0.5) / ARTIST_SHARP_BANDS as f32);
        let top = ui::snapped(solid + step * index as f32, window);
        let foot = ui::snapped(solid + step * (index + 1) as f32, window);
        slice(Pixels::ZERO, top, foot - top, 1. - fade)
    });

    div()
        .absolute()
        .top(-rise)
        .left(-ARTIST_BLEED)
        .right(-ARTIST_BLEED)
        .h(height)
        .child(frost)
        .child(slice(
            Pixels::ZERO,
            Pixels::ZERO,
            ui::snapped(solid, window),
            1.,
        ))
        .children(sharpening)
        // The scrim runs from partway down the hero to the picture's foot, so the name stays
        // legible, and ends in the page colour, so the frost meets the page without an edge.
        .child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .top(rise + tall * ARTIST_SCRIM_FROM)
                .bottom_0()
                .bg(linear_gradient(
                    180.,
                    linear_color_stop(theme.background.opacity(0.), 0.),
                    linear_color_stop(theme.background.opacity(ARTIST_SCRIM), ARTIST_SCRIM_AT),
                ))
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .h(relative(1. - ARTIST_SCRIM_AT))
                        .bg(linear_gradient(
                            180.,
                            linear_color_stop(theme.background.opacity(0.), 0.),
                            linear_color_stop(theme.background, 1.),
                        )),
                ),
        )
        .into_any_element()
}

/// Smoothstep over `0..=1`, the ease the dissolving bands fall off by.
fn smooth(through: f32) -> f32 {
    through * through * (3. - 2. * through)
}

fn eyebrow_label(label: SharedString, cx: &App) -> Div {
    let theme = cx.theme();

    div()
        .text_size(theme.text(Text::Small))
        .text_color(theme.muted_foreground)
        .font_weight(FontWeight::SEMIBOLD)
        .child(upper(label))
}
