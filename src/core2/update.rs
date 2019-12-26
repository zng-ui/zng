use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;

struct UpdateNote<T> {
    last_update: RefCell<Option<T>>,
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
    n: UpdateNotice<T>,
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

bitflags! {
    /// What to pump in a Ui tree after an update is applied.
    #[derive(Default)]
    pub(crate) struct UpdateFlags: u8 {
        const UPDATE = 0b0000_0001;
        const UPD_HP = 0b0000_0010;
        const LAYOUT = 0b0000_0100;
        const RENDER = 0b0000_1000;
    }
}

/// Schedule updates for next [UiNone::update] call.
#[derive(Default)]
pub struct NextUpdate {
    update: UpdateFlags,
    updates: Vec<Box<dyn FnOnce(&mut Vec<Box<dyn FnOnce()>>)>>,
    cleanup: Vec<Box<dyn FnOnce()>>,
}

/// Error caused by a call to `[notify_reuse](NextUpdate::notify_reuse)` when
/// the notifier has no previous update.
#[derive(Debug)]
pub struct NoLastUpdate;
impl std::error::Error for NoLastUpdate {}
impl std::fmt::Display for NoLastUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "no `last_update`")
    }
}

impl NextUpdate {
    /// Schedules an update notification.
    pub fn notify<T: 'static>(&mut self, sender: &UpdateNotifier<T>, new_update: T) {
        let note = Rc::clone(&sender.n.note);

        self.update.insert(if note.is_high_pressure {
            UpdateFlags::UPD_HP
        } else {
            UpdateFlags::UPDATE
        });

        self.updates.push(Box::new(move |cleanup| {
            *note.last_update.borrow_mut() = Some(new_update);
            note.is_new.set(true);
            cleanup.push(Box::new(move || note.is_new.set(false)));
        }));
    }

    /// Schedules an update notification that only modifies the previous notification.
    pub fn notify_reuse<T: 'static>(
        &mut self,
        sender: &UpdateNotifier<T>,
        modify_update: impl FnOnce(&mut T) + 'static,
    ) -> Result<(), NoLastUpdate> {
        if sender.n.note.last_update.borrow().is_none() {
            return Err(NoLastUpdate);
        }

        let note = Rc::clone(&sender.n.note);

        self.update.insert(if note.is_high_pressure {
            UpdateFlags::UPD_HP
        } else {
            UpdateFlags::UPDATE
        });

        self.updates.push(Box::new(move |cleanup| {
            if let Some(note) = note.last_update.borrow_mut().as_mut() {
                modify_update(note);
            } else {
                panic!("{}", NoLastUpdate)
            }
            note.is_new.set(true);
            cleanup.push(Box::new(move || note.is_new.set(false)));
        }));

        Ok(())
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

/// Global notifier update.
#[derive(Default)]
pub struct EventUpdate {
    update: NextUpdate,
}

impl EventUpdate {
    /// Applies an update notification.
    pub fn notify<T: 'static>(&mut self, sender: &UpdateNotifier<T>, new_update: T) {
        self.update.notify(sender, new_update)
    }

    /// Applies an update notification that only modifies the previous notification.
    pub fn notify_reuse<T: 'static>(
        &mut self,
        sender: &UpdateNotifier<T>,
        modify_update: impl FnOnce(&mut T) + 'static,
    ) -> Result<(), NoLastUpdate> {
        self.update.notify_reuse(sender, modify_update)
    }

    /// Returns what updates where applied.
    #[inline]
    pub(crate) fn apply(&mut self) -> UpdateFlags {
        self.update.apply()
    }
}
