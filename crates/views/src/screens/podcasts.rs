use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Entity, Pixels, Render, ScrollHandle, SharedString, Window, div,
};
use i18n::t;
use music::{PodcastScope, Track};
use router::{Destination, navigate};
use state::{Io, Playback, PodcastDetail, Podcasts, Sonora};
use ui::{
    ActiveTheme as _, Button, Card, Listing as _, Scrollbar, Scroller, TabBar, TableDelegate,
    TableEvent, TableState, Text, table, vacant,
};

use crate::chrome::{Chrome, Searchable, Toolbar, Tooled};
use crate::shared::hero::{HeroMetaStrip, HeroPlayButton, PageHero};
use crate::shared::text::holds;
use crate::shared::tracks::{EPISODE_COLUMNS, TrackSource, Tracks};
use crate::shared::{cells, page, trouble};

/// The account's podcast collections, with independent lists and a playable voice table.
pub(crate) struct PodcastsView {
    podcasts: Entity<Podcasts>,
    playback: Entity<Playback>,
    width: Pixels,
    scrollbar: Entity<Scrollbar>,
    table: Entity<TableState<TrackSource>>,
    toolbar: Entity<Toolbar>,
    query: String,
}

struct LibraryEpisodes(Entity<Podcasts>);

impl Tracks for LibraryEpisodes {
    fn tracks<'a>(&self, cx: &'a App) -> &'a [Track] {
        self.0.read(cx).tracks()
    }

    fn is_loading(&self, cx: &App) -> bool {
        let podcasts = self.0.read(cx);
        podcasts.is_loading() && podcasts.count(podcasts.scope()).is_none()
    }
}

impl PodcastsView {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        let session = Sonora::global(cx).session.clone();
        let playback = Sonora::global(cx).playback.clone();
        let podcasts = cx.new(|cx| Podcasts::new(session, Io::global(cx), cx));
        let id = cx.entity_id();
        let scrollbar = cx.new(|_| Scrollbar::new(ScrollHandle::new()).watching(id));
        let scroll = scrollbar.read(cx).scroll().clone();
        let width = ui::MIN_CONTENT;
        let table = cx.new(|cx| {
            let menu = cx.new(|_| Scrollbar::inset().watching(id));
            let source = TrackSource::new(
                EPISODE_COLUMNS,
                LibraryEpisodes(podcasts.clone()),
                playback.clone(),
                menu,
                cx,
            )
            .episodes()
            .table(cx.weak_entity());
            TableState::new(TableDelegate::new(source, width, cx), cx).follow(scroll)
        });
        cx.observe(&podcasts, |this, _, cx| {
            this.table.rebuild(cx);
            cx.notify();
        })
        .detach();
        cx.observe(&playback, |this, _, cx| {
            this.table.refresh(cx);
            cx.notify();
        })
        .detach();
        cx.subscribe(&table, |this, _, event, cx| match event {
            TableEvent::DoubleClicked(display) => {
                page::play(&this.table, &this.playback, *display, cx)
            }
            TableEvent::Activated(display) => {
                page::play_or_toggle(&this.table, &this.playback, *display, cx)
            }
            _ => {}
        })
        .detach();
        let chrome = Chrome::entity(cx);
        cx.observe(&chrome, |_, _, cx| cx.notify()).detach();
        Self {
            podcasts,
            playback,
            width,
            scrollbar,
            table,
            toolbar: Toolbar::searchable(&cx.entity(), cx),
            query: String::new(),
        }
    }

    pub(crate) fn open(&mut self, cx: &mut Context<Self>) {
        self.podcasts.update(cx, |podcasts, cx| podcasts.load(cx));
    }

    pub(crate) fn refresh(&mut self, cx: &mut Context<Self>) {
        self.podcasts
            .update(cx, |podcasts, cx| podcasts.refresh(cx));
    }

    pub(crate) fn is_loading(&self, cx: &App) -> bool {
        self.podcasts.read(cx).is_loading()
    }

    fn select(&mut self, scope: PodcastScope, cx: &mut Context<Self>) {
        if self.podcasts.read(cx).scope() == scope {
            return;
        }
        self.podcasts
            .update(cx, |podcasts, cx| podcasts.select(scope, cx));
        self.table.update(cx, |table, cx| {
            table.delegate_mut().clear_selection();
            table.rebuild(cx);
        });
        self.scrollbar
            .read(cx)
            .scroll()
            .set_offset(gpui::Point::default());
        cx.notify();
    }

    /// Keeps all four collection choices available while one tab loads or fails.
    fn tabs(&self, cx: &Context<Self>) -> impl IntoElement {
        let podcasts = self.podcasts.read(cx);
        TabBar::new("podcast-library-tabs")
            .max_w_full()
            .items(PodcastScope::ALL.into_iter().map(|scope| {
                let key = scope_key(scope);
                let name = i18n::lookup(key, None);
                let label = match podcasts.count(scope) {
                    Some(count) => t!("podcasts-tab-count", name = name.as_ref(), count = count),
                    None => name,
                };
                Button::new(key)
                    .ghost()
                    .label(label)
                    .selected(podcasts.scope() == scope)
                    .on_click(cx.listener(move |this, _, _, cx| this.select(scope, cx)))
            }))
    }
}

