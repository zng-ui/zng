//! App event API.

#[allow(unused_imports)] // nightly
use retain_mut::RetainMut;
use unsafe_any::UnsafeAny;

use crate::app::{AppEventSender, AppShutdown, RecvFut, TimeoutOrAppShutdown};
use crate::command::AnyCommand;
use crate::context::{AppContext, InfoContext, UpdatesTrace, WidgetContext};
use crate::crate_util::{Handle, HandleOwner, WeakHandle};
use crate::handler::{AppHandler, AppHandlerArgs, AppWeakHandle, WidgetHandler};
use crate::var::Vars;
use crate::widget_info::{EventSlot, WidgetInfoBuilder, WidgetSubscriptions};
use crate::{impl_ui_node, UiNode};
use std::cell::RefCell;
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;
use std::rc::Rc;
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
    /// already not called when this is `true`. Direct event listeners in [`UiNode`] and [`AppExtension`]
    /// must check if this is `true`.
    ///
    /// [`UiNode`]: crate::UiNode
    /// [`AppExtension`]: crate::app::AppExtension
    fn stop_propagation_requested(&self) -> bool;

    /// Calls `handler` and stops propagation if propagation is still allowed.
    ///
    /// Returns the `handler` result if it was called.
    fn handle<F, R>(&self, handler: F) -> Option<R>
    where
        F: FnOnce(&Self) -> R,
    {
        if self.stop_propagation_requested() {
            None
        } else {
            let r = handler(self);
            self.stop_propagation();
            Some(r)
        }
    }
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
#[cfg_attr(doc_nightly, doc(notable_trait))]
pub trait Event: Debug + Clone + Copy + 'static {
    /// Event arguments type.
    type Args: EventArgs;

    /// Schedule an event update.
    #[inline(always)]
    fn notify<Evs: WithEvents>(self, events: &mut Evs, args: Self::Args) {
        events.with_events(|events| events.notify::<Self>(self, args));
    }

    /// Gets the event arguments if the update is for `Self`.
    #[inline(always)]
    fn update<U: EventUpdateArgs>(self, args: &U) -> Option<&EventUpdate<Self>> {
        args.args_for::<Self>()
    }

    /// Gets the [`EventSlot`] assigned to this event type.
    fn slot(self) -> EventSlot;
}

/// [`EventUpdateArgs`] for event `E`, dereferences to the argument.
pub struct EventUpdate<E: Event> {
    args: E::Args,
    slot: EventSlot,
}
impl<E: Event> EventUpdate<E> {
    /// New event update.
    pub fn new(event: E, args: E::Args) -> Self {
        EventUpdate { args, slot: event.slot() }
    }

    /// Clone the arguments.
    #[allow(clippy::should_implement_trait)] // that is what we want.
    pub fn clone(&self) -> E::Args {
        self.args.clone()
    }

    pub(crate) fn boxed(self) -> BoxedEventUpdate {
        BoxedEventUpdate {
            event_type: TypeId::of::<E>(),
            slot: self.slot,
            args: Box::new(self),
            debug_fmt: debug_fmt::<E>,
            debug_fmt_any: debug_fmt_any::<E>,
        }
    }

    fn boxed_send(self) -> BoxedSendEventUpdate
    where
        E::Args: Send,
    {
        BoxedSendEventUpdate {
            event_type: TypeId::of::<E>(),
            slot: self.slot,
            args: Box::new(self),
            debug_fmt: debug_fmt::<E>,
            debug_fmt_any: debug_fmt_any::<E>,
        }
    }

    /// Change the event type if the event args type is the same
    pub(crate) fn transmute_event<E2: Event<Args = E::Args>>(&self) -> &EventUpdate<E2> {
        // SAFETY: this is a change on the type system only, the data layout is the same.
        unsafe { mem::transmute(self) }
    }
}
impl<E: Event> crate::private::Sealed for EventUpdate<E> {}
impl<E: Event> fmt::Debug for EventUpdate<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "EventUpdate<{}>({:#?})", type_name::<E>(), self.args)
        } else {
            write!(f, "EventUpdate<{}>({:?})", type_name::<E>(), self.args)
        }
    }
}
impl<E: Event> Deref for EventUpdate<E> {
    type Target = E::Args;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.args
    }
}

/// Construct with [`debug_fmt`].
type DebugFmtFn = unsafe fn(&dyn UnsafeAny, &mut fmt::Formatter) -> fmt::Result;
unsafe fn debug_fmt<E: Event>(args: &dyn UnsafeAny, f: &mut fmt::Formatter) -> fmt::Result {
    let args = args.downcast_ref_unchecked::<E::Args>();
    write!(f, "{}\n{:?}", type_name::<E>(), args)
}

/// Boxed [`EventUpdateArgs`].
pub struct BoxedEventUpdate {
    event_type: TypeId,
    slot: EventSlot,
    args: Box<dyn UnsafeAny>,
    debug_fmt: DebugFmtFn,
    debug_fmt_any: DebugFmtAnyFn,
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
impl crate::private::Sealed for BoxedEventUpdate {}
impl fmt::Debug for BoxedEventUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "boxed ")?;
        // SAFETY: we trust w e build the type correctly.
        unsafe { (self.debug_fmt)(&*self.args, f) }
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
            event_type_id: self.event_type,
            debug_fmt: self.debug_fmt_any,
            slot: self.slot,
            event_update_args: unsafe {
                // SAFETY: no different then the EventUpdate::as_any()
                self.args.downcast_ref_unchecked()
            },
        }
    }

    fn slot(&self) -> EventSlot {
        self.slot
    }
}

