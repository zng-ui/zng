//! App event API.

use crate::context::{AlreadyRegistered, AppContext, UpdateRequest, Updates, WidgetContext};
use crate::profiler::profile_scope;
use crate::widget_base::IsEnabled;
use crate::{impl_ui_node, AnyMap, UiNode};
use std::cell::{Cell, RefCell, UnsafeCell};
use std::fmt::Debug;
use std::rc::Rc;
use std::time::Instant;
use std::{any::*, collections::VecDeque};

/// [`Event`] arguments.
pub trait EventArgs: Debug + Clone + 'static {
    /// Gets the instant this event happen.
    fn timestamp(&self) -> Instant;
    /// If this event arguments is relevant to the widget context.
    fn concerns_widget(&self, _: &mut WidgetContext) -> bool;

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
pub trait Event: 'static {
    /// Event arguments type.
    type Args: EventArgs;
    /// If the event is updated in the high-pressure lane.
    const IS_HIGH_PRESSURE: bool = false;

    /// New event emitter.
    fn emitter() -> EventEmitter<Self::Args> {
        EventEmitter::new(Self::IS_HIGH_PRESSURE)
    }

    /// New event listener that never updates.
    fn never() -> EventListener<Self::Args> {
        EventListener::never(Self::IS_HIGH_PRESSURE)
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

struct EventChannelInner<T> {
    data: UnsafeCell<Vec<T>>,
    listener_count: Cell<usize>,
    last_update: Cell<u32>,
    is_high_pressure: bool,
}

struct EventChannel<T: 'static> {
    r: Rc<EventChannelInner<T>>,
}
impl<T: 'static> Clone for EventChannel<T> {
    fn clone(&self) -> Self {
        EventChannel { r: Rc::clone(&self.r) }
    }
}
impl<T: 'static> EventChannel<T> {
    pub(crate) fn notify(&self, events: &Events, new_update: T) {
        let me = Rc::clone(&self.r);
        events.push_change(Box::new(move |update_id, updates| {
            // SAFETY: this is safe because Events requires a mutable reference to apply changes.
            let data = unsafe { &mut *me.data.get() };

            if me.last_update.get() != update_id {
                data.clear();
                me.last_update.set(update_id);
            }

            data.push(new_update);

            if me.is_high_pressure {
                updates.update_hp = true;
            } else {
                updates.update = true;
            }
        }));
    }

    /// Gets a reference to the updates that happened in between calls of [`UiNode::update`](crate::core::UiNode::update).
    pub fn updates<'a>(&'a self, events: &'a Events) -> &'a [T] {
        if self.r.last_update.get() == events.update_id() {
            // SAFETY: This is safe because we are bounding the value lifetime with
            // the `Events` lifetime and we require a mutable reference to `Events` to
            // modify the value.
            unsafe { &*self.r.data.get() }.as_ref()
        } else {
            // SAFETY: same reason as the `if` case.
            // `last_update` only changes during `push_change` also.
            unsafe { &mut *self.r.data.get() }.clear();
            &[]
        }
    }

    /// If this update is notified using the [`UiNode::update_hp`](crate::core::UiNode::update_hp) method.
    pub fn is_high_pressure(&self) -> bool {
        self.r.is_high_pressure
    }

    pub fn listener_count(&self) -> usize {
        self.r.listener_count.get()
    }

    pub fn has_listeners(&self) -> bool {
        self.listener_count() > 0
    }

    pub fn on_new_listener(&self) {
        self.r.listener_count.set(self.r.listener_count.get() + 1)
    }

    pub fn on_drop_listener(&self) {
        self.r.listener_count.set(self.r.listener_count.get() - 1)
    }
}
impl<T: Clone> EventChannel<T> {
    pub fn buffered_listener(&self, events: &Events) -> BufEventListener<T> {
        let buffer = BufEventListener { queue: Default::default() };
        let buffer_ = buffer.clone();
        let me = Rc::clone(&self.r);
        events.push_buffer(Box::new(move |update_id| {
            // we keep this buffer alive only if there is a copy of it alive out there.
            let retain = Rc::strong_count(&buffer_.queue) > 1;
            if retain {
                // SAFETY: this is safe because Events requires a mutable reference to apply changes
                // and is till borrowed as mutable but has finished applying changes.

                if me.last_update.get() != update_id {
                    unsafe { &mut *me.data.get() }.clear();
                } else {
                    let data = unsafe { &*me.data.get() };
                    if !data.is_empty() {
                        let mut buf = buffer_.queue.borrow_mut();
                        for e in data {
                            buf.push_back(e.clone());
                        }
                    }
                }
            }
            retain
        }));
        buffer
    }
}

