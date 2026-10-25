use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    AnyElement, App, ClickEvent, Context, Entity, FocusHandle, Focusable, FontWeight, MouseButton,
    MouseDownEvent, Pixels, Point, Render, ScrollHandle, SharedString, WeakEntity, Window, div, px,
    svg,
};
use i18n::t;
use input::SEARCH_CONTEXT;
use music::{Album, Playlist, ReleaseType, SavedArtist, Track};
use router::{Destination, navigate};
use ui::Input;

use crate::chrome::Chrome;
use crate::shared::menus::{ItemMenu, album_menu, artist_menu, playlist_menu};
use state::{
    AlbumHit, ArtistHit, Genres, Hit, Kind, Network, Playback, PlaylistHit, Search, Sonora,
};
use ui::ActiveTheme as _;
use ui::{
    Activate, Button, Card, Deck, Deselect, Pinnable, Popup, Room, Scrollbar, Scroller, SelectLeft,
    SelectNext, SelectPrevious, SelectRight, Separator, TabBar, Text, Theme, VAST, clock, eyebrow,
    scrolled, snapped, tabular, vacant,
};

use crate::shared::cards;
use crate::shared::cells;
use crate::shared::pins::Pinned as _;
use crate::shared::shelves;
use crate::shared::trouble;

const RAIL: Pixels = gpui::px(12.);
/// The widest a recent search's chip grows before its text fades out.
const RECENT_WIDE: f32 = 280.;
const ROW_GAP: f32 = 0.25;
/// A Liquid Glass result row, Cider's: how much taller than the theme's list row it stands,
/// the gap between rows in text heights, its artwork as a share of its height, and its title
/// this much larger than the body text.
const LIQUID_AIR: Pixels = gpui::px(18.);
const LIQUID_GAP: f32 = 0.5;
const LIQUID_ART: f32 = 0.8;
const LIQUID_TITLE: f32 = 1.12;
const SONGS: &[Kind] = &[Kind::Song];
const ARTISTS: &[Kind] = &[Kind::Artist];
const RELEASES: &[Kind] = &[Kind::Album, Kind::Playlist];
const ALBUMS: &[Kind] = &[Kind::Album];
const PLAYLISTS: &[Kind] = &[Kind::Playlist];
/// The tabs Liquid Glass sorts the results under, Cider's four, in order, with their names.
const TABS: [(Kind, &str); 4] = [
    (Kind::Song, "search-songs"),
    (Kind::Artist, "search-artists"),
    (Kind::Album, "search-albums"),
    (Kind::Playlist, "search-playlists"),
];
use crate::shared::tracks::{PlaybackStatus, playback_status};

type Play = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

enum Press {
    Song(Box<Track>),
    Artist(String),
    Album(String),
    Playlist(String),
}

#[derive(Clone)]
enum HitMenu {
    Song(Box<Track>),
    Album(AlbumHit),
    Playlist(PlaylistHit),
    Artist(ArtistHit),
}

impl HitMenu {
    fn of(hit: &Hit) -> Option<Self> {
        match hit {
            Hit::Song(track) => Some(Self::Song(Box::new(track.clone()))),
            Hit::Album(album) => Some(Self::Album(album.clone())),
            Hit::Playlist(list) => Some(Self::Playlist(list.clone())),
            Hit::Artist(artist) if artist.id.is_some() => Some(Self::Artist(artist.clone())),
            Hit::Artist(_) => None,
        }
    }
}

pub(crate) struct SearchView {
    input: Entity<Input>,
    search: Entity<Search>,
    genres: Entity<Genres>,
    playback: Entity<Playback>,
    playback_status: PlaybackStatus,
    songs: Entity<Scrollbar>,
    artists: Entity<Scrollbar>,
    albums: Entity<Scrollbar>,
    mixed: Entity<Scrollbar>,
    browsing: Entity<Scrollbar>,
    track_menu: ItemMenu,
    context_menu: Option<(HitMenu, Point<Pixels>)>,
    focus: FocusHandle,
    cursor: Option<(usize, usize)>,
    rows: [usize; 3],
    lead: Rc<Cell<Pixels>>,
    /// The kind of result the Liquid Glass tabs show.
    tab: Kind,
}

impl SearchView {
    pub(crate) fn new(
        search: Entity<Search>,
        genres: Entity<Genres>,
        playback: Entity<Playback>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&genres, |_, _, cx| cx.notify()).detach();
        genres.update(cx, |genres, cx| genres.load(cx));
        let input = cx.new(|cx| {
            Input::new("search-placeholder", cx)
                .icon("icons/search.svg")
                .clearable()
        });

        cx.observe(&input, |this, input, cx| {
            let query = input.read(cx).text().to_owned();
            this.search.update(cx, |search, cx| search.ask(&query, cx));
        })
        .detach();

