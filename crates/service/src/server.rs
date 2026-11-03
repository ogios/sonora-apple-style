//! The local server's command boundary and ordered state subscriptions.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result, bail, ensure};
use gpui::{App, AppContext as _, Entity, Global, Subscription, Task};
use interprocess::local_socket::tokio::{Listener, Stream};
use interprocess::local_socket::traits::tokio::Listener as _;
use music::{Quality, Track, Transcode};
use protocol::{Call, Event, Message, Request, RequestBody, Snapshot, VERSION};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinSet;

use crate::client::Model as _;
use crate::{Io, Origin, Repeat, Sleep, Sonora};

const HANDSHAKE: Duration = Duration::from_secs(5);
const CHANNEL_SIZE: usize = 64;
static NEXT_PEER: AtomicU64 = AtomicU64::new(1);

/// The server's lifetime, subscriptions and revision sequence on the GPUI thread.
struct Server {
    instance: String,
    revision: u64,
    peers: HashMap<u64, Peer>,
    dirty: HashSet<&'static str>,
    scheduled: bool,
    last: BTreeMap<String, Value>,
    _incoming: Task<()>,
    network: crate::IoTask<()>,
    shutdown: Option<Task<()>>,
    pages: HashMap<(u64, String), Page>,
    genres: HashMap<u64, Entity<crate::Genres>>,
    stopping: bool,
    leases: HashMap<(u64, u64), String>,
    signal_watch: Option<Task<()>>,
    signal_follow: Option<crate::Following>,
    telemetry: Option<Task<()>>,
    paused_toasts: HashSet<u64>,
    presentations: HashMap<u64, (bool, bool)>,
}

impl Global for Server {}

impl Drop for Server {
    fn drop(&mut self) {
        self.network.abort();
        for peer in self.peers.values() {
            peer.stop.send_replace(true);
        }
    }
}

/// A bounded reply queue and a cancellation signal for one connected client.
#[derive(Clone)]
struct Peer {
    outgoing: mpsc::Sender<Message>,
    stop: watch::Sender<bool>,
}

impl Peer {
    /// A slow client reconnects and obtains a fresh snapshot instead of missing events.
    fn send(&self, message: Message) {
        if self.outgoing.try_send(message).is_err() {
            self.stop.send_replace(true);
        }
    }
}

/// Work received on Tokio and applied to authoritative state on the service thread.
enum Incoming {
    Request {
        peer_id: u64,
        peer: Peer,
        request: Request,
    },
    Disconnected(u64),
}

/// A page retained by one GUI and released when that GUI drops it or disconnects.
struct Page {
    entity: PageEntity,
    _subscription: Subscription,
    _events: Option<Subscription>,
}

/// On-demand read models whose backend tasks run entirely in the server process.
#[derive(Clone)]
enum PageEntity {
    Artist(Entity<crate::ArtistDetail>),
    Detail(Entity<crate::Detail>),
    Genres(Entity<crate::Genres>),
    Genre(Entity<crate::GenreDetails>),
    Home(Entity<crate::Home>),
    Profile(Entity<crate::Profile>),
    Search(Entity<crate::Search>),
    Song(Entity<crate::SongDetail>),
    Tags(Entity<crate::Tags>),
}

/// Validates a command's argument count and deserializes its values in declaration order.
struct Arguments(std::vec::IntoIter<Value>);

impl Arguments {
    fn new(call: &Call, expected: usize) -> Result<Self> {
        ensure!(
            call.arguments.len() == expected,
            "the command has the wrong number of arguments"
        );
        Ok(Self(call.arguments.clone().into_iter()))
    }

    fn next<T: DeserializeOwned>(&mut self) -> Result<T> {
        let value = self.0.next().context("a command argument is missing")?;
        serde_json::from_value(value).context("cannot decode the command argument")
    }
}

/// Dispatches an explicit allowlist of typed entity commands.
macro_rules! commands {
    (@invoke $call:expr, $entity:expr, $cx:expr, $method:ident()) => {{
        Arguments::new($call, 0)?;
        let result = $entity.update($cx, |entity, cx| entity.$method(cx));
        serde_json::to_value(result).context("cannot encode the command result")
    }};
    (@invoke $call:expr, $entity:expr, $cx:expr, $method:ident($($arg:ident: $ty:ty => $value:expr),+)) => {{
        let mut arguments = Arguments::new($call, [$(stringify!($arg)),+].len())?;
        $(let $arg: $ty = arguments.next()?;)+
        let result = $entity.update($cx, |entity, cx| entity.$method($($value,)+ cx));
        serde_json::to_value(result).context("cannot encode the command result")
    }};
    ($call:expr, $entity:expr, $cx:expr, {$($method:ident($($arg:ident: $ty:ty => $value:expr),*)),* $(,)?}) => {
        match $call.method.as_str() {
            $(stringify!($method) => commands!(@invoke $call, $entity, $cx, $method($($arg: $ty => $value),*)),)*
            _ => bail!("the service does not support this command"),
        }
    };
}

