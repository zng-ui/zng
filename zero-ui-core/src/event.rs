//! App event API.

use fnv::FnvHashMap;
use retain_mut::RetainMut;
use unsafe_any::UnsafeAny;

use crate::app::{AppEventSender, AppShutdown, RecvFut, TimeoutOrAppShutdown};
use crate::command::DynCommand;
use crate::context::{AppContext, AppContextMut, Updates, WidgetContext, WidgetContextMut};
use crate::task::WidgetTask;
use crate::widget_base::IsEnabled;
use crate::{impl_ui_node, UiNode};
use std::cell::{Cell, RefCell};
use std::fmt::{self, Debug};
use std::future::Future;
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{any::*, collections::VecDeque};

/// [`Event`] arguments.
pub trait EventArgs: Debug + Clone + 'static {
    /// Gets the instant this event happen.
    fn timestamp(&self) -> Instant;
    /// If this event arguments is relevant to the widget context.
    fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool;

    /// Requests that subsequent handlers skip this event.
    ///
    /// Cloned arguments signal stop for all clones.
    fn stop_propagation(&self);

    /// If the handler must skip this event.
    ///
    /// Note that property level handlers don't need to check this, as those handlers are
    /// already not called when this is `true`. [`UiNode`](crate::UiNode) and
    /// [`AppExtension`](crate::app::AppExtension) implementers must check if this is `true`.
    fn stop_propagation_requested(&self) -> bool;
}

/// [`Event`] arguments that can be canceled.
pub trait CancelableEventArgs: EventArgs {
    /// If the originating action must be canceled.
    fn cancel_requested(&self) -> bool;
    /// Cancel the originating action.
    ///
    /// Cloned arguments signal cancel for all clones.
    fn cancel(&self);
}

/// Identifies an event type.
///
/// Use [`event!`](macro@event) to declare.
pub trait Event: Debug + Clone + Copy + 'static {
    /// Event arguments type.
    type Args: EventArgs;

    /// Schedule an event update.
    #[doc(hidden)]
    #[inline(always)]
    fn notify(events: &mut Events, args: Self::Args) {
        events.notify::<Self>(args);
    }

    /// Gets the event arguments if the update is for `Self`.
    #[inline(always)]
    fn update<U: EventUpdateArgs>(args: &U) -> Option<&EventUpdate<Self>> {
        args.args_for::<Self>()
    }
}

mod protected {
    pub trait EventUpdateArgs {}
}

/// [`EventUpdateArgs`] for event `E`, dereferences to the argument.
pub struct EventUpdate<E: Event>(pub E::Args);
impl<E: Event> EventUpdate<E> {
    /// Clone the arguments.
    #[allow(clippy::should_implement_trait)] // that is what we want.
    pub fn clone(&self) -> E::Args {
        self.0.clone()
    }

    fn boxed(self) -> BoxedEventUpdate {
        BoxedEventUpdate {
            event_type: TypeId::of::<E>(),
            event_name: type_name::<E>(),
            args: Box::new(self),
        }
    }

    fn boxed_send(self) -> BoxedSendEventUpdate
    where
        E::Args: Send,
    {
        BoxedSendEventUpdate {
            event_type: TypeId::of::<E>(),
            event_name: type_name::<E>(),
            args: Box::new(self),
        }
    }
}
impl<E: Event> protected::EventUpdateArgs for EventUpdate<E> {}
impl<E: Event> fmt::Debug for EventUpdate<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "EventUpdate<{}>({:#?})", type_name::<E>(), self.0)
        } else {
            write!(f, "EventUpdate<{}>({:?})", type_name::<E>(), self.0)
        }
    }
}
impl<E: Event> Deref for EventUpdate<E> {
    type Target = E::Args;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Boxed [`EventUpdateArgs`].
pub struct BoxedEventUpdate {
    event_type: TypeId,
    event_name: &'static str,
    args: Box<dyn UnsafeAny>,
}
impl BoxedEventUpdate {
    /// Unbox the arguments for `Q` if the update is for `Q`.
    pub fn unbox_for<Q: Event>(self) -> Result<Q::Args, Self> {
        if self.event_type == TypeId::of::<Q>() {
            Ok(unsafe {
                // SAFETY: its the same type
                *self.args.downcast_unchecked()
            })
        } else {
            Err(self)
        }
    }
}
impl protected::EventUpdateArgs for BoxedEventUpdate {}
impl fmt::Debug for BoxedEventUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "boxed {}", self.event_name)
    }
}
impl EventUpdateArgs for BoxedEventUpdate {
    #[inline(always)]
    fn args_for<Q: Event>(&self) -> Option<&EventUpdate<Q>> {
        if self.event_type == TypeId::of::<Q>() {
            Some(unsafe {
                // SAFETY: its the same type
                self.args.downcast_ref_unchecked()
            })
        } else {
            None
        }
    }

    #[inline(always)]
    fn as_any(&self) -> AnyEventUpdate {
        AnyEventUpdate {
            event_type: self.event_type,
            event_name: self.event_name,
            args: unsafe {
                // SAFETY: no different then the EventUpdate::as_any()
                self.args.downcast_ref_unchecked()
            },
        }
    }
}

/// A [`BoxedEventUpdate`] that is [`Send`].
pub struct BoxedSendEventUpdate {
    event_type: TypeId,
    event_name: &'static str,
    args: Box<dyn UnsafeAny + Send>,
}
impl BoxedSendEventUpdate {
    /// Unbox the arguments for `Q` if the update is for `Q`.
    pub fn unbox_for<Q: Event>(self) -> Result<Q::Args, Self>
    where
        Q::Args: Send,
    {
        if self.event_type == TypeId::of::<Q>() {
            Ok(unsafe {
                // SAFETY: its the same type
                *<dyn UnsafeAny>::downcast_unchecked(self.args)
            })
        } else {
            Err(self)
        }
    }

    /// Convert to [`BoxedEventUpdate`].
    pub fn forget_send(self) -> BoxedEventUpdate {
        BoxedEventUpdate {
            event_type: self.event_type,
            event_name: self.event_name,
            args: self.args,
        }
    }
}
impl protected::EventUpdateArgs for BoxedSendEventUpdate {}
impl fmt::Debug for BoxedSendEventUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "boxed send {}", self.event_name)
    }
}

/// Type erased [`EventUpdateArgs`].
pub struct AnyEventUpdate<'a> {
    event_type: TypeId,
    event_name: &'static str,
    args: &'a (),
}
impl<'a> fmt::Debug for AnyEventUpdate<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "any {}", self.event_name)
    }
}
impl<'a> protected::EventUpdateArgs for AnyEventUpdate<'a> {}
impl<'a> EventUpdateArgs for AnyEventUpdate<'a> {
    #[inline(always)]
    fn args_for<Q: Event>(&self) -> Option<&EventUpdate<Q>> {
        if self.event_type == TypeId::of::<Q>() {
            Some(unsafe {
                // SAFETY: its the same type.
                #[allow(clippy::transmute_ptr_to_ptr)]
                mem::transmute(self.args)
            })
        } else {
            None
        }
    }