/// Read-only reference to an event channel.
pub struct EventListener<T: 'static> {
    chan: EventChannel<T>,
}
impl<T: 'static> Clone for EventListener<T> {
    fn clone(&self) -> Self {
        EventListener::new(self.chan.clone())
    }
}
impl<T: 'static> EventListener<T> {
    fn new(chan: EventChannel<T>) -> Self {
        chan.on_new_listener();
        EventListener { chan }
    }

    fn never(is_high_pressure: bool) -> Self {
        EventEmitter::new(is_high_pressure).into_listener()
    }

    /// New [`response`](EventEmitter::response) that never updates.
    pub fn response_never() -> Self {
        EventListener::never(false)
    }

    /// Gets a reference to the updates that happened in between calls of [`UiNode::update`](crate::UiNode::update).
    pub fn updates<'a>(&'a self, events: &'a Events) -> &'a [T] {
        self.chan.updates(events)
    }

    /// If [`updates`](EventListener::updates) is not empty.
    pub fn has_updates<'a>(&'a self, events: &'a Events) -> bool {
        !self.updates(events).is_empty()
    }

    /// If this update is notified using the [`UiNode::update_hp`](crate::UiNode::update_hp) method.
    pub fn is_high_pressure(&self) -> bool {
        self.chan.is_high_pressure()
    }
}
impl<T: EventArgs> EventListener<T> {
    /// Filters out updates that are flagged [`stop_propagation`](EventArgs::stop_propagation).
    pub fn updates_filtered<'a>(&'a self, events: &'a Events) -> impl Iterator<Item = &'a T> {
        self.updates(events).iter().filter(|a| !a.stop_propagation_requested())
    }
}
impl<T: Clone> EventListener<T> {
    /// Updates are only visible for one update cycle, this is very efficient but
    /// you can miss updates if you are not checking updates for every update call.
    ///
    /// A buffered event listener avoids this by cloning every update and keeping it in a FILO queue
    /// that can be drained any time, without the need for the `events` reference even.
    ///
    /// Every call to this method creates a new buffer, independent of any previous generated buffers,
    /// the buffer objects can be cloned and clones all point to the same buffer.
    pub fn make_buffered(&self, events: &Events) -> BufEventListener<T> {
        self.chan.buffered_listener(events)
    }
}
impl<T: 'static> Drop for EventListener<T> {
    fn drop(&mut self) {
        self.chan.on_drop_listener();
    }
}

/// Read-write reference to an event channel.
pub struct EventEmitter<T: 'static> {
    chan: EventChannel<T>,
}
impl<T: 'static> Clone for EventEmitter<T> {
    fn clone(&self) -> Self {
        EventEmitter { chan: self.chan.clone() }
    }
}
impl<T: 'static> EventEmitter<T> {
    fn new(is_high_pressure: bool) -> Self {
        EventEmitter {
            chan: EventChannel {
                r: Rc::new(EventChannelInner {
                    data: UnsafeCell::default(),
                    listener_count: Cell::new(0),
                    last_update: Cell::new(0),
                    is_high_pressure,
                }),
            },
        }
    }

    /// New emitter for a service request response.
    ///
    /// The emitter is expected to update only once so it is not high-pressure.
    pub fn response() -> Self {
        Self::new(false)
    }

    /// Number of listener to this event emitter.
    pub fn listener_count(&self) -> usize {
        self.chan.listener_count()
    }

    /// If this event emitter has any listeners.
    pub fn has_listeners(&self) -> bool {
        self.chan.has_listeners()
    }

    /// Gets a reference to the updates that happened in between calls of [`UiNode::update`](crate::UiNode::update).
    pub fn updates<'a>(&'a self, events: &'a Events) -> &'a [T] {
        self.chan.updates(events)
    }

    /// If [`updates`](EventEmitter::updates) is not empty.
    pub fn has_updates<'a>(&'a self, events: &'a Events) -> bool {
        !self.updates(events).is_empty()
    }

    /// If this event is notified using the [`UiNode::update_hp`](crate::UiNode::update_hp) method.
    pub fn is_high_pressure(&self) -> bool {
        self.chan.is_high_pressure()
    }

    /// Schedules an update notification.
    pub fn notify(&self, events: &Events, new_update: T) {
        self.chan.notify(events, new_update);
    }

    /// Gets a new event listener linked with this emitter.
    pub fn listener(&self) -> EventListener<T> {
        EventListener::new(self.chan.clone())
    }

    /// Converts this emitter instance into a listener.
    pub fn into_listener(self) -> EventListener<T> {
        EventListener::new(self.chan)
    }
}
impl<T: Clone> EventEmitter<T> {
    /// Create a buffered event listener.
    ///
    /// See [`EventListener::make_buffered`] for more details.
    pub fn buffered_listener(&self, events: &Events) -> BufEventListener<T> {
        self.chan.buffered_listener(events)
    }
}

