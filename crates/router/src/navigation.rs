use gpui::{Context, EventEmitter};

use crate::{Destination, SettingsPage};

/// The history direction of a page change, so shells can animate arrivals and departures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationDirection {
    Forward,
    Backward,
}

/// A destination change together with the direction it takes through history.
pub enum NavigationEvent {
    Moved {
        destination: Destination,
        direction: NavigationDirection,
        entry: u64,
        tab_switch: bool,
    },
}

pub struct Navigation {
    trail: Vec<(u64, Destination)>,
    next_entry: u64,
    at: usize,
    /// The settings destination last on show, which opening settings again returns to.
    last_settings: Option<Destination>,
}

impl EventEmitter<NavigationEvent> for Navigation {}

impl Navigation {
    pub fn new(start: Destination) -> Self {
        Self {
            trail: vec![(0, start)],
            next_entry: 1,
            at: 0,
            last_settings: None,
        }
    }

    pub fn current(&self) -> Destination {
        self.trail[self.at].1.clone()
    }

    /// The identity of the current history visit, retained when moving back or forward.
    pub fn current_entry(&self) -> u64 {
        self.trail[self.at].0
    }

    /// Whether a visit still belongs to history after a new branch or replacement.
    pub fn has_entry(&self, entry: u64) -> bool {
        self.trail.iter().any(|(id, _)| *id == entry)
    }

    pub fn can_go_back(&self) -> bool {
        self.at > 0
    }

    pub fn can_go_forward(&self) -> bool {
        self.at + 1 < self.trail.len()
    }

    pub fn go(&mut self, destination: Destination, cx: &mut Context<Self>) {
        self.go_with_tab(destination, false, cx);
    }

    /// Switches peer tabs with a content transition, retaining normal Back and Forward history.
    pub fn go_tab(&mut self, destination: Destination, cx: &mut Context<Self>) {
        self.go_with_tab(destination, true, cx);
    }

    /// Adds a visit, distinguishing a peer-tab change from page navigation.
    fn go_with_tab(&mut self, destination: Destination, tab_switch: bool, cx: &mut Context<Self>) {
        if self.current() == destination {
            return;
        }

        let replacing = self.current() == Destination::Fullscreen;
        self.trail.truncate(self.at + 1);
        let entry = (self.next_entry, destination);
        self.next_entry += 1;
        match replacing {
            true => self.trail[self.at] = entry,
            false => {
                self.trail.push(entry);
                self.at = self.trail.len() - 1;
            }
        }
        self.arrive(NavigationDirection::Forward, tab_switch, cx);
    }

    pub fn back(&mut self, cx: &mut Context<Self>) {
        if !self.can_go_back() {
            return;
        }
        self.at -= 1;
        self.arrive(NavigationDirection::Backward, false, cx);
    }

    /// Replaces the current page without leaving its stale destination in navigation history.
    pub fn replace(&mut self, destination: Destination, cx: &mut Context<Self>) {
        if self.current() == destination {
            return;
        }
        self.trail[self.at] = (self.next_entry, destination);
        self.next_entry += 1;
        self.arrive(NavigationDirection::Forward, false, cx);
    }

    /// Returns from a drill-in to `parent`. Stepping back keeps the trail free of a parent,
    /// drill-in, parent loop when the page before shows the parent's settings tab, which also
    /// puts that tab back where it was scrolled to. Anywhere else the drill-in is replaced so Back
    /// does not reopen it.
    pub fn up(&mut self, parent: Destination, cx: &mut Context<Self>) {
        let tab = |destination: &Destination| destination.settings_page().map(SettingsPage::tab);
        let before = self.at.checked_sub(1).map(|at| &self.trail[at].1);
        match before.is_some_and(|before| {
            *before == parent || (tab(before).is_some() && tab(before) == tab(&parent))
        }) {
            true => self.back(cx),
            false => {
                if self.current() == parent {
                    return;
                }
                self.trail[self.at] = (self.next_entry, parent);
                self.next_entry += 1;
                self.arrive(NavigationDirection::Backward, false, cx);
            }
        }
    }

    /// Opens settings on the destination last shown there, or on General the first time.
    pub fn open_settings(&mut self, cx: &mut Context<Self>) {
        let destination = self
            .last_settings
            .clone()
            .unwrap_or(Destination::Settings(SettingsPage::General));
        self.go(destination, cx);
    }

    pub fn forward(&mut self, cx: &mut Context<Self>) {
        if !self.can_go_forward() {
            return;
        }
        self.at += 1;
        self.arrive(NavigationDirection::Forward, false, cx);
    }

    fn arrive(&mut self, direction: NavigationDirection, tab_switch: bool, cx: &mut Context<Self>) {
        if self.current().in_settings() {
            self.last_settings = Some(self.current());
        }
        cx.emit(NavigationEvent::Moved {
            destination: self.current(),
            direction,
            entry: self.current_entry(),
            tab_switch,
        });
        cx.notify();
    }
}