/// A [`BoxedEventUpdate`] that is [`Send`].
pub struct BoxedSendEventUpdate {
    event_type: TypeId,
    slot: EventSlot,
    args: Box<dyn UnsafeAny + Send>,
    debug_fmt: DebugFmtFn,
    debug_fmt_any: DebugFmtAnyFn,
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
            slot: self.slot,
            args: self.args,
            debug_fmt: self.debug_fmt,
            debug_fmt_any: self.debug_fmt_any,
        }
    }
}
impl crate::private::Sealed for BoxedSendEventUpdate {}
impl fmt::Debug for BoxedSendEventUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "boxed send")?;
        // SAFETY: we trust w e build the type correctly.
        unsafe { (self.debug_fmt)(&*self.args, f) }
    }
}

type DebugFmtAnyFn = unsafe fn(&(), &mut fmt::Formatter) -> fmt::Result;

unsafe fn debug_fmt_any<E: Event>(args: &(), f: &mut fmt::Formatter) -> fmt::Result {
    let args: &EventUpdate<E> = mem::transmute(args);
    write!(f, "{}\n{:?}", type_name::<E>(), args)
}

/// Type erased [`EventUpdateArgs`].
pub struct AnyEventUpdate<'a> {
    event_type_id: TypeId,
    slot: EventSlot,
    // this is a reference to a `EventUpdate<Q>`.
    event_update_args: &'a (),
    debug_fmt: DebugFmtAnyFn,
}
impl<'a> AnyEventUpdate<'a> {
    /// Gets the [`TypeId`] of the event type represented by `self`.
    #[inline]
    pub fn event_type_id(&self) -> TypeId {
        self.event_type_id
    }
}
impl<'a> fmt::Debug for AnyEventUpdate<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "any")?;
        // SAFETY: we trust w e build the type correctly.
        unsafe { (self.debug_fmt)(self.event_update_args, f) }
    }
}
impl<'a> crate::private::Sealed for AnyEventUpdate<'a> {}
impl<'a> EventUpdateArgs for AnyEventUpdate<'a> {
    #[inline(always)]
    fn args_for<Q: Event>(&self) -> Option<&EventUpdate<Q>> {
        if self.event_type_id == TypeId::of::<Q>() {
            Some(unsafe {
                // SAFETY: its the same type.
                #[allow(clippy::transmute_ptr_to_ptr)]
                mem::transmute(self.event_update_args)
            })
        } else {
            None
        }
    }

    #[inline(always)]
    fn as_any(&self) -> AnyEventUpdate {
        AnyEventUpdate {
            event_type_id: self.event_type_id,
            slot: self.slot,
            event_update_args: self.event_update_args,
            debug_fmt: self.debug_fmt,
        }
    }

    fn slot(&self) -> EventSlot {
        self.slot
    }
}

/// Represents an event update.
pub trait EventUpdateArgs: fmt::Debug + crate::private::Sealed {
    /// Gets the the update arguments if the event updating is `Q`.
    fn args_for<Q: Event>(&self) -> Option<&EventUpdate<Q>>;

    /// Type erased event update.
    fn as_any(&self) -> AnyEventUpdate;

    /// Returns the [`EventSlot`] that represents the event type.
    fn slot(&self) -> EventSlot;
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
            event_type_id: TypeId::of::<E>(),
            debug_fmt: debug_fmt_any::<E>,
            slot: self.slot,
            event_update_args: unsafe {
                // SAFETY: nothing will be done with it other then a validated restore in `args_for`.
                #[allow(clippy::transmute_ptr_to_ptr)]
                mem::transmute(self)
            },
        }
    }

    fn slot(&self) -> EventSlot {
        self.slot
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
    slot: EventSlot,
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
            slot: self.slot,
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
        UpdatesTrace::log_event::<E>();

        let update = EventUpdate::<E> { args, slot: self.slot }.boxed_send();
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
    handle: HandleOwner<()>,
    handler: Box<dyn FnMut(&mut AppContext, &BoxedEventUpdate, &dyn AppWeakHandle)>,
}

/// Represents an app context event handler created by [`Events::on_event`] or [`Events::on_pre_event`].
///
/// Drop all clones of this handle to drop the handler, or call [`unsubscribe`](Self::unsubscribe) to drop the handle
/// without dropping the handler.
#[derive(Clone, PartialEq, Eq, Hash)]
#[must_use = "the event handler unsubscribes if the handle is dropped"]
pub struct OnEventHandle(Handle<()>);
impl OnEventHandle {
    fn new() -> (HandleOwner<()>, OnEventHandle) {
        let (owner, handle) = Handle::new(());
        (owner, OnEventHandle(handle))
    }

