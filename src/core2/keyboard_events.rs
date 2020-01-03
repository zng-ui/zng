use super::*;
use glutin::event::KeyboardInput;
pub use glutin::event::{ScanCode, VirtualKeyCode};
use std::time::{Duration, Instant};
pub use webrender::api::LayoutPoint;

pub type Key = VirtualKeyCode;

/// [KeyInput] event args.
#[derive(Debug, Clone)]
pub struct KeyInputArgs {
    pub timestamp: Instant,
    pub window_id: WindowId,
    pub device_id: DeviceId,
    pub scancode: ScanCode,
    pub state: ElementState,
    pub key: Option<Key>,
    pub modifiers: ModifiersState,
}

pub struct KeyboardEvents {
    key_input: EventEmitter<KeyInputArgs>,
    key_down: EventEmitter<KeyInputArgs>,
    key_up: EventEmitter<KeyInputArgs>,
}

impl Default for KeyboardEvents {
    fn default() -> Self {
        KeyboardEvents {
            key_input: EventEmitter::new(false),
            key_down: EventEmitter::new(false),
            key_up: EventEmitter::new(false),
        }
    }
}

impl AppExtension for KeyboardEvents {
    fn register(&mut self, r: &mut AppRegister) {}

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut EventContext) {
        match *event {
            WindowEvent::KeyboardInput {
                device_id,
                input:
                    KeyboardInput {
                        scancode,
                        state,
                        virtual_keycode: key,
                        modifiers,
                    },
                ..
            } => {
                let args = KeyInputArgs {
                    timestamp: Instant::now(),
                    window_id,
                    device_id,
                    scancode,
                    key,
                    modifiers,
                    state,
                };

                ctx.push_notify(self.key_input.clone(), args.clone());

                match state {
                    ElementState::Pressed => {
                        ctx.push_notify(self.key_down.clone(), args);
                        todo!()
                    }
                    ElementState::Released => {
                        ctx.push_notify(self.key_up.clone(), args);
                    }
                }
            }
            _ => {}
        }
    }
}