        cx.observe(&search, |this, _, cx| {
            this.track_menu.reset(cx);
            this.context_menu = None;
            this.cursor = None;
            this.rows = [0; 3];
            cx.notify();
        })
        .detach();
        let chrome = Chrome::entity(cx);
        cx.observe(&chrome, |_, _, cx| cx.notify()).detach();
        let library = Sonora::global(cx).library.clone();
        cx.observe(&library, |_, _, cx| cx.notify()).detach();
        let current_playback = playback_status(&playback, cx);
        cx.observe(&playback, |this, playback, cx| {
            let current = playback_status(&playback, cx);
            if this.playback_status != current {
                this.playback_status = current;
                cx.notify();
            }
        })
        .detach();

        let asked = input.read(cx).text().to_owned();
        search.update(cx, |search, cx| search.ask(&asked, cx));

        let me = cx.entity_id();
        let playlist_scrollbar = cx.new(|_| Scrollbar::inset().watching(me));

        Self {
            input,
            search,
            genres,
            playback,
            playback_status: current_playback,
            songs: cx.new(|_| Scrollbar::new(ScrollHandle::new()).watching(me)),
            artists: cx.new(|_| Scrollbar::new(ScrollHandle::new()).watching(me)),
            albums: cx.new(|_| Scrollbar::new(ScrollHandle::new()).watching(me)),
            mixed: cx.new(|_| Scrollbar::new(ScrollHandle::new()).watching(me)),
            browsing: cx.new(|_| Scrollbar::new(ScrollHandle::new()).watching(me)),
            track_menu: ItemMenu::new(playlist_scrollbar, cx),
            context_menu: None,
            focus: cx.focus_handle(),
            cursor: None,
            rows: [0; 3],
            lead: Rc::new(Cell::new(Pixels::ZERO)),
            tab: Kind::Song,
        }
    }

    pub(crate) fn focus(&self, window: &mut Window, cx: &mut App) {
        self.input.update(cx, |input, cx| input.focus(window, cx));
    }

    fn select_next(&mut self, _: &SelectNext, window: &mut Window, cx: &mut Context<Self>) {
        if self.search.read(cx).query().trim().is_empty() {
            return;
        }
        let stacked = stacked(window, cx);
        match self.cursor {
            None => {
                let Some(column) = self.first_filled(0, stacked, cx) else {
                    return;
                };
                self.place(column, 0);
            }
            Some((column, row)) => {
                let last = self
                    .seats(kinds(column, stacked), cx)
                    .len()
                    .saturating_sub(1);
                if row < last {
                    self.place(column, row + 1);
                }
            }
        }
        window.focus(&self.focus, cx);
        self.reveal(window, cx);
        cx.notify();
    }

    fn select_previous(&mut self, _: &SelectPrevious, window: &mut Window, cx: &mut Context<Self>) {
        let Some((column, row)) = self.cursor else {
            return;
        };
        if row == 0 {
            self.cursor = None;
            self.input.update(cx, |input, cx| input.focus(window, cx));
            cx.notify();
            return;
        }
        self.place(column, row - 1);
        window.focus(&self.focus, cx);
        self.reveal(window, cx);
        cx.notify();
    }

    fn select_left(&mut self, _: &SelectLeft, window: &mut Window, cx: &mut Context<Self>) {
        let stacked = stacked(window, cx);
        let Some((column, _)) = self.cursor else {
            return;
        };
        let Some(next) = self.prev_filled(column, stacked, cx) else {
            return;
        };
        self.hop(next, stacked, cx);
        window.focus(&self.focus, cx);
        self.reveal(window, cx);
        cx.notify();
    }

    fn select_right(&mut self, _: &SelectRight, window: &mut Window, cx: &mut Context<Self>) {
        let stacked = stacked(window, cx);
        let Some((column, _)) = self.cursor else {
            return;
        };
        let Some(next) = self.next_filled(column, stacked, cx) else {
            return;
        };
        self.hop(next, stacked, cx);
        window.focus(&self.focus, cx);
        self.reveal(window, cx);
        cx.notify();
    }

    fn activate_hit(&mut self, _: &Activate, window: &mut Window, cx: &mut Context<Self>) {
        let Some((column, row)) = self.cursor else {
            return;
        };
        self.remember(cx);
        let stacked = stacked(window, cx);
        let seats = self.seats(self.kinds(column, stacked, cx), cx);
        let Some(&(at, _)) = seats.get(row) else {
            return;
        };
        let Some(hit) = self.search.read(cx).hits().get(at).cloned() else {
            return;
        };
        match hit {
            Hit::Song(track) => {
                let current = track.id.is_some() && track.id == self.playback_status.0;
                self.playback.update(cx, |playback, cx| match current {
                    true => playback.toggle_play(cx),
                    false => playback.play_radio(&track, cx),
                });
            }
            Hit::Artist(artist) => {
                if let Some(id) = artist.id {
                    navigate(Destination::Artist(id.into()), cx);
                }
            }
            Hit::Album(album) => navigate(Destination::Album(album.id.into()), cx),
            Hit::Playlist(list) => navigate(Destination::Playlist(list.id.into()), cx),
        }
    }

    /// Keeps what is in the search field among the recent searches, once the user has acted
    /// on it: pressed Enter in the field or opened one of its results.
    fn remember(&self, cx: &mut Context<Self>) {
        let query = self.input.read(cx).text().to_owned();
        let settings = Sonora::global(cx).settings.clone();
        settings.update(cx, |settings, cx| settings.remember_search(&query, cx));
        cx.notify();
    }

    /// Searches for a recent query again, moving it back to the top.
    fn recall(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| {
            input.set_text(query.to_owned(), cx);
            input.focus(window, cx);
        });
        self.remember(cx);
    }

    /// The recent searches over the browse grid while the field is empty: each one a chip
    /// that runs it again, with a cross that forgets it, and a button that forgets them all.
    fn recent(&self, gutter: Pixels, cx: &Context<Self>) -> Option<AnyElement> {
        let history = Sonora::global(cx)
            .settings
            .read(cx)
            .search_history()
            .to_vec();
        if history.is_empty() {
            return None;
        }
        let theme = *cx.theme();
        let liquid = ui::liquid(cx);
        let clear = Button::new("search-recent-clear")
            .ghost()
            .small()
            .label(t!("search-recent-clear"))
            .on_click(cx.listener(|_, _, _, cx| {
                let settings = Sonora::global(cx).settings.clone();
                settings.update(cx, |settings, cx| settings.clear_search_history(cx));
                cx.notify();
            }));
        let chips = history.into_iter().enumerate().map(|(place, query)| {
            let asked = query.clone();
            let forgotten = query.clone();
            div()
                .id(("search-recent", place))
                .flex()
                .flex_none()
                .items_center()
                .gap_1()
                .max_w(px(RECENT_WIDE))
                .h(theme.metrics.control_small)
                .pl_3()
                .pr_1()
                .map(|chip| match liquid {
                    true => chip.rounded_full(),
                    false => chip.rounded(theme.radius),
                })
                .bg(theme.secondary)
                .cursor_pointer()
                .hover(|chip| chip.bg(theme.secondary_hover))
                .on_click(cx.listener(move |this, _, window, cx| this.recall(&asked, window, cx)))
                .child(
                    svg()
                        .path(icons::path("icons/rotate-ccw-clock.svg"))
                        .size(theme.text(Text::Body))
                        .flex_none()
                        .text_color(theme.muted_foreground),
                )
                .child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .child(ui::fade_line(SharedString::from(query))),
                )
                .child(
                    Button::new(("search-recent-remove", place))
                        .ghost()
                        .small()
                        .icon("icons/x.svg")
                        .tooltip("search-recent-remove")
                        .on_click(cx.listener(move |_, _, _, cx| {
                            cx.stop_propagation();
                            let settings = Sonora::global(cx).settings.clone();
                            settings
                                .update(cx, |settings, cx| settings.forget_search(&forgotten, cx));
                            cx.notify();
                        })),
                )
        });

        Some(
            div()
                .flex()
                .flex_col()
                .flex_none()
                .gap_2()
                .px(gutter)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(eyebrow(t!("search-recent"), cx))
                        .child(clear),
                )
                .child(div().flex().flex_wrap().gap_2().children(chips))
                .into_any_element(),
        )
    }

    fn deselect_hit(&mut self, _: &Deselect, window: &mut Window, cx: &mut Context<Self>) {
        if self.cursor.take().is_none() {
            return;
        }
        self.input.update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    fn place(&mut self, column: usize, row: usize) {
        if let Some(slot) = self.rows.get_mut(column) {
            *slot = row;
        }
        self.cursor = Some((column, row));
    }

    fn hop(&mut self, column: usize, stacked: bool, cx: &App) {
        let last = self
            .seats(kinds(column, stacked), cx)
            .len()
            .saturating_sub(1);
        let row = self.rows.get(column).copied().unwrap_or(0).min(last);
        self.place(column, row);
    }

    /// The kinds of result a column lists: the chosen tab's under Liquid Glass, and otherwise
    /// the column's own, or every kind when the columns are stacked.
    fn kinds(&self, column: usize, stacked: bool, cx: &App) -> &'static [Kind] {
        match ui::liquid(cx) {
            true => tab_kinds(self.tab),
            false => kinds(column, stacked),
        }
    }

    /// How many columns the results stand in: one under Liquid Glass, whose tabs show one kind.
    fn columns(&self, stacked: bool, cx: &App) -> usize {
        match ui::liquid(cx) {
            true => 1,
            false => columns(stacked),
        }
    }

    fn first_filled(&self, start: usize, stacked: bool, cx: &App) -> Option<usize> {
        (start..self.columns(stacked, cx))
            .find(|&column| !self.seats(self.kinds(column, stacked, cx), cx).is_empty())
    }

    fn next_filled(&self, column: usize, stacked: bool, cx: &App) -> Option<usize> {
        self.first_filled(column + 1, stacked, cx)
    }

    fn prev_filled(&self, column: usize, stacked: bool, cx: &App) -> Option<usize> {
        (0..column)
            .rev()
            .find(|&at| !self.seats(self.kinds(at, stacked, cx), cx).is_empty())
    }

    fn reveal(&self, window: &Window, cx: &App) {
        let Some((column, row)) = self.cursor else {
            return;
        };
        let stacked = stacked(window, cx);
        let liquid = ui::liquid(cx);
        let scroll = match liquid {
            true => self.tab_bar(),
            false => self.bar(column, stacked),
        }
        .read(cx)
        .scroll()
        .clone();
        let height = snapped(hit_height(cx), window);
        let gap = hit_gap(cx);
        let above = match stacked && !liquid {
            true => self.lead.get(),
            false => Pixels::ZERO,
        };
        let top = (height + gap) * row as f32 + above;
        let visible = scroll.bounds().size.height;
        if visible <= Pixels::ZERO {
            return;
        }
        let offset = scroll.offset();
        let shown = -offset.y;
        let delta = if top < shown {
            top - shown
        } else if top + height > shown + visible {
            top + height - (shown + visible)
        } else {
            return;
        };
        scroll.set_offset(gpui::point(offset.x, offset.y - delta));
    }

    fn bar(&self, column: usize, stacked: bool) -> &Entity<Scrollbar> {
        match (stacked, column) {
            (true, _) => &self.mixed,
            (false, 0) => &self.songs,
            (false, 1) => &self.artists,
            _ => &self.albums,
        }
    }

    /// The scrollbar of the list the chosen Liquid Glass tab shows.
    fn tab_bar(&self) -> &Entity<Scrollbar> {
        match self.tab {
            Kind::Song => &self.songs,
            Kind::Artist => &self.artists,
            Kind::Album => &self.albums,
            Kind::Playlist => &self.mixed,
        }
    }

    /// Cider's search results under Liquid Glass: a bar of four tabs, one per kind, over the
    /// list of the chosen kind across the page's width.
    fn tabbed(&self, gutter: Pixels, window: &Window, cx: &Context<Self>) -> AnyElement {
        let bar = TabBar::new("search-tabs").items(TABS.map(|(kind, key)| {
            Button::new(key)
                .label(i18n::lookup(key, None))
                .ghost()
                .selected(self.tab == kind)
                .on_click(cx.listener(move |this, _, _, cx| {
                    if this.tab != kind {
                        this.tab = kind;
                        this.cursor = None;
                        cx.notify();
                    }
                }))
        }));

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap_4()
            .child(div().flex().px(gutter).child(bar))
            .child(self.deck(tab_kinds(self.tab), gutter, None, window, cx))
            .into_any_element()
    }

    fn subtitle(&self, hit: &Hit, place: usize, compact: bool, theme: &Theme) -> AnyElement {
        if let (Hit::Album(album), false) = (hit, compact) {
            return cards::released(
                format!("album-artist-{place}"),
                album.year,
                None,
                &album.artist_refs,
                album.artists.clone(),
                theme,
            )
            .into_any_element();
        }

        let (kind, id, artists, fallback) = match hit {
            Hit::Song(track) => (
                Kind::Song,
                format!("song-artist-{place}"),
                track.artist_refs.clone(),
                track.artists.clone(),
            ),
            Hit::Album(album) => (
                Kind::Album,
                format!("album-artist-{place}"),
                album.artist_refs.clone(),
                album.artists.clone(),
            ),
            Hit::Playlist(list) if !list.owner.is_empty() => (
                Kind::Playlist,
                format!("playlist-owner-{place}"),
                Vec::new(),
                list.owner.clone(),
            ),
            Hit::Artist(_) | Hit::Playlist(_) => {
                return meta(hit, compact).into_any_element();
            }
        };

        let links = cells::artist_links(id, artists, fallback, theme.muted_foreground).truncate();

        match compact {
            true => div()
                .flex()
                .min_w_0()
                .gap_1()
                .child(div().flex_none().child(tag(kind)))
                .child(links)
                .into_any_element(),
            false => links.into_any_element(),
        }
    }

    fn row(
        &self,
        hit: &Hit,
        place: usize,
        compact: bool,
        chosen: bool,
        me: &WeakEntity<Self>,
        cx: &App,
    ) -> AnyElement {
        let theme = *cx.theme();
        let meta = self.subtitle(hit, place, compact, &theme);

        let card = match hit {
            Hit::Song(track) => {
                let current = track.id.is_some() && track.id == self.playback_status.0;
                let tint = match current {
                    true => theme.primary,
                    false => theme.foreground,
                };
                Card::new(("song", place), track.name.clone())
                    .cover(track.cover.clone())
                    .tint(tint)
                    .meta(meta)
                    .when(track.explicit, |card| card.explicit())
                    .trailing(
                        div()
                            .flex_none()
                            .whitespace_nowrap()
                            .text_size(theme.text(Text::Small))
                            .text_color(theme.muted_foreground)
                            .font_features(tabular())
                            .child(clock(track.duration)),
                    )
                    .press(pressed(Press::Song(Box::new(track.clone())), me))
            }
            Hit::Artist(artist) => {
                let card = Card::new(("artist", place), artist.name.clone())
                    .cover(artist.cover.clone())
                    .circle()
                    .underline()
                    .meta(meta);
                match &artist.id {
                    Some(id) => card.press(pressed(Press::Artist(id.clone()), me)),
                    None => card,
                }
            }
            Hit::Album(album) => Card::new(("album", place), album.name.clone())
                .cover(album.cover.clone())
                .underline()
                .meta(meta)
                .press(pressed(Press::Album(album.id.clone()), me)),
            Hit::Playlist(list) => Card::new(("playlist", place), list.name.clone())
                .cover(list.cover.clone())
                .underline()
                .meta(meta)
                .press(pressed(Press::Playlist(list.id.clone()), me)),
        };

        // Under Liquid Glass a result is roomier: taller, with larger artwork and title.
        let card = match ui::liquid(cx) {
            true => {
                let height = hit_height(cx);
                card.h(height)
                    .art(gpui::px((height * LIQUID_ART / gpui::px(1.)).round()))
                    .gap_4()
                    .text_size(theme.text(Text::Body) * LIQUID_TITLE)
            }
            false => card,
        };
        card.when_some(self.transport(hit, me, cx), |card, (playing, play)| {
            card.play(playing, play)
        })
        .when_some(hit.pin(), Pinnable::pin)
        .when_some(HitMenu::of(hit), |card, target| card.menu(menu(target, me)))
        .chosen(chosen)
        .into_any_element()
    }

    fn transport(&self, hit: &Hit, me: &WeakEntity<Self>, cx: &App) -> Option<(bool, Play)> {
        let me = me.clone();
        let origin = match hit {
            Hit::Song(track) => {
                let current = track.id.is_some() && track.id == self.playback_status.0;
                let playing = current && self.playback_status.1 == Some(true);
                let track = track.clone();
                let play: Play = Box::new(move |_, _, cx| {
                    me.update(cx, |this, cx| {
                        this.playback.update(cx, |playback, cx| match current {
                            true => playback.toggle_play(cx),
                            false => playback.play_radio(&track, cx),
                        });
                    })
                    .ok();
                });
                return Some((playing, play));
            }
            Hit::Artist(artist) => {
                state::Origin::artist(artist.id.clone()?).named(artist.name.clone())
            }
            Hit::Album(album) => state::Origin::album(album.id.clone()).named(album.name.clone()),
            Hit::Playlist(list) => {
                state::Origin::playlist(list.id.clone()).named(list.name.clone())
            }
        };
        let playing = self.playback.read(cx).playing_from(&origin, cx) == Some(true);
        let play: Play = Box::new(move |_, _, cx| {
            me.update(cx, |this, cx| {
                this.playback
                    .update(cx, |playback, cx| playback.toggle_origin(&origin, cx));
            })
            .ok();
        });

        Some((playing, play))
    }

    fn best(&self, cx: &Context<Self>) -> Option<AnyElement> {
        let theme = *cx.theme();
        let hit = self.search.read(cx).best()?;
        let (kind, title, artists, target) = match hit {
            Hit::Song(track) => (
                Kind::Song,
                track.name.clone(),
                track.artist_refs.clone(),
                Some(Press::Song(Box::new(track.clone()))),
            ),
            Hit::Artist(artist) => (
                Kind::Artist,
                artist.name.clone(),
                Vec::new(),
                artist.id.clone().map(Press::Artist),
            ),
            Hit::Album(album) => (
                Kind::Album,
                album.name.clone(),
                album.artist_refs.clone(),
                Some(Press::Album(album.id.clone())),
            ),
            Hit::Playlist(list) => (
                Kind::Playlist,
                list.name.clone(),
                Vec::new(),
                Some(Press::Playlist(list.id.clone())),
            ),
        };

        let me = cx.entity().downgrade();
        let card = Card::new("best", title)
            .art(theme.metrics.cover * 0.45)
            .cover(cover(hit).clone())
            .when(matches!(kind, Kind::Artist), Card::circle)
            .eyebrow(noun(kind))
            .size(Text::Title)
            .weight(FontWeight::BOLD)
            .bare_meta(
                cells::artist_links(
                    "best-artist-link",
                    artists,
                    meta(hit, false),
                    theme.muted_foreground,
                )
                .text_size(theme.text(Text::Small))
                .truncate(),
            )
            .flat()
            .gap_4()
            .p_3()
            .bg(theme.secondary)
            .when_some(self.transport(hit, &me, cx), |card, (playing, play)| {
                card.play(playing, play)
            })
            .when_some(hit.pin(), Pinnable::pin)
            .when_some(target, |card, target| card.press(pressed(target, &me)))
            .when_some(HitMenu::of(hit), |card, target| {
                card.on_mouse_down(MouseButton::Right, menu(target, &me))
            })
            .into_any_element();

        Some(
            div()
                .flex()
                .flex_col()
                .flex_none()
                .gap_2()
                .child(eyebrow(t!("search-best-match"), cx).pb_1())
                .child(card)
                .into_any_element(),
        )
    }

    /// The page search shows in place of its results. Nothing here works without the network,
    /// so the No connection state covers the whole screen the moment it is gone, and a failed
    /// search or a failed browse shows its own reason otherwise. Asking again is the retry,
    /// since neither loader counts a failure as served.
    fn failure(&self, asked: bool, gutter: Pixels, cx: &Context<Self>) -> Option<AnyElement> {
        let offline = Network::lost(cx);
        let (id, reason, retry) = match asked {
            true => {
                let reason = match offline {
                    true => None,
                    false => Some(self.search.read(cx).error()?.to_owned()),
                };
                let search = self.search.clone();
                let query = search.read(cx).query().to_owned();
                let retry: Box<dyn Fn(&mut App)> = Box::new(move |cx| {
                    search.update(cx, |search, cx| search.ask(&query, cx));
                });
                ("search-lost", reason, retry)
            }
            false => {
                let reason = match offline {
                    true => None,
                    false => Some(self.genres.read(cx).error()?.to_owned()),
                };
                let genres = self.genres.clone();
                let retry: Box<dyn Fn(&mut App)> = Box::new(move |cx| {
                    genres.update(cx, |genres, cx| genres.load(cx));
                });
                ("search-browse-lost", reason, retry)
            }
        };

        Some(
            trouble::lost(
                id,
                t!("trouble-not-loaded"),
                reason.as_deref(),
                move |_, _, cx| retry(cx),
                cx,
            )
            .flex_1()
            .min_h_0()
            .px(gutter)
            .into_any_element(),
        )
    }

    fn shell(
        &self,
        title: SharedString,
        body: AnyElement,
        gutter: Pixels,
        cx: &Context<Self>,
    ) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .gap_1()
            .child(div().pl(gutter).child(eyebrow(title, cx).pb_1()))
            .child(body)
            .into_any_element()
    }

    fn column(&self, kind: Kind, window: &Window, cx: &Context<Self>) -> AnyElement {
        let (title, only) = match kind {
            Kind::Song => (t!("search-songs"), SONGS),
            Kind::Artist => (t!("search-artists"), ARTISTS),
            Kind::Album | Kind::Playlist => (t!("search-albums-playlists"), RELEASES),
        };
        let body = self.deck(only, RAIL, None, window, cx);

        self.shell(title, body, RAIL, cx)
    }

    fn browse(&self, gutter: Pixels, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let found = self.genres.read(cx).genres();
        let theme = *cx.theme();
        let pad = theme.metrics.inset;
        let width = cells::content_width(window, pad * 2., cx);
        let plates = shelves::grid("genre", found, width, window, cx);

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap_3()
            .children(
                self.recent(gutter, cx)
                    .map(|recent| div().pb_3().child(recent)),
            )
            .child(div().px(gutter).child(eyebrow(t!("search-browse"), cx)))
            .child(
                crate::chrome::edge_fade(Scroller::new("search-browse", &self.browsing), false, cx)
                    .bottom_inset(crate::chrome::Chrome::bottom(cx))
                    .px(gutter)
                    .pb(pad)
                    .child(plates),
            )
            .into_any_element()
    }

    fn everything(&self, gutter: Pixels, window: &Window, cx: &Context<Self>) -> AnyElement {
        let lead = div()
            .flex()
            .flex_col()
            .gap_6()
            .children(self.best(cx))
            .child(eyebrow(t!("search-results"), cx).pb_1())
            .into_any_element();

        self.deck(&Kind::ALL, gutter, Some(lead), window, cx)
    }

    fn seats(&self, only: &[Kind], cx: &App) -> Vec<(usize, usize)> {
        let mut taken = [0; Kind::ALL.len()];

        self.search
            .read(cx)
            .hits()
            .iter()
            .enumerate()
            .filter_map(|(at, hit)| {
                let kind = hit.kind();
                let place = taken[seat(kind)];
                taken[seat(kind)] += 1;
                only.contains(&kind).then_some((at, place))
            })
            .collect()
    }

    fn deck(
        &self,
        only: &[Kind],
        gutter: Pixels,
        lead: Option<AnyElement>,
        window: &Window,
        cx: &Context<Self>,
    ) -> AnyElement {
        let (id, bar) = match only {
            [Kind::Song] => ("search-songs", &self.songs),
            [Kind::Artist] => ("search-artists", &self.artists),
            [Kind::Album, Kind::Playlist] | [Kind::Album] => ("search-albums", &self.albums),
            [Kind::Playlist] => ("search-playlists", &self.mixed),
            _ => ("search-all", &self.mixed),
        };
        let seats = self.seats(only, cx);
        if seats.is_empty() {
            let empty = vacant(t!("search-no-matches"), cx).flex_none();
            return match lead {
                Some(lead) => div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .px(gutter)
                    .child(lead)
                    .child(empty)
                    .into_any_element(),
                None => empty.into_any_element(),
            };
        }

        let compact = only.len() > 1;
        let column = match ui::liquid(cx) {
            true => 0,
            false => column_of(only),
        };
        let theme = *cx.theme();
        let row = snapped(hit_height(cx), window);
        let scroll = bar.read(cx).scroll().clone();
        let me = cx.entity().downgrade();
        let measured = self.lead.clone();
        let tracked = scroll.clone();
        let deck = Deck::new(format!("{id}-deck"))
            .rows((0..seats.len()).map(|_| row))
            .gap(hit_gap(cx))
            .when(lead.is_some(), |deck| {
                deck.on_measure(move |top, _, _| {
                    let content = tracked.bounds().origin.y - scrolled(&tracked);
                    measured.set(top - content);
                })
            })
            .draw(move |index, _, cx| {
                let Some(view) = me.upgrade() else {
                    return div().into_any_element();
                };
                let Some(&(at, place)) = seats.get(index) else {
                    return div().into_any_element();
                };
                let this = view.read(cx);
                let Some(hit) = this.search.read(cx).hits().get(at) else {
                    return div().into_any_element();
                };

                this.row(
                    hit,
                    place,
                    compact,
                    this.cursor == Some((column, index)),
                    &view.downgrade(),
                    cx,
                )
            });

        div()
            .flex_1()
            .min_h_0()
            .child(
                crate::chrome::edge_fade(Scroller::new(id, bar), false, cx)
                    .bottom_inset(crate::chrome::Chrome::bottom(cx))
                    .px(gutter)
                    .pb(theme.metrics.inset)
                    .child(div().flex().flex_col().gap_1().children(lead).child(deck)),
            )
            .into_any_element()
    }
}

