//! Ordered service entities, events and cancellable tasks on a Tokio local executor.

use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::task::{Context as PollContext, Poll};
use std::time::Duration;

use anyhow::{Context as _, Result};
use futures::future::{AbortHandle, Abortable};
use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;
use tokio::sync::{Notify, oneshot};

mod credentials;

/// An account name and its opaque secret bytes in the platform credential store.
pub type Credential = (String, Vec<u8>);

type Callback = Box<dyn FnMut(Option<&dyn Any>, &mut App) -> bool>;
type Deferred = Box<dyn FnOnce(&mut App)>;

/// The identity of a service entity, independent of any presentation framework.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityId(u64);

impl EntityId {
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

/// State installed once for the lifetime of the service loop.
pub trait Global: 'static {}

/// Declares an entity's semantic event type.
pub trait EventEmitter<E: 'static>: 'static {}

/// A shared UTF-8 value used in serializable service models.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SharedString(Arc<str>);

impl SharedString {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for SharedString {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl PartialEq<&str> for SharedString {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl AsRef<str> for SharedString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SharedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<String> for SharedString {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl From<&str> for SharedString {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

/// A future whose work is cancelled when its owner drops it, unless detached explicitly.
pub struct Task<T> {
    result: oneshot::Receiver<T>,
    abort: Option<AbortHandle>,
}

impl<T> Unpin for Task<T> {}

impl<T> Future for Task<T> {
    type Output = T;
    fn poll(mut self: Pin<&mut Self>, cx: &mut PollContext<'_>) -> Poll<T> {
        match Pin::new(&mut self.result).poll(cx) {
            Poll::Ready(Ok(value)) => Poll::Ready(value),
            Poll::Ready(Err(_)) => panic!("service task ended without a result"),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        if let Some(abort) = self.abort.take() {
            abort.abort();
        }
    }
}

impl<T: 'static> Task<T> {
    pub fn ready(value: T) -> Self {
        let (sender, result) = oneshot::channel();
        let _ = sender.send(value);
        Self {
            result,
            abort: None,
        }
    }

    pub fn detach(mut self) {
        self.abort = None;
    }

    /// Runs foreground work on the service thread and ties cancellation to its handle.
    fn local(future: impl Future<Output = T> + 'static) -> Self {
        let (sender, result) = oneshot::channel();
        let (abort, registration) = AbortHandle::new_pair();
        tokio::task::spawn_local(async move {
            if let Ok(value) = Abortable::new(future, registration).await {
                let _ = sender.send(value);
            }
        });
        Self {
            result,
            abort: Some(abort),
        }
    }
}

/// Bounded I/O workers also run short, independent background computations.
#[derive(Clone)]
pub struct BackgroundExecutor(Handle);

impl BackgroundExecutor {
    pub fn timer(&self, duration: Duration) -> tokio::time::Sleep {
        tokio::time::sleep(duration)
    }

    pub fn spawn<T: Send + 'static>(
        &self,
        future: impl Future<Output = T> + Send + 'static,
    ) -> Task<T> {
        let (sender, result) = oneshot::channel();
        let (abort, registration) = AbortHandle::new_pair();
        self.0.spawn(async move {
            if let Ok(value) = Abortable::new(future, registration).await {
                let _ = sender.send(value);
            }
        });
        Task {
            result,
            abort: Some(abort),
        }
    }
}

/// Releases an entity when its final strong handle disappears.
struct Lease {
    id: EntityId,
    released: Rc<RefCell<Vec<EntityId>>>,
    wake: Rc<Notify>,
}

impl Drop for Lease {
    fn drop(&mut self) {
        self.released.borrow_mut().push(self.id);
        self.wake.notify_one();
    }
}

/// A strong handle; the final handle releases its state and owned tasks.
pub struct Entity<T: 'static> {
    lease: Rc<Lease>,
    marker: PhantomData<fn() -> T>,
}

impl<T> Clone for Entity<T> {
    fn clone(&self) -> Self {
        Self {
            lease: self.lease.clone(),
            marker: PhantomData,
        }
    }
}

impl<T> fmt::Debug for Entity<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.entity_id().fmt(f)
    }
}

impl<T> PartialEq for Entity<T> {
    fn eq(&self, other: &Self) -> bool {
        self.entity_id() == other.entity_id()
    }
}
impl<T> Eq for Entity<T> {}

impl<T> Entity<T> {
    pub fn entity_id(&self) -> EntityId {
        self.lease.id
    }
    pub fn downgrade(&self) -> WeakEntity<T> {
        WeakEntity {
            lease: Rc::downgrade(&self.lease),
            marker: PhantomData,
        }
    }
    pub fn read<'a>(&self, cx: &'a App) -> &'a T {
        cx.entities
            .get(&self.lease.id)
            .and_then(|value| value.downcast_ref())
            .expect("service entity is released or already being updated")
    }
    pub fn update<C: AppContext, R>(
        &self,
        cx: &mut C,
        update: impl FnOnce(&mut T, &mut Context<T>) -> R,
    ) -> C::Result<R> {
        cx.update_entity(self, update)
    }
}