    /// Create a handle to nothing, the handle always in the *unsubscribed* state.
    #[inline]
    pub fn dummy() -> Self {
        OnEventHandle(Handle::dummy(()))
    }

    /// Drop the handle but does **not** unsubscribe.
    ///
    /// The handler stays in memory for the duration of the app or until another handle calls [`unsubscribe`](Self::unsubscribe.)
    #[inline]
    pub fn permanent(self) {
        self.0.permanent();
    }

    /// If another handle has called [`permanent`](Self::permanent).
    /// If `true` the var binding will stay active until the app shutdown, unless [`unsubscribe`](Self::unsubscribe) is called.
    #[inline]
    pub fn is_permanent(&self) -> bool {
        self.0.is_permanent()
    }

    /// Drops the handle and forces the handler to drop.
    #[inline]
    pub fn unsubscribe(self) {
        self.0.force_drop()
    }

    /// If another handle has called [`unsubscribe`](Self::unsubscribe).
    ///
    /// The handler is already dropped or will be dropped in the next app update, this is irreversible.
    #[inline]
    pub fn is_unsubscribed(&self) -> bool {
        self.0.is_dropped()
    }

    /// Create a weak handle.
    #[inline]
    pub fn downgrade(&self) -> WeakOnEventHandle {
        WeakOnEventHandle(self.0.downgrade())
    }
}

