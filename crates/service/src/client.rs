//! GUI read models and command routing for an independent Sonora server.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};

use anyhow::{Context as _, Result};
use gpui::{App, Context, Global, Task};
use protocol::{Call, Snapshot};
use serde_json::Value;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc};

const COMMAND_CAPACITY: usize = 64;

/// Provider metadata needed by views, without a provider client, credentials or playback factory.
#[derive(Clone, serde::Deserialize)]
pub(crate) struct ProviderData {
    pub slug: String,
    pub name: String,
    pub options: Vec<music::SignIn>,
    pub web_sign_in: bool,
    pub protected: bool,
    pub stored: bool,
    pub guest: bool,
    pub active: bool,
    pub pending: bool,
    pub error: Option<crate::Failure>,
    pub listening_to: String,
}

impl ProviderData {
    pub(crate) fn info(&self) -> crate::ProviderInfo {
        crate::ProviderInfo {
            slug: intern(&self.slug),
            name: intern(&self.name),
            options: self.options.clone(),
            web_sign_in: self.web_sign_in,
            protected: self.protected,
            stored: self.stored,
            guest: self.guest,
            active: self.active,
            pending: self.pending,
            error: self.error.clone(),
            listening_to: intern(&self.listening_to),
        }
    }
}

#[async_trait::async_trait]
impl music::MusicProvider for ProviderData {
    fn name(&self) -> &'static str {
        intern(&self.name)
    }
    fn slug(&self) -> &'static str {
        intern(&self.slug)
    }
    fn sign_in_options(&self) -> Vec<music::SignIn> {
        self.options.clone()
    }
    fn stored(&self) -> bool {
        self.stored
    }
    fn stored_guest(&self) -> bool {
        self.guest
    }
    fn protected(&self) -> bool {
        self.protected
    }
    fn listening_to(&self) -> &'static str {
        intern(&self.listening_to)
    }

    async fn restore(&self) -> Result<Option<music::ProviderSession>> {
        anyhow::bail!("provider sessions are owned by the external service")
    }

    async fn sign_in(
        &self,
        _: music::SignIn,
        _: music::PromptSink,
        _: music::InputSource,
    ) -> Result<music::ProviderSession> {
        anyhow::bail!("provider authentication requires a service command")
    }

    fn sign_out(&self) {}
}

/// Interns the finite provider registry because `MusicProvider` names and slugs have static lifetimes.
pub(crate) fn intern(value: &str) -> &'static str {
    static NAMES: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    let mut names = NAMES.get_or_init(Mutex::default).lock().unwrap();
    names
        .entry(value.to_owned())
        .or_insert_with(|| Box::leak(value.to_owned().into_boxed_str()))
}

type Apply = std::rc::Rc<dyn Fn(&Value, &mut App) -> Result<()>>;

/// Weak callbacks for a model's presentation state and its ordered semantic events.
struct Handlers {
    state: Apply,
    signal: Apply,
}

/// A command capacity permit follows its request; lifecycle releases share its ordering queue.
struct Queued {
    call: Call,
    _permit: Option<OwnedSemaphorePermit>,
    epoch: Option<u64>,
}

/// Service data that a GUI entity may mirror without running its backend orchestration.
pub(crate) trait Model: 'static {
    const KIND: &'static str;
    const DYNAMIC: bool;
    fn apply_signal(&mut self, _data: &Value, _cx: &mut Context<Self>) -> Result<()>
    where
        Self: Sized,
    {
        anyhow::bail!("the read model does not accept semantic events")
    }
    fn remote_snapshot(&self, _cx: &gpui::App) -> Value;
    fn apply_remote(&mut self, data: &Value, cx: &mut Context<Self>) -> Result<()>
    where
        Self: Sized;
}

/// Per-object presentation values shared with callbacks during an ordered update.
struct Cached {
    instance: String,
    revision: u64,
    objects: BTreeMap<String, Arc<Value>>,
}

/// A connection and its cache update handlers, owned for the GUI application's lifetime.
struct Remote {
    snapshot: Cached,
    commands: mpsc::UnboundedSender<Queued>,
    capacity: Arc<Semaphore>,
    settings: Option<crate::AppSettings>,
    connection: protocol::Client,
    models: HashMap<String, Handlers>,
    _updates: Option<Task<()>>,
    _requests: Option<Task<()>>,
    clipboard: Option<Task<()>>,
    online: bool,
    epoch: u64,
    reopen: HashMap<String, Call>,
    presentation: HashMap<String, Call>,
    leases: Arc<Mutex<HashMap<u64, String>>>,
}