/// A non-owning handle used by tasks and subscriptions to avoid retaining released pages.
pub struct WeakEntity<T: 'static> {
    lease: Weak<Lease>,
    marker: PhantomData<fn() -> T>,
}

impl<T> Clone for WeakEntity<T> {
    fn clone(&self) -> Self {
        Self {
            lease: self.lease.clone(),
            marker: PhantomData,
        }
    }
}

impl<T> WeakEntity<T> {
    pub fn upgrade(&self) -> Option<Entity<T>> {
        self.lease.upgrade().map(|lease| Entity {
            lease,
            marker: PhantomData,
        })
    }
    pub fn update<C: AppContext, R>(
        &self,
        cx: &mut C,
        update: impl FnOnce(&mut T, &mut Context<T>) -> R,
    ) -> Result<R> {
        let entity = self.upgrade().context("the service entity was released")?;
        cx.try_update(|cx| cx.edit(&entity, update))
    }
}

/// A scoped subscription. Detaching keeps it until the watched or owning entity is released.
pub struct Subscription {
    active: Rc<Cell<bool>>,
    wake: Rc<Notify>,
    detached: bool,
}

impl Subscription {
    pub fn detach(mut self) {
        self.detached = true;
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if !self.detached {
            self.active.set(false);
            self.wake.notify_one();
        }
    }
}

/// A deferred subscription whose callbacks do not retain their owning entity.
struct Watch {
    active: Rc<Cell<bool>>,
    ready: Rc<Cell<bool>>,
    owner: Option<EntityId>,
    event: Option<TypeId>,
    callback: Callback,
}

/// Mutations publish effects only after their entity has been stored again.
enum Effect {
    Changed(EntityId),
    Event(EntityId, Box<dyn Any>),
    Deferred(Deferred),
}

/// Serializes authoritative mutations and delivers ordered subscriptions without a platform.
pub struct App {
    entities: HashMap<EntityId, Box<dyn Any>>,
    globals: HashMap<TypeId, Box<dyn Any>>,
    observers: HashMap<EntityId, Vec<Watch>>,
    effects: VecDeque<Effect>,
    changed: HashSet<EntityId>,
    released: Rc<RefCell<Vec<EntityId>>>,
    wake: Rc<Notify>,
    quit: Rc<Notify>,
    this: Weak<RefCell<App>>,
    background: BackgroundExecutor,
    next: u64,
    depth: usize,
}

/// Entity access shared by synchronous callbacks and asynchronous tasks.
pub trait AppContext {
    type Result<T>;
    fn try_update<R>(&mut self, update: impl FnOnce(&mut App) -> R) -> Result<R>;
    fn update_entity<T: 'static, R>(
        &mut self,
        entity: &Entity<T>,
        update: impl FnOnce(&mut T, &mut Context<T>) -> R,
    ) -> Self::Result<R>;
}

impl AppContext for App {
    type Result<T> = T;

    fn try_update<R>(&mut self, update: impl FnOnce(&mut App) -> R) -> Result<R> {
        Ok(update(self))
    }
    fn update_entity<T: 'static, R>(
        &mut self,
        entity: &Entity<T>,
        update: impl FnOnce(&mut T, &mut Context<T>) -> R,
    ) -> R {
        self.edit(entity, update)
    }
}

