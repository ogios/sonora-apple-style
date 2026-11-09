//! The Theme section of Appearance: one card per theme on a rail that scrolls sideways, each
//! card a soft, flat miniature of the Sonora window painted in that theme.

use gpui::prelude::*;
use gpui::{
    AnyElement, Context, Div, Pixels, SharedString, div, linear_color_stop, linear_gradient, px,
    svg,
};
use i18n::t;
use state::ThemeFamily;
use ui::{ActiveTheme as _, Look, Menu, MenuItem, OptionCard, Picker, Rail, Theme, ThemeKind};

use super::SettingsView;

/// How wide a theme card stands.
const CARD: Pixels = px(150.);
/// How tall a theme card's picture stands, its border included.
const THUMB: Pixels = px(96.);
/// How thick the card's border is, which the miniature sits inside.
const EDGE: Pixels = px(1.);
/// The miniature window inside the card's border, in whole pixels.
const WIDTH: f32 = 148.;
const HEIGHT: f32 = 94.;
/// How many variants of a family its card shows side by side at most.
const SLICES: usize = 4;
/// The popover key of the variant menu, which one card at a time owns.
const VARIANTS: &str = "theme-variants";
/// The side of a variant's swatch in the menu.
const SWATCH: f32 = 14.;
/// How large the open folder glyph draws on its card.
const GLYPH: Pixels = px(18.);
/// How much of the theme's corner radius the miniature's tiles keep.
const RADIUS_SCALE: f32 = 0.3;
/// How strongly the bars show against what they sit on, the first bar of the sidebar standing
/// for the active entry, and how much of its strength the accent tile keeps.
const BAR: f32 = 0.2;
const ACTIVE_BAR: f32 = 0.4;
const ACCENT: f32 = 0.9;

/// The sidebar's width. It runs the full height, so it alone carries the left corners.
const SIDEBAR: f32 = 38.;
/// The columns a cut between two palettes may run down: the sidebar's edge and the middle of
/// each gap between two tiles, so a cut never splits a tile or the play button.
const SEAMS: [f32; 3] = [SIDEBAR, 76., 110.];
/// How far a slice runs on under the slice after it.
const SEAM_OVERLAP: f32 = 1.;
/// The sidebar's entries and the content's heading, as left, top, width and height.
const ENTRIES: [[f32; 4]; 3] = [[8., 14., 22., 5.], [8., 25., 16., 5.], [8., 36., 19., 5.]];
const HEADING: [f32; 4] = [45., 14., 34., 5.];
/// The row of tiles under the heading by their left edges, the first one in the accent.
const TILES: [f32; 3] = [45., 79., 113.];
const TILE_TOP: f32 = 27.;
const TILE: f32 = 28.;
/// The player floating at the foot of the content, and its play button.
const PLAYER: [f32; 4] = [45., 66., 96., 14.];
const PLAY: [f32; 4] = [50., 70., 6., 6.];

/// One card of the rail: a built-in theme or a family of theme files.
enum Entry {
    Kind(ThemeKind),
    Family(ThemeFamily),
}

impl SettingsView {
    /// The theme cards on one rail: System, Dark and Light, then every family of theme files,
    /// then the card that opens the themes folder. While the adaptive theme is on only the
    /// neutral themes can be chosen, since the tint is laid over those. A saved theme whose file
    /// is gone keeps a card of its own, chosen and disabled, so the page says what became of it.
    pub(super) fn theme_cards(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = *cx.theme();
        let (selected, chosen_family, adaptive, families) = {
            let settings = self.settings.read(cx);
            (
                settings.theme().to_owned(),
                settings
                    .theme_variant()
                    .map(|(family, _)| family.id.clone()),
                settings.adaptive_theme(),
                settings.theme_families().to_vec(),
            )
        };
        let built_in = ThemeKind::ALL.iter().any(|kind| kind.id() == selected);

        let mut cards = ThemeKind::ALL
            .into_iter()
            .map(Entry::Kind)
            .chain(families.into_iter().map(Entry::Family))
            .map(|entry| {
                let chosen = match &entry {
                    Entry::Kind(kind) => kind.id() == selected,
                    Entry::Family(family) => chosen_family.as_ref() == Some(&family.id),
                };
                self.theme_card(entry, chosen, adaptive, cx)
            })
            .collect::<Vec<_>>();

        if !built_in && chosen_family.is_none() {
            cards.push(
                OptionCard::new("unavailable-theme")
                    .w(CARD)
                    .thumb_height(THUMB)
                    .label(t!("theme-unavailable", name = selected.clone()))
                    .selected(true)
                    .disabled(true)
                    .into_any_element(),
            );
        }

        let settings = self.settings.clone();
        cards.push(
            OptionCard::new("open-theme-folder")
                .w(CARD)
                .thumb_height(THUMB)
                .dashed()
                .thumb(
                    div()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            svg()
                                .path(icons::path("icons/folder-plus.svg"))
                                .size(GLYPH)
                                .text_color(theme.muted_foreground),
                        ),
                )
                .label(t!("settings-theme-folder"))
                .on_click(move |_, _, cx| {
                    let path =
                        settings.update(cx, |settings, _| settings.ensure_themes_directory());
                    if let Err(error) = open::that_detached(&path) {
                        log::warn!("settings: cannot open {}: {error}", path.display());
                    }
                })
                .into_any_element(),
        );