impl Global for Remote {}

/// Installs server-backed read models for a GUI. No concrete music provider is created here.
pub fn start(
    io: crate::Io,
    connection: protocol::Client,
    snapshot: Snapshot,
    cx: &mut App,
) -> Result<()> {
    anyhow::ensure!(
        connection.instance() == snapshot.instance,
        "the snapshot belongs to another service instance"
    );
    let mut events = connection.events()?;
    let snapshot = (*connection.snapshots().borrow().clone()).clone();
    let settings = crate::AppSettings::from_remote(
        snapshot
            .objects
            .get("settings")
            .context("the server snapshot has no settings")?,
    )?;
    let session = snapshot
        .objects
        .get("session")
        .context("the server snapshot has no session")?;
    let providers: Vec<ProviderData> = serde_json::from_value(session["providers"].clone())?;
    let local: ProviderData = serde_json::from_value(session["local_provider"].clone())?;
    let (commands, mut requests) = mpsc::unbounded_channel::<Queued>();
    cx.set_global(Remote {
        snapshot: Cached {
            instance: snapshot.instance.clone(),
            revision: snapshot.revision,
            objects: BTreeMap::new(),
        },
        commands,
        capacity: Arc::new(Semaphore::new(COMMAND_CAPACITY)),
        settings: Some(settings),
        connection,
        models: HashMap::new(),
        _updates: None,
        _requests: None,
        clipboard: None,
        online: true,
        epoch: 0,
        reopen: HashMap::new(),
        presentation: HashMap::new(),
        leases: Arc::new(Mutex::new(HashMap::new())),
    });
    crate::init(
        cx,
        io,
        storage::Database::standard(),
        providers
            .into_iter()
            .map(|provider| Arc::new(provider) as Arc<dyn music::MusicProvider>)
            .collect(),
        Arc::new(local),
        Vec::new(),
    );
    crate::Toasts::entity(cx);
    apply(snapshot, cx)?;
    let updates = cx.spawn(async move |cx| {
        loop {
            while let Some(event) = events.recv().await {
                cx.update(|cx| {
                    if let Err(error) = apply_notice(event, cx) {
                        log::error!("client: cannot apply service state: {error:#}");
                        crate::Toasts::show(
                            crate::Outcome::Failed,
                            "toast-service-state-invalid",
                            cx,
                        );
                    }
                });
            }
            let notice = cx.update(|cx| {
                let remote = cx.global_mut::<Remote>();
                remote.online = false;
                remote.epoch += 1;
                crate::Toasts::post(
                    crate::Toast::new(crate::Outcome::Warning, "toast-service-disconnected")
                        .lasting(),
                    cx,
                )
            });
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(1))
                    .await;
                let io = cx.update(|cx| crate::Io::global(cx));
                let connected = io
                    .owned(async {
                        let connection = protocol::Client::connect().await?;
                        let snapshot = connection.subscribe().await?;
                        let events = connection.events()?;
                        let latest = (*connection.snapshots().borrow().clone()).clone();
                        anyhow::ensure!(
                            latest.revision >= snapshot.revision,
                            "the service snapshot moved backwards"
                        );
                        Ok::<_, anyhow::Error>((connection, latest, events))
                    })
                    .await;
                let Ok(Ok((connection, snapshot, next_events))) = connected else {
                    continue;
                };
                let installed = cx.update(|cx| -> Result<()> {
                    apply(snapshot, cx)?;
                    let remote = cx.global_mut::<Remote>();
                    remote.connection = connection;
                    let mut calls = remote
                        .reopen
                        .values()
                        .chain(remote.presentation.values())
                        .cloned()
                        .collect::<Vec<_>>();
                    calls.extend(remote.leases.lock().unwrap().iter().map(|(id, kind)| {
                        Call::new(
                            "signal",
                            "acquire",
                            vec![serde_json::json!(id), serde_json::json!(kind)],
                        )
                    }));
                    for call in calls {
                        remote
                            .commands
                            .send(Queued {
                                call,
                                _permit: None,
                                epoch: Some(remote.epoch),
                            })
                            .ok();
                    }
                    remote.online = true;
                    crate::Toasts::close(notice, cx);
                    Ok(())
                });
                if let Err(error) = installed {
                    log::error!("client: cannot install reconnected service state: {error:#}");
                    continue;
                }
                events = next_events;
                break;
            }
        }
    });
    let requests = cx.spawn(async move |cx| {
        while let Some(queued) = requests.recv().await {
            let selected = cx.update(|cx| {
                let remote = cx.global::<Remote>();
                (remote.online && queued.epoch.is_none_or(|epoch| epoch == remote.epoch))
                    .then(|| (remote.connection.clone(), crate::Io::global(cx)))
            });
            let Some((connection, io)) = selected else {
                continue;
            };
            let seek = queued.call.object == "playback"
                && matches!(queued.call.method.as_str(), "seek" | "seek_fraction");
            let result = io
                .owned(async move { connection.call::<Value>(queued.call).await })
                .await;
            drop(queued._permit);
            if let Err(error) = result
                .map_err(anyhow::Error::from)
                .and_then(|result| result)
            {
                cx.update(|cx| {
                    if seek {
                        let playback = crate::Sonora::global(cx).playback.clone();
                        playback.update(cx, |playback, cx| playback.cancel_remote_seek(cx));
                    }
                    log::warn!("client: service command failed: {error:#}");
                    crate::Toasts::post(
                        crate::Toast::new(crate::Outcome::Failed, "toast-service-action-failed")
                            .detail(format!("{error:#}")),
                        cx,
                    );
                });
            }
        }
    });
    let remote = cx.global_mut::<Remote>();
    remote._updates = Some(updates);
    remote._requests = Some(requests);
    Ok(())
}

