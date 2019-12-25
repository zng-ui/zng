use crate::core::{LayoutSize, NextFrame, UiValues, WebRenderEvent};
use fnv::FnvHashMap;
use glutin::event::{DeviceEvent, DeviceId, Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowId;
use std::any::{Any, TypeId};
use std::cell::{Cell, Ref, RefCell};
use std::ops::Deref;
use std::rc::{Rc, Weak};

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

/// Weak reference to an update channel.
pub struct UpdateNotifier<T> {
    note: Weak<UpdateNote<T>>,
}

impl<T> Clone for UpdateNotifier<T> {
    fn clone(&self) -> Self {
        UpdateNotifier {
            note: Weak::clone(&self.note),
        }
    }
}

impl<T> UpdateNotifier<T> {
    /// Makes a new listener.
    pub fn upgrade(&self) -> Result<UpdateNotice<T>, DeadUpdateChannel> {
        if let Some(note) = self.note.upgrade() {
            Ok(UpdateNotice { note })
        } else {
            Err(DeadUpdateChannel)
        }
    }
}

/// Strong reference to an update channel.
pub struct StrongUpdateNotifier<T> {
    note: Rc<UpdateNote<T>>,
}
impl<T> StrongUpdateNotifier<T> {
    pub fn new(is_high_pressure: bool, first_update: T) -> Self {
        let note = Rc::new(UpdateNote {
            last_update: RefCell::new(first_update),
            is_new: Cell::new(true),
            is_high_pressure,
        });

        StrongUpdateNotifier { note }
    }
    pub fn listener(&self) -> UpdateNotice<T> {
        UpdateNotice {
            note: Rc::clone(&self.note),
        }
    }
}

/// [UpdateSender] has no linked [UpdateListener] alive so the channel was droped.
#[derive(Debug)]
pub struct DeadUpdateChannel;

impl std::fmt::Display for DeadUpdateChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "`UpdateSender` has no linked `UpdateListener` alive so the channel was droped"
        )
    }
}
impl std::error::Error for DeadUpdateChannel {}

/// Creates a new update channel.
///
/// # Arguments
/// * `is_high_pressure`: If this update is notified using the [UiNode::update_hp].
pub fn update_channel<T>(is_high_pressure: bool, first_update: T) -> (UpdateNotifier<T>, UpdateNotice<T>) {
    let note = Rc::new(UpdateNote {
        last_update: RefCell::new(first_update),
        is_new: Cell::new(true),
        is_high_pressure,
    });

    (
        UpdateNotifier {
            note: Rc::downgrade(&note),
        },
        UpdateNotice { note },
    )
}

mod private {
    pub trait Sealed {}
}

/// Abstraction over a direct owned `T` or an `UpdateNotice<T>`.
pub trait Var<T>: private::Sealed {
    type RefType: for<'a> VarRefType<'a, T>;

    /// Borrows the value. Returns `&T` when owned or `Ref<T>` when it is an update notice.
    fn borrow(&self) -> <Self::RefType as VarRefType<'_, T>>::Type;

    /// If the value was just updated. Always false if owned or the same as [UpdateNotice::is_new].
    fn is_new(&self) -> bool;
}

#[doc(hidden)]
pub trait VarRefType<'a, T: 'a> {
    type Type: Deref<Target = T>;
}

pub struct OwnedVar<T>(pub T);

impl<'a, T: 'a> VarRefType<'a, T> for OwnedVar<T> {
    type Type = &'a T;
}

impl<'a, T: 'a> VarRefType<'a, T> for UpdateNotice<T> {
    type Type = Ref<'a, T>;
}

impl<T> private::Sealed for OwnedVar<T> {}
impl<T: 'static> Var<T> for OwnedVar<T> {
    type RefType = Self;

    fn borrow(&self) -> &T {
        &self.0
    }

    fn is_new(&self) -> bool {
        false
    }
}

impl<T> private::Sealed for UpdateNotice<T> {}
impl<T: 'static> Var<T> for UpdateNotice<T> {
    type RefType = Self;

    fn borrow(&self) -> Ref<T> {
        UpdateNotice::last_update(self)
    }

    fn is_new(&self) -> bool {
        UpdateNotice::is_new(self)
    }
}