mod actions;
mod settings;

/// Starts accepting clients after service state exists. The listener must be bound first.
pub fn attach(listener: Listener, cx: &mut App) {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let instance = format!("{}-{stamp:x}-{:x}", std::process::id(), fastrand::u64(..));
    let (sender, mut receiver) = mpsc::channel(CHANNEL_SIZE);
    let identity = instance.clone();
    let network = Io::global(cx).owned(async move {
        let mut connections = JoinSet::new();
        loop {
            tokio::select! {
                accepted = listener.accept() => match accepted {
                    Ok(stream) => {
                        let sender = sender.clone();
                        let instance = identity.clone();
                        connections.spawn(async move {
                            let peer_id = NEXT_PEER.fetch_add(1, Ordering::Relaxed);
                            if let Err(error) = serve(stream, peer_id, sender.clone(), instance).await {
                                log::debug!("server: client {peer_id} disconnected: {error:#}");
                            }
                            sender.send(Incoming::Disconnected(peer_id)).await.ok();
                        });
                    }
                    Err(error) => {
                        log::error!("server: cannot accept a client: {error}");
                        break;
                    }
                },
                _ = connections.join_next(), if !connections.is_empty() => {}
            }
        }
    });
    let incoming = cx.spawn(async move |cx| {
        while let Some(incoming) = receiver.recv().await {
            cx.update(|cx| receive(incoming, cx));
        }
    });
    cx.set_global(Server {
        instance,
        revision: 0,
        peers: HashMap::new(),
        dirty: HashSet::new(),
        scheduled: false,
        last: BTreeMap::new(),
        _incoming: incoming,
        network,
        shutdown: None,
        pages: HashMap::new(),
        genres: HashMap::new(),
        stopping: false,
        presentations: HashMap::new(),
        paused_toasts: HashSet::new(),
        leases: HashMap::new(),
        signal_watch: None,
        signal_follow: None,
        telemetry: None,
    });
    let sonora = Sonora::global(cx);
    let playback = sonora.playback.clone();
    let queue = sonora.queue.clone();
    let lyrics = sonora.lyrics.clone();
    let session = sonora.session.clone();
    let settings = sonora.settings.clone();
    let devices = sonora.devices.clone();
    let library = sonora.library.clone();
    let outputs = sonora.outputs.clone();
    let history = sonora.history.clone();
    let network = sonora.network.clone();
    let drm = sonora.drm.clone();
    let usage = sonora.usage.clone();
    let cover = sonora.cover.clone();
    let requirements = sonora.requirements.clone();
    cx.observe(&playback, |_, cx| dirty("playback", cx))
        .detach();
    cx.observe(&queue, |_, cx| {
        dirty("queue", cx);
        dirty("playback", cx);
    })
    .detach();
    cx.observe(&lyrics, |_, cx| dirty("lyrics", cx)).detach();
    cx.observe(&session, |_, cx| dirty("session", cx)).detach();
    cx.observe(&settings, |_, cx| dirty("settings", cx))
        .detach();
    cx.observe(&devices, |_, cx| dirty("playback", cx)).detach();
    cx.observe(&library, |_, cx| dirty("library", cx)).detach();
    cx.observe(&outputs, |_, cx| dirty("outputs", cx)).detach();
    cx.observe(&history, |_, cx| dirty("history", cx)).detach();
    cx.observe(&network, |_, cx| dirty("network", cx)).detach();
    cx.observe(&drm, |_, cx| dirty("drm", cx)).detach();
    cx.observe(&usage, |_, cx| dirty("usage", cx)).detach();
    cx.observe(&cover, |_, cx| dirty("cover", cx)).detach();
    cx.observe(&requirements, |_, cx| dirty("requirements", cx))
        .detach();
    let entity = Sonora::global(cx).downloads.clone();
    cx.observe(&entity, |_, cx| dirty("downloads", cx)).detach();

    let entity = Sonora::global(cx).pins.clone();
    cx.observe(&entity, |_, cx| dirty("pins", cx)).detach();

    let entity = Sonora::global(cx).devices.clone();
    cx.observe(&entity, |_, cx| dirty("devices", cx)).detach();

    let entity = Sonora::global(cx).scan.clone();
    cx.observe(&entity, |_, cx| dirty("scan", cx)).detach();

    let entity = Sonora::global(cx).wake.clone();
    cx.observe(&entity, |_, cx| dirty("wake", cx)).detach();

    let entity = Sonora::global(cx).links.clone();
    cx.observe(&entity, |_, cx| dirty("links", cx)).detach();

    macro_rules! events {
        ($root:ident, $event:ty, [$($name:literal),+]) => {{
            let entity = Sonora::global(cx).$root.clone();
            cx.subscribe(&entity, |_, event: &$event, cx| {
                let objects = [$($name),+]
                .into_iter()
                .map(|name| (name.to_owned(), object(name, cx)))
                .collect();
                publish_signal(
                    stringify!($root),
                    serde_json::to_value(event).expect("model event serializes"),
                    objects,
                    None,
                    cx,
                );
            })
            .detach();
        }};
    }
    events!(
        session,
        crate::SessionEvent,
        [
            "session",
            "queue",
            "playback",
            "lyrics",
            "library",
            "network",
            "downloads",
            "devices"
        ]
    );
    events!(
        playback,
        crate::playback::PlaybackEvent,
        ["playback", "queue", "lyrics"]
    );
    events!(
        library,
        crate::LibraryEvent,
        ["library", "queue", "playback"]
    );
    events!(network, crate::Reconnected, ["network", "session"]);
    events!(downloads, crate::DownloadsChanged, ["downloads"]);
    events!(links, crate::Opened, ["links", "session", "devices"]);
    let level = Sonora::global(cx).equalizer.read(cx).output_level();
    cx.observe(&level, |_, cx| dirty("equalizer", cx)).detach();

    let entity = Sonora::global(cx).equalizer.clone();
    cx.observe(&entity, |_, cx| dirty("equalizer", cx)).detach();

    let entity = Sonora::global(cx).scrobbling.clone();
    cx.observe(&entity, |_, cx| dirty("scrobbling", cx))
        .detach();
    let entity = Sonora::global(cx).signal.clone();
    cx.observe(&entity, |_, cx| dirty("signal", cx)).detach();

    let toasts = crate::Toasts::entity(cx);
    cx.observe(&toasts, |_, cx| dirty("toasts", cx)).detach();
}

