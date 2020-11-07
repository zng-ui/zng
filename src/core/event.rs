//! App event API.

use super::context::{UpdateRequest, Updates, WidgetContext};
use super::AnyMap;
use std::any::*;
use std::cell::{Cell, RefCell, UnsafeCell};
use std::fmt::Debug;
use std::rc::Rc;
use std::time::Instant;

/// [`Event`] arguments.
pub trait EventArgs: Debug + Clone + 'static {
    /// Gets the instant this event happen.
    fn timestamp(&self) -> Instant;
    /// If this event arguments is relevant to the widget context.
    fn concerns_widget(&self, _: &mut WidgetContext) -> bool;

    /// Requests that subsequent handlers skip this event.
    fn stop_propagation(&self);

    /// If the handler must skip this event.
    ///
    /// Note that property level handlers don't need to check this, as those handlers are
    /// already not called when this is `true`. [`UiNode`](zero_ui::core::UiNode) and
    /// [`AppExtension`](zero_ui::core::app::AppExtension) implementers must check if this is `true`.
    fn stop_propagation_requested(&self) -> bool;
}

/// [`Event`] arguments that can be canceled.
pub trait CancelableEventArgs: EventArgs {
    /// If the originating action must be canceled.
    fn cancel_requested(&self) -> bool;
    /// Cancel the originating action.
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

    /// Gets a reference to the updates that happened in between calls of [`UiNode::update`](crate::core::UiNode::update).
    pub fn updates<'a>(&'a self, events: &'a Events) -> &'a [T] {
        self.chan.updates(events)
    }

    /// If [`updates`](EventListener::updates) is not empty.
    pub fn has_updates<'a>(&'a self, events: &'a Events) -> bool {
        !self.updates(events).is_empty()
    }

    /// If this update is notified using the [`UiNode::update_hp`](crate::core::UiNode::update_hp) method.
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

    /// Gets a reference to the updates that happened in between calls of [`UiNode::update`](crate::core::UiNode::update).
    pub fn updates<'a>(&'a self, events: &'a Events) -> &'a [T] {
        self.chan.updates(events)
    }

    /// If [`updates`](EventEmitter::updates) is not empty.
    pub fn has_updates<'a>(&'a self, events: &'a Events) -> bool {
        !self.updates(events).is_empty()
    }

    /// If this event is notified using the [`UiNode::update_hp`](crate::core::UiNode::update_hp) method.
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

singleton_assert!(SingletonEvents);

/// Access to application events.
///
/// Only a single instance of this type exists at a time.
pub struct Events {
    events: AnyMap,
    update_id: u32,
    #[allow(clippy::type_complexity)]
    pending: RefCell<Vec<Box<dyn FnOnce(u32, &mut UpdateRequest)>>>,
    _singleton: SingletonEvents,
}

impl Events {
    /// Produces the instance of `Events`. Only a single
    /// instance can exist at a time, panics if called
    /// again before dropping the previous instance.
    pub fn instance() -> Self {
        Events {
            events: Default::default(),
            update_id: 0,
            pending: RefCell::default(),
            _singleton: SingletonEvents::assert_new(),
        }
    }

    /// Register a new event for the duration of the application.
    pub fn register<E: Event>(&mut self, listener: EventListener<E::Args>) {
        assert_eq!(E::IS_HIGH_PRESSURE, listener.is_high_pressure());
        self.events.insert(TypeId::of::<E>(), Box::new(listener));
    }

    /// Creates an event listener if the event is registered in the application.
    pub fn try_listen<E: Event>(&self) -> Option<EventListener<E::Args>> {
        if let Some(any) = self.events.get(&TypeId::of::<E>()) {
            // SAFETY: This is safe because args are always the same type as key in
            // `AppRegister::register_event` witch is the only place where insertion occurs.
            Some(any.downcast_ref::<EventListener<E::Args>>().unwrap().clone())
        } else {
            None
        }
    }

    /// Creates an event listener.
    ///
    /// # Panics
    /// If the event is not registered in the application.
    pub fn listen<E: Event>(&self) -> EventListener<E::Args> {
        self.try_listen::<E>()
            .unwrap_or_else(|| panic!("event `{}` is required", type_name::<E>()))
    }

    pub(super) fn update_id(&self) -> u32 {
        self.update_id
    }

    pub(super) fn push_change(&self, change: Box<dyn FnOnce(u32, &mut UpdateRequest)>) {
        self.pending.borrow_mut().push(change);
    }

    pub(super) fn apply(&mut self, updates: &mut Updates) {
        self.update_id = self.update_id.wrapping_add(1);

        let pending = self.pending.get_mut();
        if !pending.is_empty() {
            let mut ups = UpdateRequest::default();
            for f in pending.drain(..) {
                f(self.update_id, &mut ups);
            }
            updates.push_updates(ups);
        }
    }
}

pub use zero_ui_macros::{cancelable_event_args, event, event_args, event_hp};
