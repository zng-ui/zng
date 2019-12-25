use super::{AppExtension, AppRegister, EventNotifier, EventUpdate, UpdateNotifier};
use glutin::event::{DeviceEvent, DeviceId, Event, WindowEvent};
use glutin::window::WindowId;

pub struct MouseDownArgs {}
pub struct MouseDown {}

impl EventNotifier for MouseDown {
    type Args = MouseDownArgs;
}

pub struct MouseEvents {
    mouse_down: UpdateNotifier<MouseDownArgs>,
}

impl Default for MouseEvents {
    fn default() -> Self {
        MouseEvents {
            mouse_down: UpdateNotifier::new(false, MouseDownArgs {}),
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