/// Stops owned audio and persists pending settings before leaving the server event loop.
pub fn shutdown_on_signal(cx: &mut App) {
    let signal = Io::global(cx).spawn(async {
        #[cfg(unix)]
        {
            let mut terminate =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
            tokio::select! {
                result = tokio::signal::ctrl_c() => result?,
                _ = terminate.recv() => {}
            }
        }
        #[cfg(not(unix))]
        tokio::signal::ctrl_c().await?;
        Ok::<(), std::io::Error>(())
    });
    let shutdown = cx.spawn(async move |cx| {
        if let Err(error) = signal.await {
            log::error!("server: cannot wait for shutdown: {error}");
        }
        let finish = cx.update(|cx| {
            let server = cx.global_mut::<Server>();
            server.stopping = true;
            server.network.abort();
            for peer in server.peers.values() {
                peer.stop.send_replace(true);
            }
            server.pages.clear();
            server.genres.clear();
            server.leases.clear();
            server.signal_watch = None;
            server.signal_follow = None;
            server.telemetry = None;
            let session = Sonora::global(cx).session.clone();
            session.update(cx, |session, _| session.shutdown());
            let devices = Sonora::global(cx).devices.clone();
            devices.update(cx, |devices, _| devices.shutdown());
            let playback = Sonora::global(cx).playback.clone();
            playback.update(cx, |playback, cx| playback.shutdown(cx));
            let settings = Sonora::global(cx).settings.clone();
            settings.update(cx, |settings, cx| settings.flush(cx))
        });
        if let Err(error) = finish.await {
            log::error!("server: cannot persist shutdown state: {error:#}");
        }
        cx.update(|cx| cx.quit());
    });
    cx.global_mut::<Server>().shutdown = Some(shutdown);
}