    #[inline(always)]
    fn as_any(&self) -> AnyEventUpdate {
        AnyEventUpdate {
            event_type: self.event_type,
            event_name: self.event_name,
            args: self.args,
        }
    }
}

/// Represents an event update.
pub trait EventUpdateArgs: protected::EventUpdateArgs + fmt::Debug {
    /// Gets the the update arguments if the event updating is `Q`.
    fn args_for<Q: Event>(&self) -> Option<&EventUpdate<Q>>;

    /// Type erased event update.
    fn as_any(&self) -> AnyEventUpdate;
}
impl<E: Event> EventUpdateArgs for EventUpdate<E> {
    #[inline(always)]
    fn args_for<Q: Event>(&self) -> Option<&EventUpdate<Q>> {
        if TypeId::of::<E>() == TypeId::of::<Q>() {
            Some(unsafe {
                // SAFETY: its the same type.
                #[allow(clippy::transmute_ptr_to_ptr)]
                std::mem::transmute(self)
            })
        } else {
            None
        }
    }

    #[inline(always)]
    fn as_any(&self) -> AnyEventUpdate {
        AnyEventUpdate {
            event_type: TypeId::of::<E>(),
            event_name: type_name::<E>(),
            args: unsafe {
                // SAFETY: nothing will be done with it other then a validated restore in `args_for`.
                #[allow(clippy::transmute_ptr_to_ptr)]
                mem::transmute(self)
            },
        }
    }
}

/// Identifies an event type for an action that can be canceled.
///
/// # Auto-Implemented
///
/// This trait is auto-implemented for all events with cancellable arguments.
pub trait CancelableEvent: Event + 'static {
    /// Cancelable event arguments type.
    type CancelableArgs: CancelableEventArgs;
}
impl<A: CancelableEventArgs, E: Event<Args = A>> CancelableEvent for E {
    type CancelableArgs = A;
}

/// A buffered event listener.
///
/// This `struct` is a refence to the buffer, clones of it point to the same buffer. This `struct`
/// is not `Send`, you can use an [`Events::receiver`] for that.
#[derive(Clone)]
pub struct EventBuffer<E: Event> {
    queue: Rc<RefCell<VecDeque<E::Args>>>,
}
impl<E: Event> EventBuffer<E> {
    /// If there are any updates in the buffer.
    #[inline]
    pub fn has_updates(&self) -> bool {
        !RefCell::borrow(&self.queue).is_empty()
    }

    /// Take the oldest event in the buffer.
    #[inline]
    pub fn pop_oldest(&self) -> Option<E::Args> {
        self.queue.borrow_mut().pop_front()
    }

    /// Take the oldest `n` events from the buffer.
    ///
    /// The result is sorted from oldest to newer.
    #[inline]
    pub fn pop_oldest_n(&self, n: usize) -> Vec<E::Args> {
        self.queue.borrow_mut().drain(..n).collect()
    }

    /// Take all the events from the buffer.
    ///
    /// The result is sorted from oldest to newest.
    #[inline]
    pub fn pop_all(&self) -> Vec<E::Args> {
        self.queue.borrow_mut().drain(..).collect()
    }

    /// Create an empty buffer that will always stay empty.
    #[inline]
    pub fn never() -> Self {
        EventBuffer { queue: Default::default() }
    }
}

/// An event update sender that can be used from any thread and without access to [`Events`].
///
/// Use [`Events::sender`] to create a sender.
pub struct EventSender<E>
where
    E: Event,
    E::Args: Send,
{
    sender: AppEventSender,
    _event: PhantomData<E>,
}
impl<E> Clone for EventSender<E>
where
    E: Event,
    E::Args: Send,
{
    fn clone(&self) -> Self {
        EventSender {
            sender: self.sender.clone(),
            _event: PhantomData,
        }
    }
}
impl<E> fmt::Debug for EventSender<E>
where
    E: Event,
    E::Args: Send,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EventSender<{}>", type_name::<E>())
    }
}
impl<E> EventSender<E>
where
    E: Event,
    E::Args: Send,
{
    /// Send an event update.
    pub fn send(&self, args: E::Args) -> Result<(), AppShutdown<E::Args>> {
        let update = EventUpdate::<E>(args).boxed_send();
        self.sender.send_event(update).map_err(|e| {
            if let Ok(e) = e.0.unbox_for::<E>() {
                AppShutdown(e)
            } else {
                unreachable!()
            }
        })
    }
}

/// An event update receiver that can be used from any thread and without access to [`Events`].
///
/// Use [`Events::receiver`] to create a receiver, drop to stop listening.
pub struct EventReceiver<E>
where
    E: Event,
    E::Args: Send,
{
    receiver: flume::Receiver<E::Args>,
}
impl<E> Clone for EventReceiver<E>
where
    E: Event,
    E::Args: Send,
{
    fn clone(&self) -> Self {
        EventReceiver {
            receiver: self.receiver.clone(),
        }
    }
}
impl<E> Debug for EventReceiver<E>
where
    E: Event,
    E::Args: Send,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EventReceiver<{}>", type_name::<E>())
    }
}
impl<E> EventReceiver<E>
where
    E: Event,
    E::Args: Send,
{
    /// Receives the oldest send update, blocks until the event updates.
    #[inline]
    pub fn recv(&self) -> Result<E::Args, AppShutdown<()>> {
        self.receiver.recv().map_err(|_| AppShutdown(()))
    }

    /// Tries to receive the oldest sent update not received, returns `Ok(args)` if there was at least
    /// one update, or returns `Err(None)` if there was no update or returns `Err(AppHasShutdown)` if the connected
    /// app has shutdown.
    #[inline]
    pub fn try_recv(&self) -> Result<E::Args, Option<AppShutdown<()>>> {
        self.receiver.try_recv().map_err(|e| match e {
            flume::TryRecvError::Empty => None,
            flume::TryRecvError::Disconnected => Some(AppShutdown(())),
        })
    }

    /// Receives the oldest send update, blocks until the event updates or until the `deadline` is reached.
    #[inline]
    pub fn recv_deadline(&self, deadline: Instant) -> Result<E::Args, TimeoutOrAppShutdown> {
        self.receiver.recv_deadline(deadline).map_err(TimeoutOrAppShutdown::from)
    }

    /// Receives the oldest send update, blocks until the event updates or until timeout.
    #[inline]
    pub fn recv_timeout(&self, dur: Duration) -> Result<E::Args, TimeoutOrAppShutdown> {
        self.receiver.recv_timeout(dur).map_err(TimeoutOrAppShutdown::from)
    }

    /// Returns a future that receives the oldest send update, awaits until an event update occurs.
    #[inline]
    pub fn recv_async(&self) -> RecvFut<E::Args> {
        self.receiver.recv_async().into()
    }

    /// Turns into a future that receives the oldest send update, awaits until an event update occurs.
    #[inline]
    pub fn into_recv_async(self) -> RecvFut<'static, E::Args> {
        self.receiver.into_recv_async().into()
    }

    /// Creates a blocking iterator over event updates, if there are no updates sent the iterator blocks,
    /// the iterator only finishes when the app shuts-down.
    #[inline]
    pub fn iter(&self) -> flume::Iter<E::Args> {
        self.receiver.iter()
    }

    /// Create a non-blocking iterator over event updates, the iterator finishes if
    /// there are no more updates sent.
    #[inline]
    pub fn try_iter(&self) -> flume::TryIter<E::Args> {
        self.receiver.try_iter()
    }
}
impl<E> From<EventReceiver<E>> for flume::Receiver<E::Args>
where
    E: Event,
    E::Args: Send,
{
    fn from(e: EventReceiver<E>) -> Self {
        e.receiver
    }
}
impl<'a, E> IntoIterator for &'a EventReceiver<E>
where
    E: Event,
    E::Args: Send,
{
    type Item = E::Args;

    type IntoIter = flume::Iter<'a, E::Args>;

    fn into_iter(self) -> Self::IntoIter {
        self.receiver.iter()
    }
}
impl<E> IntoIterator for EventReceiver<E>
where
    E: Event,
    E::Args: Send,
{
    type Item = E::Args;

    type IntoIter = flume::IntoIter<E::Args>;

    fn into_iter(self) -> Self::IntoIter {
        self.receiver.into_iter()
    }
}