/// A buffered [`EventListener`].
///
/// This `struct` can be created by calling [`EventListener::make_buffered`] or [`EventEmitter::buffered_listener`]
/// the documentation of `make_buffered` contains more details.
///
/// This `struct` is a refence to the buffer, clones of it point to the same buffer. This `struct`
/// is not `Send`, you can use a [`Sync::event_receiver`](crate::sync::Sync::event_receiver) for that.
#[derive(Clone)]
pub struct BufEventListener<T: Clone> {
    queue: Rc<RefCell<VecDeque<T>>>,
}
impl<T: Clone> BufEventListener<T> {
    /// If there are any updates in the buffer.
    #[inline]
    pub fn has_updates(&self) -> bool {
        !self.queue.borrow().is_empty()
    }

    /// Take the oldest event in the buffer.
    #[inline]
    pub fn pop_oldest(&self) -> Option<T> {
        self.queue.borrow_mut().pop_front()
    }

    /// Take the oldest `n` events from the buffer.
    ///
    /// The result is sorted from oldest to newer.
    #[inline]
    pub fn pop_oldest_n(&self, n: usize) -> Vec<T> {
        self.queue.borrow_mut().drain(..n).collect()
    }

    /// Take all the events from the buffer.
    ///
    /// The result is sorted from oldest to newest.
    #[inline]
    pub fn pop_all(&self) -> Vec<T> {
        self.queue.borrow_mut().drain(..).collect()
    }

    /// Create an empty buffer that will always stay empty.
    #[inline]
    pub fn never() -> Self {
        BufEventListener { queue: Default::default() }
    }
}

thread_singleton!(SingletonEvents);

/// Access to application events.
///
/// Only a single instance of this type exists at a time.
pub struct Events {
    events: AnyMap,
    update_id: u32,
    #[allow(clippy::type_complexity)]
    pending: RefCell<Vec<Box<dyn FnOnce(u32, &mut UpdateRequest)>>>,
    #[allow(clippy::type_complexity)]
    buffers: RefCell<Vec<Box<dyn Fn(u32) -> Retain>>>,
    app_pre_handlers: AppHandlers,
    app_handlers: AppHandlers,
    _singleton: SingletonEvents,
}

type AppHandlerWeak = std::rc::Weak<RefCell<dyn FnMut(&mut AppContext)>>;

#[derive(Default)]
struct AppHandlers(RefCell<Vec<AppHandlerWeak>>);
impl AppHandlers {
    pub fn push(&self, handler: &EventHandler) {
        self.0.borrow_mut().push(Rc::downgrade(&handler.0));
    }

    pub fn notify(&self, ctx: &mut AppContext) {
        let mut handlers = self.0.borrow_mut();
        let mut live_handlers = Vec::with_capacity(handlers.len());
        handlers.retain(|h| {
            if let Some(handler) = h.upgrade() {
                live_handlers.push(handler);
                true
            } else {
                false
            }
        });
        drop(handlers);

        for handler in live_handlers {
            handler.borrow_mut()(ctx);
        }
    }
}

/// A *global* event handler created by [`Events::on_event`].
#[derive(Clone)]
pub struct EventHandler(Rc<RefCell<dyn FnMut(&mut AppContext)>>);
impl EventHandler {
    pub(self) fn new(handler: impl FnMut(&mut AppContext) + 'static) -> Self {
        Self(Rc::new(RefCell::new(handler)))
    }
}

impl Events {
    /// If an instance of `Events` already exists in the  current thread.
    #[inline]
    pub fn instantiated() -> bool {
        SingletonEvents::in_use()
    }

    /// Produces the instance of `Events`. Only a single
    /// instance can exist in a thread at a time, panics if called
    /// again before dropping the previous instance.
    #[inline]
    pub fn instance() -> Self {
        Events {
            events: Default::default(),
            update_id: 0,
            pending: RefCell::default(),
            buffers: RefCell::default(),
            app_pre_handlers: AppHandlers::default(),
            app_handlers: AppHandlers::default(),
            _singleton: SingletonEvents::assert_new("Events"),
        }
    }