/// Validates the initial handshake, then pumps requests and replies independently.
async fn serve(
    stream: Stream,
    peer_id: u64,
    incoming: mpsc::Sender<Incoming>,
    instance: String,
) -> Result<()> {
    let mut reader = &stream;
    let request: Request = tokio::time::timeout(HANDSHAKE, protocol::read(&mut reader))
        .await
        .context("the client handshake timed out")??;
    ensure!(
        matches!(request.body, RequestBody::Hello { version: VERSION }),
        "the client protocol is incompatible"
    );
    protocol::write(
        &mut reader,
        &Message::Welcome {
            version: VERSION,
            instance,
            pid: std::process::id(),
        },
    )
    .await?;
    let (outgoing, mut messages) = mpsc::channel(CHANNEL_SIZE);
    let (stop, mut stopped) = watch::channel(false);
    let peer = Peer { outgoing, stop };
    let reading = async {
        loop {
            let request = protocol::read::<Request>(&mut reader).await?;
            incoming
                .send(Incoming::Request {
                    peer_id,
                    peer: peer.clone(),
                    request,
                })
                .await
                .context("the service stopped accepting requests")?;
        }
    };
    let writing = async {
        let mut writer = &stream;
        while let Some(message) = messages.recv().await {
            protocol::write(&mut writer, &message).await?;
        }
        Ok(())
    };
    tokio::select! {
        result = reading => result,
        result = writing => result,
        _ = stopped.changed() => Ok(()),
    }
}

fn receive(incoming: Incoming, cx: &mut App) {
    if cx.global::<Server>().stopping {
        if let Incoming::Request { peer, .. } = incoming {
            peer.stop.send_replace(true);
        }
        return;
    }
    let Incoming::Request {
        peer_id,
        peer,
        request,
    } = incoming
    else {
        if let Incoming::Disconnected(peer) = incoming {
            let server = cx.global_mut::<Server>();
            server.peers.remove(&peer);
            server.pages.retain(|(owner, _), _| *owner != peer);
            server.genres.remove(&peer);
            server.leases.retain(|(owner, _), _| *owner != peer);
            server.paused_toasts.remove(&peer);
            server.presentations.remove(&peer);
            let paused = !server.paused_toasts.is_empty();
            crate::Toasts::entity(cx).update(cx, |toasts, cx| toasts.pause(paused, cx));
            refresh_telemetry(cx);
            refresh_wake(cx);
        }
        return;
    };
    let result = match request.body {
        RequestBody::Hello { .. } => Err(anyhow::anyhow!("the connection is already initialized")),
        RequestBody::Snapshot => serde_json::to_value(snapshot(peer_id, cx)).map_err(Into::into),
        RequestBody::Subscribe => {
            let snapshot = snapshot(peer_id, cx);
            cx.global_mut::<Server>()
                .peers
                .insert(peer_id, peer.clone());
            serde_json::to_value(snapshot).map_err(Into::into)
        }
        RequestBody::Call(call) => dispatch(&call, peer_id, &peer, cx),
    };
    peer.send(Message::Reply {
        id: request.id,
        result: result.map_err(|error| format!("{error:#}")),
    });
}

fn snapshot(peer_id: u64, cx: &App) -> Snapshot {
    let server = cx.global::<Server>();
    Snapshot {
        instance: server.instance.clone(),
        revision: server.revision,
        objects: [
            "playback",
            "queue",
            "lyrics",
            "session",
            "settings",
            "library",
            "outputs",
            "history",
            "network",
            "drm",
            "usage",
            "cover",
            "requirements",
            "toasts",
            "signal",
            "scrobbling",
            "equalizer",
            "links",
            "wake",
            "scan",
            "devices",
            "pins",
            "downloads",
        ]
        .into_iter()
        .map(|name| (name.to_owned(), object(name, cx)))
        .chain(
            server
                .pages
                .iter()
                .filter(|((owner, _), _)| *owner == peer_id)
                .map(|((_, name), page)| (name.clone(), page.entity.snapshot(cx))),
        )
        .collect(),
    }
}

fn object(name: &str, cx: &App) -> Value {
    let sonora = Sonora::global(cx);
    match name {
        "playback" => sonora.playback.read(cx).snapshot(cx),
        "queue" => sonora.queue.read(cx).snapshot(cx),
        "lyrics" => sonora.lyrics.read(cx).snapshot(),
        "session" => sonora.session.read(cx).snapshot(),
        "settings" => sonora.settings.read(cx).snapshot(),
        "library" => sonora.library.read(cx).remote_snapshot(cx),
        "outputs" => sonora.outputs.read(cx).remote_snapshot(cx),
        "history" => sonora.history.read(cx).remote_snapshot(cx),
        "network" => sonora.network.read(cx).remote_snapshot(cx),
        "drm" => sonora.drm.read(cx).remote_snapshot(cx),
        "usage" => sonora.usage.read(cx).remote_snapshot(cx),
        "cover" => sonora.cover.read(cx).remote_snapshot(cx),
        "requirements" => sonora.requirements.read(cx).remote_snapshot(cx),
        "downloads" => sonora.downloads.read(cx).remote_snapshot(cx),
        "pins" => sonora.pins.read(cx).remote_snapshot(cx),
        "devices" => sonora.devices.read(cx).remote_snapshot(cx),
        "scan" => sonora.scan.read(cx).remote_snapshot(cx),
        "wake" => sonora.wake.read(cx).remote_snapshot(cx),
        "links" => sonora.links.read(cx).remote_snapshot(cx),
        "equalizer" => sonora.equalizer.read(cx).remote_snapshot(cx),
        "scrobbling" => sonora.scrobbling.read(cx).remote_snapshot(cx),
        "signal" => sonora.signal.read(cx).remote_snapshot(cx),
        "toasts" => crate::Toasts::installed(cx).read(cx).remote_snapshot(cx),
        _ => Value::Null,
    }
}