/// An Ui tree node.
pub trait UiNode: 'static {
    /// Called every time the node is plugged in an Ui tree.
    fn init(&mut self, values: &mut UiValues, update: &mut NextUpdate);

    /// Called every time the node is unplugged from an Ui tree.
    fn deinit(&mut self, values: &mut UiValues, update: &mut NextUpdate);

    /// Called every time a low pressure event update happens.
    ///
    /// # Event Pressure
    /// See [update_hp] for more information about event pressure rate.
    fn update(&mut self, values: &mut UiValues, update: &mut NextUpdate);

    /// Called every time a high pressure event update happens.
    ///
    /// # Event Pressure
    /// Some events occur alot more times then others, for performance reasons this
    /// event source may choose to be propagated in the this hight pressure lane.
    ///
    /// Event sources that are high pressure mention this in their documentation.
    fn update_hp(&mut self, values: &mut UiValues, update: &mut NextUpdate);

    /// Called every time a layout update is needed.
    ///
    /// # Arguments
    /// * `available_size`: The total available size for the node. Can contain positive infinity to
    /// indicate the parent will accommodate any size.
    ///
    /// # Return
    /// Must return the nodes desired size. Must not contain infinity or NaN.
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize;

    /// Called every time a layout update is needed, after [measure].
    ///
    /// # Arguments
    /// * `final_size`: The size the parent node reserved for the node. Must reposition its contents
    /// to fit this size. The value does not contain infinity or NaNs.
    fn arrange(&mut self, final_size: LayoutSize);

    /// Called every time a new frame must be rendered.
    ///
    /// # Arguments
    /// * `f`: Contains the next frame draw instructions.
    fn render(&self, f: &mut NextFrame);

    /// Box this component, unless it is already `Box<dyn UiNode>`.
    fn into_box(self) -> Box<dyn UiNode>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
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
    pub fn notify<T: 'static>(&mut self, sender: &UpdateNotifier<T>, new_update: T) -> Result<(), DeadUpdateChannel> {
        self.notify_reuse(sender, move |u| *u = new_update)
    }

    /// Applies an update notification that only modifies the previous notification.
    pub fn notify_reuse<T: 'static>(
        &mut self,
        sender: &UpdateNotifier<T>,
        modify_update: impl FnOnce(&mut T) + 'static,
    ) -> Result<(), DeadUpdateChannel> {
        if let Some(note) = sender.note.upgrade() {
            self.update.insert(if note.is_high_pressure {
                UpdateFlags::UPD_HP
            } else {
                UpdateFlags::UPDATE
            });
            modify_update(&mut *note.last_update.borrow_mut());
            note.is_new.set(true);
            self.cleanup.push(Box::new(move || note.is_new.set(false)));
            Ok(())
        } else {
            Err(DeadUpdateChannel)
        }
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
    pub fn notify<T: 'static>(&mut self, sender: &UpdateNotifier<T>, new_update: T) -> Result<(), DeadUpdateChannel> {
        self.notify_reuse(sender, move |u| *u = new_update)
    }

    /// Schedules an update notification that only modifies the previous notification.
    pub fn notify_reuse<T: 'static>(
        &mut self,
        sender: &UpdateNotifier<T>,
        modify_update: impl FnOnce(&mut T) + 'static,
    ) -> Result<(), DeadUpdateChannel> {
        if let Some(note) = sender.note.upgrade() {
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

            Ok(())
        } else {
            Err(DeadUpdateChannel)
        }
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

#[derive(Default)]
pub struct AppRegister {
    events: FnvHashMap<TypeId, Box<dyn Any>>,
}

impl AppRegister {
    pub fn register_event<E: EventNotifier>(&mut self, listener: UpdateNotice<E::Args>) {
        self.events.insert(TypeId::of::<E>(), Box::new(listener));
    }

    pub fn listener<E: EventNotifier>(&self) -> Option<UpdateNotice<E::Args>> {
        if let Some(any) = self.events.get(&TypeId::of::<E>()) {
            any.downcast_ref::<UpdateNotice<E::Args>>().cloned()
        } else {
            None
        }
    }
}

/// An [App] extension.
pub trait AppExtension: 'static {
    /// Register this extension.
    fn register(&mut self, r: &mut AppRegister);

    /// Called when the OS sends an event to a device.
    fn on_device_event(&mut self, _device_id: DeviceId, _event: &DeviceEvent, _update: &mut EventUpdate) {}

    /// Called when the OS sends an event to a window.
    fn on_window_event(&mut self, _window_id: WindowId, _event: &WindowEvent, _update: &mut EventUpdate) {}
}

