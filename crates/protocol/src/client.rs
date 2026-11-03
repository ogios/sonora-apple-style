use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow, bail, ensure};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::{mpsc, oneshot, watch};

use crate::{Call, Message, Notice, Request, RequestBody, Snapshot, VERSION};

const HANDSHAKE: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const OUTBOUND: usize = 64;

/// Whether an established connection is live, or why its server can no longer be reached.
#[derive(Clone, Debug)]
pub enum ConnectionState {
    Connected,
    Disconnected(String),
}

/// A multiplexed local connection. Commands are acknowledged once and never retried implicitly.
#[derive(Clone)]
pub struct Client(Arc<Inner>);

/// The reply destination and whether its result establishes a subscription's state barrier.
struct Pending {
    snapshot: bool,
    reply: oneshot::Sender<Result<Value, String>>,
}

/// Shared connection handles; dropping the final client stops both I/O pumps.
struct Inner {
    instance: String,
    pid: u32,
    next: AtomicU64,
    outgoing: mpsc::Sender<Request>,
    pending: Arc<Mutex<HashMap<u64, Pending>>>,
    snapshots: watch::Receiver<Arc<Snapshot>>,
    state: watch::Receiver<ConnectionState>,
    events: Mutex<Option<mpsc::Receiver<Notice>>>,
    events_enabled: Arc<AtomicBool>,
    stop: watch::Sender<bool>,
}

/// Removes a pending reply when its caller is cancelled or its deadline expires.
struct Waiting {
    id: u64,
    pending: Arc<Mutex<HashMap<u64, Pending>>>,
}

impl Drop for Waiting {
    fn drop(&mut self) {
        self.pending.lock().unwrap().remove(&self.id);
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.stop.send_replace(true);
    }
}

impl Client {
    /// Connects and verifies the server's protocol before starting the connection pumps.
    pub async fn connect() -> Result<Self> {
        let stream = tokio::time::timeout(HANDSHAKE, crate::connect())
            .await
            .context("cannot connect to the service before the deadline")?
            .context("cannot connect to the service")?;
        let mut wire = &stream;
        crate::write(
            &mut wire,
            &Request {
                id: 0,
                body: RequestBody::Hello { version: VERSION },
            },
        )
        .await?;
        let welcome: Message = tokio::time::timeout(HANDSHAKE, crate::read(&mut wire))
            .await
            .context("the service handshake timed out")??;
        let Message::Welcome {
            version,
            instance,
            pid,
        } = welcome
        else {
            bail!("the service refused the protocol handshake");
        };
        ensure!(
            version == VERSION,
            "the service protocol version is incompatible"
        );
        let (outgoing, mut requests) = mpsc::channel::<Request>(OUTBOUND);
        let pending = Arc::new(Mutex::new(HashMap::<u64, Pending>::new()));
        let (snapshots, snapshots_rx) = watch::channel(Arc::new(Snapshot {
            instance: instance.clone(),
            ..Snapshot::default()
        }));
        let (state, state_rx) = watch::channel(ConnectionState::Connected);
        let (events, events_rx) = mpsc::channel(OUTBOUND);
        let (stop, mut stopped) = watch::channel(false);
        let active = pending.clone();
        let events_enabled = Arc::new(AtomicBool::new(false));
        let deliver_events = events_enabled.clone();
        tokio::spawn(async move {
            let reader = receive(&stream, &active, &snapshots, &events, &deliver_events);
            let writer = async {
                let mut writer = &stream;
                while let Some(request) = requests.recv().await {
                    crate::write(&mut writer, &request).await?;
                }
                Ok::<(), anyhow::Error>(())
            };
            let reason = tokio::select! {
                result = reader => result.err().map_or_else(|| "the service disconnected".to_owned(), |error| format!("{error:#}")),
                result = writer => result.err().map_or_else(|| "the service connection closed".to_owned(), |error| format!("{error:#}")),
                _ = stopped.changed() => "the service client closed".to_owned(),
            };
            state.send_replace(ConnectionState::Disconnected(reason.clone()));
            for (_, waiting) in active.lock().unwrap().drain() {
                waiting.reply.send(Err(reason.clone())).ok();
            }
        });
        Ok(Self(Arc::new(Inner {
            instance,
            pid,
            next: AtomicU64::new(1),
            outgoing,
            pending,
            snapshots: snapshots_rx,
            state: state_rx,
            events: Mutex::new(Some(events_rx)),
            events_enabled,
            stop,
        })))
    }

    pub fn instance(&self) -> &str {
        &self.0.instance
    }

    pub fn pid(&self) -> u32 {
        self.0.pid
    }