/// Weak [`OnEventHandle`].
#[derive(Clone)]
pub struct WeakOnEventHandle(WeakHandle<()>);
impl WeakOnEventHandle {
    /// Gets the strong handle if it is still subscribed.
    pub fn upgrade(&self) -> Option<OnEventHandle> {
        self.0.upgrade().map(OnEventHandle)
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

    commands: Vec<AnyCommand>,

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
            commands: vec![],
            _singleton: SingletonEvents::assert_new("Events"),
        }
    }

    /// Called by [`Event::notify`] to schedule a notification.
    pub fn notify<E: Event>(&mut self, event: E, args: E::Args) {
        UpdatesTrace::log_event::<E>();
        let update = EventUpdate::<E>::new(event, args);
        self.updates.push(update.boxed());
    }

    pub(crate) fn notify_app_event(&mut self, update: BoxedSendEventUpdate) {
        self.updates.push(update.forget_send());
    }

    pub(crate) fn register_command(&mut self, command: AnyCommand) {
        if self
            .commands
            .iter()
            .any(|c| c.command_type_id() == command.command_type_id() && c.scope() == command.scope())
        {
            panic!("command `{command:?}` is already registered")
        }
        self.commands.push(command);
    }

    /// Creates an event buffer that listens to `E`. The event updates are pushed as soon as possible, before
    /// the UI and [`on_event`](Self::on_event) are notified.
    ///
    /// Drop the buffer to stop listening.
    pub fn pre_buffer<E: Event>(&mut self, event: E) -> EventBuffer<E> {
        Self::push_buffer::<E>(&mut self.pre_buffers, event)
    }

    /// Creates an event buffer that listens to `E`. The event updates are pushed only after
    /// the UI and [`on_event`](Self::on_event) are notified.
    ///
    /// Drop the buffer to stop listening.
    pub fn buffer<E: Event>(&mut self, event: E) -> EventBuffer<E> {
        Self::push_buffer::<E>(&mut self.buffers, event)
    }

    fn push_buffer<E: Event>(buffers: &mut Vec<BufferEntry>, event: E) -> EventBuffer<E> {
        let buf = EventBuffer::never();
        let weak = Rc::downgrade(&buf.queue);
        buffers.push(Box::new(move |args| {
            let mut retain = false;
            if let Some(rc) = weak.upgrade() {
                if let Some(args) = event.update(args) {
                    rc.borrow_mut().push_back(args.clone());
                }
                retain = true;
            }
            retain
        }));
        buf
    }

    /// Creates a sender that can raise an event from other threads and without access to [`Events`].
    pub fn sender<E>(&mut self, event: E) -> EventSender<E>
    where
        E: Event,
        E::Args: Send,
    {
        EventSender {
            sender: self.app_event_sender.clone(),
            slot: event.slot(),
            _event: PhantomData,
        }
    }

    /// Creates a channel that can listen to event from another thread. The event updates are sent as soon as possible, before
    /// the UI and [`on_event`](Self::on_event) are notified.
    ///
    /// Drop the receiver to stop listening.
    pub fn pre_receiver<E>(&mut self, event: E) -> EventReceiver<E>
    where
        E: Event,
        E::Args: Send,
    {
        Self::push_receiver::<E>(&mut self.pre_buffers, event)
    }

    /// Creates a channel that can listen to event from another thread. The event updates are sent only after the
    /// UI and [`on_event`](Self::on_event) are notified.
    ///
    /// Drop the receiver to stop listening.
    pub fn receiver<E>(&mut self, event: E) -> EventReceiver<E>
    where
        E: Event,
        E::Args: Send,
    {
        Self::push_receiver::<E>(&mut self.buffers, event)
    }

    fn push_receiver<E>(buffers: &mut Vec<BufferEntry>, event: E) -> EventReceiver<E>
    where
        E: Event,
        E::Args: Send,
    {
        let (sender, receiver) = flume::unbounded();

        buffers.push(Box::new(move |e| {
            let mut retain = true;
            if let Some(args) = event.update(e) {
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
    /// [`unsubscribe`](crate::handler::AppWeakHandle::unsubscribe) in the third parameter of [`app_hn!`] or [`async_app_hn!`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use zero_ui_core::event::*;
    /// # use zero_ui_core::handler::app_hn;
    /// # use zero_ui_core::focus::{FocusChangedEvent, FocusChangedArgs};
    /// # fn example(ctx: &mut zero_ui_core::context::AppContext) {
    /// let handle = ctx.events.on_pre_event(FocusChangedEvent, app_hn!(|_ctx, args: &FocusChangedArgs, _| {
    ///     println!("focused: {:?}", args.new_focus);
    /// }));
    /// # }
    /// ```
    /// The example listens to all `FocusChangedEvent` events, independent of widget context and before all UI handlers.
    ///
    /// # Handlers
    ///
    /// the event handler can be any type that implements [`AppHandler`], there are multiple flavors of handlers, including
    /// async handlers that allow calling `.await`. The handler closures can be declared using [`app_hn!`], [`async_app_hn!`],
    /// [`app_hn_once!`] and [`async_app_hn_once!`].
    ///
    /// ## Async
    ///
    /// Note that for async handlers only the code before the first `.await` is called in the *preview* moment, code after runs in
    /// subsequent event updates, after the event has already propagated, so calling [`stop_propagation`](EventArgs::stop_propagation)
    /// only causes the desired effect before the first `.await`.
    ///
    /// [`app_hn!`]: crate::handler::app_hn!
    /// [`async_app_hn!`]: crate::handler::async_app_hn!
    /// [`app_hn_once!`]: crate::handler::app_hn_once!
    /// [`async_app_hn!`]: crate::handler::async_app_hn_once!
    pub fn on_pre_event<E, H>(&mut self, event: E, handler: H) -> OnEventHandle
    where
        E: Event,
        H: AppHandler<E::Args>,
    {
        Self::push_event_handler(&mut self.pre_handlers, event, true, handler)
    }

    /// Creates an event handler.
    ///
    /// The event `handler` is called for every update of `E` that are not marked [`stop_propagation`](EventArgs::stop_propagation).
    /// The handler is called after all [`on_pre_event`],(Self::on_pre_event) all UI handlers and all [`on_event`](Self::on_event) handlers
    /// registered before this one.
    ///
    /// Returns a [`OnEventHandle`] that can be used to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`unsubscribe`](crate::handler::AppWeakHandle::unsubscribe) in the third parameter of [`app_hn!`] or [`async_app_hn!`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use zero_ui_core::event::*;
    /// # use zero_ui_core::handler::app_hn;
    /// # use zero_ui_core::focus::{FocusChangedEvent, FocusChangedArgs};
    /// # fn example(ctx: &mut zero_ui_core::context::AppContext) {
    /// let handle = ctx.events.on_event(FocusChangedEvent, app_hn!(|_ctx, args: &FocusChangedArgs, _| {
    ///     println!("focused: {:?}", args.new_focus);
    /// }));
    /// # }
    /// ```
    /// The example listens to all `FocusChangedEvent` events, independent of widget context, after the UI was notified.
    ///
    /// # Handlers
    ///
    /// the event handler can be any type that implements [`AppHandler`], there are multiple flavors of handlers, including
    /// async handlers that allow calling `.await`. The handler closures can be declared using [`app_hn!`], [`async_app_hn!`],
    /// [`app_hn_once!`] and [`async_app_hn_once!`].
    ///
    /// ## Async
    ///
    /// Note that for async handlers only the code before the first `.await` is called in the *preview* moment, code after runs in
    /// subsequent event updates, after the event has already propagated, so calling [`stop_propagation`](EventArgs::stop_propagation)
    /// only causes the desired effect before the first `.await`.
    ///
    /// [`app_hn!`]: crate::handler::app_hn!
    /// [`async_app_hn!`]: crate::handler::async_app_hn!
    /// [`app_hn_once!`]: crate::handler::app_hn_once!
    /// [`async_app_hn!`]: crate::handler::async_app_hn_once!
    pub fn on_event<E, H>(&mut self, event: E, handler: H) -> OnEventHandle
    where
        E: Event,
        H: AppHandler<E::Args>,
    {
        Self::push_event_handler(&mut self.pos_handlers, event, false, handler)
    }

    fn push_event_handler<E, H>(handlers: &mut Vec<OnEventHandler>, event: E, is_preview: bool, mut handler: H) -> OnEventHandle
    where
        E: Event,
        H: AppHandler<E::Args>,
    {
        let (handle_owner, handle) = OnEventHandle::new();
        let handler = move |ctx: &mut AppContext, args: &BoxedEventUpdate, handle: &dyn AppWeakHandle| {
            if let Some(args) = event.update(args) {
                if !args.stop_propagation_requested() {
                    handler.event(ctx, args, &AppHandlerArgs { handle, is_preview });
                }
            }
        };
        handlers.push(OnEventHandler {
            handle: handle_owner,
            handler: Box::new(handler),
        });
        handle
    }

    pub(crate) fn has_pending_updates(&mut self) -> bool {
        !self.updates.is_empty()
    }

    #[must_use]
    pub(super) fn apply_updates(&mut self, vars: &Vars) -> Vec<BoxedEventUpdate> {
        for command in &self.commands {
            command.update_state(vars);
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
            !e.handle.is_dropped() && {
                (e.handler)(ctx, args, &e.handle.weak_handle());
                !e.handle.is_dropped()
            }
        });
    }

    /// Commands that had handles generated in this app.
    ///
    /// When [`Command::new_handle`] is called for the first time in an app, the command gets registered here.
    ///
    /// [`Command::new_handle`]: crate::command::Command::new_handle
    #[inline]
    pub fn commands(&self) -> impl Iterator<Item = AnyCommand> + '_ {
        self.commands.iter().copied()
    }
}

/// Represents a type that can provide access to [`Events`] inside the window of function call.
///
/// This is used to make event notification less cumbersome to use, it is implemented to all sync and async context types
/// and [`Events`] it-self.
///
/// # Examples
///
/// The example demonstrate how this `trait` simplifies calls to [`Event::notify`].
///
/// ```
/// # use zero_ui_core::{var::*, event::*, context::*};
/// # event_args! { pub struct BarArgs { pub msg: &'static str, .. fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool { true } } }
/// # event! { pub BarEvent: BarArgs; }
/// # struct Foo { } impl Foo {
/// fn update(&mut self, ctx: &mut WidgetContext) {
///     BarEvent.notify(ctx, BarArgs::now("we are not borrowing `ctx` so can use it directly"));
///
///    // ..
///    let services = &mut ctx.services;
///    BarEvent.notify(ctx, BarArgs::now("we are partially borrowing `ctx` but not `ctx.vars` so we use that"));
/// }
///
/// async fn handler(&mut self, mut ctx: WidgetContextMut) {
///     BarEvent.notify(&mut ctx, BarArgs::now("async contexts can also be used"));
/// }
/// # }
/// ```
pub trait WithEvents {
    /// Calls `action` with the [`Events`] reference.
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R;
}
impl WithEvents for Events {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        action(self)
    }
}
impl<'a> WithEvents for crate::context::AppContext<'a> {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        action(self.events)
    }
}
impl<'a> WithEvents for crate::context::WindowContext<'a> {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        action(self.events)
    }
}
impl<'a> WithEvents for crate::context::WidgetContext<'a> {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        action(self.events)
    }
}
impl WithEvents for crate::context::AppContextMut {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        self.with(move |ctx| action(ctx.events))
    }
}
impl WithEvents for crate::context::WidgetContextMut {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        self.with(move |ctx| action(ctx.events))
    }
}
#[cfg(any(test, doc, feature = "test_util"))]
impl WithEvents for crate::context::TestWidgetContext {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        action(&mut self.events)
    }
}
impl WithEvents for crate::app::HeadlessApp {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        action(self.ctx().events)
    }
}