struct OnEventHandler {
    handle: OnEventHandle,
    handler: Box<dyn FnMut(&mut AppContext, &BoxedEventUpdate) -> Retain>,
}

/// Event arguments for an [`on_event`](Events::on_event) or [`on_pre_event`](Events::on_pre_event) handler.
pub struct AppEventArgs<'a, E: EventArgs> {
    /// The actual event arguments, this `struct` dereferences to this value.
    pub args: &'a E,
    unsubscribe: Cell<bool>,
}
impl<'a, E: EventArgs> AppEventArgs<'a, E> {
    /// Drops the event handler.
    #[inline]
    pub fn unsubscribe(&self) {
        self.unsubscribe.set(true)
    }
}
impl<'a, E: EventArgs> Deref for AppEventArgs<'a, E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        self.args
    }
}

/// Event arguments for an [`on_event_async`](Events::on_event_async) or [`on_pre_event_async`](Events::on_event_async) handler.
#[derive(Clone)]
pub struct AppAsyncEventArgs<E: EventArgs> {
    /// The actual event arguments, this `struct` dereferences to this value.
    pub args: E,
    unsubscribe: std::sync::Weak<OnEventHandleData>,
}
impl<E: EventArgs> AppAsyncEventArgs<E> {
    /// Flags the event handler and any running async tasks for dropping.
    #[inline]
    pub fn unsubscribe(&self) {
        if let Some(handle) = self.unsubscribe.upgrade() {
            handle.unsubscribe.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
}
impl<E: EventArgs> Deref for AppAsyncEventArgs<E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        &self.args
    }
}

/// Represents an app context event handler created by [`Events::on_event`] or [`Events::on_pre_event`].
///
/// Drop all clones of this handle to drop the handler, or call [`forget`](Self::forget) to drop the handle
/// without dropping the handler.
#[derive(Clone)]
#[must_use = "the event handler unsubscribes if the handle is dropped"]
pub struct OnEventHandle(Arc<OnEventHandleData>);
impl OnEventHandle {
    /// Drops the handle but does **not** drop the handler closure.
    ///
    /// This method does not work like [`std::mem::forget`], **no memory is leaked**, the handle
    /// memory is released immediately and the handler memory is released when application shuts-down.
    #[inline]
    pub fn forget(self) {
        self.0.forget.store(true, Ordering::Relaxed);
    }

    /// Drops the handle and flags the handler closure to drop even if there are other handles alive.
    #[inline]
    pub fn unsubscribe(self) {
        self.0.unsubscribe.store(true, Ordering::Relaxed);
    }

    fn retain(&self, min_count: usize) -> bool {
        self.0.retain(min_count)
    }

    fn new() -> Self {
        OnEventHandle(Arc::new(OnEventHandleData {
            forget: AtomicBool::new(false),
            unsubscribe: AtomicBool::new(false),
        }))
    }
}
struct OnEventHandleData {
    forget: AtomicBool,
    unsubscribe: AtomicBool,
}
impl OnEventHandleData {
    fn retain(self: &Arc<Self>, min_count: usize) -> bool {
        !self.unsubscribe.load(Ordering::Relaxed) && (self.forget.load(Ordering::Relaxed) || Arc::strong_count(self) > min_count)
    }
}

thread_singleton!(SingletonEvents);

type BufferEntry = Box<dyn Fn(&BoxedEventUpdate) -> Retain>;

/// Access to application events.
///
/// An instance of this struct is available in [`AppContext`] and derived contexts.
pub struct Events {
    app_event_sender: AppEventSender,

    updates: Vec<BoxedEventUpdate>,

    pre_buffers: Vec<BufferEntry>,
    buffers: Vec<BufferEntry>,
    pre_handlers: Vec<OnEventHandler>,
    pos_handlers: Vec<OnEventHandler>,

    commands: FnvHashMap<TypeId, DynCommand>,

    _singleton: SingletonEvents,
}
impl Events {
    /// If an instance of `Events` already exists in the  current thread.
    #[inline]
    pub(crate) fn instantiated() -> bool {
        SingletonEvents::in_use()
    }

    /// Produces the instance of `Events`. Only a single
    /// instance can exist in a thread at a time, panics if called
    /// again before dropping the previous instance.
    #[inline]
    pub(crate) fn instance(app_event_sender: AppEventSender) -> Self {
        Events {
            app_event_sender,
            updates: vec![],
            pre_buffers: vec![],
            buffers: vec![],
            pre_handlers: vec![],
            pos_handlers: vec![],
            commands: FnvHashMap::default(),
            _singleton: SingletonEvents::assert_new("Events"),
        }
    }

    pub(crate) fn notify<E: Event>(&mut self, args: E::Args) {
        let update = EventUpdate::<E>(args);
        self.updates.push(update.boxed());
    }

    pub(crate) fn notify_app_event(&mut self, update: BoxedSendEventUpdate) {
        self.updates.push(update.forget_send());
    }

    pub(crate) fn register_command(&mut self, command: DynCommand) {
        if let Some(already) = self.commands.insert(command.command_type_id(), command) {
            self.commands.insert(already.command_type_id(), already);
            panic!("command `{:?}` is already registered", command)
        }
    }

    /// Creates an event buffer that listens to `E`. The event updates are pushed as soon as possible, before
    /// the UI and [`on_event`](Self::on_event) are notified.
    ///
    /// Drop the buffer to stop listening.
    pub fn pre_buffer<E: Event>(&mut self) -> EventBuffer<E> {
        Self::push_buffer::<E>(&mut self.pre_buffers)
    }

