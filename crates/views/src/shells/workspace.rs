use std::time::Duration;

use gpui::prelude::*;
use gpui::{AnyView, App, Context, Entity, FocusHandle, Pixels, Render, StyleRefinement};
use gpui::{Window, deferred, div};
use input::WORKSPACE_CONTEXT;
use state::{AppSettings, Playback, Queue, SideTab, Sonora};
use ui::{
    Activate, ActiveTheme as _, Deselect, Entrance, Remove, SelectNext, SelectPrevious, entering,
    settling, shown_listing, veiled,
};

use crate::chrome::{
    AppNotice, Chrome, JamCode, JamJoin, PlayerBar, SidebarLeft, SidebarRight, TitleBarOptions,
    ToastStack,
};
use crate::shared::confirm::Confirm;
use crate::shared::menus::CardMenu;
use crate::shared::playlist_editor::PlaylistEditor;
use crate::shared::tag_editor::TagEditor;
use crate::shared::widevine::WidevinePrompt;
use crate::shells::Shell;

pub(crate) struct Workspace {
    sidebar: Entity<SidebarLeft>,
    player_bar: Entity<PlayerBar>,
    playback: Entity<Playback>,
    /// How tall the strip over the player bar stands, which the pages' bottom clearance adds.
    strip: Pixels,
    settings: Entity<AppSettings>,
    sidebar_right: Entity<SidebarRight>,
    playlist_editor: Entity<PlaylistEditor>,
    tag_editor: Entity<TagEditor>,
    confirm: Entity<Confirm>,
    card_menu: Entity<CardMenu>,
    widevine: Entity<WidevinePrompt>,
    jam_join: Entity<JamJoin>,
    jam_code: Entity<JamCode>,
    toasts: Entity<ToastStack>,
    notice: Entity<AppNotice>,
    content: AnyView,
    /// A screen's own header, floated over the top of the page and outside its transition.
    /// The page pads itself to start beneath it.
    header: Option<AnyView>,
    /// Whether the page is laid out cached. A page that redraws part of itself every frame is
    /// not, since a cached view that redraws builds every view in it again, and it caches its
    /// own parts instead.
    cached: bool,
    transition: Option<Entrance>,
    /// The left and right sidebars' widths sliding as they open and close.
    left_drawer: ui::Drawer,
    right_drawer: ui::Drawer,
    focus: FocusHandle,
}

impl Workspace {
    pub fn new(
        playback: Entity<Playback>,
        queue: Entity<Queue>,
        content: AnyView,
        cx: &mut Context<Self>,
    ) -> Self {
        let sidebar = cx.new(SidebarLeft::new);
        let sidebar_right = cx.new(|cx| SidebarRight::new(queue.clone(), playback.clone(), cx));
        let player_bar = cx.new(|cx| PlayerBar::new(playback.clone(), queue, cx));
        let settings = Sonora::global(cx).settings.clone();
        cx.observe(&settings, |_, _, cx| cx.notify()).detach();
        let devices = Sonora::global(cx).devices.clone();
        cx.observe(&devices, |this, _, cx| this.restrip(cx))
            .detach();
        cx.observe(&playback, |this, _, cx| this.restrip(cx))
            .detach();

        Self {
            sidebar,
            player_bar,
            playback,
            strip: Pixels::ZERO,
            settings,
            sidebar_right,
            playlist_editor: PlaylistEditor::entity(cx),
            tag_editor: TagEditor::entity(cx),
            confirm: Confirm::entity(cx),
            card_menu: CardMenu::entity(cx),
            widevine: cx.new(WidevinePrompt::new),
            jam_join: cx.new(JamJoin::new),
            jam_code: cx.new(JamCode::new),
            toasts: cx.new(ToastStack::new),
            notice: cx.new(AppNotice::new),
            content,
            header: None,
            cached: true,
            transition: None,
            left_drawer: ui::Drawer::new(),
            right_drawer: ui::Drawer::new(),
            focus: cx.focus_handle(),
        }
    }