impl App {
    /// Runs service state on the calling thread; network and background work use `handle`.
    pub fn run(handle: Handle, initialize: impl FnOnce(&mut App)) {
        let local = tokio::task::LocalSet::new();
        handle.clone().block_on(local.run_until(async move {
            let wake = Rc::new(Notify::new());
            let quit = Rc::new(Notify::new());
            let app = Rc::new_cyclic(|this| {
                RefCell::new(Self {
                    entities: HashMap::new(),
                    globals: HashMap::new(),
                    observers: HashMap::new(),
                    effects: VecDeque::new(),
                    changed: HashSet::new(),
                    released: Rc::new(RefCell::new(Vec::new())),
                    wake: wake.clone(),
                    quit: quit.clone(),
                    this: this.clone(),
                    background: BackgroundExecutor(handle),
                    next: 0,
                    depth: 1,
                })
            });
            {
                let mut cx = app.borrow_mut();
                initialize(&mut cx);
                cx.depth = 0;
                cx.flush();
            }
            loop {
                tokio::select! {
                    _ = quit.notified() => break,
                    _ = wake.notified() => app.borrow_mut().flush(),
                }
            }
        }));
    }

    /// Constructs an entity before publishing any effects produced by its constructor.
    pub fn create<T: 'static>(&mut self, build: impl FnOnce(&mut Context<T>) -> T) -> Entity<T> {
        self.next += 1;
        let entity = Entity {
            lease: Rc::new(Lease {
                id: EntityId(self.next),
                released: self.released.clone(),
                wake: self.wake.clone(),
            }),
            marker: PhantomData,
        };
        self.depth += 1;
        let value = build(&mut Context {
            app: self,
            entity: entity.downgrade(),
        });
        self.entities.insert(entity.entity_id(), Box::new(value));
        self.depth -= 1;
        self.flush();
        entity
    }

    /// Mutates one entity transactionally and rejects reentrant access to the same state.
    fn edit<T: 'static, R>(
        &mut self,
        entity: &Entity<T>,
        update: impl FnOnce(&mut T, &mut Context<T>) -> R,
    ) -> R {
        let mut value = self
            .entities
            .remove(&entity.entity_id())
            .expect("service entity is already being updated");
        self.depth += 1;
        let result = update(
            value
                .downcast_mut()
                .expect("service entity type matches its handle"),
            &mut Context {
                app: self,
                entity: entity.downgrade(),
            },
        );
        self.entities.insert(entity.entity_id(), value);
        self.depth -= 1;
        self.flush();
        result
    }

    pub fn set_global<T: Global>(&mut self, value: T) {
        self.globals.insert(TypeId::of::<T>(), Box::new(value));
    }
    pub fn has_global<T: Global>(&self) -> bool {
        self.globals.contains_key(&TypeId::of::<T>())
    }
    pub fn try_global<T: Global>(&self) -> Option<&T> {
        self.globals
            .get(&TypeId::of::<T>())
            .and_then(|value| value.downcast_ref())
    }
    pub fn global<T: Global>(&self) -> &T {
        self.try_global().expect("service global is installed")
    }
    pub fn global_mut<T: Global>(&mut self) -> &mut T {
        self.globals
            .get_mut(&TypeId::of::<T>())
            .and_then(|value| value.downcast_mut())
            .expect("service global is installed")
    }
    pub fn background_executor(&self) -> &BackgroundExecutor {
        &self.background
    }
    pub fn background_spawn<T: Send + 'static>(
        &self,
        future: impl Future<Output = T> + Send + 'static,
    ) -> Task<T> {
        self.background.spawn(future)
    }
    pub fn read_credentials(&self, url: &str) -> Task<Result<Option<Credential>>> {
        self.background.spawn(credentials::read(url.to_owned()))
    }
    pub fn write_credentials(&self, url: &str, account: &str, password: &[u8]) -> Task<Result<()>> {
        self.background.spawn(credentials::write(
            url.to_owned(),
            account.to_owned(),
            password.to_vec(),
        ))
    }
    pub fn quit(&mut self) {
        self.quit.notify_one();
    }
    pub fn defer(&mut self, callback: impl FnOnce(&mut App) + 'static) {
        self.effects.push_back(Effect::Deferred(Box::new(callback)));
    }

    pub fn spawn<R: 'static>(
        &mut self,
        task: impl AsyncFnOnce(&mut AsyncApp) -> R + 'static,
    ) -> Task<R> {
        let mut cx = AsyncApp {
            app: self.this.clone(),
            background: self.background.clone(),
        };
        Task::local(async move { task(&mut cx).await })
    }

    /// Activates after queued effects so a new subscription sees only subsequent changes.
    fn watch(
        &mut self,
        id: EntityId,
        owner: Option<EntityId>,
        event: Option<TypeId>,
        callback: Callback,
    ) -> Subscription {
        let active = Rc::new(Cell::new(true));
        let ready = Rc::new(Cell::new(false));
        let activation = ready.clone();
        self.defer(move |_| activation.set(true));
        self.observers.entry(id).or_default().push(Watch {
            active: active.clone(),
            ready,
            owner,
            event,
            callback,
        });
        Subscription {
            active,
            wake: self.wake.clone(),
            detached: false,
        }
    }

    pub fn observe<T: 'static>(
        &mut self,
        entity: &Entity<T>,
        mut callback: impl FnMut(Entity<T>, &mut App) + 'static,
    ) -> Subscription {
        let weak = entity.downgrade();
        self.watch(
            entity.entity_id(),
            None,
            None,
            Box::new(move |_, cx| {
                let Some(entity) = weak.upgrade() else {
                    return false;
                };
                callback(entity, cx);
                true
            }),
        )
    }

    pub fn subscribe<T: EventEmitter<E>, E: 'static>(
        &mut self,
        entity: &Entity<T>,
        mut callback: impl FnMut(Entity<T>, &E, &mut App) + 'static,
    ) -> Subscription {
        let weak = entity.downgrade();
        self.watch(
            entity.entity_id(),
            None,
            Some(TypeId::of::<E>()),
            Box::new(move |event, cx| {
                let Some(entity) = weak.upgrade() else {
                    return false;
                };
                callback(
                    entity,
                    event
                        .and_then(|event| event.downcast_ref())
                        .expect("event type matches subscription"),
                    cx,
                );
                true
            }),
        )
    }

    /// Delivers ordered effects and releases subscriptions owned by dropped entities.
    fn flush(&mut self) {
        if self.depth != 0 {
            return;
        }
        self.depth = 1;
        self.observers.retain(|_, observers| {
            observers.retain(|watch| watch.active.get());
            !observers.is_empty()
        });
        loop {
            let released = std::mem::take(&mut *self.released.borrow_mut());
            if !released.is_empty() {
                for id in &released {
                    self.entities.remove(id);
                    self.observers.remove(id);
                }
                self.observers.retain(|_, observers| {
                    observers.retain(|watch| {
                        watch.active.get()
                            && !watch.owner.is_some_and(|owner| released.contains(&owner))
                    });
                    !observers.is_empty()
                });
            }
            let Some(effect) = self.effects.pop_front() else {
                break;
            };
            let (id, event) = match effect {
                Effect::Changed(id) => {
                    self.changed.remove(&id);
                    (id, None)
                }
                Effect::Event(id, event) => (id, Some(event)),
                Effect::Deferred(callback) => {
                    callback(self);
                    continue;
                }
            };
            let kind = event.as_ref().map(|event| (**event).type_id());
            let mut observers = self.observers.remove(&id).unwrap_or_default();
            observers.retain_mut(|watch| {
                watch.active.get()
                    && (!watch.ready.get()
                        || watch.event != kind
                        || (watch.callback)(event.as_deref(), self))
            });
            observers.extend(self.observers.remove(&id).unwrap_or_default());
            if !observers.is_empty() {
                self.observers.insert(id, observers);
            }
        }
        self.depth = 0;
    }
}