impl Render for PodcastsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.table.claim(cx);
        let theme = *cx.theme();
        let inset = theme.metrics.inset;
        let under = Chrome::under_bar(window, cx);
        page::resize(&self.table, &mut self.width, inset, window, cx);
        let scroll = self.scrollbar.read(cx).scroll().clone();
        let viewport = page::viewport(&scroll, inset, window);
        self.table
            .update(cx, |table, _| table.set_viewport(viewport));
        let podcasts = self.podcasts.read(cx);
        let scope = podcasts.scope();
        let loading = podcasts.is_loading();
        let initial = podcasts.count(scope).is_none();
        let reason = podcasts.error().map(str::to_owned);
        let failed = initial && reason.is_some();
        let has_more = podcasts.has_more();
        let voices = scope == PodcastScope::Liked || !podcasts.tracks().is_empty();
        let items = podcasts.items();
        let cards: Vec<_> = items
            .iter()
            .filter(|podcast| {
                holds(&podcast.name, &self.query) || holds(&podcast.host, &self.query)
            })
            .map(|podcast| {
                let id = SharedString::from(podcast.id.clone());
                let caption = match &podcast.latest_episode {
                    Some(episode) => t!("podcasts-latest", name = episode.as_str()),
                    None => SharedString::from(podcast.host.clone()),
                };
                Card::new(id.clone(), SharedString::from(podcast.name.clone()))
                    .cover(podcast.cover.clone())
                    .fallback("icons/radio.svg")
                    .meta(caption)
                    .trailing(
                        div()
                            .text_size(theme.text(Text::Small))
                            .text_color(theme.muted_foreground)
                            .child(t!("count-episodes", count = podcast.episode_count)),
                    )
                    .press(move |_, _, cx| navigate(Destination::Podcast(id.clone()), cx))
            })
            .collect();
        let empty = !loading && !failed && cards.is_empty() && self.table.row_count(cx) == 0;
        let caption = match self.query.is_empty() {
            false => t!("library-no-matches"),
            true => i18n::lookup(scope_empty_key(scope), None),
        };
        let hero = PageHero::new("podcasts-hero", t!("nav-podcasts"))
            .fallback("icons/radio.svg")
            .accent()
            .when(voices, |hero| {
                hero.actions(HeroPlayButton::listed(
                    "play-library-voices",
                    t!("podcast-play"),
                    &self.table,
                    self.playback.clone(),
                ))
            });
        let header = div()
            .px(inset)
            .child(hero)
            .child(div().py(theme.metrics.pad).child(self.tabs(cx)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .when(loading && initial && !voices, |this| {
                        this.children((0..6).map(Card::skeleton))
                    })
                    .children(cards),
            )
            .when(failed, |this| {
                this.child(trouble::lost(
                    "podcasts-retry",
                    t!("podcasts-not-loaded"),
                    reason.as_deref(),
                    cx.listener(|this, _, _, cx| this.refresh(cx)),
                    cx,
                ))
            });
        div().relative().size_full().child(
            div()
                .absolute()
                .top(-under)
                .left_0()
                .right_0()
                .bottom_0()
                .child(
                    crate::chrome::edge_fade(
                        Scroller::new("podcasts-page", &self.scrollbar),
                        voices,
                        cx,
                    )
                    .bottom_inset(Chrome::bottom(cx))
                    .pt(inset + under)
                    .pb(inset)
                    .child(header)
                    .when(voices && !failed, |this| {
                        this.child(table(&self.table).mx(cells::table_gutter(cx)))
                    })
                    .when(empty, |this| this.child(vacant(caption, cx)))
                    .when(has_more, |this| {
                        this.child(
                            div().flex().justify_center().p(theme.metrics.pad).child(
                                Button::new("load-podcast-collection")
                                    .outline()
                                    .label(match (loading, reason.is_some()) {
                                        (true, _) => t!("podcasts-loading"),
                                        (_, true) => t!("podcasts-load-retry"),
                                        _ => t!("podcasts-load-more"),
                                    })
                                    .disabled(loading)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.podcasts
                                            .update(cx, |podcasts, cx| podcasts.load_more(cx))
                                    })),
                            ),
                        )
                    }),
                ),
        )
    }
}