        // the ring around the chosen card stands outside it, so the rail leaves it room
        let room = theme.metrics.pad;
        Rail::new("theme-cards")
            .py(room)
            .arrows_at(room + THUMB / 2.)
            .children(cards)
            .into_any_element()
    }

    /// One card: its miniature and its name. A family of several variants names the variant a
    /// click picks after its own name, and that line opens the menu of all of them.
    fn theme_card(
        &self,
        entry: Entry,
        chosen: bool,
        adaptive: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let look = self.look(cx);
        let radius = cx.theme().radius;

        match entry {
            Entry::Kind(kind) => {
                let palettes = match kind {
                    ThemeKind::System => vec![Theme::dark(), Theme::light()],
                    kind => vec![Theme::for_kind(kind)],
                };
                OptionCard::new(kind.id())
                    .w(CARD)
                    .thumb_height(THUMB)
                    .thumb(sliced(&palettes, radius))
                    .label(kind.label())
                    .selected(chosen)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let overrides = this.settings.update(cx, |settings, cx| {
                            settings.set_theme(kind.id(), cx);
                            settings.theme_overrides()
                        });
                        Theme::fade(Look { kind, ..look }, &overrides, cx);
                        cx.notify();
                    }))
                    .into_any_element()
            }
            Entry::Family(family) => {
                let palettes = family
                    .variants
                    .iter()
                    .take(SLICES)
                    .map(|variant| variant.palette())
                    .collect::<Vec<_>>();
                let several = family.variants.len() > 1;
                // the variant on show while the family is chosen, else the one a click would pick
                let settings = self.settings.read(cx);
                let picked = match chosen {
                    true => settings.theme_variant().map(|(_, variant)| variant),
                    false => settings.family_variant(&family.id, cx),
                }
                .filter(|_| several)
                .map(|variant| (variant.id.clone(), variant.name.clone()));
                let label = match &picked {
                    Some((_, name)) => format!("{} {name}", family.name),
                    None => family.name.clone(),
                };
                let id = family.id.clone();

                let card =
                    OptionCard::new(SharedString::from(format!("theme-family:{}", family.id)))
                        .w(CARD)
                        .thumb_height(THUMB)
                        .thumb(sliced(&palettes, radius))
                        .label(label)
                        .selected(chosen)
                        .disabled(adaptive)
                        .when(adaptive, |card| card.tooltip("settings-theme-unavailable"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let overrides = this.settings.update(cx, |settings, cx| {
                                settings.set_theme_family(&id, cx);
                                settings.theme_overrides()
                            });
                            Theme::fade(
                                Look {
                                    kind: ThemeKind::Dark,
                                    ..look
                                },
                                &overrides,
                                cx,
                            );
                            cx.notify();
                        }));
                match picked {
                    Some((picked, _)) => self.with_variants(card, &family, &picked, cx),
                    None => card,
                }
                .into_any_element()
            }
        }
    }

    /// Gives a family's card the menu of its variants, `picked` checked, opened from its name.
    /// Only the family whose menu opened last owns the popover, so the name on any other card
    /// hands it to its own.
    fn with_variants(
        &self,
        card: OptionCard,
        family: &ThemeFamily,
        picked: &str,
        cx: &mut Context<Self>,
    ) -> OptionCard {
        if self.theme_variants.as_deref() != Some(family.id.as_str()) {
            let id = family.id.clone();
            return card.on_variants(cx.listener(move |this, _, _, cx| {
                this.theme_variants = Some(id.clone());
                this.popovers.open(VARIANTS);
                cx.notify();
            }));
        }

        let look = self.look(cx);
        let items = family.variants.iter().map(|variant| {
            let id = variant.id.clone();
            MenuItem::new(
                SharedString::from(format!("theme-variant:{}", variant.id)),
                variant.name.clone(),
            )
            .leading(swatch(&variant.palette()))
            .checked(picked == variant.id)
            .on_click(cx.listener(move |this, _, _, cx| {
                let overrides = this.settings.update(cx, |settings, cx| {
                    settings.set_theme(id.clone(), cx);
                    settings.theme_overrides()
                });
                Theme::fade(
                    Look {
                        kind: ThemeKind::Dark,
                        ..look
                    },
                    &overrides,
                    cx,
                );
                this.popovers.close();
                cx.notify();
            }))
        });
        let menu = Menu::new("theme-variants-menu")
            .w(Picker::NARROW)
            .items(items);

        card.variant_menu(VARIANTS, &self.popovers, menu)
    }
}