    /// Register a new event for the duration of the application.
    pub fn try_register<E: Event>(&mut self, listener: EventListener<E::Args>) -> Result<(), AlreadyRegistered> {
        debug_assert_eq!(E::IS_HIGH_PRESSURE, listener.is_high_pressure());

        match self.events.entry(TypeId::of::<E>()) {
            std::collections::hash_map::Entry::Occupied(_) => Err(AlreadyRegistered {
                type_name: type_name::<E>(),
            }),
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(Box::new(listener));
                Ok(())
            }
        }
    }

    /// Register a new event for the duration of the application.
    ///
    /// # Panics
    ///
    /// Panics if the event type is already registered.
    #[track_caller]
    pub fn register<E: Event>(&mut self, listener: EventListener<E::Args>) {
        self.try_register::<E>(listener).unwrap()
    }

    /// Creates an event listener if the event is registered in the application.
    pub fn try_listen<E: Event>(&self) -> Option<EventListener<E::Args>> {
        self.events
            .get(&TypeId::of::<E>())
            .map(|any| any.downcast_ref::<EventListener<E::Args>>().unwrap().clone())
    }

    /// Creates a buffered event listener if the event is registered in the application.
    pub fn try_listen_buf<E: Event>(&self) -> Option<BufEventListener<E::Args>> {
        self.try_listen::<E>().map(|l| l.make_buffered(self))
    }

    /// Creates an event listener.
    ///
    /// # Panics
    ///
    /// If the event is not registered in the application.
    pub fn listen<E: Event>(&self) -> EventListener<E::Args> {
        self.try_listen::<E>()
            .unwrap_or_else(|| panic!("event `{}` is required", type_name::<E>()))
    }

    /// Creates a buffered event listener.
    ///
    /// # Panics
    ///
    /// If the event is not registered in the application.
    pub fn listen_buf<E: Event>(&self) -> BufEventListener<E::Args> {
        self.listen::<E>().make_buffered(self)
    }

    /// Creates an event listener or returns [`E::never()`](Event::never).
    pub fn listen_or_never<E: Event>(&self) -> EventListener<E::Args> {
        self.try_listen::<E>().unwrap_or_else(E::never)
    }

    /// Creates a buffered event listener or returns [`E::never()`](Event::never).
    pub fn listen_or_never_buf<E: Event>(&self) -> BufEventListener<E::Args> {
        self.listen_or_never::<E>().make_buffered(self)
    }

    /// Creates a preview event handler if the event is registered in the application.
    ///
    /// See [`on_pre_event`](Self::on_pre_event) for more details.
    pub fn try_on_pre_event<E, H>(&self, mut handler: H) -> Option<EventHandler>
    where
        E: Event,
        H: FnMut(&mut AppContext, &E::Args) + 'static,
    {
        self.try_listen::<E>().map(|l| {
            let handler = EventHandler::new(move |ctx| {
                for update in l.updates_filtered(ctx.events) {
                    handler(ctx, update)
                }
            });
            self.app_pre_handlers.push(&handler);
            handler
        })
    }

    /// Creates an event handler if the event is registered in the application.
    ///
    /// See [`on_event`](Self::on_event) for more details.
    pub fn try_on_event<E, H>(&self, mut handler: H) -> Option<EventHandler>
    where
        E: Event,
        H: FnMut(&mut AppContext, &E::Args) + 'static,
    {
        self.try_listen::<E>().map(|l| {
            let handler = EventHandler::new(move |ctx| {
                for update in l.updates_filtered(ctx.events) {
                    handler(ctx, update)
                }
            });
            self.app_handlers.push(&handler);
            handler
        })
    }

    /// Creates a preview event handler.
    ///
    /// The event `handler` is called for every update of `E` that are not marked [`stop_propagation`](EventArgs::stop_propagation).
    /// The handler is called before UI handlers and [`on_event`](Self::on_event) handlers, it is called after all previous registered
    /// preview handlers.
    ///
    /// Drop all clones of the [`EventHandler`] object to unsubscribe.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::events::*;
    /// # use zero_ui_core::focus::FocusChangedEvent;
    /// # fn example(ctx: &mut zero_ui_core::context::AppContext) {
    /// let handler = ctx.events.on_pre_event::<FocusChangedEvent, _>(|_ctx, args| {
    ///     println!("focused: {:?}", args.new_focus);
    /// });
    /// # }
    /// ```
    /// The example listens to all `FocusChangedEvent` events, independent of widget context and before all UI handlers.
    ///
    /// # Panics
    ///
    /// If the event is not registered in the application.
    pub fn on_pre_event<E, H>(&self, handler: H) -> EventHandler
    where
        E: Event,
        H: FnMut(&mut AppContext, &E::Args) + 'static,
    {
        self.try_on_pre_event::<E, H>(handler)
            .unwrap_or_else(|| panic!("event `{}` is required", type_name::<E>()))
    }

    /// Creates an event handler.
    ///
    /// The event `handler` is called for every update of `E` that are not marked [`stop_propagation`](EventArgs::stop_propagation).
    /// The handler is called after all [`on_pre_event`],(Self::on_pre_event) all UI handlers and all [`on_event`](Self::on_event) handlers
    /// registered before this one.
    ///
    /// Creating a [listener](Events::listen) is slightly more efficient then this and also gives you access to args marked
    /// with [`stop_propagation`](EventArgs::stop_propagation), this method exists for the convenience of listening on
    /// an event at the app level without having to declare an [`AppExtension`](crate::app::AppExtension) or a weird property.
    ///
    /// Drop all clones of the [`EventHandler`] object to unsubscribe.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::events::*;
    /// # use zero_ui_core::focus::FocusChangedEvent;
    /// # fn example(ctx: &mut zero_ui_core::context::AppContext) {
    /// let handler = ctx.events.on_event::<FocusChangedEvent, _>(|_ctx, args| {
    ///     println!("focused: {:?}", args.new_focus);
    /// });
    /// # }
    /// ```
    /// The example listens to all `FocusChangedEvent` events, independent of widget context, after the UI was notified.
    ///
    /// # Panics
    ///
    /// If the event is not registered in the application.
    pub fn on_event<E, H>(&self, handler: H) -> EventHandler
    where
        E: Event,
        H: FnMut(&mut AppContext, &E::Args) + 'static,
    {
        self.try_on_event::<E, H>(handler)
            .unwrap_or_else(|| panic!("event `{}` is required", type_name::<E>()))
    }

    pub(super) fn update_id(&self) -> u32 {
        self.update_id
    }

    pub(super) fn push_change(&self, change: Box<dyn FnOnce(u32, &mut UpdateRequest)>) {
        self.pending.borrow_mut().push(change);
    }

    pub(super) fn push_buffer(&self, buffer: Box<dyn Fn(u32) -> Retain>) {
        self.buffers.borrow_mut().push(buffer);
    }

    pub(super) fn apply(&mut self, updates: &mut Updates) {
        self.update_id = self.update_id.wrapping_add(1);

        let pending = self.pending.get_mut();
        if !pending.is_empty() {
            let mut ups = UpdateRequest::default();
            for f in pending.drain(..) {
                f(self.update_id, &mut ups);
            }
            updates.schedule_updates(ups);

            self.buffers.borrow_mut().retain(|b| b(self.update_id));
        }
    }

    pub(super) fn on_pre_events(&self, ctx: &mut AppContext) {
        self.app_pre_handlers.notify(ctx);
    }

    pub(super) fn on_events(&self, ctx: &mut AppContext) {
        self.app_handlers.notify(ctx);
    }
}