impl Searchable for PodcastsView {
    fn search(&mut self, query: &str, cx: &mut Context<Self>) {
        self.query = query.to_owned();
        self.table.set_query(query, cx);
        cx.notify();
    }

    fn hint() -> SharedString {
        "filter-podcasts".into()
    }
}

impl Tooled for PodcastsView {
    fn toolbar(&self) -> Entity<Toolbar> {
        self.toolbar.clone()
    }

    fn tools(&self, _cx: &App) -> Vec<AnyElement> {
        Vec::new()
    }
}

struct EpisodeTracks(Entity<PodcastDetail>);

impl Tracks for EpisodeTracks {
    fn tracks<'a>(&self, cx: &'a App) -> &'a [Track] {
        self.0.read(cx).tracks()
    }

    fn is_loading(&self, cx: &App) -> bool {
        let detail = self.0.read(cx);
        detail.is_loading() && detail.podcast().is_none()
    }
}

/// A podcast's metadata and paged episode table, using the shared playback and queue controls.
pub(crate) struct PodcastView {
    detail: Entity<PodcastDetail>,
    playback: Entity<Playback>,
    width: Pixels,
    scrollbar: Entity<Scrollbar>,
    table: Entity<TableState<TrackSource>>,
    toolbar: Entity<Toolbar>,
}

impl PodcastView {
    pub(crate) fn new(playback: Entity<Playback>, cx: &mut Context<Self>) -> Self {
        let session = Sonora::global(cx).session.clone();
        let detail = cx.new(|cx| PodcastDetail::new(session, Io::global(cx), cx));
        let id = cx.entity_id();
        let scrollbar = cx.new(|_| Scrollbar::new(ScrollHandle::new()).watching(id));
        let scroll = scrollbar.read(cx).scroll().clone();
        let width = ui::MIN_CONTENT;
        let table = cx.new(|cx| {
            let menu = cx.new(|_| Scrollbar::inset().watching(id));
            let source = TrackSource::new(
                EPISODE_COLUMNS,
                EpisodeTracks(detail.clone()),
                playback.clone(),
                menu,
                cx,
            )
            .episodes()
            .table(cx.weak_entity());
            TableState::new(TableDelegate::new(source, width, cx), cx).follow(scroll)
        });
        cx.observe(&detail, |this, _, cx| {
            this.table.rebuild(cx);
            cx.notify();
        })
        .detach();
        cx.observe(&playback, |this, _, cx| {
            this.table.refresh(cx);
            cx.notify();
        })
        .detach();
        let chrome = Chrome::entity(cx);
        cx.observe(&chrome, |_, _, cx| cx.notify()).detach();
        cx.subscribe(&table, |this, _, event, cx| match event {
            TableEvent::DoubleClicked(display) => {
                page::play(&this.table, &this.playback, *display, cx)
            }
            TableEvent::Activated(display) => {
                page::play_or_toggle(&this.table, &this.playback, *display, cx)
            }
            _ => {}
        })
        .detach();
        Self {
            detail,
            playback,
            width,
            scrollbar,
            table,
            toolbar: Toolbar::searchable(&cx.entity(), cx),
        }
    }

    pub(crate) fn open(&mut self, id: &str, cx: &mut Context<Self>) {
        self.detail.update(cx, |detail, cx| detail.open(id, cx));
    }

    pub(crate) fn refresh(&mut self, cx: &mut Context<Self>) {
        self.detail.update(cx, |detail, cx| detail.refresh(cx));
    }

    pub(crate) fn is_loading(&self, cx: &App) -> bool {
        self.detail.read(cx).is_loading()
    }
}

