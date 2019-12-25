use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;

struct UpdateNote<T> {
    last_update: RefCell<T>,
    is_new: Cell<bool>,
    is_high_pressure: bool,
}

/// Strong reference to an update channel.
pub struct UpdateNotice<T> {
    note: Rc<UpdateNote<T>>,
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
    pub fn last_update(&self) -> Ref<T> {
        self.note.last_update.borrow()
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
    note: Rc<UpdateNote<T>>,
}
impl<T> UpdateNotifier<T> {
    /// Starts a new update notifier.
    ///
    /// # Arguments
    /// * `is_high_pressure`: If this notifier must be observed in the [UiNode::update_hp] band.
    /// * `initial_value`: An initial value for `[last_update]`. Please note that `[is_new]` starts at `false`.
    pub fn new(is_high_pressure: bool, initial_value: T) -> Self {
        let note = Rc::new(UpdateNote {
            last_update: RefCell::new(initial_value),
            is_new: Cell::new(false),
            is_high_pressure,
        });

        UpdateNotifier { note }
    }

    /// Gets a reference to the last update.
    pub fn last_update(&self) -> Ref<T> {
        self.note.last_update.borrow()
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

    /// Gets a new update listener.
    pub fn listener(&self) -> UpdateNotice<T> {
        UpdateNotice {
            note: Rc::clone(&self.note),
        }
    }
}

bitflags! {
    /// What to pump in a Ui tree after an update is applied.
    #[derive(Default)]
    pub struct UpdateFlags: u8 {
        const UPDATE = 0b0000_0001;
        const UPD_HP = 0b0000_0010;
        const LAYOUT = 0b0000_0100;
        const RENDER = 0b0000_1000;
    }
}

/// Global notifier update.
#[derive(Default)]
pub struct EventUpdate {
    update: UpdateFlags,
    cleanup: Vec<Box<dyn FnOnce()>>,
}

impl EventUpdate {
    /// Applies an update notification.
    pub fn notify<T: 'static>(&mut self, sender: &UpdateNotifier<T>, new_update: T) {
        self.notify_reuse(sender, move |u| *u = new_update)
    }

    /// Applies an update notification that only modifies the previous notification.
    pub fn notify_reuse<T: 'static>(
        &mut self,
        sender: &UpdateNotifier<T>,
        modify_update: impl FnOnce(&mut T) + 'static,
    ) {
        let note = Rc::clone(&sender.note);

        self.update.insert(if note.is_high_pressure {
            UpdateFlags::UPD_HP
        } else {
            UpdateFlags::UPDATE
        });
        modify_update(&mut *note.last_update.borrow_mut());
        note.is_new.set(true);
        self.cleanup.push(Box::new(move || note.is_new.set(false)));
    }

    /// Returns what updates where applied.
    #[inline]
    pub fn apply(&mut self) -> UpdateFlags {
        std::mem::replace(&mut self.update, UpdateFlags::empty())
    }
}

/// Schedule updates for next [UiNone::update] call.
pub struct NextUpdate {
    update: UpdateFlags,
    updates: Vec<Box<dyn FnOnce(&mut Vec<Box<dyn FnOnce()>>)>>,
    cleanup: Vec<Box<dyn FnOnce()>>,
}

impl NextUpdate {
    /// Schedules an update notification.
    pub fn notify<T: 'static>(&mut self, sender: &UpdateNotifier<T>, new_update: T) {
        self.notify_reuse(sender, move |u| *u = new_update)
    }

    /// Schedules an update notification that only modifies the previous notification.
    pub fn notify_reuse<T: 'static>(
        &mut self,
        sender: &UpdateNotifier<T>,
        modify_update: impl FnOnce(&mut T) + 'static,
    ) {
        let note = Rc::clone(&sender.note);

        self.update.insert(if note.is_high_pressure {
            UpdateFlags::UPD_HP
        } else {
            UpdateFlags::UPDATE
        });

        self.updates.push(Box::new(move |cleanup| {
            modify_update(&mut *note.last_update.borrow_mut());
            note.is_new.set(true);
            cleanup.push(Box::new(move || note.is_new.set(false)));
        }));
    }

    /// Cleanup the previous update and applies the new one.
    ///
    /// Returns what update methods must be pumped.
    pub(crate) fn apply(&mut self) -> UpdateFlags {
        for cleanup in self.cleanup.drain(..) {
            cleanup();
        }

        for update in self.updates.drain(..) {
            update(&mut self.cleanup);
        }

        std::mem::replace(&mut self.update, UpdateFlags::empty())
    }
}