fn stacked(window: &Window, cx: &App) -> bool {
    !Chrome::room(window, cx).fits(Room::Wide)
}

fn columns(stacked: bool) -> usize {
    match stacked {
        true => 1,
        false => 3,
    }
}

fn kinds(column: usize, stacked: bool) -> &'static [Kind] {
    match (stacked, column) {
        (true, _) => &Kind::ALL,
        (false, 0) => SONGS,
        (false, 1) => ARTISTS,
        _ => RELEASES,
    }
}

fn tab_kinds(kind: Kind) -> &'static [Kind] {
    match kind {
        Kind::Song => SONGS,
        Kind::Artist => ARTISTS,
        Kind::Album => ALBUMS,
        Kind::Playlist => PLAYLISTS,
    }
}

fn column_of(only: &[Kind]) -> usize {
    match only {
        [Kind::Song] => 0,
        [Kind::Artist] => 1,
        [Kind::Album, Kind::Playlist] => 2,
        _ => 0,
    }
}

fn seat(kind: Kind) -> usize {
    match kind {
        Kind::Song => 0,
        Kind::Artist => 1,
        Kind::Album => 2,
        Kind::Playlist => 3,
    }
}

fn pressed(
    target: Press,
    me: &WeakEntity<SearchView>,
) -> impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static {
    let me = me.clone();

    move |_, _, cx| {
        me.update(cx, |this, cx| this.remember(cx)).ok();
        me.update(cx, |this, cx| match &target {
            Press::Song(track) => this
                .playback
                .update(cx, |playback, cx| playback.play_radio(track, cx)),
            Press::Artist(id) => navigate(Destination::Artist(id.clone().into()), cx),
            Press::Album(id) => navigate(Destination::Album(id.clone().into()), cx),
            Press::Playlist(id) => navigate(Destination::Playlist(id.clone().into()), cx),
        })
        .ok();
    }
}

