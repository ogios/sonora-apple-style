use gpui::prelude::*;
use gpui::{AnyElement, App, ElementId, Entity, Pixels, RenderOnce, SharedString, Window, div, px};
use music::Album;
use state::Playback;
use ui::{Card, Reveal};

use crate::shared::cards;

pub(crate) const CARD_MIN: Pixels = px(130.);
pub(crate) const CARD_MAX: Pixels = px(190.);
const CARD_GAP: Pixels = px(32.);

#[derive(Clone, Copy)]
pub(crate) struct CardLayout {
    pub(crate) columns: usize,
    pub(crate) card: Pixels,
    pub(crate) gap: Pixels,
}

impl CardLayout {
    fn new(available: Pixels) -> Self {
        let available = available.max(CARD_MIN);
        let columns = (((available + CARD_GAP) / (CARD_MIN + CARD_GAP))
            .floor()
            .max(1.)) as usize;
        let count = columns as f32;
        let spread = available - CARD_GAP * (count - 1.);
        let card = (spread / count).min(CARD_MAX).floor();
        let gap = match columns > 1 {
            true => ((available - card * count) / (count - 1.)).floor(),
            false => Pixels::ZERO,
        };

        Self { columns, card, gap }
    }
}

/// Where a grid's cards are remembered as they cross in from their skeletons: a name for the
/// grid, unique to its page, the place of its first card in the whole grid, and whether its
/// cards are round.
#[derive(Clone)]
pub(crate) struct Revealing {
    pub(crate) key: SharedString,
    pub(crate) first: usize,
    pub(crate) circle: bool,
}

impl Revealing {
    pub(crate) fn new(key: impl Into<SharedString>, first: usize) -> Self {
        Self {
            key: key.into(),
            first,
            circle: false,
        }
    }

    pub(crate) fn circle(mut self, circle: bool) -> Self {
        self.circle = circle;
        self
    }

    fn id(&self, place: usize) -> ElementId {
        ElementId::NamedInteger(self.key.clone(), place as u64)
    }

    /// The skeleton card at `place`, as the grid draws it while its cards are on their way.
    fn skeleton(key: SharedString, place: usize, card: Pixels, circle: bool) -> AnyElement {
        Card::skeleton(ElementId::NamedInteger(key, place as u64))
            .tile(card)
            .when(circle, Card::circle)
            .into_any_element()
    }

    /// `count` skeleton cards from the grid's first place on, each remembered as waiting so
    /// the card that replaces it crosses in over it.
    pub(crate) fn skeletons(&self, count: usize, card: Pixels, cx: &App) -> Vec<AnyElement> {
        (self.first..self.first + count)
            .map(|place| {
                ui::reveal::waiting(&self.id(place), cx);
                Self::skeleton(self.key.clone(), place, card, self.circle)
            })
            .collect()
    }

    /// Wraps `cards`, the grid's from its first place on, so each crosses in over its skeleton
    /// if one stood there, a row lower a little after the one above and so on across it. The
    /// skeletons left over in the last row, where fewer cards came than were guessed, fade out
    /// in their places. Those in rows below go with the rows, which the layout no longer has.
    pub(crate) fn wrap(
        self,
        cards: Vec<AnyElement>,
        layout: CardLayout,
        window: &mut Window,
        cx: &App,
    ) -> Vec<AnyElement> {
        let columns = layout.columns.max(1);
        let end = self.first + cards.len();
        let leavers = match end % columns {
            0 => Vec::new(),
            filled => ui::reveal::leavers(
                (end..end + columns - filled).map(|place| self.id(place)),
                window,
                cx,
            ),
        };
        let leaving = leavers
            .into_iter()
            .enumerate()
            .map(|(index, (_, shows))| {
                div()
                    .flex_none()
                    .opacity(shows)
                    .child(Self::skeleton(
                        self.key.clone(),
                        end + index,
                        layout.card,
                        self.circle,
                    ))
                    .into_any_element()
            })
            .collect::<Vec<_>>();
        let mut wrapped: Vec<AnyElement> = cards
            .into_iter()
            .enumerate()
            .map(|(index, card)| {
                let place = self.first + index;
                let key = self.key.clone();
                let circle = self.circle;
                Reveal::new(self.id(place), Some(card), move |_, _| {
                    Self::skeleton(key, place, layout.card, circle)
                })
                .order(place / columns + place % columns)
                .flex_none()
                .w(layout.card)
                .into_any_element()
            })
            .collect();
        wrapped.extend(leaving);
        wrapped
    }
}