/// Mutation context for one entity; callbacks run only after the entity is stored again.
pub struct Context<'a, T: 'static> {
    app: &'a mut App,
    entity: WeakEntity<T>,
}

impl<T> Deref for Context<'_, T> {
    type Target = App;
    fn deref(&self) -> &App {
        self.app
    }
}
impl<T> DerefMut for Context<'_, T> {
    fn deref_mut(&mut self) -> &mut App {
        self.app
    }
}
impl<T> AppContext for Context<'_, T> {
    type Result<R> = R;

    fn try_update<R>(&mut self, update: impl FnOnce(&mut App) -> R) -> Result<R> {
        Ok(update(self.app))
    }
    fn update_entity<U: 'static, R>(
        &mut self,
        entity: &Entity<U>,
        update: impl FnOnce(&mut U, &mut Context<U>) -> R,
    ) -> R {
        self.app.edit(entity, update)
    }
}

impl<T> Context<'_, T> {
    pub fn entity(&self) -> Entity<T> {
        self.entity.upgrade().expect("entity context has an owner")
    }
    pub fn weak_entity(&self) -> WeakEntity<T> {
        self.entity.clone()
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity().entity_id()
    }
    /// Coalesces change notifications until the current mutation finishes.
    pub fn notify(&mut self) {
        let id = self.entity_id();
        if self.app.changed.insert(id) {
            self.app.effects.push_back(Effect::Changed(id));
        }
    }
    /// Queues a typed semantic event behind the current state mutation.
    pub fn emit<E: 'static>(&mut self, event: E)
    where
        T: EventEmitter<E>,
    {
        self.app
            .effects
            .push_back(Effect::Event(self.entity_id(), Box::new(event)));
    }
    pub fn spawn<R: 'static>(
        &mut self,
        task: impl AsyncFnOnce(WeakEntity<T>, &mut AsyncApp) -> R + 'static,
    ) -> Task<R> {
        let weak = self.entity.clone();
        self.app.spawn(async move |cx| task(weak, cx).await)
    }
    pub fn observe<U: 'static>(
        &mut self,
        entity: &Entity<U>,
        mut callback: impl FnMut(&mut T, Entity<U>, &mut Context<T>) + 'static,
    ) -> Subscription {
        let owner = self.entity.clone();
        let target = entity.downgrade();
        self.app.watch(
            entity.entity_id(),
            Some(self.entity_id()),
            None,
            Box::new(move |_, cx| {
                let Some(target) = target.upgrade() else {
                    return false;
                };
                owner
                    .update(cx, |this, cx| callback(this, target, cx))
                    .is_ok()
            }),
        )
    }
    pub fn subscribe<U: EventEmitter<E>, E: 'static>(
        &mut self,
        entity: &Entity<U>,
        mut callback: impl FnMut(&mut T, Entity<U>, &E, &mut Context<T>) + 'static,
    ) -> Subscription {
        let owner = self.entity.clone();
        let target = entity.downgrade();
        self.app.watch(
            entity.entity_id(),
            Some(self.entity_id()),
            Some(TypeId::of::<E>()),
            Box::new(move |event, cx| {
                let Some(target) = target.upgrade() else {
                    return false;
                };
                let event = event
                    .and_then(|event| event.downcast_ref())
                    .expect("event type matches subscription");
                owner
                    .update(cx, |this, cx| callback(this, target, event, cx))
                    .is_ok()
            }),
        )
    }
}