type Retain = bool;

///<span data-del-macro-root></span> Declares new [`EventArgs`] types.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::{event::event_args, WidgetPath, text::{Text, formatx}};
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
///
///         /// Optional validation, if defined the generated `new` and `now` functions call it and unwrap the result.
///         ///
///         /// The error type can be any type that implement `Debug`.
///         fn validate(&self) -> Result<(), Text> {
///             if self.arg.contains("error") {
///                 return Err(formatx!("invalid arg `{}`", self.arg));
///             }
///             Ok(())
///         }
///     }
///
///     // multiple structs can be declared in the same call.
///     // pub struct MyOtherEventArgs { /**/ }
/// }
/// ```
///
/// [`EventArgs`]: crate::event::EventArgs
#[macro_export]
macro_rules! event_args {
    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, $ctx:tt: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }

            $(
                $(#[$validate_outer:meta])*
                fn validate(&$self_v:ident) -> Result<(), $ValidationError:path> { $($validate:tt)+ }
            )?
        }
    )+) => {$(
        $crate::__event_args! {
            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*

                ..

                $(#[$concerns_widget_outer])*
                fn concerns_widget(&$self, $ctx: &mut WidgetContext) -> bool { $($concerns_widget)+ }

                $(
                    $(#[$validate_outer])*
                    fn validate(&$self_v) -> Result<(), $ValidationError> { $($validate)+ }
                )?
            }
        }
    )+};
}
#[doc(hidden)]
#[macro_export]
macro_rules! __event_args {
    // match validate
    (
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, $ctx:tt: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }

            $(#[$validate_outer:meta])*
            fn validate(&$self_v:ident) -> Result<(), $ValidationError:path> { $($validate:tt)+ }
        }
    ) => {
        $crate::__event_args! {common=>

            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*
                ..
                $(#[$concerns_widget_outer])*
                fn concerns_widget(&$self, $ctx: &mut WidgetContext) -> bool { $($concerns_widget)+ }
            }
        }
        impl $Args {
            /// New args from values that convert [into](Into) the argument types.
            ///
            /// # Panics
            ///
            /// Panics if the arguments are invalid.
            #[inline]
            #[track_caller]
            #[allow(clippy::too_many_arguments)]
            pub fn new(timestamp: impl Into<std::time::Instant>, $($arg : impl Into<$arg_ty>),*) -> Self {
                let args = $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    stop_propagation: std::sync::Arc::default(),
                };
                args.assert_valid();
                args
            }

            /// New args from values that convert [into](Into) the argument types.
            ///
            /// Returns an error if the constructed arguments are invalid.
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn try_new(timestamp: impl Into<std::time::Instant>, $($arg : impl Into<$arg_ty>),*) -> Result<Self, $ValidationError> {
                let args = $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    stop_propagation: std::sync::Arc::default(),
                };
                args.validate()?;
                Ok(args)
            }

            /// Arguments for event that happened now (`Instant::now`).
            ///
            /// # Panics
            ///
            /// Panics if the arguments are invalid.
            #[inline]
            #[track_caller]
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new(std::time::Instant::now(), $($arg),*)
            }

            /// Arguments for event that happened now (`Instant::now`).
            ///
            /// Returns an error if the constructed arguments are invalid.
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn try_now($($arg : impl Into<$arg_ty>),*) -> Result<Self, $ValidationError> {
                Self::try_new(std::time::Instant::now(), $($arg),*)
            }

            $(#[$validate_outer])*
            pub fn validate(&$self_v) -> Result<(), $ValidationError> {
                $($validate)+
            }

            /// Panics if the arguments are invalid.
            #[track_caller]
            pub fn assert_valid(&self) {
                if let Err(e) = self.validate() {
                    panic!("invalid `{}`, {e:?}", stringify!($Args));
                }
            }
        }
    };

    // match no validate
    (
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, $ctx:tt: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }
        }
    ) => {
        $crate::__event_args! {common=>

            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*
                ..
                $(#[$concerns_widget_outer])*
                fn concerns_widget(&$self, $ctx: &mut WidgetContext) -> bool { $($concerns_widget)+ }
            }
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
            ///
            /// # Panics
            ///
            /// Panics if the arguments are invalid.
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new(std::time::Instant::now(), $($arg),*)
            }
        }
    };

    // common code between validating and not.
    (common=>

        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, $ctx:tt: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }
        }
    ) => {
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

            /// Calls `handler` and stops propagation if propagation is still allowed.
            ///
            /// Returns the `handler` result if it was called.
            #[inline]
            pub fn handle<F, R>(&self, handler: F) -> Option<R>
            where
                F: FnOnce(&Self) -> R,
            {
                <Self as $crate::event::EventArgs>::handle(self, handler)
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
    };
}
#[doc(inline)]
pub use crate::event_args;