/// Publishes an event with the authoritative state it describes before later deferred changes.
fn publish_signal(
    name: &str,
    value: Value,
    objects: BTreeMap<String, Value>,
    peer: Option<&Peer>,
    cx: &mut App,
) {
    let server = cx.global_mut::<Server>();
    if peer.is_none() {
        server.last.extend(objects.clone());
    }
    server.revision += 1;
    let message = Message::Signal(protocol::Signal {
        instance: server.instance.clone(),
        revision: server.revision,
        object: name.to_owned(),
        value,
        objects,
    });
    match peer {
        Some(peer) => peer.send(message),
        None => {
            for peer in server.peers.values() {
                peer.send(message.clone());
            }
        }
    }
}

fn dirty(name: &'static str, cx: &mut App) {
    let server = cx.global_mut::<Server>();
    server.dirty.insert(name);
    if server.scheduled {
        return;
    }
    server.scheduled = true;
    cx.defer(flush);
}

fn flush(cx: &mut App) {
    let names = {
        let server = cx.global_mut::<Server>();
        server.scheduled = false;
        std::mem::take(&mut server.dirty)
    };
    let objects = names
        .into_iter()
        .map(|name| (name.to_owned(), object(name, cx)))
        .collect::<BTreeMap<_, _>>();
    let server = cx.global_mut::<Server>();
    let objects = objects
        .into_iter()
        .filter(|(name, value)| server.last.get(name) != Some(value))
        .collect::<BTreeMap<_, _>>();
    if objects.is_empty() {
        return;
    }
    server.last.extend(objects.clone());
    server.revision += 1;
    let event = Message::Changed(Event {
        instance: server.instance.clone(),
        revision: server.revision,
        objects,
        removed: Vec::new(),
    });
    for peer in server.peers.values() {
        peer.send(event.clone());
    }
}

/// Applies display inhibition only while the same connected GUI is both focused and fullscreen.
fn refresh_wake(cx: &mut App) {
    let shown = cx
        .global::<Server>()
        .presentations
        .values()
        .any(|(fullscreen, focused)| *fullscreen && *focused);
    let wake = Sonora::global(cx).wake.clone();
    wake.update(cx, |wake, cx| {
        wake.set_fullscreen(shown, cx);
        wake.set_focused(shown, cx);
    });
}

/// Shares one analyzer watch and one sampler across every client's retained subscriptions.
fn refresh_telemetry(cx: &mut App) {
    let server = cx.global::<Server>();
    let watching = server.leases.values().any(|kind| kind == "watch");
    let following = server.leases.values().any(|kind| kind == "follow");
    let sampling = server.leases.values().any(|kind| kind != "follow");
    let signal = Sonora::global(cx).signal.clone();
    if watching && cx.global::<Server>().signal_watch.is_none() {
        let task = signal.update(cx, |signal, cx| signal.watch(cx));
        cx.global_mut::<Server>().signal_watch = Some(task);
    } else if !watching {
        cx.global_mut::<Server>().signal_watch = None;
    }
    if following && cx.global::<Server>().signal_follow.is_none() {
        let held = signal.update(cx, |signal, cx| signal.follow(cx));
        cx.global_mut::<Server>().signal_follow = Some(held);
    } else if !following {
        cx.global_mut::<Server>().signal_follow = None;
    }
    if sampling && cx.global::<Server>().telemetry.is_none() {
        let task = cx.spawn(async move |cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(33))
                    .await;
                cx.update(|cx| dirty("signal", cx));
            }
        });
        cx.global_mut::<Server>().telemetry = Some(task);
    } else if !sampling {
        cx.global_mut::<Server>().telemetry = None;
    }
}

