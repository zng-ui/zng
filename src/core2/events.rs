use super::*;
pub use glutin::event::{ModifiersState, MouseButton};
use std::any::type_name;
use std::cell::{Cell, UnsafeCell};
use std::fmt::Debug;
use std::rc::Rc;
use std::time::Instant;

/// [Event] arguments.
pub trait EventArgs: Debug + Clone + 'static {
    /// Gets the instant this event happen.
    fn timestamp(&self) -> Instant;
}

/// Identifies an event type.
pub trait Event: 'static {
    /// Event arguments.
    type Args: EventArgs;
}

/// [VisitedVar] signal to stop propagation of event.
pub struct Stop<E: Event> {
    _event: std::marker::PhantomData<E>,
}
impl<E: Event> VisitedVar for Stop<E> {
    type Type = ();
}

struct EventData<T> {
    data: UnsafeCell<Vec<T>>,
    context: AppContextOwnership,
    is_high_pressure: bool,
}

struct EventChannel<T: 'static> {
    r: Rc<EventData<T>>,
}
impl<T: 'static> Clone for EventChannel<T> {
    fn clone(&self) -> Self {
        EventChannel { r: Rc::clone(&self.r) }
    }
}
impl<T: 'static> EventChannel<T> {
    pub(crate) fn notify(self, mut_ctx_id: AppContextId, new_update: T, cleanup: &mut Vec<Box<dyn FnOnce()>>) {
        self.r.context.check(mut_ctx_id, || {
            format!(
                "cannot update `EventChannel<{}>` because it is borrowed in a different context",
                type_name::<T>()
            )
        });

        // SAFETY: This is safe because borrows are bound to a context that
        // is the only place where the value can be changed and this change is
        // only applied when the context is mut.
        let data = unsafe { &mut *self.r.data.get() };
        data.push(new_update);

        if data.len() == 1 {
            // register for cleanup once
            cleanup.push(Box::new(move || {
                unsafe { &mut *self.r.data.get() }.clear();
            }))
        }
    }

    /// Gets a reference to the updates that happened in between calls of [UiNode::update].
    pub fn updates(&self, ctx: &AppContext) -> &[T] {
        self.r.context.check(ctx.id(), || {
            format!(
                "cannot read `EventChannel<{}>` because it is borrowed in a different context",
                type_name::<T>()
            )
        });

        // SAFETY: This is safe because borrows are bound to a context that
        // is the only place where the value can be changed and this change is
        // only applied when the context is mut.
        unsafe { &*self.r.data.get() }.as_ref()
    }

    /// Gets if this update is notified using the [UiNode::update_hp] method.
    pub fn is_high_pressure(&self) -> bool {
        self.r.is_high_pressure
    }
}

/// Read-only reference to an event channel.
pub struct EventListener<T: 'static> {
    chan: EventChannel<T>,
}
impl<T: 'static> Clone for EventListener<T> {
    fn clone(&self) -> Self {
        EventListener {
            chan: self.chan.clone(),
        }
    }
}
impl<T: 'static> EventListener<T> {
    /// Gets a reference to the updates that happened in between calls of [UiNode::update].
    pub fn updates(&self, ctx: &AppContext) -> &[T] {
        self.chan.updates(ctx)
    }

    /// Gets if this update is notified using the [UiNode::update_hp] method.
    pub fn is_high_pressure(&self) -> bool {
        self.chan.is_high_pressure()
    }
}

/// Read-write reference to an event channel.
pub struct EventEmitter<T: 'static> {
    chan: EventChannel<T>,
}
impl<T: 'static> Clone for EventEmitter<T> {
    fn clone(&self) -> Self {
        EventEmitter {
            chan: self.chan.clone(),
        }
    }
}
impl<T: 'static> EventEmitter<T> {
    /// New event emitter.
    ///
    /// # Arguments
    /// * `is_high_pressure`: If this event is notified using the [UiNode::update_hp] method.
    pub fn new(is_high_pressure: bool) -> Self {
        EventEmitter {
            chan: EventChannel {
                r: Rc::new(EventData {
                    data: UnsafeCell::default(),
                    context: AppContextOwnership::default(),
                    is_high_pressure,
                }),
            },
        }
    }

    /// Gets a reference to the updates that happened in between calls of [UiNode::update].
    pub fn updates(&self, ctx: &AppContext) -> &[T] {
        self.chan.updates(ctx)
    }

    /// Gets if this event is notified using the [UiNode::update_hp] method.
    pub fn is_high_pressure(&self) -> bool {
        self.chan.is_high_pressure()
    }

    /// Gets a new event listener linked with this emitter.
    pub fn listener(&self) -> EventListener<T> {
        EventListener {
            chan: self.chan.clone(),
        }
    }

    pub(crate) fn notify(self, mut_ctx_id: AppContextId, new_update: T, cleanup: &mut Vec<Box<dyn FnOnce()>>) {
        self.chan.notify(mut_ctx_id, new_update, cleanup);
    }
}