    /// Creates an event buffer that listens to `E`. The event updates are pushed only after
    /// the UI and [`on_event`](Self::on_event) are notified.
    ///
    /// Drop the buffer to stop listening.
    pub fn buffer<E: Event>(&mut self) -> EventBuffer<E> {
        Self::push_buffer::<E>(&mut self.buffers)
    }

    fn push_buffer<E: Event>(buffers: &mut Vec<BufferEntry>) -> EventBuffer<E> {
        let buf = EventBuffer::never();
        let weak = Rc::downgrade(&buf.queue);
        buffers.push(Box::new(move |args| {
            let mut retain = false;
            if let Some(rc) = weak.upgrade() {
                if let Some(args) = E::update(args) {
                    rc.borrow_mut().push_back(args.clone());
                }
                retain = true;
            }
            retain
        }));
        buf
    }

    /// Creates a sender that can raise an event from other threads and without access to [`Events`].
    pub fn sender<A, E>(&mut self) -> EventSender<E>
    where
        E: Event,
        E::Args: Send,
    {
        EventSender {
            sender: self.app_event_sender.clone(),
            _event: PhantomData,
        }
    }

    /// Creates a channel that can listen to event from another thread. The event updates are sent as soon as possible, before
    /// the UI and [`on_event`](Self::on_event) are notified.
    ///
    /// Drop the receiver to stop listening.
    pub fn pre_receiver<E>(&mut self) -> EventReceiver<E>
    where
        E: Event,
        E::Args: Send,
    {
        Self::push_receiver::<E>(&mut self.pre_buffers)
    }

    /// Creates a channel that can listen to event from another thread. The event updates are sent only after the
    /// UI and [`on_event`](Self::on_event) are notified.
    ///
    /// Drop the receiver to stop listening.
    pub fn receiver<E>(&mut self) -> EventReceiver<E>
    where
        E: Event,
        E::Args: Send,
    {
        Self::push_receiver::<E>(&mut self.buffers)
    }

    fn push_receiver<E>(buffers: &mut Vec<BufferEntry>) -> EventReceiver<E>
    where
        E: Event,
        E::Args: Send,
    {
        let (sender, receiver) = flume::unbounded();

        buffers.push(Box::new(move |e| {
            let mut retain = true;
            if let Some(args) = E::update(e) {
                retain = sender.send(args.clone()).is_ok();
            }
            retain
        }));

        EventReceiver { receiver }
    }

    /// Creates a preview event handler.
    ///
    /// The event `handler` is called for every update of `E` that are not marked [`stop_propagation`](EventArgs::stop_propagation).
    /// The handler is called before UI handlers and [`on_event`](Self::on_event) handlers, it is called after all previous registered
    /// preview handlers.
    ///
    /// Returns a [`OnEventHandle`] that can be used to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`AppEventArgs::unsubscribe`].
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::event::*;
    /// # use zero_ui_core::focus::FocusChangedEvent;
    /// # fn example(ctx: &mut zero_ui_core::context::AppContext) {
    /// let handle = ctx.events.on_pre_event(FocusChangedEvent, |_ctx, args| {
    ///     println!("focused: {:?}", args.new_focus);
    /// });
    /// # }
    /// ```
    /// The example listens to all `FocusChangedEvent` events, independent of widget context and before all UI handlers.
    ///
    /// # Panics
    ///
    /// If the event is not registered in the application.
    pub fn on_pre_event<E, H>(&mut self, _: E, handler: H) -> OnEventHandle
    where
        E: Event,
        H: FnMut(&mut AppContext, &AppEventArgs<E::Args>) + 'static,
    {
        Self::push_event_handler::<E, H>(&mut self.pre_handlers, handler)
    }

    /// Creates an async preview event handler.
    ///
    /// The event `handler` is called for every update of `E` that are not marked [`stop_propagation`](EventArgs::stop_propagation).
    /// The handler is called before UI handlers and [`on_event`](Self::on_event) handlers, it is called after all previous registered
    /// preview handlers. Only the code up to the first `await` is executed immediately the code afterwards is executed in app updates.
    ///
    /// Returns a [`OnEventHandle`] that can be used to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`AppEventArgs::unsubscribe`]. Unsubscribe using the handle also cancels running async tasks.
    pub fn on_pre_event_async<E, F, H>(&mut self, handler: H) -> OnEventHandle
    where
        E: Event,
        F: Future<Output = ()> + 'static,
        H: FnMut(AppContextMut, AppAsyncEventArgs<E::Args>) -> F + 'static,
    {
        Self::push_async_event_handler::<E, F, H>(&mut self.pre_handlers, handler)
    }

    /// Creates an event handler.
    ///
    /// The event `handler` is called for every update of `E` that are not marked [`stop_propagation`](EventArgs::stop_propagation).
    /// The handler is called after all [`on_pre_event`],(Self::on_pre_event) all UI handlers and all [`on_event`](Self::on_event) handlers
    /// registered before this one.
    ///
    /// Returns a [`OnEventHandle`] that can be used to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`AppEventArgs::unsubscribe`].
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::event::*;
    /// # use zero_ui_core::focus::FocusChangedEvent;
    /// # fn example(ctx: &mut zero_ui_core::context::AppContext) {
    /// let handle = ctx.events.on_event(FocusChangedEvent, |_ctx, args| {
    ///     println!("focused: {:?}", args.new_focus);
    /// });
    /// # }
    /// ```
    /// The example listens to all `FocusChangedEvent` events, independent of widget context, after the UI was notified.
    ///
    /// # Panics
    ///
    /// If the event is not registered in the application.
    pub fn on_event<E, H>(&mut self, _: E, handler: H) -> OnEventHandle
    where
        E: Event,
        H: FnMut(&mut AppContext, &AppEventArgs<E::Args>) + 'static,
    {
        Self::push_event_handler::<E, H>(&mut self.pos_handlers, handler)
    }

    /// Creates an async event handler.
    ///
    /// The event `handler` is called for every update of `E` that are not marked [`stop_propagation`](EventArgs::stop_propagation).
    /// The handler is called after all [`on_pre_event`],(Self::on_pre_event) all UI handlers and all [`on_event`](Self::on_event) handlers
    /// registered before this one. Only the code up to the first `await` is executed immediately the code afterwards is executed in app updates.
    ///
    /// Returns a [`OnEventHandle`] that can be used to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`AppEventArgs::unsubscribe`]. Unsubscribe using the handle also cancels running async tasks.
    pub fn on_event_async<E, F, H>(&mut self, _: E, handler: H) -> OnEventHandle
    where
        E: Event,
        F: Future<Output = ()> + 'static,
        H: FnMut(AppContextMut, AppAsyncEventArgs<E::Args>) -> F + 'static,
    {
        Self::push_async_event_handler::<E, F, H>(&mut self.pos_handlers, handler)
    }

