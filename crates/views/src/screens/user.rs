use gpui::prelude::*;
use gpui::{
    AnyElement, Context, Entity, Pixels, Render, ScrollHandle, SharedString, Window, div, px,
};
use i18n::t;
use state::{Playback, Profile};
use ui::{ActiveTheme as _, Scrollbar, Scroller, heading, vacant};

use crate::chrome::Chrome;
use crate::shared::album_grid::{CardGrid, Revealing};
use crate::shared::cards;
use crate::shared::cells;
use crate::shared::hero::{HeroMetaStrip, PageHero};
use crate::shared::trouble;

/// The name the playlists' cards are remembered by as they cross in from skeletons.
const PLAYLIST_REVEAL: &str = "user-playlist";
const FALLBACK: &str = "icons/user.svg";
const PENDING: usize = 6;
const STEADY: Pixels = px(0.5);

pub(crate) struct UserView {
    profile: Entity<Profile>,
    playback: Entity<Playback>,
    scrollbar: Entity<Scrollbar>,
    width: Pixels,
}

impl UserView {
    pub(crate) fn new(
        profile: Entity<Profile>,
        playback: Entity<Playback>,
        cx: &mut Context<Self>,
    ) -> Self {
        let id = cx.entity_id();

        cx.observe(&profile, |this, _, cx| {
            this.scrollbar
                .read(cx)
                .scroll()
                .set_offset(gpui::Point::default());
            cx.notify();
        })
        .detach();
        cx.observe(&playback, |_, _, cx| cx.notify()).detach();

        let chrome = Chrome::entity(cx);
        cx.observe(&chrome, |_, _, cx| cx.notify()).detach();

        Self {
            profile,
            playback,
            scrollbar: cx.new(|_| Scrollbar::new(ScrollHandle::new()).watching(id)),
            width: Pixels::ZERO,
        }
    }

    fn header(&self, cx: &Context<Self>) -> impl IntoElement {
        let profile = self.profile.read(cx);
        let user = profile.user();
        let title = user
            .map(|user| SharedString::from(user.name.clone()))
            .or_else(|| profile.id().map(|id| SharedString::from(id.to_owned())))
            .unwrap_or_default();

        let mut strip = HeroMetaStrip::new();
        if let Some(followers) = user.and_then(|user| user.followers) {
            let value = cells::count(followers);
            strip = strip.text(t!("user-followers", count = followers, value = &value));
        }
        if let Some(following) = user.and_then(|user| user.following) {
            let value = cells::count(following);
            strip = strip.text(t!("user-following", count = following, value = &value));
        }

        PageHero::new("user-hero", title)
            .pending(user.is_none() && profile.is_loading())
            .cover(user.and_then(|user| user.avatar.clone()))
            .circle()
            .fallback(FALLBACK)
            .eyebrow(t!("user-eyebrow"))
            .meta(strip)
    }

    fn playlists(&self, cx: &Context<Self>) -> impl IntoElement {
        let profile = self.profile.read(cx);
        let loading = profile.is_loading();
        let listed = profile.playlists();
        let layout = CardGrid::layout(self.width);

        let cards = match loading {
            true => Revealing::new(PLAYLIST_REVEAL, 0).skeletons(PENDING, layout.card, cx),
            false => listed
                .iter()
                .enumerate()
                .map(|(place, playlist)| {
                    cards::playlist_card(("user-playlist", place), playlist, &self.playback, cx)
                        .tile(layout.card)
                        .flat()
                        .into_any_element()
                })
                .collect::<Vec<_>>(),
        };
        let cards: Vec<AnyElement> = match loading {
            true => cards,
            false => Revealing::new(PLAYLIST_REVEAL, 0)
                .wrap(cards, layout)
                .collect(),
        };
        let empty = cards.is_empty();

        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(heading(t!("user-playlists"), cx))
            .when(empty, |this| {
                this.child(vacant(t!("user-playlists-empty"), cx))
            })
            .when(!empty, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .w_full()
                        .gap_x(layout.gap)
                        .gap_y_6()
                        .children(cards),
                )
            })
    }
}

impl UserView {
    /// The page a profile that did not load shows instead of its header and playlists. Opening
    /// the same profile again is the retry, since a failed load leaves the page empty.
    fn failure(&self, cx: &Context<Self>) -> Option<AnyElement> {
        let id = self.profile.read(cx).id()?.to_owned();
        let reason = match trouble::unreachable(&id, cx) {
            true => None,
            false => Some(self.profile.read(cx).error()?.to_owned()),
        };
        let profile = self.profile.clone();

        Some(
            trouble::lost(
                "user-lost",
                t!("trouble-not-loaded"),
                reason.as_deref(),
                move |_, _, cx| {
                    let id = id.clone();
                    profile.update(cx, |profile, cx| profile.open(&id, cx));
                },
                cx,
            )
            .size_full()
            .into_any_element(),
        )
    }
}

impl Render for UserView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(failure) = self.failure(cx) {
            return div().flex().flex_col().size_full().child(failure);
        }

        let theme = *cx.theme();
        let pad = theme.metrics.inset;
        let room = cells::content_width(window, pad * 2., cx);
        if (room - self.width).abs() >= STEADY {
            self.width = room;
        }

        div().flex().flex_col().size_full().child(
            crate::chrome::edge_fade(Scroller::new("user-page", &self.scrollbar), false, cx)
                .bottom_inset(crate::chrome::Chrome::bottom(cx))
                .p(pad)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_8()
                        .child(self.header(cx))
                        .child(self.playlists(cx)),
                ),
        )
    }
}