    /// Reads how tall the strip over the player bar stands, and redraws only when that changes,
    /// since playback notifies on every tick.
    fn restrip(&mut self, cx: &mut Context<Self>) {
        let strip = PlayerBar::strip(self.playback.read(cx), cx);
        if self.strip != strip {
            self.strip = strip;
            self.toasts
                .update(cx, |toasts, cx| toasts.set_strip(strip, cx));
            cx.notify();
        }
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus, cx);
    }

    pub fn toggle_sidebar(&self, cx: &mut Context<Self>) {
        self.sidebar.update(cx, |sidebar, cx| sidebar.toggle(cx));
    }

    pub fn toggle_sidebar_right(&self, cx: &mut Context<Self>) {
        self.sidebar_right.update(cx, |panel, cx| panel.toggle(cx));
    }

    pub fn show_side(&self, tab: SideTab, cx: &mut Context<Self>) {
        self.sidebar_right
            .update(cx, |panel, cx| panel.show(tab, cx));
    }

    #[allow(dead_code)]
    pub fn content(&self) -> &AnyView {
        &self.content
    }

    /// Shows a page, with the header it wants above it or none, and laid out `cached` or not.
    /// The three arrive together so a header can never outlive its screen.
    pub fn set_content(
        &mut self,
        content: AnyView,
        header: Option<AnyView>,
        cached: bool,
        cx: &mut Context<Self>,
    ) {
        self.content = content;
        self.header = header;
        self.cached = cached;
        cx.notify();
    }

    pub fn reveal_content(&mut self, cx: &mut Context<Self>) -> Duration {
        if cx.reduce_motion() {
            self.transition = None;
            return Duration::ZERO;
        }

        let entrance = Entrance::start();
        self.transition = Some(entrance);
        cx.notify();
        entrance.span()
    }

    pub fn finish_transition(&mut self, cx: &mut Context<Self>) {
        if self.transition.take().is_some() {
            cx.notify();
        }
    }

    fn hidden(&mut self, window: &mut Window, cx: &Context<Self>) -> f32 {
        if cx.reduce_motion() {
            self.transition = None;
            return 0.;
        }
        let Some(transition) = self.transition else {
            return 0.;
        };
        if transition.running() {
            window.request_animation_frame();
        }
        transition.hidden()
    }

    #[allow(dead_code)]
    pub fn player_bar(&self) -> &Entity<PlayerBar> {
        &self.player_bar
    }
}

