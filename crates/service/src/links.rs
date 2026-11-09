//! Links handed to the app from outside, by the system or on the command line, opened on the
//! provider they belong to.

use crate::runtime::{Context, Entity, EventEmitter, Task};
use music::link::{self, Linked};

use crate::{Devices, Io, Outcome, Session, SessionState, Toasts};

/// A page a link asked for, for the app to navigate to. It never carries a Jam or a short link.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Opened(pub Linked);

/// A link waiting for its provider to be the signed-in one.
struct Waiting {
    slug: &'static str,
    linked: Linked,
    /// Whether the session was already asked to switch to the provider, so a switch that ends
    /// somewhere else drops the link instead of asking again.
    switched: bool,
}

/// Opens links to any provider's pages and Jams. A link to a provider other than the current
/// one switches to it when it has an account stored, and opens once that provider is signed in.
/// A page comes out as an `Opened` event and a Jam goes to `Devices` to be asked about.
pub struct Links {
    session: Entity<Session>,
    devices: Entity<Devices>,
    waiting: Option<Waiting>,
    following: Option<Task<()>>,
}

impl EventEmitter<Opened> for Links {}

impl crate::client::Model for Links {
    #[cfg(sonora_gui)]
    const KIND: &'static str = "links";
    #[cfg(sonora_gui)]
    const DYNAMIC: bool = false;

    #[cfg(not(sonora_gui))]
    fn remote_snapshot(&self, _: &crate::runtime::App) -> serde_json::Value {
        serde_json::json!({})
    }

    #[cfg(sonora_gui)]
    fn apply_remote(&mut self, _: &serde_json::Value, _: &mut Context<Self>) -> anyhow::Result<()> {
        Ok(())
    }

    #[cfg(sonora_gui)]
    fn apply_signal(
        &mut self,
        data: &serde_json::Value,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        cx.emit(serde_json::from_value::<Opened>(data.clone())?);
        Ok(())
    }
}

impl Links {
    pub fn new(session: Entity<Session>, devices: Entity<Devices>, cx: &mut Context<Self>) -> Self {
        #[cfg(sonora_gui)]
        if crate::client::register::<Self>(cx) {
            return Self {
                session,
                devices,
                waiting: None,
                following: None,
            };
        }
        cx.observe(&session, |this, _, cx| this.advance(cx))
            .detach();
        Self {
            session,
            devices,
            waiting: None,
            following: None,
        }
    }

    /// Opens `text` when it is a provider's link and says whether it was one. A short link is
    /// followed first, so what it points at opens a moment later.
    pub fn open(&mut self, text: &str, cx: &mut Context<Self>) -> bool {
        #[cfg(sonora_gui)]
        if crate::client::is_remote(cx) {
            let Some(address) = music::link::address(text) else {
                return false;
            };
            if address.scheme() == "file" {
                return false;
            }
            crate::client::command("open", vec![serde_json::json!(text)], cx);
            return true;
        }

        let Some((slug, linked)) = self.session.read(cx).read_link(text) else {
            return false;
        };
        match linked {
            Linked::Short => self.follow(slug, text.trim().to_owned(), cx),
            linked => self.arrive(slug, linked, cx),
        }
        true
    }

    /// Fetches a short link and opens what its address or its page leads to on `slug`.
    fn follow(&mut self, slug: &'static str, short: String, cx: &mut Context<Self>) {
        let fetch = Io::global(cx).spawn(async move { link::follow(&short).await });
        self.following = Some(cx.spawn(async move |this, cx| {
            let followed = fetch.await.ok().flatten();
            this.update(cx, |this, cx| {
                this.following = None;
                let found = followed.and_then(|followed| {
                    let session = this.session.read(cx);
                    std::iter::once(followed.address.as_str())
                        .chain(link::addresses(&followed.page))
                        .find_map(|address| {
                            let (owner, linked) = session.read_link(address)?;
                            (owner == slug && linked != Linked::Short).then_some(linked)
                        })
                });
                match found {
                    Some(linked) => this.arrive(slug, linked, cx),
                    None => Toasts::show(Outcome::Failed, "toast-link-unreadable", cx),
                }
            })
            .ok();
        }));
    }

    fn arrive(&mut self, slug: &'static str, linked: Linked, cx: &mut Context<Self>) {
        self.waiting = Some(Waiting {
            slug,
            linked,
            switched: false,
        });
        self.advance(cx);
    }

    /// Opens the waiting link once its provider is signed in, asks the session to switch to that
    /// provider when it is not, and drops the link when there is no account to switch to.
    fn advance(&mut self, cx: &mut Context<Self>) {
        let Some(waiting) = &self.waiting else {
            return;
        };
        let session = self.session.read(cx);
        if session.is_pending() {
            return;
        }
        let here = session.provider_slug() == Some(waiting.slug);
        let ready = matches!(
            session.state(),
            SessionState::SignedIn(_) | SessionState::Offline(_)
        );
        if here && ready {
            if let Some(waiting) = self.waiting.take() {
                self.deliver(waiting.linked, cx);
            }
            return;
        }
        if waiting.switched {
            self.waiting = None;
            return;
        }
        let slug = waiting.slug;
        match session.account(slug) {
            Some((_, true)) => {
                if let Some(waiting) = &mut self.waiting {
                    waiting.switched = true;
                }
                self.session
                    .update(cx, |session, cx| session.switch(slug, cx));
            }
            Some((name, false)) => {
                self.waiting = None;
                Toasts::about(Outcome::Info, "toast-link-sign-in", name, cx);
            }
            None => self.waiting = None,
        }
    }

    fn deliver(&mut self, linked: Linked, cx: &mut Context<Self>) {
        match linked {
            Linked::Jam(jam) => self
                .devices
                .update(cx, |devices, cx| devices.peek(&jam, cx)),
            Linked::Short => {}
            linked => cx.emit(Opened(linked)),
        }
    }
}