/// Resolves a provider's share address in the service and writes the result to the GUI clipboard.
pub fn copy_link(kind: music::MediaKind, id: String, cx: &mut App) {
    let connection = cx.global::<Remote>().connection.clone();
    let io = crate::Io::global(cx);
    let task = cx.spawn(async move |cx| {
        let result = io
            .owned(async move {
                connection
                    .call::<Option<String>>(Call::new(
                        "session",
                        "share_url",
                        vec![serde_json::json!(kind), serde_json::json!(id)],
                    ))
                    .await
            })
            .await;
        cx.update(|cx| match result {
            Ok(Ok(Some(url))) => {
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(url));
                crate::Toasts::show(crate::Outcome::Done, "toast-link-copied", cx);
            }
            _ => crate::Toasts::show(crate::Outcome::Failed, "toast-link-unreadable", cx),
        });
    });
    cx.global_mut::<Remote>().clipboard = Some(task);
}

/// Takes the prevalidated preference cache while the common entity graph is constructed.
pub(crate) fn take_settings(cx: &mut App) -> Option<crate::AppSettings> {
    cx.try_global::<Remote>()?;
    cx.global_mut::<Remote>().settings.take()
}

/// Replaces an entire connection snapshot while preserving entity identities.
fn apply(snapshot: Snapshot, cx: &mut App) -> Result<()> {
    let remote = cx.global_mut::<Remote>();
    remote.snapshot.instance = snapshot.instance;
    remote.snapshot.revision = snapshot.revision;
    remote.snapshot.objects.clear();
    apply_objects(snapshot.objects, cx)
}

/// Updates and compares only published objects; unchanged catalogues stay shared in the cache.
fn apply_objects(objects: BTreeMap<String, Value>, cx: &mut App) -> Result<()> {
    let mut handlers = Vec::new();
    let remote = cx.global_mut::<Remote>();
    for (name, value) in objects {
        if remote
            .snapshot
            .objects
            .get(&name)
            .is_some_and(|previous| previous.as_ref() == &value)
        {
            continue;
        }
        let data = Arc::new(value);
        if let Some(handler) = remote.models.get(&name) {
            handlers.push((handler.state.clone(), data.clone()));
        }
        remote.snapshot.objects.insert(name, data);
    }
    for (handler, data) in handlers {
        handler(&data, cx)?;
    }
    Ok(())
}

