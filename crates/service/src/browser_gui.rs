//! Executes server browser requests in the GUI's throwaway native browser sessions.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use futures::FutureExt as _;
use gpui::{App, AppContext as _, Context, Entity, Global, Task};
use protocol::browser::{Page as Request, Reply};
use serde_json::Value;

pub(crate) use webview::{Page, Poll, Target, supported};

const POLL: Duration = Duration::from_millis(100);

/// A native page and its result retained until the server acknowledges delivery.
struct Open {
    page: Option<Page>,
    persistent: bool,
    script: u64,
    last: Option<String>,
    pending: Option<Reply>,
    delivery: Option<crate::IoTask<Result<Value>>>,
}

/// Native pages assigned to this GUI connection and their polling task.
struct Browser {
    client: String,
    open: HashMap<u64, Open>,
    _poll: Task<()>,
}

/// Retains browser ownership for the GUI application lifetime.
struct Attached {
    _browser: Entity<Browser>,
}
impl Global for Attached {}

impl crate::client::Model for Browser {
    const KIND: &'static str = "browser";
    const DYNAMIC: bool = false;

    fn apply_remote(&mut self, data: &Value, _cx: &mut Context<Self>) -> Result<()> {
        let pages: Vec<Request> = serde_json::from_value(data["pages"].clone())?;
        self.open.retain(|id, _| {
            pages
                .iter()
                .any(|page| page.id == *id && page.client == self.client)
        });
        for request in pages.into_iter().filter(|page| page.client == self.client) {
            let open = self.open.entry(request.id).or_insert_with(|| {
                let target = request.target;
                let persistent = target.script.is_some();
                let title = match persistent {
                    true => target.title,
                    false => crate::localization::t!("login-window-title", provider = target.title)
                        .to_string(),
                };
                let page = Page::open(Target {
                    url: target.url,
                    landing: target.landing,
                    domain: target.domain,
                    proof: target.proof,
                    title,
                    agent: target.agent,
                    script: target.script,
                });
                match page {
                    Ok(page) => Open {
                        page: Some(page),
                        persistent,
                        script: 0,
                        last: None,
                        pending: None,
                        delivery: None,
                    },
                    Err(error) => {
                        log::warn!("browser: cannot open the requested page: {error:#}");
                        Open {
                            page: None,
                            persistent,
                            script: 0,
                            last: None,
                            pending: Some(Reply::Closed),
                            delivery: None,
                        }
                    }
                }
            });
            if let Some((sequence, script)) = request.script
                && sequence > open.script
                && let Some(page) = &mut open.page
            {
                page.run(&script);
                open.script = sequence;
            }
        }
        Ok(())
    }
}

impl Browser {
    /// Collects page results and sends them with protocol backpressure.
    fn poll(&mut self, cx: &mut Context<Self>) {
        for (id, open) in &mut self.open {
            if let Some(delivery) = open.delivery.as_mut() {
                match delivery.now_or_never() {
                    None => continue,
                    Some(Ok(Ok(_))) => {
                        open.delivery = None;
                        open.pending = None;
                    }
                    Some(result) => {
                        log::warn!("browser: cannot deliver page result: {result:?}");
                        open.delivery = None;
                        open.pending = None;
                        open.page = None;
                    }
                }
            }
            if let Some(reply) = open.pending.clone() {
                open.delivery = Some(crate::client::browser_reply(*id, reply, cx));
                continue;
            }
            let Some(page) = &mut open.page else {
                continue;
            };
            let result = match open.persistent {
                true => page.peek(),
                false => page.poll(),
            };
            let reply = match result {
                Poll::Pending => continue,
                Poll::Closed => {
                    open.page = None;
                    Reply::Closed
                }
                Poll::Cookies(header) => {
                    if open.last.as_ref() == Some(&header) {
                        continue;
                    }
                    open.last = Some(header.clone());
                    if !open.persistent {
                        open.page = None;
                    }
                    Reply::Cookies(header)
                }
            };
            open.pending = Some(reply.clone());
            open.delivery = Some(crate::client::browser_reply(*id, reply, cx));
        }
    }
}

/// Registers the browser read model and advertises capability for the current connection.
pub(crate) fn attach(cx: &mut App) {
    let client = format!("{}-{:016x}", std::process::id(), fastrand::u64(..));
    let browser = cx.new(|cx| {
        crate::client::register::<Browser>(cx);
        let poll = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(POLL).await;
                if this.update(cx, |this, cx| this.poll(cx)).is_err() {
                    break;
                }
            }
        });
        Browser {
            client: client.clone(),
            open: HashMap::new(),
            _poll: poll,
        }
    });
    if supported() {
        crate::client::send("browser", "attach", vec![serde_json::json!(client)], cx);
    }
    cx.set_global(Attached { _browser: browser });
}
