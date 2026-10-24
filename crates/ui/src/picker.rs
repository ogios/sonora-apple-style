use gpui::prelude::*;
use gpui::{AnyElement, App, ClickEvent, Hsla, Pixels, SharedString, StyleRefinement, Window, px};

use crate::button::Button;
use crate::menu::{Menu, MenuItem, TRIGGER_GAP};
use crate::popover::{Open, Popover, Popovers};
use crate::scrollbar::Scrollbar;
use crate::theme::ActiveTheme as _;

enum Face {
    Label(SharedString),
    Plain(SharedString),
    Icon(&'static str),
}

#[derive(IntoElement)]
pub struct Picker {
    style: StyleRefinement,
    key: &'static str,
    group: Popovers,
    face: Face,
    /// A small element drawn before the current choice on the trigger.
    leading: Option<AnyElement>,
    tooltip: Option<&'static str>,
    above: bool,
    tint: Option<Hsla>,
    selected: bool,
    small: bool,
    width: Pixels,
    left: bool,
    sticky: bool,
    disabled: bool,
    items: Vec<MenuItem>,
    menu: Option<Menu>,
    /// How tall the menu grows before it scrolls, unbounded when `None`.
    scroll_after: Option<Pixels>,
    on_open: Option<Open>,
    /// The side of a round glass trigger, for an icon picker standing among round buttons.
    round: Option<Pixels>,
}

impl Picker {
    pub const NARROW: Pixels = px(170.);
    pub const REGULAR: Pixels = px(190.);
    pub const WIDE: Pixels = px(260.);

    pub fn new(key: &'static str, group: &Popovers, current: impl Into<SharedString>) -> Self {
        Self {
            style: StyleRefinement::default(),
            key,
            group: group.clone(),
            face: Face::Label(current.into()),
            leading: None,
            tooltip: None,
            above: false,
            tint: None,
            selected: false,
            small: true,
            width: Self::REGULAR,
            left: false,
            sticky: false,
            disabled: false,
            items: Vec::new(),
            menu: None,
            scroll_after: None,
            on_open: None,
            round: None,
        }
    }

    pub fn icon(key: &'static str, group: &Popovers, icon: &'static str) -> Self {
        Self {
            face: Face::Icon(icon),
            ..Self::new(key, group, "")
        }
    }

    /// A labelled trigger without the chevron, for a popover that holds a control rather than a
    /// list of choices.
    pub fn plain(key: &'static str, group: &Popovers, label: impl Into<SharedString>) -> Self {
        Self {
            face: Face::Plain(label.into()),
            ..Self::new(key, group, "")
        }
    }

    /// Draws `leading` before the current choice on the trigger, as the menu's items may draw
    /// one before theirs.
    pub fn leading(mut self, leading: impl IntoElement) -> Self {
        self.leading = Some(leading.into_any_element());
        self
    }

    pub fn tooltip(mut self, key: &'static str) -> Self {
        self.tooltip = Some(key);
        self
    }

    pub fn tooltip_above(mut self, key: &'static str) -> Self {
        self.tooltip = Some(key);
        self.above = true;
        self
    }

    pub fn tint(mut self, tint: Hsla) -> Self {
        self.tint = Some(tint);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Draws an icon trigger as a round button `side` across in the filled style, which is a
    /// slab of glass under Liquid Glass, for a picker among the round buttons of a hero.
    pub fn round(mut self, side: Pixels) -> Self {
        self.round = Some(side);
        self.small = false;
        self
    }

    pub fn large(mut self) -> Self {
        self.small = false;
        self
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    pub fn left(mut self) -> Self {
        self.left = true;
        self
    }

    pub fn sticky(mut self) -> Self {
        self.sticky = true;
        self
    }

    /// Holds the trigger still, so the menu cannot open, as while an earlier choice is applied.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn item(mut self, item: MenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = MenuItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Runs `handler` when a click opens the menu, as for reading afresh what it lists.
    pub fn on_open(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Box::new(handler));
        self
    }

    pub fn menu(mut self, menu: Menu) -> Self {
        self.menu = Some(menu);
        self
    }

    /// Caps the menu at `height` and scrolls what does not fit. Ignored when a whole `menu` is
    /// given, which brings its own.
    pub fn scroll_after(mut self, height: Pixels) -> Self {
        self.scroll_after = Some(height);
        self
    }
}

impl Styled for Picker {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Picker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            style,
            key,
            group,
            face,
            leading,
            tooltip,
            above,
            tint,
            selected,
            small,
            width,
            left,
            sticky,
            disabled,
            items,
            menu,
            scroll_after,
            on_open,
            round,
        } = self;

        let theme = *cx.theme();
        let drop = match small {
            true => theme.metrics.control_small,
            false => theme.metrics.control,
        } + TRIGGER_GAP;

        let button = match face {
            Face::Label(current) => Button::new(SharedString::from(format!("{key}-picker")))
                .label(current)
                .trailing("icons/chevron-down.svg")
                .outline(),
            Face::Plain(label) => Button::new(SharedString::from(format!("{key}-picker")))
                .label(label)
                .outline(),
            Face::Icon(icon) => Button::new(SharedString::from(format!("{key}-picker")))
                .icon(icon)
                .ghost(),
        };
        let button = button
            .when(small, Button::small)
            .when_some(round, |button, side| button.secondary().size(side).px_0())
            .when_some(leading, Button::leading)
            .when_some(tooltip, |button, key| match above {
                true => button.tooltip_above(key),
                false => button.tooltip(key),
            })
            .when_some(tint, Button::tint)
            .disabled(disabled);

        let menu = menu.unwrap_or_else(|| {
            let menu = Menu::new(SharedString::from(format!("{key}-menu"))).w(width);
            match scroll_after {
                Some(height) => {
                    let view = window.current_view();
                    let scrollbar = window.use_keyed_state(
                        SharedString::from(format!("{key}-menu-scroll")),
                        cx,
                        |_, _| Scrollbar::inset().watching(view),
                    );
                    menu.max_h(height).scrollbar(scrollbar)
                }
                None => menu,
            }
        });
        let menu = menu.items(items).top(drop).when_else(
            left,
            |menu| menu.left_0(),
            |menu| menu.right_0(),
        );

        let mut popover = Popover::new(key, group)
            .button(button)
            .selected(selected)
            .menu(menu)
            .when(!sticky, Popover::commands)
            .when_some(on_open, |popover, on_open| popover.on_open(on_open));
        *popover.style() = style;
        popover
    }
}