    fn push_event_handler<E, H>(handlers: &mut Vec<OnEventHandler>, mut handler: H) -> OnEventHandle
    where
        E: Event,
        H: FnMut(&mut AppContext, &AppEventArgs<E::Args>) + 'static,
    {
        let handle = OnEventHandle::new();
        let handler = move |ctx: &mut AppContext, args: &BoxedEventUpdate| {
            let mut retain = true;
            if let Some(args) = E::update(args) {
                if !args.stop_propagation_requested() {
                    let args = AppEventArgs {
                        args: &args.0,
                        unsubscribe: Cell::new(false),
                    };
                    handler(ctx, &args);
                    retain = !args.unsubscribe.get();
                }
            }
            retain
        };
        handlers.push(OnEventHandler {
            handle: handle.clone(),
            handler: Box::new(handler),
        });
        handle
    }

    fn push_async_event_handler<E, F, H>(handlers: &mut Vec<OnEventHandler>, mut handler: H) -> OnEventHandle
    where
        E: Event,
        F: Future<Output = ()> + 'static,
        H: FnMut(AppContextMut, AppAsyncEventArgs<E::Args>) -> F + 'static,
    {
        let handle = OnEventHandle::new();
        let weak_handle = Arc::downgrade(&handle.0);
        let handler = move |ctx: &mut AppContext, args: &BoxedEventUpdate| {
            let mut retain = true;
            if let Some(args) = E::update(args) {
                if !args.stop_propagation_requested() {
                    // event matches, will notify causing an async task to start.

                    let args = AppAsyncEventArgs {
                        args: args.clone(),
                        unsubscribe: weak_handle.clone(),
                    };
                    let call = &mut handler;
                    let mut task = ctx.async_task(move |ctx| call(ctx, args));

                    if task.update(ctx).is_none() {
                        // executed the task up to the first `.await` and it did not complete.

                        // check if unsubscribe was requested.
                        if let Some(handle) = weak_handle.upgrade() {
                            retain = handle.retain(2);
                        } else {
                            retain = false;
                        }
                        if retain {
                            // did not unsubscribe, schedule an `on_pre_update` handler to execute the task.

                            let weak_handle = weak_handle.clone();
                            ctx.updates
                                .on_pre_update(move |ctx, args| {
                                    // check if the event unsubscribe was requested.
                                    if let Some(handle) = weak_handle.upgrade() {
                                        if handle.retain(2) {
                                            // task still active, do update.
                                            if task.update(ctx).is_none() && handle.retain(2) {
                                                // task updated and did not request event unsubscribe.
                                                return;
                                            }
                                        }
                                    }

                                    // unsubscribe the task executor.
                                    // can be a cancel if the event unsubscribed
                                    // or can be because the task is finished.
                                    args.unsubscribe();
                                })
                                .forget();
                        }
                    }
                }
            }
            retain
        };
        handlers.push(OnEventHandler {
            handle: handle.clone(),
            handler: Box::new(handler),
        });
        handle
    }

    #[must_use]
    pub(super) fn apply_updates(&mut self, updates: &mut Updates) -> Vec<BoxedEventUpdate> {
        if !self.updates.is_empty() {
            updates.update();
        }
        self.updates.drain(..).collect()
    }

    pub(super) fn on_pre_events(ctx: &mut AppContext, args: &BoxedEventUpdate) {
        ctx.events.pre_buffers.retain(|buf| buf(args));

        let mut handlers = mem::take(&mut ctx.events.pre_handlers);
        Self::notify_retain(&mut handlers, ctx, args);
        handlers.extend(mem::take(&mut ctx.events.pre_handlers));
        ctx.events.pre_handlers = handlers;
    }

    pub(super) fn on_events(ctx: &mut AppContext, args: &BoxedEventUpdate) {
        let mut handlers = mem::take(&mut ctx.events.pos_handlers);
        Self::notify_retain(&mut handlers, ctx, args);
        handlers.extend(mem::take(&mut ctx.events.pos_handlers));
        ctx.events.pos_handlers = handlers;

        ctx.events.buffers.retain(|buf| buf(args));
    }

    fn notify_retain(handlers: &mut Vec<OnEventHandler>, ctx: &mut AppContext, args: &BoxedEventUpdate) {
        handlers.retain_mut(|e| {
            let mut retain = e.handle.retain(1);
            if retain {
                retain = (e.handler)(ctx, args);
            }
            retain
        });
    }
}

type Retain = bool;

/// Declares new [`EventArgs`](crate::event::EventArgs) types.
///
/// # Example
///
/// ```
/// # use zero_ui_core::event::event_args;
/// use zero_ui_core::render::WidgetPath;
///
/// event_args! {
///     /// My event arguments.
///     pub struct MyEventArgs {
///         /// My argument.
///         pub arg: String,
///         /// My event target.
///         pub target: WidgetPath,
///
///         ..
///
///         /// If `ctx.path.widget_id()` is in the `self.target` path.
///         fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
///             self.target.contains(ctx.path.widget_id())
///         }
///     }
///
///     // multiple structs can be declared in the same call.
///     // pub struct MyOtherEventArgs { /**/ }
/// }
/// ```
#[macro_export]
macro_rules! event_args {
    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, $ctx:ident: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }
        }
    )+) => {$(
        $(#[$outer])*
        #[derive(Debug, Clone)]
        $vis struct $Args {
            /// When the event happened.
            pub timestamp: std::time::Instant,
            $($(#[$arg_outer])* $arg_vis $arg : $arg_ty,)*

            // Arc<AtomicBool> so we don't cause the $Args:!Send and block the user from creating event channels.
            stop_propagation: std::sync::Arc<std::sync::atomic::AtomicBool>,
        }
        impl $Args {
            /// New args from values that convert [into](Into) the argument types.
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn new(timestamp: impl Into<std::time::Instant>, $($arg : impl Into<$arg_ty>),*) -> Self {
                $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    stop_propagation: std::sync::Arc::default(),
                }
            }

            /// Arguments for event that happened now (`Instant::now`).
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new(std::time::Instant::now(), $($arg),*)
            }

            /// Requests that subsequent handlers skip this event.
            ///
            /// Cloned arguments signal stop for all clones.
            #[inline]
            pub fn stop_propagation(&self) {
                <Self as $crate::event::EventArgs>::stop_propagation(self)
            }

            /// If the handler must skip this event.
            ///
            /// Note that property level handlers don't need to check this, as those handlers are
            /// already not called when this is `true`. [`UiNode`](crate::UiNode) and
            /// [`AppExtension`](crate::app::AppExtension) implementers must check if this is `true`.
            #[inline]
            pub fn stop_propagation_requested(&self) -> bool {
                <Self as $crate::event::EventArgs>::stop_propagation_requested(self)
            }

            /// If the event described by these arguments is relevant in the given widget context.
            #[inline]
            pub fn concerns_widget(&self, ctx: &mut $crate::context::WidgetContext) -> bool {
                <Self as $crate::event::EventArgs>::concerns_widget(self, ctx)
            }
        }
        impl $crate::event::EventArgs for $Args {
            #[inline]
            fn timestamp(&self) -> std::time::Instant {
                self.timestamp
            }

            #[inline]
            $(#[$concerns_widget_outer])*
            fn concerns_widget(&$self, $ctx: &mut $crate::context::WidgetContext) -> bool {
                $($concerns_widget)+
            }

            #[inline]
            fn stop_propagation(&self) {
                self.stop_propagation.store(true, std::sync::atomic::Ordering::Relaxed);
            }

            #[inline]
            fn stop_propagation_requested(&self) -> bool {
                self.stop_propagation.load(std::sync::atomic::Ordering::Relaxed)
            }
        }
    )+};

    // match discard WidgetContext in concerns_widget.
    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, _: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }
        }
    )+) => {
        $crate::event_args! { $(

            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*
                ..
                $(#[$concerns_widget_outer])*
                fn concerns_widget(&$self, _ctx: &mut WidgetContext) -> bool { $($concerns_widget)+ }
            }

        )+ }
    };
}