fn menu(
    target: HitMenu,
    me: &WeakEntity<SearchView>,
) -> impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static {
    let me = me.clone();

    move |event: &MouseDownEvent, window, cx| {
        window.prevent_default();
        me.update(cx, |this, cx| {
            this.track_menu.reset(cx);
            this.context_menu = Some((target.clone(), event.position));
            cx.notify();
        })
        .ok();
    }
}

fn album_of(hit: &AlbumHit, cx: &App) -> Album {
    Sonora::global(cx)
        .library
        .read(cx)
        .album(&hit.id)
        .cloned()
        .unwrap_or_else(|| Album {
            id: hit.id.clone(),
            name: hit.name.clone(),
            artists: hit.artists.clone(),
            artist_refs: hit.artist_refs.clone(),
            cover: hit.cover.clone(),
            cover_large: hit.cover.clone(),
            release_type: ReleaseType::Album,
            year: 0,
            track_count: 0,
            release_date: String::new(),
            label: String::new(),
            copyrights: Vec::new(),
            added_at: None,
        })
}

fn playlist_of(hit: &PlaylistHit, cx: &App) -> Playlist {
    Sonora::global(cx)
        .library
        .read(cx)
        .playlist(&hit.id)
        .cloned()
        .unwrap_or_else(|| Playlist {
            id: hit.id.clone(),
            name: hit.name.clone(),
            owner: hit.owner.clone(),
            owner_id: String::new(),
            owned: false,
            collaborative: false,
            blend: false,
            public: false,
            cover: hit.cover.clone(),
            track_count: 0,
            modified_at: None,
        })
}