    /// The latest coherent state; updates are retained even if a GUI frame has not read them yet.
    pub fn snapshots(&self) -> watch::Receiver<Arc<Snapshot>> {
        self.0.snapshots.clone()
    }

    pub fn connection_state(&self) -> watch::Receiver<ConnectionState> {
        self.0.state.clone()
    }

    /// Takes the ordered notification receiver. A connection has one presentation consumer.
    pub fn events(&self) -> Result<mpsc::Receiver<Notice>> {
        let receiver = self
            .0
            .events
            .lock()
            .unwrap()
            .take()
            .context("the service events already have a consumer")?;
        self.0.events_enabled.store(true, Ordering::Release);
        Ok(receiver)
    }

    /// Installs the initial state before processing later events on the same connection.
    pub async fn subscribe(&self) -> Result<Snapshot> {
        self.request(RequestBody::Subscribe).await
    }

    pub async fn snapshot(&self) -> Result<Snapshot> {
        self.request(RequestBody::Snapshot).await
    }

    /// Carries out one operation, returning a server error or a connection failure to the caller.
    pub async fn call<T: DeserializeOwned>(&self, call: Call) -> Result<T> {
        self.request(RequestBody::Call(call)).await
    }

    async fn request<T: DeserializeOwned>(&self, body: RequestBody) -> Result<T> {
        if let ConnectionState::Disconnected(reason) = &*self.0.state.borrow() {
            bail!("{reason}");
        }
        let snapshot = matches!(body, RequestBody::Snapshot | RequestBody::Subscribe);
        let id = self.0.next.fetch_add(1, Ordering::Relaxed);
        let (reply, response) = oneshot::channel();
        self.0
            .pending
            .lock()
            .unwrap()
            .insert(id, Pending { snapshot, reply });
        let _waiting = Waiting {
            id,
            pending: self.0.pending.clone(),
        };
        let done = tokio::time::timeout(REQUEST_TIMEOUT, async {
            self.0
                .outgoing
                .send(Request { id, body })
                .await
                .context("cannot send a service request")?;
            response
                .await
                .context("the service connection closed before replying")?
                .map_err(|error| anyhow!(error))
        })
        .await;
        let value = done.context("the service request timed out")??;
        serde_json::from_value(value).context("cannot decode the service reply")
    }
}

/// Applies replies and ordered state events until the server closes the connection.
async fn receive(
    stream: &interprocess::local_socket::tokio::Stream,
    active: &Mutex<HashMap<u64, Pending>>,
    snapshots: &watch::Sender<Arc<Snapshot>>,
    events: &mpsc::Sender<Notice>,
    events_enabled: &AtomicBool,
) -> Result<()> {
    let mut reader = stream;
    loop {
        match crate::read::<Message>(&mut reader).await? {
            Message::Reply { id, result } => {
                let waiting = active.lock().unwrap().remove(&id);
                if let Some(waiting) = waiting {
                    if waiting.snapshot
                        && let Ok(value) = &result
                    {
                        let snapshot = serde_json::from_value(value.clone())
                            .context("cannot decode the service snapshot")?;
                        snapshots.send_replace(Arc::new(snapshot));
                    }
                    waiting.reply.send(result).ok();
                }
            }
            Message::Changed(event) => {
                {
                    let snapshot = snapshots.borrow();
                    ensure!(
                        snapshot.instance == event.instance,
                        "the service instance changed during a connection"
                    );
                    ensure!(
                        event.revision > snapshot.revision,
                        "the service sent an unordered event"
                    );
                }
                snapshots.send_modify(|snapshot| {
                    let next = Arc::make_mut(snapshot);
                    next.revision = event.revision;
                    next.objects.extend(event.objects.clone());
                    for name in &event.removed {
                        next.objects.remove(name);
                    }
                });
                if events_enabled.load(Ordering::Acquire) {
                    events
                        .send(Notice::Changed(event))
                        .await
                        .map_err(|_| anyhow!("the GUI event receiver closed"))?;
                }
            }
            Message::Signal(signal) => {
                {
                    let snapshot = snapshots.borrow();
                    ensure!(
                        snapshot.instance == signal.instance,
                        "the service instance changed during a connection"
                    );
                    ensure!(
                        signal.revision > snapshot.revision,
                        "the service sent an unordered signal"
                    );
                }
                snapshots.send_modify(|snapshot| {
                    let next = Arc::make_mut(snapshot);
                    next.revision = signal.revision;
                    next.objects.extend(signal.objects.clone());
                });
                if events_enabled.load(Ordering::Acquire) {
                    events
                        .send(Notice::Signal(signal))
                        .await
                        .map_err(|_| anyhow!("the GUI event receiver closed"))?;
                }
            }
            Message::Welcome { .. } => bail!("the service repeated its handshake"),
        }
    }
}
