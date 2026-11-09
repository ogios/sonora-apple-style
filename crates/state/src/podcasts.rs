use std::collections::HashSet;

use gpui::{Context, Entity, Task};
use music::{Podcast, PodcastScope, Track};
use tokio::task::AbortHandle;

use crate::{Io, Session, SessionEvent, join};

/// One independently loaded tab, including its pagination and cancellable request.
#[derive(Default)]
struct PodcastCollection {
    podcasts: Vec<Podcast>,
    tracks: Vec<Track>,
    total: Option<usize>,
    next_offset: Option<usize>,
    loaded: bool,
    loading: bool,
    error: Option<String>,
    task: Option<Task<()>>,
    request: Option<AbortHandle>,
}

impl Drop for PodcastCollection {
    fn drop(&mut self) {
        if let Some(request) = self.request.take() {
            request.abort();
        }
    }
}

/// The account's four podcast collections. First pages load together to supply the tab counts.
pub struct Podcasts {
    scope: PodcastScope,
    collections: [PodcastCollection; 4],
    session: Entity<Session>,
    io: Io,
}

impl Podcasts {
    pub fn new(session: Entity<Session>, io: Io, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&session, |this, _, event, cx| match event {
            SessionEvent::SignedOut => {
                this.collections = std::array::from_fn(|_| PodcastCollection::default());
                cx.notify();
            }
            SessionEvent::SignedIn => this.load(cx),
            _ => {}
        })
        .detach();
        Self {
            scope: PodcastScope::Saved,
            collections: std::array::from_fn(|_| PodcastCollection::default()),
            session,
            io,
        }
    }

    pub fn scope(&self) -> PodcastScope {
        self.scope
    }

    pub fn count(&self, scope: PodcastScope) -> Option<usize> {
        self.collections[scope.index()].total
    }

    pub fn items(&self) -> &[Podcast] {
        &self.collections[self.scope.index()].podcasts
    }

    pub fn tracks(&self) -> &[Track] {
        &self.collections[self.scope.index()].tracks
    }

    pub fn is_loading(&self) -> bool {
        self.collections[self.scope.index()].loading
    }

    pub fn has_more(&self) -> bool {
        self.collections[self.scope.index()].next_offset.is_some()
    }

    pub fn error(&self) -> Option<&str> {
        self.collections[self.scope.index()].error.as_deref()
    }

    pub fn select(&mut self, scope: PodcastScope, cx: &mut Context<Self>) {
        self.scope = scope;
        if !self.collections[scope.index()].loaded {
            self.load_page(scope, 0, cx);
        }
        cx.notify();
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.collections[self.scope.index()] = PodcastCollection::default();
        self.load_page(self.scope, 0, cx);
    }

    pub fn load(&mut self, cx: &mut Context<Self>) {
        for scope in PodcastScope::ALL {
            if !self.collections[scope.index()].loaded {
                self.load_page(scope, 0, cx);
            }
        }
    }

    pub fn load_more(&mut self, cx: &mut Context<Self>) {
        if let Some(offset) = self.collections[self.scope.index()].next_offset {
            self.load_page(self.scope, offset, cx);
        }
    }

    /// Loads one tab without blocking the others; a failed continuation preserves its earlier rows.
    fn load_page(&mut self, scope: PodcastScope, offset: usize, cx: &mut Context<Self>) {
        if !self.session.read(cx).capabilities().podcasts {
            return;
        }
        let Some(client) = self.session.read(cx).client() else {
            return;
        };
        let collection = &mut self.collections[scope.index()];
        if collection.loading {
            return;
        }
        collection.loading = true;
        collection.error = None;
        cx.notify();

        let request = self
            .io
            .spawn(async move { client.podcast_library(scope, offset).await });
        collection.request = Some(request.abort_handle());
        collection.task = Some(cx.spawn(async move |this, cx| {
            let result = join(request).await;
            this.update(cx, |this, cx| {
                let collection = &mut this.collections[scope.index()];
                collection.loading = false;
                collection.request = None;
                match crate::settled(result, cx) {
                    Ok(page) => {
                        collection.total = Some(page.total);
                        collection.next_offset = page.next_offset;
                        let mut podcasts: HashSet<String> = collection
                            .podcasts
                            .iter()
                            .map(|item| item.id.clone())
                            .collect();
                        collection.podcasts.extend(
                            page.podcasts
                                .into_iter()
                                .filter(|item| podcasts.insert(item.id.clone())),
                        );
                        let mut tracks: HashSet<String> = collection
                            .tracks
                            .iter()
                            .filter_map(|track| track.id.clone())
                            .collect();
                        collection
                            .tracks
                            .extend(page.tracks.into_iter().filter(|track| {
                                track.id.as_ref().is_none_or(|id| tracks.insert(id.clone()))
                            }));
                        collection.loaded = true;
                    }
                    Err(reason) => collection.error = Some(reason),
                }
                cx.notify();
            })
            .ok();
        }));
    }
}