#[doc(inline)]
pub use crate::event_args;

/// Declares new [`CancelableEventArgs`](crate::event::CancelableEventArgs) types.
///
/// Same syntax as [`event_args!`](macro.event_args.html) but the generated args is also cancelable.
///
/// # Example
///
/// ```
/// # use zero_ui_core::event::cancelable_event_args;
/// # use zero_ui_core::render::WidgetPath;
/// cancelable_event_args! {
///     /// My event arguments.
///     pub struct MyEventArgs {
///         /// My argument.
///         pub arg: String,
///         /// My event target.
///         pub target: WidgetPath,
///
///         ..
///
///         /// If `ctx.path.widget_id()` is in the `self.target` path.
///         fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
///             self.target.contains(ctx.path.widget_id())
///         }
///     }
///
///     // multiple structs can be declared in the same call.
///     // pub struct MyOtherEventArgs { /**/ }
/// }
/// ```
#[macro_export]
macro_rules! cancelable_event_args {

    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, $ctx:ident: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }
        }
    )+) => {$(
        $(#[$outer])*
        #[derive(Debug, Clone)]
        $vis struct $Args {
            /// When the event happened.
            pub timestamp: std::time::Instant,
            $($(#[$arg_outer])* $arg_vis $arg : $arg_ty,)*
            // Arc<AtomicBool> so we don't cause the $Args:!Send and block the user from creating event channels.
            stop_propagation: std::sync::Arc<std::sync::atomic::AtomicBool>,
            cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
        }
        impl $Args {
            /// New args from values that convert [into](Into) the argument types.
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn new(timestamp: impl Into<std::time::Instant>, $($arg : impl Into<$arg_ty>),*) -> Self {
                $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    stop_propagation: std::sync::Arc::default(),
                    cancel: std::sync::Arc::default(),
                }
            }

            /// Arguments for event that happened now (`Instant::now`).
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new(std::time::Instant::now(), $($arg),*)
            }

            /// Requests that subsequent handlers skip this event.
            ///
            /// Cloned arguments signal stop for all clones.
            #[inline]
            pub fn stop_propagation(&self) {
                <Self as $crate::event::EventArgs>::stop_propagation(self)
            }

            /// If the handler must skip this event.
            ///
            /// Note that property level handlers don't need to check this, as those handlers are
            /// already not called when this is `true`. [`UiNode`](crate::UiNode) and
            /// [`AppExtension`](crate::app::AppExtension) implementers must check if this is `true`.
            #[inline]
            pub fn stop_propagation_requested(&self) -> bool {
                <Self as $crate::event::EventArgs>::stop_propagation_requested(self)
            }

            /// Cancel the originating action.
            ///
            /// Cloned arguments signal cancel for all clones.
            #[inline]
            pub fn cancel(&self) {
                <Self as $crate::event::CancelableEventArgs>::cancel(self)
            }

            /// If the originating action must be canceled.
            #[inline]
            pub fn cancel_requested(&self) -> bool {
                <Self as $crate::event::CancelableEventArgs>::cancel_requested(self)
            }
        }
        impl $crate::event::EventArgs for $Args {
            #[inline]
            fn timestamp(&self) -> std::time::Instant {
                self.timestamp
            }

            #[inline]
            $(#[$concerns_widget_outer])*
            fn concerns_widget(&$self, $ctx: &mut $crate::context::WidgetContext) -> bool {
                $($concerns_widget)+
            }

            #[inline]
            fn stop_propagation(&self) {
                self.stop_propagation.store(true, std::sync::atomic::Ordering::Relaxed);
            }

            #[inline]
            fn stop_propagation_requested(&self) -> bool {
                self.stop_propagation.load(std::sync::atomic::Ordering::Relaxed)
            }
        }
        impl $crate::event::CancelableEventArgs for $Args {
            #[inline]
            fn cancel_requested(&self) -> bool {
                self.cancel.load(std::sync::atomic::Ordering::Relaxed)
            }

            #[inline]
            fn cancel(&self) {
                self.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
    )+};

    // match discard WidgetContext in concerns_widget.
    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, _: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }
        }
    )+) => {
        $crate::cancelable_event_args! { $(

            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*
                ..
                $(#[$concerns_widget_outer])*
                fn concerns_widget(&$self, _ctx: &mut WidgetContext) -> bool { $($concerns_widget)+ }
            }

        )+ }
    };
}

#[doc(inline)]
pub use crate::cancelable_event_args;