fn artist_of(hit: &ArtistHit, cx: &App) -> SavedArtist {
    let id = hit.id.clone().unwrap_or_default();
    Sonora::global(cx)
        .library
        .read(cx)
        .artist(&id)
        .cloned()
        .unwrap_or_else(|| SavedArtist {
            id,
            name: hit.name.clone(),
            cover: hit.cover.clone(),
            added_at: None,
        })
}

fn cover(hit: &Hit) -> Option<String> {
    match hit {
        Hit::Song(track) => track.cover.clone(),
        Hit::Artist(artist) => artist.cover.clone(),
        Hit::Album(album) => album.cover.clone(),
        Hit::Playlist(list) => list.cover.clone(),
    }
}

fn meta(hit: &Hit, compact: bool) -> SharedString {
    match hit {
        Hit::Song(track) => SharedString::from(track.artists.clone()),
        Hit::Album(album) => SharedString::from(album.artists.clone()),
        Hit::Artist(artist) => match compact {
            true => noun(Kind::Artist),
            false => held(artist.saved),
        },
        Hit::Playlist(list) => match list.owner.is_empty() {
            true => noun(Kind::Playlist),
            false => SharedString::from(list.owner.clone()),
        },
    }
}

fn tag(kind: Kind) -> SharedString {
    let noun = noun(kind);
    t!("search-tag", kind = &noun)
}