/// A podcast visit and its loaded episodes. Further pages append without discarding earlier ones.
pub struct PodcastDetail {
    id: Option<String>,
    podcast: Option<Podcast>,
    tracks: Vec<Track>,
    next_offset: Option<usize>,
    loading: bool,
    error: Option<String>,
    session: Entity<Session>,
    io: Io,
    task: Option<Task<()>>,
    request: Option<AbortHandle>,
}

impl PodcastDetail {
    pub fn new(session: Entity<Session>, io: Io, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&session, |this, _, event, cx| match event {
            SessionEvent::SignedOut => {
                this.clear();
                this.id = None;
                cx.notify();
            }
            SessionEvent::SignedIn if this.id.is_some() => this.refresh(cx),
            _ => {}
        })
        .detach();
        Self {
            id: None,
            podcast: None,
            tracks: Vec::new(),
            next_offset: None,
            loading: false,
            error: None,
            session,
            io,
            task: None,
            request: None,
        }
    }

    pub fn podcast(&self) -> Option<&Podcast> {
        self.podcast.as_ref()
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn has_more(&self) -> bool {
        self.next_offset.is_some()
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn open(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.id.as_deref() == Some(id) && (self.loading || self.podcast.is_some()) {
            return;
        }
        self.clear();
        self.id = Some(id.to_owned());
        self.load(0, cx);
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.clear();
        self.load(0, cx);
    }

    pub fn load_more(&mut self, cx: &mut Context<Self>) {
        if let Some(offset) = self.next_offset {
            self.load(offset, cx);
        }
    }

    /// Fetches the first page with its metadata, or appends one continuation page.
    fn load(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.loading || !self.session.read(cx).capabilities().podcasts {
            return;
        }
        let Some(id) = self.id.clone() else {
            return;
        };
        let Some(client) = self.session.read(cx).client() else {
            return;
        };
        self.loading = true;
        self.error = None;
        cx.notify();
        let request = self.io.spawn(async move {
            let podcast = match offset {
                0 => Some(client.podcast(&id).await?),
                _ => None,
            };
            let page = client.podcast_episodes(&id, offset).await?;
            anyhow::Ok((podcast, page))
        });
        self.request = Some(request.abort_handle());
        self.task = Some(cx.spawn(async move |this, cx| {
            let result = join(request).await;
            this.update(cx, |this, cx| {
                this.loading = false;
                this.request = None;
                match crate::settled(result, cx) {
                    Ok((podcast, page)) => {
                        if let Some(podcast) = podcast {
                            this.podcast = Some(podcast);
                        }
                        this.next_offset = page.next_offset;
                        let mut seen: HashSet<String> = this
                            .tracks
                            .iter()
                            .filter_map(|track| track.id.clone())
                            .collect();
                        this.tracks.extend(page.tracks.into_iter().filter(|track| {
                            track.id.as_ref().is_none_or(|id| seen.insert(id.clone()))
                        }));
                    }
                    Err(reason) => this.error = Some(reason),
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn clear(&mut self) {
        self.task = None;
        if let Some(request) = self.request.take() {
            request.abort();
        }
        self.podcast = None;
        self.tracks.clear();
        self.next_offset = None;
        self.loading = false;
        self.error = None;
    }
}

impl Drop for PodcastDetail {
    fn drop(&mut self) {
        if let Some(request) = self.request.take() {
            request.abort();
        }
    }
}