impl Shell for Workspace {
    fn title_bar(&self, content: Option<AnyView>, cx: &App) -> TitleBarOptions {
        let sidebar = self.sidebar.read(cx);

        TitleBarOptions {
            navigation: true,
            sidebar_open: sidebar.is_open(),
            sidebar_right: Some(self.sidebar_right.read(cx).is_open()),
            offset: match sidebar.overlays() {
                true => sidebar.occupied_width(),
                false => Chrome::left(cx),
            },
            border: true,
            content,
            transparent: false,
        }
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let right_open = self.sidebar_right.read(cx).occupied_width(window) > Pixels::ZERO;
        let right_full = self.sidebar_right.read(cx).panel_width();
        let right = self.right_drawer.width(right_open, right_full, window, cx);
        let right_sliding = self.right_drawer.sliding();
        self.sidebar_right
            .update(cx, |sidebar, _| sidebar.set_sliding(right_sliding));
        let reserved = self.sidebar_right.read(cx).reserved_width();
        self.sidebar
            .update(cx, |sidebar, cx| sidebar.adapt(reserved, window, cx));
        // The sidebars slide open and shut: the space each takes eases between nothing and its
        // width, and the panel rides along inside it, clipped, while it does. An overlaid left
        // sidebar floats over the page and takes no space, so it has nothing to slide.
        let left_full = self.sidebar.read(cx).panel_width();
        let left_open = self.sidebar.read(cx).occupied_width() > Pixels::ZERO;
        let left = self.left_drawer.width(left_open, left_full, window, cx);
        let left_sliding = self.left_drawer.sliding() && !self.sidebar.read(cx).overlays();
        self.sidebar
            .update(cx, |sidebar, _| sidebar.set_sliding(left_sliding));
        let overlay_width = self.sidebar.read(cx).overlay_width();
        Chrome::publish(left, right, reserved, cx);
        let covered = self.sidebar_right.read(cx).covers_content(window);
        let overlay = self.sidebar.read(cx).overlays();
        let bar_height = PlayerBar::height(window, cx);
        // Liquid Glass floats the player bar over the page, inset from the window's edges and
        // beside the sidebar's slab, so the page always runs on underneath it.
        let liquid = ui::liquid(cx);
        let gap = match liquid {
            true => ui::liquid_gap(cx),
            false => Pixels::ZERO,
        };
        let underlap = self.settings.read(cx).player_bar_translucent() || liquid;
        let bottom = match underlap {
            true => bar_height + gap,
            false => Pixels::ZERO,
        } + self.strip;
        Chrome::set_bottom(bottom, cx);
        // A cached view is laid out from the style given here and its own root
        // style is never consulted, so it can only be cached while it is in the
        // flow at a width this knows: an overlaid sidebar places itself, and a
        // closed one hides itself and takes no space at all.
        let sidebar_width = self.sidebar.read(cx).occupied_width();
        let sidebar = match (left_sliding, overlay || sidebar_width == gpui::Pixels::ZERO) {
            (true, _) => slot(
                self.sidebar.clone().into_any_element(),
                left,
                left_full,
                false,
            ),
            (false, true) => self.sidebar.clone().into_any_element(),
            (false, false) => self
                .sidebar
                .clone()
                .cached(StyleRefinement::default().w(sidebar_width).h_full())
                .into_any_element(),
        };
        // The header is cached at the height it last measured, so a page that redraws every
        // frame leaves it be. Until it has measured itself it is laid out as it is.
        let top = crate::screens::settings::Layout::of(window, cx);
        let header = self.header.clone().map(|header| match top.measured {
            true => header
                .cached(StyleRefinement::default().w_full().h(top.top))
                .into_any_element(),
            false => header.into_any_element(),
        });
        let hidden = self.hidden(window, cx);
        // The scrim is what fades the outgoing page: it is the page's own colour at
        // full strength, so covering the content with it costs no layout and the
        // content view keeps its cache. A see-through window has no such colour. The
        // scrim would be one more translucent layer over the one the root already
        // paints, and the content area would sit visibly darker than the chrome
        // around it for the length of the transition — a quad can only add coverage,
        // never replace it. There the content carries the fade itself, which is the
        // same curve at the price of leaving the cached layout path while it runs.
        //
        // Liquid Glass is the same case: the page floats over the colour field rather than over
        // the theme's background, so a scrim of the background would cover the field and the
        // page with a flat dark sheet at the start of every switch, a black flash.
        let dissolving = cx.theme().transparent || liquid;
        let scrim = match dissolving {
            true => 0.,
            false => hidden,
        };
        let backdrop = cx.theme().background;
        // That price has to be paid for real, not merely accepted: opacity is baked
        // into a primitive as it is painted, and a cached view replays its primitives
        // until something notifies it, so a fading page that stayed cached would be
        // recorded on the first frame, nearly clear, and replayed that way long after
        // the transition had ended. While it carries its own fade the content is
        // rendered uncached. Every screen sizes its root `size_full`, so the layout
        // is the one the cache would have used, and only the frames of the
        // transition pay for the render.
        let content = match (dissolving && hidden > 0.) || !self.cached {
            true => self.content.clone().into_any_element(),
            false => self
                .content
                .clone()
                .cached(StyleRefinement::default().size_full())
                .into_any_element(),
        };

        div()
            .relative()
            .flex()
            .flex_col()
            .w_full()
            .flex_1()
            .min_h_0()
            .key_context(super::shell_context(WORKSPACE_CONTEXT))
            .track_focus(&self.focus)
            .on_action(|_: &SelectNext, window, cx| {
                if let Some(table) = shown_listing(cx) {
                    table.select_next(window, cx);
                    cx.stop_propagation();
                }
            })
            .on_action(|_: &SelectPrevious, window, cx| {
                if let Some(table) = shown_listing(cx) {
                    table.select_previous(window, cx);
                    cx.stop_propagation();
                }
            })
            .on_action(|_: &Deselect, _, cx| {
                if let Some(table) = shown_listing(cx) {
                    table.deselect(cx);
                    cx.stop_propagation();
                }
            })
            .on_action(|_: &Activate, _, cx| {
                if let Some(table) = shown_listing(cx) {
                    table.activate(cx);
                    cx.stop_propagation();
                }
            })
            .on_action(|_: &Remove, _, cx| {
                if let Some(table) = shown_listing(cx) {
                    table.remove(cx);
                    cx.stop_propagation();
                }
            })
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .when(!overlay, |this| this.child(sidebar))
                    .child(
                        div()
                            .relative()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .ml(overlay_width)
                            // Under Liquid Glass the pages sit on one large rounded sheet,
                            // inset from the sidebars and the window's edges, the way Apple
                            // Music floats its content.
                            .when(liquid, |this| {
                                ui::liquid_sheet(this.ml(overlay_width + gap).mr(gap).mb(gap), cx)
                                    .overflow_hidden()
                            })
                            .when(overlay, |this| this.overflow_hidden())
                            .when(covered, |this| this.hidden())
                            .child(
                                div()
                                    .relative()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .min_h_0()
                                    .when(hidden > 0., |this| this.overflow_hidden())
                                    .child(
                                        div()
                                            .absolute()
                                            .left(-overlay_width)
                                            .right_0()
                                            .top_0()
                                            .bottom_0()
                                            .flex()
                                            .flex_col()
                                            .map(|this| match dissolving {
                                                true => entering(this, hidden),
                                                false => veiled(this, hidden),
                                            })
                                            .when(liquid && hidden > 0., |this| {
                                                settling(this, hidden)
                                            })
                                            .child(content),
                                    )
                                    .when(scrim > 0., |this| {
                                        this.child(
                                            div()
                                                .absolute()
                                                .left_0()
                                                .right_0()
                                                .top_0()
                                                .bottom_0()
                                                .bg(backdrop)
                                                .opacity(scrim),
                                        )
                                    })
                                    // The header comes last, so it paints over the page and
                                    // the scrim, and spans the same width the page does
                                    // beneath an overlaid sidebar.
                                    .when_some(header, |this, header| {
                                        this.child(
                                            div()
                                                .absolute()
                                                .left(-overlay_width)
                                                .right_0()
                                                .top_0()
                                                .child(header),
                                        )
                                    }),
                            ),
                    )
                    .map(|this| match right_sliding {
                        true => this.child(slot(
                            self.sidebar_right.clone().into_any_element(),
                            right,
                            right_full,
                            true,
                        )),
                        false => this.child(self.sidebar_right.clone()),
                    })
                    .when(overlay, |this| this.child(self.sidebar.clone())),
            )
            .child(
                div()
                    .relative()
                    .when(underlap, |bar| bar.absolute().left_0().right_0().bottom_0())
                    .when(liquid, |bar| {
                        // Docked inside the foot of the content sheet, clear of its edges.
                        bar.when(!overlay, |bar| bar.left(left))
                            .right(right)
                            .px(gap * 2.)
                            .pb(gap * 2.)
                    })
                    .child(
                        self.player_bar
                            .clone()
                            .cached(StyleRefinement::default().w_full().h(bar_height)),
                    )
                    .child(self.toasts.clone()),
            )
            // deferred so the cards sit above the sidebars' resize grips, and ahead of the
            // dialogs so those still cover them
            .child(deferred(self.notice.clone()))
            .child(self.card_menu.clone())
            .child(self.playlist_editor.clone())
            .child(self.tag_editor.clone())
            .child(self.confirm.clone())
            .child(self.widevine.clone())
            .child(self.jam_join.clone())
            .child(self.jam_code.clone())
    }
}

/// A sidebar sliding open or shut: `width` of room, clipped, with the panel at its `full` width
/// pinned to the edge it slides from, so it travels in from that edge rather than squeezing.
fn slot(panel: gpui::AnyElement, width: Pixels, full: Pixels, right: bool) -> gpui::AnyElement {
    div()
        .relative()
        .flex_none()
        .h_full()
        .w(width)
        .overflow_hidden()
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .w(full)
                .flex()
                .map(|this| match right {
                    true => this.left_0(),
                    false => this.right_0(),
                })
                .child(panel),
        )
        .into_any_element()
}