/// The miniature cut into one vertical slice per palette, left to right, each slice the same
/// window painted in its own palette so the layout runs on across the cuts. The cuts run down
/// `SEAMS` and show only as the change of colour. `radius` is the card's, whose inside the
/// miniature's outer corners follow.
fn sliced(palettes: &[Theme], radius: Pixels) -> Div {
    let count = palettes.len().max(1);
    let cuts = (1..count).map(|index| {
        let even = WIDTH * index as f32 / count as f32;
        SEAMS
            .into_iter()
            .min_by(|a, b| (a - even).abs().total_cmp(&(b - even).abs()))
            .unwrap_or(even)
    });
    let edges: Vec<f32> = std::iter::once(0.)
        .chain(cuts)
        .chain(std::iter::once(WIDTH))
        .collect();
    let mark = (radius * RADIUS_SCALE).floor();
    // the card does not clip to its corners, so the miniature rounds its own inside the border
    let outer = (radius - EDGE).max(Pixels::ZERO).min(px(SIDEBAR / 2.));

    div().relative().size_full().children(
        palettes
            .iter()
            .zip(edges.windows(2))
            // each slice runs one pixel on under the next, so a cut that lands between pixels
            // while the rail scrolls never opens a seam, but stops short of the far corners,
            // whose soft edges only the last slice draws
            .map(|(palette, edge)| {
                let left = edge[0];
                let right = match edge[1] < WIDTH {
                    true => edge[1] + SEAM_OVERLAP,
                    false => WIDTH,
                };
                rect([left, 0., right - left, HEIGHT])
                    .overflow_hidden()
                    .child(miniature(palette, outer, mark).left(px(-left)))
            }),
    )
}

/// The Sonora window in `palette`'s colours as a few soft, flat shapes, `WIDTH` by `HEIGHT`
/// pixels: the sidebar with three entries, a heading over a row of tiles, the first in the
/// accent, and a floating player with its play button. Nothing is outlined. The window and the
/// sidebar round the outer corners by `outer`, and `mark` rounds the tiles.
fn miniature(palette: &Theme, outer: Pixels, mark: Pixels) -> Div {
    let panel = match palette.sidebar == palette.background {
        true => palette.secondary,
        false => palette.sidebar,
    };
    let bar = |shape: [f32; 4], strength: f32| {
        rect(shape)
            .rounded_full()
            .bg(palette.foreground.opacity(strength))
    };

    div()
        .absolute()
        .top_0()
        .w(px(WIDTH))
        .h(px(HEIGHT))
        .rounded(outer)
        .bg(palette.background)
        .child(rect([0., 0., SIDEBAR, HEIGHT]).rounded_l(outer).bg(panel))
        .children(ENTRIES.into_iter().enumerate().map(|(index, entry)| {
            bar(
                entry,
                match index {
                    0 => ACTIVE_BAR,
                    _ => BAR,
                },
            )
        }))
        .child(bar(HEADING, ACTIVE_BAR))
        .children(TILES.into_iter().enumerate().map(|(index, left)| {
            let fill = match index {
                0 => palette.primary.opacity(ACCENT),
                _ => palette.muted,
            };
            rect([left, TILE_TOP, TILE, TILE]).rounded(mark).bg(fill)
        }))
        .child(rect(PLAYER).rounded_full().bg(panel))
        .child(rect(PLAY).rounded_full().bg(palette.primary))
}

/// A variant's swatch in the menu: a circle cut on the diagonal into its page colour at the top
/// left and its accent at the bottom right. The cut blends over one pixel so its edge stays
/// smooth.
fn swatch(palette: &Theme) -> Div {
    let blend = 0.5 / SWATCH;
    div()
        .size(px(SWATCH))
        .flex_none()
        .rounded_full()
        .bg(linear_gradient(
            135.,
            linear_color_stop(palette.background, 0.5 - blend),
            linear_color_stop(palette.primary, 0.5 + blend),
        ))
}

/// A box in the miniature from its left, top, width and height in pixels.
fn rect([x, y, width, height]: [f32; 4]) -> Div {
    div()
        .absolute()
        .left(px(x))
        .top(px(y))
        .w(px(width))
        .h(px(height))
}