fn dispatch(call: &Call, peer_id: u64, peer: &Peer, cx: &mut App) -> Result<Value> {
    if let Some((kind, _)) = call.object.split_once(':') {
        let key = (peer_id, call.object.clone());
        if call.method == "release" {
            cx.global_mut::<Server>().pages.remove(&key);
            let has_genres = cx
                .global::<Server>()
                .pages
                .iter()
                .any(|((owner, _), page)| {
                    *owner == peer_id
                        && matches!(page.entity, PageEntity::Genre(_) | PageEntity::Genres(_))
                });
            if !has_genres {
                cx.global_mut::<Server>().genres.remove(&peer_id);
            }
            let server = cx.global_mut::<Server>();
            server.revision += 1;
            peer.send(Message::Changed(Event {
                instance: server.instance.clone(),
                revision: server.revision,
                objects: BTreeMap::new(),
                removed: vec![call.object.clone()],
            }));
            return Ok(Value::Null);
        }
        let entity = match cx.global::<Server>().pages.get(&key) {
            Some(page) => page.entity.clone(),
            None => {
                ensure!(
                    cx.global::<Server>()
                        .pages
                        .keys()
                        .filter(|(owner, _)| *owner == peer_id)
                        .count()
                        < 512,
                    "the client has too many open read models"
                );
                let page = create_page(kind, &call.object, peer_id, peer, cx)?;
                let entity = page.entity.clone();
                cx.global_mut::<Server>().pages.insert(key, page);
                entity
            }
        };
        let result = entity.dispatch(call, cx)?;
        publish_page(peer, &call.object, entity.snapshot(cx), cx);
        return Ok(result);
    }
    let sonora = Sonora::global(cx);
    match call.object.as_str() {
        "wake" => {
            let on: bool = Arguments::new(call, 1)?.next()?;
            let flags = cx
                .global_mut::<Server>()
                .presentations
                .entry(peer_id)
                .or_default();
            match call.method.as_str() {
                "set_fullscreen" => flags.0 = on,
                "set_focused" => flags.1 = on,
                _ => bail!("unknown presentation command"),
            }
            refresh_wake(cx);
            Ok(Value::Null)
        }
        "toasts" => {
            match call.method.as_str() {
                "activate" => {
                    let mut args = Arguments::new(call, 2)?;
                    crate::Toasts::activate(args.next()?, args.next()?, cx);
                }
                "discard" => crate::Toasts::discard(Arguments::new(call, 1)?.next()?, cx),
                "dismiss" => crate::Toasts::close(Arguments::new(call, 1)?.next()?, cx),
                "pause" => {
                    let paused: bool = Arguments::new(call, 1)?.next()?;
                    let server = cx.global_mut::<Server>();
                    match paused {
                        true => {
                            server.paused_toasts.insert(peer_id);
                        }
                        false => {
                            server.paused_toasts.remove(&peer_id);
                        }
                    }
                    let paused = !server.paused_toasts.is_empty();
                    crate::Toasts::entity(cx).update(cx, |toasts, cx| toasts.pause(paused, cx));
                }
                _ => bail!("unknown toast command"),
            }
            Ok(Value::Null)
        }
        "signal" => {
            match call.method.as_str() {
                "acquire" => {
                    let mut args = Arguments::new(call, 2)?;
                    let id: u64 = args.next()?;
                    let kind: String = args.next()?;
                    ensure!(
                        matches!(kind.as_str(), "watch" | "follow" | "visualizer"),
                        "unknown telemetry subscription"
                    );
                    ensure!(
                        cx.global::<Server>()
                            .leases
                            .keys()
                            .filter(|(owner, _)| *owner == peer_id)
                            .count()
                            < 256,
                        "too many telemetry subscriptions"
                    );
                    cx.global_mut::<Server>().leases.insert((peer_id, id), kind);
                }
                "release" => {
                    let id: u64 = Arguments::new(call, 1)?.next()?;
                    cx.global_mut::<Server>().leases.remove(&(peer_id, id));
                }
                _ => bail!("unknown telemetry command"),
            }
            refresh_telemetry(cx);
            Ok(Value::Null)
        }
        "settings" => {
            let entity = sonora.settings.clone();
            settings::dispatch(call, &entity, cx)
        }
        "playback" => {
            let entity = sonora.playback.clone();
            if matches!(
                call.method.as_str(),
                "play_past" | "play_upcoming" | "play_similar" | "insert_all" | "enqueue_pin"
            ) {
                ensure!(
                    call.revision == Some(sonora.queue.read(cx).revision()),
                    "the queue changed before the command arrived"
                );
            }
            commands!(call, entity, cx, {
                play(track: Track => &track),
                start_shared(tracks: Vec<music::SharedTrack> => tracks, index: usize => index, origin: Option<Origin> => origin),
                start_any_shared(tracks: Vec<music::SharedTrack> => tracks, origin: Option<Origin> => origin),
                shuffle_any_shared(tracks: Vec<music::SharedTrack> => tracks, origin: Option<Origin> => origin),
                start(tracks: Vec<Track> => tracks, index: usize => index, origin: Option<Origin> => origin),
                start_any(tracks: Vec<Track> => tracks, origin: Option<Origin> => origin),
                shuffle_any(tracks: Vec<Track> => tracks, origin: Option<Origin> => origin),
                enqueue(track: Track => track),
                play_next(track: Track => track),
                play_last(track: Track => track),
                enqueue_all(tracks: Vec<Track> => tracks, origin: Option<Origin> => origin),
                play_next_all(tracks: Vec<Track> => tracks, origin: Option<Origin> => origin),
                play_last_all(tracks: Vec<Track> => tracks, origin: Option<Origin> => origin),
                open_paths(paths: Vec<PathBuf> => paths),
                insert_all(tracks: Vec<Track> => tracks, gap: usize => gap, origin: Option<Origin> => origin),
                enqueue_pin(pin: ui::Pin => &pin, gap: Option<usize> => gap),
                play_radio(seed: Track => &seed),
                play_origin(origin: Origin => origin),
                toggle_origin(origin: Origin => &origin),
                enqueue_album(origin: Origin => origin),
                play_album_next(origin: Origin => origin),
                play_album_last(origin: Origin => origin),
                enqueue_artist(origin: Origin => origin),
                play_artist_next(origin: Origin => origin),
                play_artist_last(origin: Origin => origin),
                enqueue_playlist(origin: Origin => origin),
                play_playlist_next(origin: Origin => origin),
                play_playlist_last(origin: Origin => origin),
                next(), previous(), resume(), pause(), toggle_play(), toggle_radio(),
                cycle_repeat(), toggle_repeat(), toggle_shuffle(),
                play_past(index: usize => index), play_upcoming(index: usize => index), play_similar(index: usize => index),
                seek(position: Duration => position), seek_fraction(fraction: f32 => fraction),
                set_volume(level: f32 => level), set_repeat(repeat: Repeat => repeat),
                set_shuffle(on: bool => on), set_sleep(sleep: Option<Sleep> => sleep),
                set_gapless(on: bool => on), set_normalisation(on: bool => on),
                set_quality(quality: Quality => quality), set_transcode(transcode: Transcode => transcode),
                set_keep_tracks(on: bool => on), set_cache_gb(gb: u32 => gb),
                measure_storage(), clear_cache(),
                remove_from_queue(ids: Vec<String> => &ids),
            })
        }
        "queue" => {
            let entity = sonora.queue.clone();
            ensure!(
                call.revision == Some(entity.read(cx).revision()),
                "the queue changed before the command arrived"
            );
            commands!(call, entity, cx, {
                clear_upcoming(), clear(), reset(),
                set_shuffle(on: bool => on), toggle_shuffle(),
                remove_upcoming(index: usize => index), remove_similar(index: usize => index),
                move_upcoming(from: usize => from, to: usize => to),
                move_upcoming_to_gap(from: usize => from, gap: usize => gap),
            })
        }
        "lyrics" => {
            let entity = sonora.lyrics.clone();
            ensure!(
                call.revision == Some(entity.read(cx).revision()),
                "the lyrics changed before the command arrived"
            );
            commands!(call, entity, cx, { choose(index: usize => index) })
        }
        _ => actions::dispatch(call, cx),
    }
}