/// Applies each publication before delivering its event, without coalescing semantic transitions.
fn apply_notice(event: protocol::Notice, cx: &mut App) -> Result<()> {
    let (instance, revision, objects, removed, signal) = match event {
        protocol::Notice::Changed(event) => (
            event.instance,
            event.revision,
            event.objects,
            event.removed,
            None,
        ),
        protocol::Notice::Signal(signal) => (
            signal.instance,
            signal.revision,
            signal.objects,
            Vec::new(),
            Some((signal.object, signal.value)),
        ),
    };
    let remote = cx.global_mut::<Remote>();
    anyhow::ensure!(
        remote.snapshot.instance == instance,
        "the service instance changed"
    );
    if revision <= remote.snapshot.revision {
        return Ok(());
    }
    remote.snapshot.revision = revision;
    for name in removed {
        remote.snapshot.objects.remove(&name);
    }
    apply_objects(objects, cx)?;
    if let Some((name, data)) = signal {
        let handler = cx
            .global::<Remote>()
            .models
            .get(&name)
            .map(|handlers| handlers.signal.clone());
        if let Some(handler) = handler {
            handler(&data, cx)?;
        }
    }
    Ok(())
}

/// Whether this application mirrors an external server instead of owning music playback.
pub(crate) fn is_remote(cx: &App) -> bool {
    cx.has_global::<Remote>()
}

/// Registers a weak cache handler; releasing a page also releases its server-side model.
pub(crate) fn register<T: Model>(cx: &mut Context<T>) -> bool {
    if !is_remote(cx) {
        return false;
    }
    let name = name::<T>(cx);
    let entity = cx.weak_entity();
    let signal_entity = entity.clone();
    cx.global_mut::<Remote>().models.insert(
        name.clone(),
        Handlers {
            state: std::rc::Rc::new(move |data, cx| {
                entity
                    .update(cx, |model, cx| model.apply_remote(data, cx))
                    .context("the remote read model was released")?
            }),
            signal: std::rc::Rc::new(move |data, cx| {
                signal_entity
                    .update(cx, |model, cx| model.apply_signal(data, cx))
                    .context("the remote read model was released")?
            }),
        },
    );
    cx.on_release(move |_, cx| {
        let remote = cx.global_mut::<Remote>();
        remote.models.remove(&name);
        remote.reopen.remove(&name);
        if T::DYNAMIC {
            remote.snapshot.objects.remove(&name);
            remote
                .commands
                .send(Queued {
                    call: Call::new(name, "release", Vec::new()),
                    _permit: None,
                    epoch: None,
                })
                .ok();
        }
    })
    .detach();
    true
}

/// An ordered service subscription that is released even when its owning GUI task is cancelled.
pub(crate) struct Lease {
    commands: mpsc::UnboundedSender<Queued>,
    release: Call,
    id: u64,
    live: Arc<Mutex<HashMap<u64, String>>>,
}

impl Drop for Lease {
    fn drop(&mut self) {
        self.live.lock().unwrap().remove(&self.id);
        self.commands
            .send(Queued {
                call: self.release.clone(),
                _permit: None,
                epoch: None,
            })
            .ok();
    }
}

/// Acquires a named server subscription and retains the cancellation command with its owner.
pub(crate) fn lease(kind: &str, cx: &mut App) -> Option<Lease> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let remote = cx.global::<Remote>();
    let Ok(permit) = remote.capacity.clone().try_acquire_owned() else {
        crate::Toasts::show(crate::Outcome::Failed, "toast-service-busy", cx);
        return None;
    };
    let commands = remote.commands.clone();
    let live = remote.leases.clone();
    live.lock().unwrap().insert(id, kind.to_owned());
    if remote.online {
        commands
            .send(Queued {
                call: Call::new(
                    "signal",
                    "acquire",
                    vec![serde_json::json!(id), serde_json::json!(kind)],
                ),
                _permit: Some(permit),
                epoch: Some(remote.epoch),
            })
            .ok()?;
    }
    Some(Lease {
        commands,
        live,
        id,
        release: Call::new("signal", "release", vec![serde_json::json!(id)]),
    })
}

/// Forwards a GUI mutation once. A server call returns false so its local implementation runs.
pub(crate) fn command<T: Model>(method: &str, arguments: Vec<Value>, cx: &mut Context<T>) -> bool {
    submit(method, arguments, cx).is_some()
}

