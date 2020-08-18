//! App event API.

use crate::core::context::{Events, WidgetContext};
use std::cell::{Cell, UnsafeCell};
use std::fmt::Debug;
use std::rc::Rc;
use std::time::Instant;

/// [`Event`] arguments.
pub trait EventArgs: Debug + Clone + 'static {
    /// Gets the instant this event happen.
    fn timestamp(&self) -> Instant;
    /// If this event arguments is relevant to the widget context.
    fn concerns_widget(&self, _: &mut WidgetContext) -> bool;
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
    pub(crate) fn notify(self, new_update: T, _assert_events_not_borrowed: &mut Events, cleanup: &mut Vec<Box<dyn FnOnce()>>) {
        // SAFETY: This is safe because borrows are bound to the `Events` instance
        // so if we have a mutable reference to it no event value is borrowed.
        let data = unsafe { &mut *self.r.data.get() };
        data.push(new_update);

        if data.len() == 1 {
            // register for cleanup once
            cleanup.push(Box::new(move || {
                unsafe { &mut *self.r.data.get() }.clear();
            }))
        }
    }

    /// Gets a reference to the updates that happened in between calls of [`UiNode::update`](crate::core::UiNode::update).
    pub fn updates<'a>(&'a self, _events: &'a Events) -> &'a [T] {
        // SAFETY: This is safe because we are bounding the value lifetime with
        // the `Events` lifetime and we require a mutable reference to `Events` to
        // modify the value.
        unsafe { &*self.r.data.get() }.as_ref()
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
                    is_high_pressure,
                }),
            },
        }
    }

    /// New emitter for a service request response.
    ///
    /// The emitter is expected to update at maximum only once so it is not high-pressure.
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

    /// Gets a new event listener linked with this emitter.
    pub fn listener(&self) -> EventListener<T> {
        EventListener::new(self.chan.clone())
    }

    /// Converts this emitter instance into a listener.
    pub fn into_listener(self) -> EventListener<T> {
        EventListener::new(self.chan)
    }

    pub(crate) fn notify(self, new_update: T, assert_events_not_borrowed: &mut Events, cleanup: &mut Vec<Box<dyn FnOnce()>>) {
        self.chan.notify(new_update, assert_events_not_borrowed, cleanup);
    }
}

pub use zero_ui_macros::{cancelable_event_args, event, event_args, event_hp};