fn held(saved: usize) -> SharedString {
    match saved {
        0 => noun(Kind::Artist),
        count => t!("search-saved", count = count),
    }
}

fn noun(kind: Kind) -> SharedString {
    match kind {
        Kind::Song => t!("kind-song"),
        Kind::Artist => t!("kind-artist"),
        Kind::Album => t!("kind-album"),
        Kind::Playlist => t!("kind-playlist"),
    }
}

impl Focusable for SearchView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for SearchView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let pad = theme.metrics.inset;
        let width = cells::content_width(window, pad * 2., cx);
        let stacked = !Chrome::room(window, cx).fits(Room::Wide);
        let inset = match width > VAST {
            true => (width - VAST) / 2.,
            false => Pixels::ZERO,
        };
        let asked = !self.search.read(cx).query().trim().is_empty();
        let context_menu = self.context_menu.clone().map(|(target, position)| {
            let menu = match target {
                HitMenu::Song(track) => self.track_menu.for_track(&track, cx),
                HitMenu::Album(hit) => album_menu(
                    album_of(&hit, cx),
                    self.playback.clone(),
                    &self.track_menu,
                    cx,
                ),
                HitMenu::Playlist(hit) => {
                    playlist_menu(playlist_of(&hit, cx), self.playback.clone(), cx)
                }
                HitMenu::Artist(hit) => {
                    artist_menu(artist_of(&hit, cx), self.playback.clone(), false, cx)
                }
            };

            Popup::new(position, menu).on_close(cx.listener(|this, _, _, cx| {
                this.context_menu = None;
                cx.notify();
            }))
        });

        let gutter = pad + inset;
        let results = match self.failure(asked, gutter, cx) {
            Some(failure) => failure,
            None => match (asked, stacked) {
                (false, _) => self.browse(gutter, window, cx),
                (true, _) if ui::liquid(cx) => self.tabbed(gutter, window, cx),
                (true, true) => self.everything(gutter, window, cx),
                (true, false) => div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .px(gutter)
                    .child(self.column(Kind::Song, window, cx))
                    .child(Separator::vertical())
                    .child(self.column(Kind::Artist, window, cx))
                    .child(Separator::vertical())
                    .child(self.column(Kind::Album, window, cx))
                    .into_any_element(),
            },
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_6()
            .pt(pad)
            .key_context(SEARCH_CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::select_previous))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::activate_hit))
            .on_action(cx.listener(Self::deselect_hit))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .px(gutter)
                    .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                        if event.keystroke.key == "enter" {
                            this.remember(cx);
                        }
                    }))
                    .child(self.input.clone()),
            )
            .when(!stacked, |this| {
                this.children(self.best(cx).map(|best| div().px(gutter).child(best)))
            })
            .child(results)
            .when_some(context_menu, |this, menu| this.child(menu))
    }
}

/// The height of one result row: under Liquid Glass taller than a list row by `LIQUID_AIR`.
fn hit_height(cx: &App) -> Pixels {
    let air = match ui::liquid(cx) {
        true => LIQUID_AIR,
        false => Pixels::ZERO,
    };
    cx.theme().metrics.list_row + air
}

/// The gap between result rows, wider under Liquid Glass.
fn hit_gap(cx: &App) -> Pixels {
    let share = match ui::liquid(cx) {
        true => LIQUID_GAP,
        false => ROW_GAP,
    };
    cx.theme().font_size * share
}