impl Render for PodcastView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.table.claim(cx);
        let theme = *cx.theme();
        let under = Chrome::under_bar(window, cx);
        page::resize(
            &self.table,
            &mut self.width,
            theme.metrics.inset,
            window,
            cx,
        );
        let scroll = self.scrollbar.read(cx).scroll().clone();
        let viewport = page::viewport(&scroll, theme.metrics.inset, window);
        self.table
            .update(cx, |table, _| table.set_viewport(viewport));

        let detail = self.detail.read(cx);
        if detail.podcast().is_none()
            && let Some(reason) = detail.error().map(str::to_owned)
        {
            return div().size_full().child(
                trouble::lost(
                    "podcast-retry",
                    t!("podcast-not-loaded"),
                    Some(&reason),
                    cx.listener(|this, _, _, cx| this.refresh(cx)),
                    cx,
                )
                .size_full(),
            );
        }
        let podcast = detail.podcast();
        let title =
            podcast.map_or_else(|| t!("podcast-kind"), |podcast| podcast.name.clone().into());
        let cover = podcast.and_then(|podcast| podcast.cover.clone());
        let host = podcast.map_or_else(String::new, |podcast| podcast.host.clone());
        let count = podcast.map_or(0, |podcast| podcast.episode_count);
        let description = podcast.map_or_else(String::new, |podcast| podcast.description.clone());
        let has_more = detail.has_more();
        let loading = detail.is_loading();
        let error = detail.error().is_some();
        let pending = podcast.is_none() && loading;
        let empty = !loading && self.table.row_count(cx) == 0;
        let hero = PageHero::new("podcast-hero", title)
            .cover(cover)
            .fallback("icons/radio.svg")
            .eyebrow(t!("podcast-kind"))
            .meta(
                HeroMetaStrip::new()
                    .text(host)
                    .text(t!("count-episodes", count = count)),
            )
            .pending(pending)
            .actions(HeroPlayButton::listed(
                "play-episodes",
                t!("podcast-play"),
                &self.table,
                self.playback.clone(),
            ));

        div().relative().size_full().child(
            div()
                .absolute()
                .top(-under)
                .left_0()
                .right_0()
                .bottom_0()
                .child(
                    crate::chrome::edge_fade(
                        Scroller::new("podcast-page", &self.scrollbar),
                        true,
                        cx,
                    )
                    .bottom_inset(Chrome::bottom(cx))
                    .pt(theme.metrics.inset + under)
                    .pb(theme.metrics.inset)
                    .child(div().px(theme.metrics.inset).child(hero).when(
                        !description.is_empty(),
                        |this| {
                            this.child(
                                div()
                                    .py(theme.metrics.pad)
                                    .text_color(theme.muted_foreground)
                                    .child(description),
                            )
                        },
                    ))
                    .child(table(&self.table).mx(cells::table_gutter(cx)))
                    .when(empty, |this| {
                        this.child(vacant(
                            match self.table.filtering(cx) {
                                true => t!("library-no-matches"),
                                false => t!("podcast-empty"),
                            },
                            cx,
                        ))
                    })
                    .when(has_more, |this| {
                        this.child(
                            div().flex().justify_center().p(theme.metrics.pad).child(
                                Button::new("load-episodes")
                                    .outline()
                                    .label(match (loading, error) {
                                        (true, _) => t!("podcast-loading"),
                                        (_, true) => t!("podcast-load-retry"),
                                        _ => t!("podcast-load-more"),
                                    })
                                    .disabled(loading)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.detail.update(cx, |detail, cx| detail.load_more(cx));
                                    })),
                            ),
                        )
                    }),
                ),
        )
    }
}

impl Searchable for PodcastView {
    fn search(&mut self, query: &str, cx: &mut Context<Self>) {
        self.table.set_query(query, cx);
        cx.notify();
    }

    fn hint() -> SharedString {
        "filter-episodes".into()
    }
}

impl Tooled for PodcastView {
    fn toolbar(&self) -> Entity<Toolbar> {
        self.toolbar.clone()
    }

    fn tools(&self, _cx: &App) -> Vec<AnyElement> {
        Vec::new()
    }
}

/// The localized label of a podcast library tab.
fn scope_key(scope: PodcastScope) -> &'static str {
    match scope {
        PodcastScope::Saved => "podcasts-saved",
        PodcastScope::Created => "podcasts-created",
        PodcastScope::Purchased => "podcasts-purchased",
        PodcastScope::Liked => "podcasts-liked",
    }
}

/// The empty state belongs to the selected collection rather than the whole library.
fn scope_empty_key(scope: PodcastScope) -> &'static str {
    match scope {
        PodcastScope::Saved => "podcasts-empty",
        PodcastScope::Created => "podcasts-created-empty",
        PodcastScope::Purchased => "podcasts-purchased-empty",
        PodcastScope::Liked => "podcasts-liked-empty",
    }
}