impl<A: AppExtension, B: AppExtension> AppExtension for (A, B) {
    fn register(&mut self, r: &mut AppRegister) {
        self.0.register(r);
        self.1.register(r);
    }

    fn on_device_event(&mut self, device_id: DeviceId, event: &DeviceEvent, update: &mut EventUpdate) {
        self.0.on_device_event(device_id, event, update);
        self.1.on_device_event(device_id, event, update);
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, update: &mut EventUpdate) {
        self.0.on_window_event(window_id, event, update);
        self.1.on_window_event(window_id, event, update);
    }
}

impl AppExtension for () {
    fn register(&mut self, _: &mut AppRegister) {}
}

/// Identifies an event type.
pub trait EventNotifier: 'static {
    /// Event arguments.
    type Args: 'static;
}

pub struct MouseDownArgs {}
pub struct MouseDown {}

impl EventNotifier for MouseDown {
    type Args = MouseDownArgs;
}

pub struct MouseEvents {
    mouse_down: StrongUpdateNotifier<MouseDownArgs>,
}

impl Default for MouseEvents {
    fn default() -> Self {
        MouseEvents {
            mouse_down: StrongUpdateNotifier::new(false, MouseDownArgs {}),
        }
    }
}

impl AppExtension for MouseEvents {
    fn register(&mut self, r: &mut AppRegister) {
        r.register_event::<MouseDown>(self.mouse_down.listener())
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, update: &mut EventUpdate) {
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

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, update: &mut EventUpdate) {}
}

pub struct App<Exts: AppExtension> {
    extensions: Exts,
}

impl<E: AppExtension> App<E> {
    /// Application without any extension.
    pub fn empty() -> App<()> {
        App { extensions: () }
    }

    /// Application with default extensions.
    pub fn default() -> App<(MouseEvents, KeyboardEvents)> {
        App {
            extensions: (MouseEvents::default(), KeyboardEvents::default()),
        }
    }

    /// Includes an [AppExtension] in the application.
    pub fn extend<F: AppExtension>(self, extension: F) -> App<(E, F)> {
        App {
            extensions: (self.extensions, extension),
        }
    }

    /// Runs the application.
    pub fn run(self) -> ! {
        let App { mut extensions } = self;

        let mut register = AppRegister::default();
        extensions.register(&mut register);

        let event_loop = EventLoop::with_user_event();
        let mut in_event_sequence = false;
        let mut event_update = EventUpdate::default();

        event_loop.run(move |event, event_loop, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::NewEvents(_) => {
                    in_event_sequence = true;
                }
                Event::EventsCleared => {
                    in_event_sequence = false;
                }

                Event::WindowEvent { window_id, event } => {
                    extensions.on_window_event(window_id, &event, &mut event_update);
                }
                Event::UserEvent(WebRenderEvent::NewFrameReady(_window_id)) => {}
                Event::DeviceEvent { device_id, event } => {
                    extensions.on_device_event(device_id, &event, &mut event_update);
                }
                _ => {}
            }

            if !in_event_sequence {
                let updates = event_update.apply();

                if updates.contains(UpdateFlags::UPDATE) {
                    todo!();
                }
                if updates.contains(UpdateFlags::UPD_HP) {
                    todo!();
                }
                if updates.contains(UpdateFlags::LAYOUT) {
                    todo!();
                }
                if updates.contains(UpdateFlags::RENDER) {
                    todo!();
                }
            }
        })
    }
}