/// Declares new [`Event`](crate::event::Event) types.
///
/// # Example
///
/// ```
/// # use zero_ui_core::event::event;
/// # use zero_ui_core::gesture::ClickArgs;
/// event! {
///     /// Event docs.
///     pub ClickEvent: ClickArgs;
///
///     /// Other event docs.
///     pub DoubleClickEvent: ClickArgs;
/// }
/// ```
#[macro_export]
macro_rules! event {
    ($(
        $(#[$outer:meta])*
        $vis:vis $Event:ident : $Args:path;
    )+) => {$(
        $(#[$outer])*
        #[derive(Clone, Copy, Debug)]
        $vis struct $Event;
        impl $Event {
            /// Gets the event arguments if the update is for this event.
            #[inline(always)]
            pub fn update<U: $crate::event::EventUpdateArgs>(args: &U) -> Option<&$crate::event::EventUpdate<$Event>> {
                <Self as $crate::event::Event>::update(args)
            }

            /// Schedule an event update.
            #[inline]
            pub fn notify(events: &mut $crate::event::Events, args: $Args) {
                <Self as $crate::event::Event>::notify(events, args);
            }
        }
        impl $crate::event::Event for $Event {
            type Args = $Args;
        }
    )+};
}
#[doc(inline)]
pub use crate::event;

/* Event Property */

#[doc(hidden)]
#[macro_export]
macro_rules! __event_property {
    (
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $Event:path,
            args: $Args:path,
            filter: $filter:expr,
        }
    ) => { $crate::paste! {
        $(#[$on_event_attrs])*
        ///
        /// # Preview
        ///
        #[doc = "You can preview this event using [`on_pre_" $event "`](fn.on_pre_"$event".html)."]
        ///
        /// # Async
        ///
        #[doc = "You can use `await` in a handler by  using [`on_" $event "_async`](fn.on_"$event"_async.html)."]
        #[$crate::property(event, default(|_, _|{}))]
        $vis fn [<on_ $event>](
            child: impl $crate::UiNode,
            handler: impl FnMut(&mut $crate::context::WidgetContext, &$Args) + 'static
        ) -> impl $crate::UiNode {
            $crate::event::on_event(child, $Event, $filter, handler)
        }

        #[doc = "Preview [`on_" $event "`](fn.on_"$event".html) event."]
        ///
        /// # Async
        ///
        #[doc = "You can use `await` in a handler by using [`on_pre_" $event "_async`](fn.on_"$event"_async.html)."]
        ///
        /// # Preview Handlers
        ///
        /// Preview event handlers are called before the main handlers, if you stop the propagation of a preview event
        /// the main event handler is not called. See [`on_pre_event`](zero_ui::core::event::on_pre_event) for more details.
        #[$crate::property(event, default(|_, _|{}))]
        $vis fn [<on_pre_ $event>](
            child: impl $crate::UiNode,
            handler: impl FnMut(&mut $crate::context::WidgetContext, &$Args) + 'static
        ) -> impl $crate::UiNode {
            $crate::event::on_pre_event(child, $Event, $filter, handler)
        }

        #[doc = "Async [`on_" $event "`](fn.on_"$event".html) event."]
        ///
        /// # Async Handlers
        ///
        /// Async event handlers run in the UI thread only, the code before the first `await` runs immediately, subsequent code
        /// runs during updates of the widget they are bound to, if the widget does not update the task does not advance and
        /// if the widget is dropped the task is canceled (dropped).
        ///
        /// The handler tasks are asynchronous but not parallel, when they are doing work they block the UI thread, you can use `Tasks`
        /// to run CPU intensive work in parallel and await for the result in the handler.
        ///
        /// See [`on_event_async`](zero_ui::core::event::on_event_async) for more details.
        #[$crate::property(event, default(|_, _| async {}))]
        $vis fn [<on_ $event _async>]<C, F, H>(
            child: C,
            handler: H
        ) -> impl $crate::UiNode
        where
            C: $crate::UiNode,
            F: std::future::Future<Output=()> + 'static,
            H: FnMut($crate::context::WidgetContextMut, $Args) -> F + 'static
        {
            $crate::event::on_event_async(child, $Event, $filter, handler)
        }

        #[doc = "Async [`on_pre_" $event "`](fn.on_pre_"$event".html) event."]
        ///
        /// # Async Handlers
        ///
        /// Async event handlers run in the UI thread only, the code before the first `await` runs immediately, subsequent code
        /// runs during updates of the widget they are bound to, if the widget does not update the task does not advance and
        /// if the widget is dropped the task is canceled (dropped).
        ///
        /// The handler tasks are asynchronous but not parallel, when they are doing work they block the UI thread, you can use `Tasks`
        /// to run CPU intensive work in parallel and await for the result in the handler.
        ///
        /// See [`on_event_async`](zero_ui::core::event::on_event_async) for more details.
        ///
        /// ## Async Preview
        ///
        /// Because only the code before the first `await` runs immediately, that is where you need to stop propagation if
        /// you want subsequent handlers to not be called for this event. After the first `await` the  code is **not** preview.
        #[$crate::property(event, default(|_, _| async {}))]
        $vis fn [<on_pre_ $event _async>]<C, F, H>(
            child: C,
            handler: H
        ) -> impl $crate::UiNode
        where
            C: $crate::UiNode,
            F: std::future::Future<Output=()> + 'static,
            H: FnMut($crate::context::WidgetContextMut, $Args) -> F + 'static
        {
            $crate::event::on_pre_event_async(child, $Event, $filter, handler)
        }
    } };
    (
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $Event:path,
            args: $Args:path,
        }
    ) => {
        $crate::__event_property! {
            $(#[$on_event_attrs])*
            $vis fn $event {
                event: $Event,
                args: $Args,
                filter: |ctx, args| $crate::event::EventArgs::concerns_widget(args, ctx),
            }
        }
    };
}
/// Declare one or more event properties.
///
/// Each declaration expands to four properties `on_$event`, `on_pre_$event`, `on_$event_async` and `on_pre_$event_async`.
/// The preview properties call [`on_pre_event`](crate::event::on_pre_event) or [`on_pre_event_async`](crate::event::on_pre_event_async),
/// the main event properties call [`on_event`](crate::event::on_event) or [`on_event_async`](crate::event::on_event_async).
///
/// # Example
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::event::{event_property, EventArgs};
/// # use zero_ui_core::keyboard::*;
/// event_property! {
///     /// on_key_down docs.
///     pub fn key_down {
///         event: KeyDownEvent,
///         args: KeyInputArgs,
///         // default filter is |ctx, args| args.concerns_widget(ctx)
///     }
///
///     pub(crate) fn space_down {
///         event: KeyDownEvent,
///         args: KeyInputArgs,
///         // optional filter:
///         filter: |ctx, args| args.concerns_widget(ctx) && args.key == Some(Key::Space),
///     }
/// }
/// ```
///
/// # Filter
///
/// App events can be listened from any `UiNode`. An event property must call the event handler only
/// in contexts where the event is relevant. Some event properties can also specialize further on top
/// of a more general app event. To implement this you can use a filter predicate.
///
/// The `filter` predicate is called if [`stop_propagation`](EventArgs::stop_propagation) is not requested and the
/// widget is [enabled](IsEnabled). It must return `true` if the event arguments are relevant in the context of the
/// widget. If it returns `true` the `handler` closure is called. See [`on_event`] and [`on_pre_event`] for more information.
///
/// If you don't provide a filter predicate the default [`args.concerns_widget(ctx)`](EventArgs::concerns_widget) is used.
/// So if you want to extend the filter and not fully replace it you must call `args.concerns_widget(ctx)` in your custom filter.
///
/// # Async
///
/// The async properties create [`UiTask`](crate::task::UiTask) future executors for every call of the handler, at a time there
/// can be more then one *handler future* executing asynchrony. The arguments are [`WidgetContextMut`](crate::context::WidgetContextMut)
/// and a clone of the event args.
#[macro_export]
macro_rules! event_property {
    ($(
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $Event:path,
            args: $Args:path $(,
            filter: $filter:expr)? $(,)?
        }
    )+) => {$(
        $crate::__event_property! {
            $(#[$on_event_attrs])*
            $vis fn $event {
                event: $Event,
                args: $Args,
                $(filter: $filter,)?
            }
        }
    )+};
}
#[doc(inline)]
pub use crate::event_property;

/// Helper for declaring event properties.
///
/// This function is used by the [`event_property!`] macro.
///
/// # Filter
///
/// The `filter` predicate is called if [`stop_propagation`](EventArgs::stop_propagation) is not requested and the
/// widget is [enabled](IsEnabled). It must return `true` if the event arguments are relevant in the context of the
/// widget. If it returns `true` the `handler` closure is called.
///
/// # Route
///
/// The event `handler` is called after the [`on_pre_event`] equivalent at the same context level. If the event
/// `filter` allows more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
#[inline]
pub fn on_event<C, E, F, H>(child: C, _event: E, filter: F, handler: H) -> impl UiNode
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    H: FnMut(&mut WidgetContext, &E::Args) + 'static,
{
    struct OnEventNode<C, E, F, H> {
        child: C,
        _event: PhantomData<E>,
        filter: F,
        handler: H,
    }
    #[impl_ui_node(child)]
    impl<C, E, F, H> UiNode for OnEventNode<C, E, F, H>
    where
        C: UiNode,
        E: Event,
        F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
        H: FnMut(&mut WidgetContext, &E::Args) + 'static,
    {
        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
            if let Some(args) = E::update(args) {
                self.child.event(ctx, args);

                if IsEnabled::get(ctx.vars) && !args.stop_propagation_requested() && (self.filter)(ctx, args) {
                    (self.handler)(ctx, args);
                }
            } else {
                self.child.event(ctx, args);
            }
        }
    }
    OnEventNode {
        child,
        _event: PhantomData::<E>,
        filter,
        handler,
    }
}

/// Helper for declaring preview event properties.
///
/// This function is used by the [`event_property!`] macro.
///
/// # Filter
///
/// The `filter` predicate is called if [`stop_propagation`](EventArgs::stop_propagation) is not requested and the
/// widget is [enabled](IsEnabled). It must return `true` if the event arguments are relevant in the context of the
/// widget. If it returns `true` the `handler` closure is called.
///
/// # Route
///
/// The event `handler` is called before the [`on_event`] equivalent at the same context level. If the event
/// `filter` allows more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
pub fn on_pre_event<C, E, F, H>(child: C, _event: E, filter: F, handler: H) -> impl UiNode
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    H: FnMut(&mut WidgetContext, &E::Args) + 'static,
{
    struct OnPreviewEventNode<C, E, F, H> {
        child: C,
        _event: PhantomData<E>,
        filter: F,
        handler: H,
    }
    #[impl_ui_node(child)]
    impl<C, E, F, H> UiNode for OnPreviewEventNode<C, E, F, H>
    where
        C: UiNode,
        E: Event,
        F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
        H: FnMut(&mut WidgetContext, &E::Args) + 'static,
    {
        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
            if let Some(args) = E::update(args) {
                if IsEnabled::get(ctx.vars) && !args.stop_propagation_requested() && (self.filter)(ctx, args) {
                    (self.handler)(ctx, args);
                }
                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }
        }
    }
    OnPreviewEventNode {
        child,
        _event: PhantomData::<E>,
        filter,
        handler,
    }
}