/// Builds a backend page using server-owned session and library handles.
fn create_page(kind: &str, name: &str, peer_id: u64, peer: &Peer, cx: &mut App) -> Result<Page> {
    let sonora = Sonora::global(cx);
    let session = sonora.session.clone();
    let library = sonora.library.clone();
    let io = Io::global(cx);
    macro_rules! page {
        ($variant:ident, $build:expr) => {{
            let entity = cx.new($build);
            let name = name.to_owned();
            let peer = peer.clone();
            let subscription = cx.observe(&entity, move |entity, cx| {
                let data = entity.read(cx).remote_snapshot(cx);
                publish_page(&peer, &name, data, cx);
            });
            Page {
                entity: PageEntity::$variant(entity),
                _subscription: subscription,
                _events: None,
            }
        }};
    }
    Ok(match kind {
        "artist" => {
            let mut page = page!(Artist, |cx| crate::ArtistDetail::new(
                session, library, io, cx
            ));
            let PageEntity::Artist(entity) = &page.entity else {
                unreachable!()
            };
            let peer = peer.clone();
            let name = name.to_owned();
            page._events = Some(cx.subscribe(
                entity,
                move |entity, event: &crate::ArtistDetailEvent, cx| {
                    publish_signal(
                        &name,
                        serde_json::to_value(event).expect("artist event serializes"),
                        BTreeMap::from([(name.clone(), entity.read(cx).remote_snapshot(cx))]),
                        Some(&peer),
                        cx,
                    );
                },
            ));
            page
        }
        "detail" => page!(Detail, |cx| crate::Detail::new(session, library, io, cx)),
        "genres" => {
            let entity = peer_genres(peer_id, cx);
            let name = name.to_owned();
            let peer = peer.clone();
            let subscription = cx.observe(&entity, move |entity, cx| {
                publish_page(&peer, &name, entity.read(cx).remote_snapshot(cx), cx);
            });
            Page {
                entity: PageEntity::Genres(entity),
                _subscription: subscription,
                _events: None,
            }
        }
        "genre" => {
            let genres = peer_genres(peer_id, cx);
            page!(Genre, |cx| crate::GenreDetails::new(
                session, genres, io, cx
            ))
        }
        "home" => page!(Home, |cx| crate::Home::new(library, session, io, cx)),
        "profile" => page!(Profile, |cx| crate::Profile::new(session, io, cx)),
        "search" => page!(Search, |cx| crate::Search::new(session, library, io, cx)),
        "song" => page!(Song, |cx| crate::SongDetail::new(session, io, cx)),
        "tags" => page!(Tags, |cx| crate::Tags::new(session, library, io, cx)),
        _ => bail!("the service does not expose this read model"),
    })
}