///<span data-del-macro-root></span> Declares new [`CancelableEventArgs`] types.
///
/// Same syntax as [`event_args!`](macro.event_args.html) but the generated args is also cancelable.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::{event::cancelable_event_args, WidgetPath, text::*};
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
///
///         /// Optional validation, if defined the generated `new` and `now` functions call it and unwrap the result.
///         ///
///         /// The error type can be any type that implement `Debug`.
///         fn validate(&self) -> Result<(), Text> {
///             if self.arg.contains("error") {
///                 return Err(formatx!("invalid arg `{}`", self.arg));
///             }
///             Ok(())
///         }
///     }
///
///     // multiple structs can be declared in the same call.
///     // pub struct MyOtherEventArgs { /**/ }
/// }
/// ```
///
/// [`CancelableEventArgs`]: crate::event::CancelableEventArgs
#[macro_export]
macro_rules! cancelable_event_args {
    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, $ctx:tt: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }
            $(
                $(#[$validate_outer:meta])*
                fn validate(&$self_v:ident) -> Result<(), $ValidationError:path> { $($validate:tt)+ }
            )?
        }
    )+) => {$(
        $crate::__cancelable_event_args! {
            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*

                ..

                $(#[$concerns_widget_outer])*
                fn concerns_widget(&$self, $ctx: &mut WidgetContext) -> bool { $($concerns_widget)+ }

                $(
                    $(#[$validate_outer])*
                    fn validate(&$self_v) -> Result<(), $ValidationError> { $($validate)+ }
                )?
            }
        }
    )+};
}
#[doc(hidden)]
#[macro_export]
macro_rules! __cancelable_event_args {
    // match validate
    (
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, $ctx:tt: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }

            $(#[$validate_outer:meta])*
            fn validate(&$self_v:ident) -> Result<(), $ValidationError:path> { $($validate:tt)+ }
        }
    ) => {
        $crate::__cancelable_event_args! {common=>

            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*
                ..
                $(#[$concerns_widget_outer])*
                fn concerns_widget(&$self, $ctx: &mut WidgetContext) -> bool { $($concerns_widget)+ }
            }
        }
        impl $Args {
            /// New args from values that convert [into](Into) the argument types.
            ///
            /// # Panics
            ///
            /// Panics if the arguments are invalid.
            #[inline]
            #[track_caller]
            #[allow(clippy::too_many_arguments)]
            pub fn new(timestamp: impl Into<std::time::Instant>, $($arg : impl Into<$arg_ty>),*) -> Self {
                let args = $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    stop_propagation: std::sync::Arc::default(),
                    cancel: std::sync::Arc::default()
                };
                args.assert_valid();
                args
            }

            /// New args from values that convert [into](Into) the argument types.
            ///
            /// Returns an error if the constructed arguments are invalid.
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn try_new(timestamp: impl Into<std::time::Instant>, $($arg : impl Into<$arg_ty>),*) -> Result<Self, $ValidationError> {
                let args = $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    stop_propagation: std::sync::Arc::default(),
                    cancel: std::sync::Arc::default()
                };
                args.validate()?;
                Ok(args)
            }

            /// Arguments for event that happened now (`Instant::now`).
            ///
            /// # Panics
            ///
            /// Panics if the arguments are invalid.
            #[inline]
            #[track_caller]
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new(std::time::Instant::now(), $($arg),*)
            }

            /// Arguments for event that happened now (`Instant::now`).
            ///
            /// Returns an error if the constructed arguments are invalid.
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn try_now($($arg : impl Into<$arg_ty>),*) -> Result<Self, $ValidationError> {
                Self::try_new(std::time::Instant::now(), $($arg),*)
            }

            $(#[$validate_outer])*
            pub fn validate(&$self_v) -> Result<(), $ValidationError> {
                $($validate)+
            }

            /// Panics if the arguments are invalid.
            #[track_caller]
            pub fn assert_valid(&self) {
                if let Err(e) = self.validate() {
                    panic!("invalid `{}`, {e:?}", stringify!($Args));
                }
            }
        }
    };

    // match no validate
    (
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, $ctx:tt: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }
        }
    ) => {
        $crate::__cancelable_event_args! {common=>

            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*
                ..
                $(#[$concerns_widget_outer])*
                fn concerns_widget(&$self, $ctx: &mut WidgetContext) -> bool { $($concerns_widget)+ }
            }
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
                    cancel: std::sync::Arc::default()
                }
            }

            /// Arguments for event that happened now (`Instant::now`).
            ///
            /// # Panics
            ///
            /// Panics if the arguments are invalid.
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new(std::time::Instant::now(), $($arg),*)
            }
        }
    };

    // common code between validating and not.
    (common=>

        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, $ctx:tt: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }
        }
    ) => {
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
    };
}

