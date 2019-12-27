use super::{AppContext, AppContextId, AppExtension, AppRegister, EventContext, VisitedVar, WindowEvent, WindowId};
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
    data: UnsafeCell<Option<T>>,
    borrowed: Cell<Option<AppContextId>>,
    is_new: Cell<bool>,
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
        if let Some(ctx_id) = self.r.borrowed.get() {
            if ctx_id != mut_ctx_id {
                panic!(
                    "cannot update `EventChannel<{}>` because it is borrowed in a different context",
                    type_name::<T>()
                )
            }
            self.r.borrowed.set(None);
        }

        // SAFETY: This is safe because borrows are bound to a context that
        // is the only place where the value can be changed and this change is
        // only applied when the context is mut.
        unsafe {
            *self.r.data.get() = Some(new_update);
        }

        cleanup.push(Box::new(move || self.r.is_new.set(false)));
    }

    /// Gets a reference to the last event arguments.
    pub fn last_update(&self, ctx: &AppContext) -> Option<&T> {
        let id = ctx.id();
        if let Some(ctx_id) = self.r.borrowed.get() {
            if ctx_id != id {
                panic!(
                    "`EventChannel<{}>` is already borrowed in a different `AppContext`",
                    type_name::<T>()
                )
            }
        } else {
            self.r.borrowed.set(Some(id));
        }

        // SAFETY: This is safe because borrows are bound to a context that
        // is the only place where the value can be changed and this change is
        // only applied when the context is mut.
        unsafe { &*self.r.data.get() }.as_ref()
    }

    /// Gets a reference to [last_update] if [is_new].
    pub fn update(&self, ctx: &AppContext) -> Option<&T> {
        if self.r.is_new.get() {
            self.last_update(ctx)
        } else {
            None
        }
    }

    /// Gets if the [last_update](EventChannel::last_update) is new.
    ///
    /// This flag stays true only for one update cicle.
    pub fn is_new(&self) -> bool {
        self.r.is_new.get()
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
    /// Gets a reference to the last event arguments.
    pub fn last_update(&self, ctx: &AppContext) -> Option<&T> {
        self.chan.last_update(ctx)
    }

    /// Gets a reference to [last_update] if [is_new].
    pub fn update(&self, ctx: &AppContext) -> Option<&T> {
        self.chan.update(ctx)
    }

    /// Gets if the [last_update](EventChannel::last_update) is new.
    ///
    /// This flag stays true only for one update cicle.
    pub fn is_new(&self) -> bool {
        self.chan.is_new()
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
                    borrowed: Cell::default(),
                    is_new: Cell::default(),
                    is_high_pressure,
                }),
            },
        }
    }

    /// Gets a reference to the last event arguments.
    pub fn last_update(&self, ctx: &AppContext) -> Option<&T> {
        self.chan.last_update(ctx)
    }

    /// Gets a reference to [last_update] if [is_new].
    pub fn update(&self, ctx: &AppContext) -> Option<&T> {
        self.chan.update(ctx)
    }

    /// Gets if the [last_update](EventChannel::last_update) is new.
    ///
    /// This flag stays true only for one update cicle.
    pub fn is_new(&self) -> bool {
        self.chan.is_new()
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

/// [MouseDown] event args.
#[derive(Debug, Clone)]
pub struct MouseDownArgs {
    pub timestamp: Instant,
}
impl EventArgs for MouseDownArgs {
    fn timestamp(&self) -> Instant {
        self.timestamp
    }
}

/// Mouse down event.
pub struct MouseDown;

impl Event for MouseDown {
    type Args = MouseDownArgs;
}

pub struct MouseEvents {
    mouse_down: EventEmitter<MouseDownArgs>,
}

impl Default for MouseEvents {
    fn default() -> Self {
        MouseEvents {
            mouse_down: EventEmitter::new(false),
        }
    }
}

impl AppExtension for MouseEvents {
    fn register(&mut self, r: &mut AppRegister) {
        r.register_event::<MouseDown>(self.mouse_down.listener())
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut EventContext) {
        //update.notify(sender: &UpdateNotifier<T>, new_update: T)
    }
}

pub struct KeyboardEvents {}

impl Default for KeyboardEvents {
    fn default() -> Self {
        KeyboardEvents {}
    }
}

impl AppExtension for KeyboardEvents {
    fn register(&mut self, r: &mut AppRegister) {}

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut EventContext) {}
}