fn publish_page(peer: &Peer, name: &str, data: Value, cx: &mut App) {
    let server = cx.global_mut::<Server>();
    server.revision += 1;
    peer.send(Message::Changed(Event {
        instance: server.instance.clone(),
        revision: server.revision,
        objects: BTreeMap::from([(name.to_owned(), data)]),
        removed: Vec::new(),
    }));
}

/// The genre overview shared by every genre page belonging to this client.
fn peer_genres(peer_id: u64, cx: &mut App) -> Entity<crate::Genres> {
    if let Some(genres) = cx.global::<Server>().genres.get(&peer_id) {
        return genres.clone();
    }
    let session = Sonora::global(cx).session.clone();
    let io = Io::global(cx);
    let genres = cx.new(|cx| crate::Genres::new(session, io, cx));
    cx.global_mut::<Server>()
        .genres
        .insert(peer_id, genres.clone());
    genres
}

impl PageEntity {
    fn snapshot(&self, cx: &App) -> Value {
        match self {
            Self::Artist(entity) => entity.read(cx).remote_snapshot(cx),
            Self::Detail(entity) => entity.read(cx).remote_snapshot(cx),
            Self::Genres(entity) => entity.read(cx).remote_snapshot(cx),
            Self::Genre(entity) => entity.read(cx).remote_snapshot(cx),
            Self::Home(entity) => entity.read(cx).remote_snapshot(cx),
            Self::Profile(entity) => entity.read(cx).remote_snapshot(cx),
            Self::Search(entity) => entity.read(cx).remote_snapshot(cx),
            Self::Song(entity) => entity.read(cx).remote_snapshot(cx),
            Self::Tags(entity) => entity.read(cx).remote_snapshot(cx),
        }
    }

    fn dispatch(&self, call: &Call, cx: &mut App) -> Result<Value> {
        match self {
            Self::Artist(entity) => {
                commands!(call, entity, cx, { open(id: String => &id), refresh() })
            }
            Self::Detail(entity) => commands!(call, entity, cx, {
                open_album(id: String => &id), open_playlist(id: String => &id),
                load_more(), refresh(), reload(),
                remove_from_playlist(id: String => id),
                remove_tracks_from_playlist(ids: Vec<String> => ids),
            }),
            Self::Genres(entity) => commands!(call, entity, cx, {
                load(), adopt(id: String => &id, cover: Option<String> => cover), forget(id: String => &id),
            }),
            Self::Genre(entity) => {
                commands!(call, entity, cx, { open(id: String => &id), refresh() })
            }
            Self::Home(entity) => commands!(call, entity, cx, {
                retry(), reload(), refresh(), feed(), set_visible(visible: bool => visible),
            }),
            Self::Profile(entity) => {
                commands!(call, entity, cx, { open(id: String => &id), refresh() })
            }
            Self::Search(entity) => commands!(call, entity, cx, { ask(query: String => &query) }),
            Self::Song(entity) => {
                commands!(call, entity, cx, { open(id: String => &id), refresh() })
            }
            Self::Tags(entity) => commands!(call, entity, cx, {
                close(), open(track: Track => track), save(tags: music::TrackTags => tags),
            }),
        }
    }
}