#[derive(IntoElement)]
pub(crate) struct CardGrid {
    layout: CardLayout,
    cards: Vec<AnyElement>,
    reveal: Option<Revealing>,
}

impl CardGrid {
    pub(crate) fn new(available: Pixels) -> Self {
        Self {
            layout: CardLayout::new(available),
            cards: Vec::new(),
            reveal: None,
        }
    }

    /// Crosses the cards in over the skeletons that stood in their places.
    pub(crate) fn reveals(mut self, reveal: Revealing) -> Self {
        self.reveal = Some(reveal);
        self
    }

    pub(crate) fn layout(available: Pixels) -> CardLayout {
        CardLayout::new(available)
    }

    pub(crate) fn children(mut self, cards: impl IntoIterator<Item = AnyElement>) -> Self {
        self.cards.extend(cards);
        self
    }
}

impl RenderOnce for CardGrid {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            layout,
            cards,
            reveal,
        } = self;
        let cards: Vec<AnyElement> = match reveal {
            Some(reveal) => reveal.wrap(cards, layout, window, cx),
            None => cards,
        };
        div().flex().w_full().gap_x(layout.gap).children(cards)
    }
}

#[derive(IntoElement)]
pub(crate) struct AlbumGrid {
    id: &'static str,
    layout: CardLayout,
    albums: Vec<(usize, Album)>,
    playback: Entity<Playback>,
    reveal: Option<Revealing>,
}

impl AlbumGrid {
    pub(crate) fn new(
        id: &'static str,
        available: Pixels,
        albums: impl IntoIterator<Item = (usize, Album)>,
        playback: Entity<Playback>,
    ) -> Self {
        Self {
            id,
            layout: CardLayout::new(available),
            albums: albums.into_iter().collect(),
            playback,
            reveal: None,
        }
    }

    /// Crosses the cards in over the skeletons that stood in their places.
    pub(crate) fn reveals(mut self, reveal: Revealing) -> Self {
        self.reveal = Some(reveal);
        self
    }
}

impl RenderOnce for AlbumGrid {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            id,
            layout,
            albums,
            playback,
            reveal,
        } = self;
        let cards: Vec<AnyElement> = albums
            .into_iter()
            .map(|(index, album)| {
                album_card(id, index, &album, &playback, layout.card, cx).into_any_element()
            })
            .collect();
        let cards: Vec<AnyElement> = match reveal {
            Some(reveal) => reveal.wrap(cards, layout, window, cx),
            None => cards,
        };

        div()
            .flex()
            .flex_wrap()
            .w_full()
            .gap_x(layout.gap)
            .gap_y_6()
            .children(cards)
    }
}

fn album_card(
    id: &'static str,
    index: usize,
    album: &Album,
    playback: &Entity<Playback>,
    width: Pixels,
    cx: &App,
) -> Card {
    cards::album_card((id, index), album, playback, cx)
        .tile(width)
        .flat()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_card_stays_within_its_bounds() {
        for width in (160..2400).step_by(3) {
            let layout = CardLayout::new(px(width as f32));
            assert!(layout.card <= CARD_MAX, "{width} yielded {:?}", layout.card);
            assert!(layout.card >= CARD_MIN, "{width} yielded {:?}", layout.card);
        }
    }

    #[test]
    fn a_row_never_outgrows_the_space_it_was_given() {
        for width in (160..2400).step_by(3) {
            let available = px(width as f32);
            let layout = CardLayout::new(available);
            let count = layout.columns as f32;
            let used = layout.card * count + layout.gap * (count - 1.);
            assert!(used <= available, "{width} packed {used:?}");
        }
    }

    #[test]
    fn cards_never_touch() {
        for width in (160..2400).step_by(3) {
            let layout = CardLayout::new(px(width as f32));
            if layout.columns > 1 {
                assert!(layout.gap >= CARD_GAP, "{width} yielded {:?}", layout.gap);
            }
        }
    }

    #[test]
    fn slack_goes_to_the_gaps_once_the_cards_are_capped() {
        let layout = CardLayout::new(CARD_MAX * 2. + CARD_GAP * 2.);

        assert_eq!(layout.card, CARD_MAX);
        assert!(layout.gap > CARD_GAP);
    }

    #[test]
    fn a_single_column_has_no_gap() {
        let layout = CardLayout::new(CARD_MIN);
        assert_eq!(layout.gap, Pixels::ZERO);
    }
}