/// Returns whether a GUI command was accepted, or `None` in the authoritative service.
pub(crate) fn submit<T: Model>(
    method: &str,
    arguments: Vec<Value>,
    cx: &mut Context<T>,
) -> Option<bool> {
    is_remote(cx).then(|| enqueue(&name::<T>(cx), method, arguments, cx))
}

/// Routes a GUI callback that has an application context rather than a model context.
pub(crate) fn send(object: &str, method: &str, arguments: Vec<Value>, cx: &mut App) -> bool {
    if !is_remote(cx) {
        return false;
    }
    enqueue(object, method, arguments, cx);
    true
}

/// Accepts an operation only while the connection and its bounded request queue are available.
fn enqueue(object: &str, method: &str, arguments: Vec<Value>, cx: &mut App) -> bool {
    let kind = object.split(':').next().unwrap_or(object);
    let opening = match (kind, method) {
        ("artist" | "genre" | "song" | "profile" | "tags", "open")
        | ("detail", "open_album" | "open_playlist")
        | ("search", "ask") => Some(Call::new(object, method, arguments.clone())),
        ("genres", _) => Some(Call::new(object, "load", Vec::new())),
        ("home", _) => Some(Call::new(object, "feed", Vec::new())),
        _ => None,
    };
    if let Some(opening) = opening {
        cx.global_mut::<Remote>()
            .reopen
            .insert(object.to_owned(), opening);
    }
    if object == "wake" || (object == "toasts" && method == "pause") {
        cx.global_mut::<Remote>().presentation.insert(
            format!("{object}.{method}"),
            Call::new(object, method, arguments.clone()),
        );
    }
    let remote = cx.global::<Remote>();
    if !remote.online {
        crate::Toasts::show(crate::Outcome::Warning, "toast-service-disconnected", cx);
        return false;
    }
    let epoch = remote.epoch;
    let mut call = Call::new(object, method, arguments);
    let revision = match object {
        "queue" => Some("queue"),
        "lyrics" => Some("lyrics"),
        "playback"
            if matches!(
                method,
                "play_past" | "play_upcoming" | "play_similar" | "insert_all" | "enqueue_pin"
            ) =>
        {
            Some("queue")
        }
        _ => None,
    };
    call.revision = revision
        .and_then(|name| remote.snapshot.objects.get(name))
        .and_then(|object| object.get("revision"))
        .and_then(Value::as_u64);
    let Ok(permit) = remote.capacity.clone().try_acquire_owned() else {
        crate::Toasts::show(crate::Outcome::Failed, "toast-service-busy", cx);
        return false;
    };
    if let Err(error) = remote.commands.send(Queued {
        call,
        _permit: Some(permit),
        epoch: Some(epoch),
    }) {
        log::warn!("client: cannot queue service command: {error}");
        crate::Toasts::show(crate::Outcome::Failed, "toast-service-busy", cx);
        return false;
    }
    true
}

fn name<T: Model>(cx: &Context<T>) -> String {
    match T::DYNAMIC {
        true => format!("{}:{}", T::KIND, cx.entity_id().as_u64()),
        false => T::KIND.to_owned(),
    }
}

/// Defines the serializable presentation fields of an entity separately from its live handles.
macro_rules! remote_model {
    ($type:ty, $kind:literal, $dynamic:literal, {$($field:ident),+ $(,)?} $(, events $event:ty)?) => {
        impl crate::client::Model for $type {
            const KIND: &'static str = $kind;
            const DYNAMIC: bool = $dynamic;
            $(fn apply_signal(&mut self, data: &serde_json::Value, cx: &mut gpui::Context<Self>) -> anyhow::Result<()> {
                cx.emit(serde_json::from_value::<$event>(data.clone())?);
                Ok(())
            })?

            fn remote_snapshot(&self, _cx: &gpui::App) -> serde_json::Value {
                serde_json::json!({$(stringify!($field): &self.$field),+})
            }

            fn apply_remote(&mut self, data: &serde_json::Value, cx: &mut gpui::Context<Self>) -> anyhow::Result<()> {
                $(let $field = serde_json::from_value(data.get(stringify!($field)).cloned()
                    .ok_or_else(|| anyhow::anyhow!("the read model is missing {}", stringify!($field)))?)?;)+
                $(self.$field = $field;)+
                cx.notify();
                Ok(())
            }
        }
    };
}
