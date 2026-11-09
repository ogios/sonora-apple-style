//! Browser request transport. Native pages are owned exclusively by connected GUI processes.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};

use anyhow::{Context as _, Result, bail, ensure};
use protocol::browser::{Page as Request, Reply};
use serde_json::Value;
use tokio::sync::mpsc;

use crate::runtime::{App, Task};
pub(crate) use protocol::browser::Target;

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// Connected browser capabilities, live requests and private response values.
#[derive(Default)]
struct State {
    next: u64,
    clients: BTreeMap<u64, String>,
    pages: BTreeMap<u64, Request>,
    replies: HashMap<u64, Reply>,
    changed: Option<mpsc::UnboundedSender<()>>,
}

/// A pending browser operation, cancelled when its owning service task releases it.
pub(crate) struct Page(u64);

/// One poll of a GUI-owned browser operation.
pub(crate) enum Poll {
    Pending,
    Closed,
    Cookies(String),
}

impl State {
    /// Schedules a publication after the current service mutation finishes.
    fn changed(&self) {
        if let Some(changed) = &self.changed {
            changed.send(()).ok();
        }
    }
}

impl Page {
    pub fn open(target: Target) -> Result<Self> {
        STATE.with_borrow_mut(|state| {
            let Some((_, client)) = state.clients.last_key_value() else {
                bail!("no connected GUI has a browser engine");
            };
            let client = client.clone();
            state.next += 1;
            let id = state.next;
            state.pages.insert(
                id,
                Request {
                    id,
                    client,
                    target,
                    script: None,
                },
            );
            state.changed();
            Ok(Self(id))
        })
    }

    pub fn poll(&mut self) -> Poll {
        self.peek()
    }

    pub fn peek(&mut self) -> Poll {
        STATE.with_borrow(|state| match state.replies.get(&self.0) {
            Some(Reply::Closed) => Poll::Closed,
            Some(Reply::Cookies(header)) => Poll::Cookies(header.clone()),
            None if state.pages.contains_key(&self.0) => Poll::Pending,
            None => Poll::Closed,
        })
    }

    pub fn run(&mut self, script: &str) {
        STATE.with_borrow_mut(|state| {
            if let Some(page) = state.pages.get_mut(&self.0) {
                let sequence = page.script.as_ref().map_or(1, |(sequence, _)| sequence + 1);
                page.script = Some((sequence, script.to_owned()));
                state.changed();
            }
        });
    }
}

impl Drop for Page {
    fn drop(&mut self) {
        STATE.with_borrow_mut(|state| {
            state.pages.remove(&self.0);
            state.replies.remove(&self.0);
            state.changed();
        });
    }
}

/// Reports capabilities advertised by live GUIs, without probing a display or browser library.
pub(crate) fn supported() -> bool {
    STATE.with_borrow(|state| !state.clients.is_empty())
}

/// Installs request publication in the service loop.
pub(crate) fn attach(cx: &mut App) -> Task<()> {
    let (sender, mut changed) = mpsc::unbounded_channel();
    STATE.with_borrow_mut(|state| state.changed = Some(sender));
    cx.spawn(async move |cx| {
        while changed.recv().await.is_some() {
            while changed.try_recv().is_ok() {}
            cx.update(crate::server::browser_changed);
        }
    })
}

/// Browser instructions contain no returned cookies or tokens.
pub(crate) fn snapshot() -> Value {
    STATE
        .with_borrow(|state| serde_json::json!({"pages": state.pages.values().collect::<Vec<_>>()}))
}

/// Binds a browser capability to its authenticated local connection.
pub(crate) fn connect(peer: u64, client: String) -> Result<()> {
    ensure!(
        !client.is_empty() && client.len() <= 128,
        "invalid browser client identity"
    );
    STATE.with_borrow_mut(|state| {
        ensure!(
            !state
                .clients
                .iter()
                .any(|(owner, found)| *owner != peer && found == &client),
            "browser client identity is already registered"
        );
        state.clients.insert(peer, client);
        state.changed();
        Ok(())
    })
}

/// Accepts a result only from the GUI connection assigned to the page.
pub(crate) fn reply(peer: u64, id: u64, reply: Reply) -> Result<()> {
    STATE.with_borrow_mut(|state| {
        let client = state
            .clients
            .get(&peer)
            .context("the client has no browser capability")?;
        let request = state
            .pages
            .get(&id)
            .context("the browser request was released")?;
        ensure!(
            &request.client == client,
            "the browser request belongs to another client"
        );
        state.replies.insert(id, reply);
        Ok(())
    })
}

/// A disconnected GUI closes its pages; accepted credentials are never replayed.
pub(crate) fn disconnect(peer: u64) {
    STATE.with_borrow_mut(|state| {
        let Some(client) = state.clients.remove(&peer) else {
            return;
        };
        let ids = state
            .pages
            .iter()
            .filter_map(|(id, page)| (page.client == client).then_some(*id))
            .collect::<Vec<_>>();
        for id in ids {
            let page = state.pages.remove(&id).expect("the browser request exists");
            let accepted = page.target.script.is_none()
                && matches!(state.replies.get(&id), Some(Reply::Cookies(_)));
            if !accepted {
                state.replies.insert(id, Reply::Closed);
            }
        }
        state.changed();
    });
}