#[doc(inline)]
pub use crate::cancelable_event_args;

///<span data-del-macro-root></span> Declares new [`Event`](crate::event::Event) types.
///
/// # Examples
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
            pub fn update<U: $crate::event::EventUpdateArgs>(self, args: &U) -> Option<&$crate::event::EventUpdate<$Event>> {
                <Self as $crate::event::Event>::update(self, args)
            }

            /// Schedule an event update.
            #[inline]
            pub fn notify<Evs: $crate::event::WithEvents>(self, events: &mut Evs, args: $Args) {
                <Self as $crate::event::Event>::notify(self, events, args);
            }
        }
        impl $crate::event::Event for $Event {
            type Args = $Args;

            fn slot(self) -> $crate::widget_info::EventSlot {
                std::thread_local! {
                    static SLOT: $crate::widget_info::EventSlot = $crate::widget_info::EventSlot::next();
                }
                SLOT.with(|s| *s)
            }
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
        #[doc = "You can preview this event using [`on_pre_"$event "`](fn.on_pre_"$event ".html)."]
        /// Otherwise the handler is only called after the widget content has a chance of handling the event by stopping propagation.
        ///
        /// # Async
        ///
        /// You can use async event handlers with this property.
        #[$crate::property(event, default( $crate::handler::hn!(|_, _|{}) ))]
        $vis fn [<on_ $event>](
            child: impl $crate::UiNode,
            handler: impl $crate::handler::WidgetHandler<$Args>,
        ) -> impl $crate::UiNode {
            $crate::event::on_event(child, $Event, $filter, handler)
        }

        #[doc = "Preview [`on_"$event "`](fn.on_"$event ".html) event."]
        ///
        /// # Preview
        ///
        /// Preview event properties call the handler before the main event property and before the widget content, if you stop
        /// the propagation of a preview event the main event handler is not called.
        ///
        /// # Async
        ///
        /// You can use async event handlers with this property, note that only the code before the fist `.await` is *preview*,
        /// subsequent code runs in widget updates.
        #[$crate::property(event, default( $crate::handler::hn!(|_, _|{}) ))]
        $vis fn [<on_pre_ $event>](
            child: impl $crate::UiNode,
            handler: impl $crate::handler::WidgetHandler<$Args>,
        ) -> impl $crate::UiNode {
            $crate::event::on_pre_event(child, $Event, $filter, handler)
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
///<span data-del-macro-root></span> Declare one or more event properties.
///
/// Each declaration expands to two properties `on_$event`, `on_pre_$event`.
/// The preview properties call [`on_pre_event`], the main event properties call [`on_event`].
///
/// # Examples
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::event::{event_property, EventArgs};
/// # use zero_ui_core::keyboard::*;
/// event_property! {
///     /// on_key_input docs.
///     pub fn key_input {
///         event: KeyInputEvent,
///         args: KeyInputArgs,
///         // default filter is |ctx, args| args.concerns_widget(ctx)
///     }
///
///     pub(crate) fn key_down {
///         event: KeyInputEvent,
///         args: KeyInputArgs,
///         // optional filter:
///         filter: |ctx, args| args.state == KeyState::Pressed && args.concerns_widget(ctx),
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
/// The `filter` predicate is called if [`stop_propagation`] is not requested. It must return `true` if the event arguments
/// are relevant in the context of the widget and event property. If it returns `true` the `handler` closure is called.
/// See [`on_event`] and [`on_pre_event`] for more information.
///
/// If you don't provide a filter predicate the default [`args.concerns_widget(ctx)`] is used.
/// So if you want to extend the filter and not fully replace it you must call `args.concerns_widget(ctx)` in your custom filter.
///
/// # Async
///
/// Async event handlers are supported by properties generated by this macro, but only the code before the first `.await` executes
/// in the event track, subsequent code runs in widget updates.
///
/// [`on_pre_event`]: crate::event::on_pre_event
/// [`on_event`]: crate::event::on_event
/// [`stop_propagation`]: EventArgs::stop_propagation
/// [`args.concerns_widget(ctx)`]: EventArgs::concerns_widget
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
/// The `filter` predicate is called if [`stop_propagation`] is not requested. It must return `true` if the event arguments are
/// relevant in the context of the widget. If it returns `true` the `handler` closure is called. Note that events that represent
/// an *interaction* with the widget must check that the widget [`allow_interaction`].
///
/// # Route
///
/// The event `handler` is called after the [`on_pre_event`] equivalent at the same context level. If the event
/// `filter` allows more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
///
/// # Async
///
/// Async event handlers are called like normal, but code after the first `.await` only runs in subsequent event updates. This means
/// that [`stop_propagation`] must be called before the first `.await`, otherwise you are only signaling
/// other async tasks handling the same event, if they are monitoring the [`stop_propagation_requested`].
///
/// [`allow_interaction`]: crate::widget_info::WidgetInfo::allow_interaction
/// [`stop_propagation`]: EventArgs::stop_propagation
/// [`stop_propagation_requested`]: EventArgs::stop_propagation_requested
#[inline]
pub fn on_event<C, E, F, H>(child: C, event: E, filter: F, handler: H) -> impl UiNode
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    H: WidgetHandler<E::Args>,
{
    struct OnEventNode<C, E, F, H> {
        child: C,
        event: E,
        filter: F,
        handler: H,
    }
    #[impl_ui_node(child)]
    impl<C, E, F, H> UiNode for OnEventNode<C, E, F, H>
    where
        C: UiNode,
        E: Event,
        F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
        H: WidgetHandler<E::Args>,
    {
        fn info(&self, ctx: &mut InfoContext, widget_info: &mut WidgetInfoBuilder) {
            self.child.info(ctx, widget_info);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.event(self.event).handler(&self.handler);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
            if let Some(args) = self.event.update(args) {
                self.child.event(ctx, args);

                if !args.stop_propagation_requested() && (self.filter)(ctx, args) {
                    self.handler.event(ctx, args);
                }
            } else {
                self.child.event(ctx, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);
            self.handler.update(ctx);
        }
    }
    OnEventNode {
        child,
        event,
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
/// The `filter` predicate is called if [`stop_propagation`] is not requested. It must return `true` if the event arguments are
/// relevant in the context of the widget. If it returns `true` the `handler` closure is called. Note that events that represent
/// an *interaction* with the widget must check that the widget [`allow_interaction`].
///
/// # Route
///
/// The event `handler` is called before the [`on_event`] equivalent at the same context level. If the event
/// `filter` allows more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
///
/// # Async
///
/// Async event handlers are called like normal, but code after the first `.await` only runs in subsequent event updates. This means
/// that [`stop_propagation`] must be called before the first `.await`, otherwise you are only signaling
/// other async tasks handling the same event, if they are monitoring the [`stop_propagation_requested`].
///
/// [`allow_interaction`]: crate::widget_info::WidgetInfo::allow_interaction
/// [`stop_propagation`]: EventArgs::stop_propagation
/// [`stop_propagation_requested`]: EventArgs::stop_propagation_requested
pub fn on_pre_event<C, E, F, H>(child: C, event: E, filter: F, handler: H) -> impl UiNode
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    H: WidgetHandler<E::Args>,
{
    struct OnPreviewEventNode<C, E, F, H> {
        child: C,
        event: E,
        filter: F,
        handler: H,
    }
    #[impl_ui_node(child)]
    impl<C, E, F, H> UiNode for OnPreviewEventNode<C, E, F, H>
    where
        C: UiNode,
        E: Event,
        F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
        H: WidgetHandler<E::Args>,
    {
        fn info(&self, ctx: &mut InfoContext, widget_info: &mut WidgetInfoBuilder) {
            self.child.info(ctx, widget_info);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.event(self.event).handler(&self.handler);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
            if let Some(args) = self.event.update(args) {
                if !args.stop_propagation_requested() && (self.filter)(ctx, args) {
                    self.handler.event(ctx, args);
                }
                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.handler.update(ctx);
            self.child.update(ctx);
        }
    }
    OnPreviewEventNode {
        child,
        event,
        filter,
        handler,
    }
}