/// Helper for declaring async event properties.
///
/// This function is used by the [`event_property!`] macro.
///
/// # Filter
///
/// The `filter` predicate is called if [`stop_propagation`](EventArgs::stop_propagation) is not requested and the
/// widget is [enabled](IsEnabled). It must return `true` if the event arguments are relevant in the context of the
/// widget. If it returns `true` the `handler` closure is called and starts executing the future.
///
/// # Route
///
/// The event `handler` is called after the [`on_pre_event`] equivalent at the same context level. If the event
/// `filter` allows more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
///
/// Code before the first `await` point is executed immediately, code after `await` points are executed in subsequent updates.
pub fn on_event_async<C, E, F, R, H>(child: C, _event: E, filter: F, handler: H) -> impl UiNode
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    R: Future<Output = ()> + 'static,
    H: FnMut(WidgetContextMut, E::Args) -> R + 'static,
{
    struct OnEventAsyncNode<C, E, F, H> {
        child: C,
        _event: PhantomData<E>,
        filter: F,
        handler: H,
        tasks: Vec<WidgetTask<()>>,
    }
    #[impl_ui_node(child)]
    impl<C, E, F, R, H> UiNode for OnEventAsyncNode<C, E, F, H>
    where
        C: UiNode,
        E: Event,
        F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
        R: Future<Output = ()> + 'static,
        H: FnMut(WidgetContextMut, E::Args) -> R + 'static,
    {
        fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            if let Some(args) = E::update(args) {
                self.child.event(ctx, args);

                if IsEnabled::get(ctx.vars) && !args.stop_propagation_requested() && (self.filter)(ctx, args) {
                    let mut task = ctx.async_task(|ctx| (self.handler)(ctx, args.clone()));
                    if task.update(ctx).is_none() {
                        self.tasks.push(task);
                    }
                }
            } else {
                self.child.event(ctx, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            self.tasks.retain_mut(|t| t.update(ctx).is_none());
        }
    }
    OnEventAsyncNode {
        child,
        _event: PhantomData::<E>,
        filter,
        handler,
        tasks: Vec::with_capacity(1),
    }
}

/// Helper for declaring async preview event properties.
///
/// This function is used by the [`event_property!`] macro.
///
/// # Filter
///
/// The `filter` predicate is called if [`stop_propagation`](EventArgs::stop_propagation) is not requested and the
/// widget is [enabled](IsEnabled). It must return `true` if the event arguments are relevant in the context of the
/// widget. If it returns `true` the `handler` closure is called and starts executing the future.
///
/// # Route
///
/// The event `handler` is called before the [`on_event`] equivalent at the same context level. If the event
/// `filter` allows more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
///
/// Code before the first `await` point is executed immediately, code after `await` points are executed in subsequent updates.
pub fn on_pre_event_async<C, E, F, R, H>(child: C, _event: E, filter: F, handler: H) -> impl UiNode
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    R: Future<Output = ()> + 'static,
    H: FnMut(WidgetContextMut, E::Args) -> R + 'static,
{
    struct OnPreviewEventAsyncNode<C, E, F, H> {
        child: C,
        _event: PhantomData<E>,
        filter: F,
        handler: H,
        tasks: Vec<WidgetTask<()>>,
    }
    #[impl_ui_node(child)]
    impl<C, E, F, R, H> UiNode for OnPreviewEventAsyncNode<C, E, F, H>
    where
        C: UiNode,
        E: Event,
        F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
        R: Future<Output = ()> + 'static,
        H: FnMut(WidgetContextMut, E::Args) -> R + 'static,
    {
        fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            if let Some(args) = E::update(args) {
                if IsEnabled::get(ctx.vars) && !args.stop_propagation_requested() && (self.filter)(ctx, args) {
                    let mut task = ctx.async_task(|ctx| (self.handler)(ctx, args.clone()));
                    if task.update(ctx).is_none() {
                        self.tasks.push(task);
                    }
                }

                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.tasks.retain_mut(|t| t.update(ctx).is_none());

            self.child.update(ctx);
        }
    }

    OnPreviewEventAsyncNode {
        child,
        _event: PhantomData::<E>,
        filter,
        handler,
        tasks: Vec::with_capacity(1),
    }
}