type Retain = bool;

/// Declares new [`EventArgs`](crate::event::EventArgs) types.
///
/// # Example
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
///
/// Expands to:
///
/// ```
/// # use zero_ui_core::event::event_args;
/// # use zero_ui_core::render::WidgetPath;
/// #
/// /// My event arguments.
/// #[derive(Debug, Clone)]
/// pub struct MyEventArgs {
///     /// When the event happened.
///     pub timestamp: std::time::Instant,
///     /// My argument.
///     pub arg: String,
///     /// My event target.
///     pub target: WidgetPath,
///
///     stop_propagation: std::rc::Rc<std::cell::Cell<bool>>
/// }
///
/// impl MyEventArgs {
///     #[inline]
///     pub fn new(
///         timestamp: impl Into<std::time::Instant>,
///         arg: impl Into<String>,
///         target: impl Into<WidgetPath>,
///     ) -> Self {
///         MyEventArgs {
///             timestamp: timestamp.into(),
///             arg: arg.into(),
///             target: target.into(),
///             stop_propagation: std::rc::Rc::default()
///         }
///     }
///
///     /// Arguments for event that happened now (`Instant::now`).
///     #[inline]
///     pub fn now(arg: impl Into<String>, target: impl Into<WidgetPath>) -> Self {
///         Self::new(std::time::Instant::now(), arg, target)
///     }
///
///     /// Requests that subsequent handlers skip this event.
///     ///
///     /// Cloned arguments signal stop for all clones.
///     #[inline]
///     pub fn stop_propagation(&self) {
///         <Self as zero_ui_core::event::EventArgs>::stop_propagation(self)
///     }
///     
///     /// If the handler must skip this event.
///     ///
///     /// Note that property level handlers don't need to check this, as those handlers are
///     /// already not called when this is `true`. [`UiNode`](crate::UiNode) and
///     /// [`AppExtension`](crate::app::AppExtension) implementers must check if this is `true`.
///     #[inline]
///     pub fn stop_propagation_requested(&self) -> bool {
///         <Self as zero_ui_core::event::EventArgs>::stop_propagation_requested(self)
///     }
/// }
///
/// impl zero_ui_core::event::EventArgs for MyEventArgs {
///     #[inline]
///     fn timestamp(&self) -> std::time::Instant {
///         self.timestamp
///     }
///
///     #[inline]
///     /// If `ctx.path.widget_id()` is in the `self.target` path.
///     fn concerns_widget(&self, ctx: &mut zero_ui_core::context::WidgetContext) -> bool {
///         self.target.contains(ctx.path.widget_id())
///     }
///
///     #[inline]
///     fn stop_propagation(&self) {
///         self.stop_propagation.set(true);
///     }
///     
///     #[inline]
///     fn stop_propagation_requested(&self) -> bool {
///         self.stop_propagation.get()
///     }
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

            stop_propagation: std::rc::Rc<std::cell::Cell<bool>>,
        }
        impl $Args {
            /// New args from values that convert [into](Into) the argument types.
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn new(timestamp: impl Into<std::time::Instant>, $($arg : impl Into<$arg_ty>),*) -> Self {
                $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    stop_propagation: std::rc::Rc::default(),
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
                self.stop_propagation.set(true);
            }

            #[inline]
            fn stop_propagation_requested(&self) -> bool {
                self.stop_propagation.get()
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
///
/// Expands to:
///
/// ```
/// # use zero_ui_core::event::event_args;
/// # use zero_ui_core::render::WidgetPath;
/// #
/// /// My event arguments.
/// #[derive(Debug, Clone)]
/// pub struct MyEventArgs {
///     /// When the event happened.
///     pub timestamp: std::time::Instant,
///     /// My argument.
///     pub arg: String,
///     /// My event target.
///     pub target: WidgetPath,
///
///     cancel: std::rc::Rc<std::cell::Cell<bool>>,
///     stop_propagation: std::rc::Rc<std::cell::Cell<bool>>,
/// }
///
/// impl MyEventArgs {
///     #[inline]
///     pub fn new(
///         timestamp: impl Into<std::time::Instant>,
///         arg: impl Into<String>,
///         target: impl Into<WidgetPath>,
///     ) -> Self {
///         MyEventArgs {
///             timestamp: timestamp.into(),
///             arg: arg.into(),
///             target: target.into(),
///             cancel: std::rc::Rc::default(),
///             stop_propagation: std::rc::Rc::default(),
///         }
///     }
///
///     /// Arguments for event that happened now (`Instant::now`).
///     #[inline]
///     pub fn now(arg: impl Into<String>, target: impl Into<WidgetPath>) -> Self {
///         Self::new(std::time::Instant::now(), arg, target)
///     }
/// }
///
/// impl zero_ui_core::event::EventArgs for MyEventArgs {
///     #[inline]
///     fn timestamp(&self) -> std::time::Instant {
///         self.timestamp
///     }
///
///     #[inline]
///     /// If `ctx.path.widget_id()` is in the `self.target` path.
///     fn concerns_widget(&self, ctx: &mut zero_ui_core::context::WidgetContext) -> bool {
///         self.target.contains(ctx.path.widget_id())
///     }
///
///     #[inline]
///     fn stop_propagation(&self) {
///         self.stop_propagation.set(true);
///     }
///     
///     #[inline]
///     fn stop_propagation_requested(&self) -> bool {
///         self.stop_propagation.get()
///     }
/// }
///
/// impl zero_ui_core::event::CancelableEventArgs for MyEventArgs {
///     /// If a listener canceled the action.
///     #[inline]
///     fn cancel_requested(&self) -> bool {
///         self.cancel.get()
///     }
///
///     /// Cancel the action.
///     ///
///     /// Cloned args are still linked, canceling one will cancel the others.
///     #[inline]
///     fn cancel(&self) {
///         self.cancel.set(true);
///     }
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
            cancel: std::rc::Rc<std::cell::Cell<bool>>,
            stop_propagation: std::rc::Rc<std::cell::Cell<bool>>,
        }
        impl $Args {
            /// New args from values that convert [into](Into) the argument types.
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn new(timestamp: impl Into<std::time::Instant>, $($arg : impl Into<$arg_ty>),*) -> Self {
                $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    cancel: std::rc::Rc::default(),
                    stop_propagation: std::rc::Rc::default(),
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
                self.stop_propagation.set(true);
            }

            #[inline]
            fn stop_propagation_requested(&self) -> bool {
                self.stop_propagation.get()
            }
        }
        impl $crate::event::CancelableEventArgs for $Args {
            #[inline]
            fn cancel_requested(&self) -> bool {
                self.cancel.get()
            }

            #[inline]
            fn cancel(&self) {
                self.cancel.set(true);
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

/// Declares new low-pressure [`Event`](crate::event::Event) types.
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
///
/// Expands to:
///
/// ```
/// # use zero_ui_core::event::event;
/// # use zero_ui_core::gesture::ClickArgs;
/// /// Event docs
/// pub struct ClickEvent;
/// impl zero_ui_core::event::Event for ClickEvent {
///     type Args = ClickArgs;
/// }
///
/// /// Other event docs
/// pub struct DoubleClickEvent;
/// impl zero_ui_core::event::Event for DoubleClickEvent {
///     type Args = ClickArgs;
/// }
/// ```
#[macro_export]
macro_rules! event {
    ($($(#[$outer:meta])* $vis:vis $Event:ident : $Args:path;)+) => {$(
        $(#[$outer])*
        $vis struct $Event;
        impl $crate::event::Event for $Event {
            type Args = $Args;
        }
        impl $Event {
            /// New event emitter.
            #[inline]
            pub fn emitter() -> $crate::event::EventEmitter<$Args> {
                <Self as $crate::event::Event>::emitter()
            }

            /// New event listener that never updates.
            pub fn never() -> $crate::event::EventListener<$Args> {
                <Self as $crate::event::Event>::never()
            }
        }
    )+};
}
#[doc(inline)]
pub use crate::event;

/// Declares new high-pressure [`Event`](crate::event::Event) types.
///
/// Same syntax as [`event!`](macro.event.html) but the event is marked [high-pressure](crate::event::Event::IS_HIGH_PRESSURE).
///
/// # Example
///
/// ```
/// # use zero_ui_core::event::event_hp;
/// # use zero_ui_core::mouse::MouseMoveArgs;
/// event_hp! {
///     /// Event docs.
///     pub MouseMoveEvent: MouseMoveArgs;
/// }
/// ```
///
/// Expands to:
///
/// ```
/// # use zero_ui_core::event::event_hp;
/// # use zero_ui_core::mouse::MouseMoveArgs;
/// /// Event docs
/// pub struct MouseMoveEvent;
/// impl zero_ui_core::event::Event for MouseMoveEvent {
///     type Args = MouseMoveArgs;
///     const IS_HIGH_PRESSURE: bool = true;
/// }
/// ```
#[macro_export]
macro_rules! event_hp {
    ($($(#[$outer:meta])* $vis:vis $Event:ident : $Args:path;)+) => {$(
        $(#[$outer])*
        $vis struct $Event;
        impl $crate::event::Event for $Event {
            type Args = $Args;
            const IS_HIGH_PRESSURE: bool = true;
        }

        impl $Event {
            /// New event emitter.
            #[inline]
            pub fn emitter() -> $crate::event::EventEmitter<$Args> {
                <Self as $crate::event::Event>::emitter()
            }

            /// New event listener that never updates.
            pub fn never() -> $crate::event::EventListener<$Args> {
                <Self as $crate::event::Event>::never()
            }
        }
    )+};
}
#[doc(inline)]
pub use crate::event_hp;

/* Event Property */

struct OnEventNode<C, E, F, H>
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool,
    H: FnMut(&mut WidgetContext, &E::Args),
{
    child: C,
    _event: E,
    listener: EventListener<E::Args>,
    filter: F,
    handler: H,
}
#[impl_ui_node(child)]
impl<C, E, F, H> OnEventNode<C, E, F, H>
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    H: FnMut(&mut WidgetContext, &E::Args) + 'static,
{
    #[UiNode]
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.listener = ctx.events.listen::<E>();
        self.child.init(ctx);
    }

    #[UiNode]
    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.listener = E::never();
        self.child.deinit(ctx);
    }

    #[UiNode]
    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        if !E::IS_HIGH_PRESSURE {
            self.do_update(ctx)
        }
    }

    #[UiNode]
    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        self.child.update_hp(ctx);

        if E::IS_HIGH_PRESSURE {
            self.do_update(ctx)
        }
    }

    fn do_update(&mut self, ctx: &mut WidgetContext) {
        if self.listener.has_updates(ctx.events) && IsEnabled::get(ctx.vars) {
            for args in self.listener.updates(ctx.events) {
                if !args.stop_propagation_requested() && (self.filter)(ctx, args) {
                    profile_scope!("on_event::<{}>", std::any::type_name::<E>());
                    (self.handler)(ctx, &args);
                }
            }
        }
    }
}

struct OnPreviewEventNode<C, E, F, H>
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool,
    H: FnMut(&mut WidgetContext, &E::Args),
{
    child: C,
    _event: E,
    listener: EventListener<E::Args>,
    filter: F,
    handler: H,
}
#[impl_ui_node(child)]
impl<C, E, F, H> OnPreviewEventNode<C, E, F, H>
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    H: FnMut(&mut WidgetContext, &E::Args) + 'static,
{
    #[UiNode]
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.listener = ctx.events.listen::<E>();
        self.child.init(ctx);
    }

    #[UiNode]
    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.listener = E::never();
        self.child.deinit(ctx);
    }

    #[UiNode]
    fn update(&mut self, ctx: &mut WidgetContext) {
        if !E::IS_HIGH_PRESSURE {
            self.do_update(ctx)
        }

        self.child.update(ctx);
    }

    #[UiNode]
    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        if E::IS_HIGH_PRESSURE {
            self.do_update(ctx)
        }

        self.child.update_hp(ctx);
    }

    fn do_update(&mut self, ctx: &mut WidgetContext) {
        if self.listener.has_updates(ctx.events) && IsEnabled::get(ctx.vars) {
            for args in self.listener.updates(ctx.events) {
                if !args.stop_propagation_requested() && (self.filter)(ctx, args) {
                    profile_scope!("on_pre_event::<{}>", std::any::type_name::<E>());
                    (self.handler)(ctx, &args);
                }
            }
        }
    }
}

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
        /// # Preview Event
        ///
        #[doc = "You can preview this event using [`on_pre_" $event "`]."]
        #[$crate::property(event, default(|_, _|{}))]
        $vis fn [<on_ $event>](
            child: impl $crate::UiNode,
            handler: impl FnMut(&mut $crate::context::WidgetContext, &$Args) + 'static
        ) -> impl $crate::UiNode {
            $crate::event::on_event(child, $Event, $filter, handler)
        }

        #[doc = "Preview [on_" $event "] event."]
        ///
        /// # Preview Events
        ///
        /// Preview events are fired before the main event, if you stop the propagation of a preview event
        /// the main event does not run. See [`on_pre_event`](crate::properties::events::on_pre_event) for more details.
        #[$crate::property(event, default(|_, _|{}))]
        $vis fn [<on_pre_ $event>](
            child: impl $crate::UiNode,
            handler: impl FnMut(&mut $crate::context::WidgetContext, &$Args) + 'static
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
/// Declare one or more event properties.
///
/// Each declaration expands to a pair of properties `on_$event` and `on_pre_$event`. The preview property
/// calls [`on_pre_event`](crate::event::on_pre_event),
/// the main event property calls [`on_event`](crate::event::on_event).
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
pub fn on_event<E: Event>(
    child: impl UiNode,
    event: E,
    filter: impl FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    handler: impl FnMut(&mut WidgetContext, &E::Args) + 'static,
) -> impl UiNode {
    OnEventNode {
        child,
        _event: event,
        listener: E::never(),
        filter,
        handler,
    }
}

/// Helper for declaring preview event properties with a custom filter.
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
pub fn on_pre_event<E: Event>(
    child: impl UiNode,
    event: E,
    filter: impl FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    handler: impl FnMut(&mut WidgetContext, &E::Args) + 'static,
) -> impl UiNode {
    OnPreviewEventNode {
        child,
        _event: event,
        listener: E::never(),
        filter,
        handler,
    }
}
