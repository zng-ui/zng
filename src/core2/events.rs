use super::{AppExtension, AppRegister, Event, EventContext, VisitedVar, WindowEvent, WindowId};
use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;

/// [VisitedVar] signal to stop propagation of event.
pub struct Stop<E: Event> {
    _event: std::marker::PhantomData<E>,
}

impl<E: Event> VisitedVar for Stop<E> {
    type Type = ();
}

pub(crate) struct UpdateNote<T> {
    pub(crate) last_update: RefCell<Option<T>>,
    pub(crate) is_new: Cell<bool>,
    pub(crate) is_high_pressure: bool,
}

/// Strong reference to an update channel.
pub struct UpdateNotice<T> {
    pub(crate) note: Rc<UpdateNote<T>>,
}

impl<T> Clone for UpdateNotice<T> {
    fn clone(&self) -> Self {
        UpdateNotice {
            note: Rc::clone(&self.note),
        }
    }
}

impl<T> UpdateNotice<T> {
    /// Gets a reference to the last update.
    pub fn last_update(&self) -> Option<Ref<T>> {
        let b = self.note.last_update.borrow();
        if b.is_none() {
            None
        } else {
            Some(Ref::map(b, |s| s.as_ref().unwrap()))
        }
    }

    /// Gets a reference to [last_update] if [is_new].
    pub fn new_update(&self) -> Option<Ref<T>> {
        if self.is_new() {
            self.last_update()
        } else {
            None
        }
    }

    /// Gets if the [last_update](UpdateNotice::last_update) is new.
    ///
    /// This flag stays true only for one update cicle.
    pub fn is_new(&self) -> bool {
        self.note.is_new.get()
    }

    /// Gets if this update is notified using the [UiNode::update_hp] method.
    pub fn is_high_pressure(&self) -> bool {
        self.note.is_high_pressure
    }
}

/// Strong reference to an update channel.
pub struct UpdateNotifier<T> {
    pub(crate) n: UpdateNotice<T>,
}
impl<T> UpdateNotifier<T> {
    /// Starts a new update notifier.
    ///
    /// # Arguments
    /// * `is_high_pressure`: If this notifier must be observed in the [UiNode::update_hp] band.
    /// * `initial_value`: An initial value for `[last_update]`. Please note that `[is_new]` starts at `false`.
    pub fn new(is_high_pressure: bool) -> Self {
        let note = Rc::new(UpdateNote {
            last_update: RefCell::new(None),
            is_new: Cell::new(false),
            is_high_pressure,
        });

        UpdateNotifier {
            n: UpdateNotice { note },
        }
    }

    /// Gets a reference to the last update.
    pub fn last_update(&self) -> Option<Ref<T>> {
        self.n.last_update()
    }

    /// Gets a reference to [last_update] if [is_new].
    pub fn new_update(&self) -> Option<Ref<T>> {
        self.n.new_update()
    }

    /// Gets if the [last_update](UpdateNotice::last_update) is new.
    ///
    /// This flag stays true only for one update cicle.
    pub fn is_new(&self) -> bool {
        self.n.is_new()
    }

    /// Gets if this update is notified using the [UiNode::update_hp] method.
    pub fn is_high_pressure(&self) -> bool {
        self.n.is_high_pressure()
    }

    /// Gets a new update listener.
    pub fn listener(&self) -> UpdateNotice<T> {
        self.n.clone()
    }
}

/// [MouseDown] event args.
#[derive(Debug, Clone)]
pub struct MouseDownArgs {}

/// Mouse down event.
pub struct MouseDown {}

impl Event for MouseDown {
    type Args = MouseDownArgs;
}

pub struct MouseEvents {
    mouse_down: UpdateNotifier<MouseDownArgs>,
}

impl Default for MouseEvents {
    fn default() -> Self {
        MouseEvents {
            mouse_down: UpdateNotifier::new(false),
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