/// Weak access to the service loop while a local asynchronous operation is suspended.
#[derive(Clone)]
pub struct AsyncApp {
    app: Weak<RefCell<App>>,
    background: BackgroundExecutor,
}

impl AsyncApp {
    pub fn update<R>(&mut self, update: impl FnOnce(&mut App) -> R) -> R {
        self.try_update(update).expect("service loop is running")
    }
    pub fn background_executor(&self) -> &BackgroundExecutor {
        &self.background
    }
    pub fn background_spawn<T: Send + 'static>(
        &self,
        future: impl Future<Output = T> + Send + 'static,
    ) -> Task<T> {
        self.background.spawn(future)
    }
}

impl AppContext for AsyncApp {
    type Result<T> = Result<T>;

    fn try_update<R>(&mut self, update: impl FnOnce(&mut App) -> R) -> Result<R> {
        let app = self.app.upgrade().context("the service loop stopped")?;
        let mut app = app
            .try_borrow_mut()
            .context("the service loop is already updating")?;
        app.depth += 1;
        let result = update(&mut app);
        app.depth -= 1;
        app.flush();
        Ok(result)
    }
    fn update_entity<T: 'static, R>(
        &mut self,
        entity: &Entity<T>,
        update: impl FnOnce(&mut T, &mut Context<T>) -> R,
    ) -> Result<R> {
        self.try_update(|cx| cx.edit(entity, update))
    }
}

/// Entity context traits used by shared orchestration modules.
pub mod prelude {
    pub use crate::AppContext as _;
}
