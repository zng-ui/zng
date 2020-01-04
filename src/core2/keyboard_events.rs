use super::*;
use glutin::event::KeyboardInput;
pub use glutin::event::{ScanCode, VirtualKeyCode};
use std::time::Instant;
pub use webrender::api::LayoutPoint;

pub type Key = VirtualKeyCode;

/// [KeyInput], [KeyDown], [KeyUp] event args.
#[derive(Debug, Clone)]
pub struct KeyInputArgs {
    pub timestamp: Instant,
    pub window_id: WindowId,
    pub device_id: DeviceId,
    pub scancode: ScanCode,
    pub state: ElementState,
    pub key: Option<Key>,
    pub modifiers: ModifiersState,
    pub repeat: bool,
}

impl EventArgs for KeyInputArgs {
    fn timestamp(&self) -> Instant {
        self.timestamp
    }
}

pub struct KeyboardEvents {
    last_key_down: Option<ScanCode>,
    key_input: EventEmitter<KeyInputArgs>,
    key_down: EventEmitter<KeyInputArgs>,
    key_up: EventEmitter<KeyInputArgs>,
}

impl Default for KeyboardEvents {
    fn default() -> Self {
        KeyboardEvents {
            last_key_down: None,
            key_input: EventEmitter::new(false),
            key_down: EventEmitter::new(false),
            key_up: EventEmitter::new(false),
        }
    }
}

impl AppExtension for KeyboardEvents {
    fn register(&mut self, r: &mut AppRegister) {
        r.register_event::<KeyInput>(self.key_input.listener());
        r.register_event::<KeyDown>(self.key_down.listener());
        r.register_event::<KeyUp>(self.key_up.listener());
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut EventContext) {
        if let WindowEvent::KeyboardInput {
            device_id,
            input:
                KeyboardInput {
                    scancode,
                    state,
                    virtual_keycode: key,
                    modifiers,
                },
            ..
        } = *event
        {
            let mut repeat = false;
            if state == ElementState::Pressed {
                repeat = self.last_key_down == Some(scancode);
                if !repeat {
                    self.last_key_down = Some(scancode);
                }
            } else {
                self.last_key_down = None;
            }

            let args = KeyInputArgs {
                timestamp: Instant::now(),
                window_id,
                device_id,
                scancode,
                key,
                modifiers,
                state,
                repeat,
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
    }
}

pub struct KeyInput;
impl Event for KeyInput {
    type Args = KeyInputArgs;
}

pub struct KeyDown;
impl Event for KeyDown {
    type Args = KeyInputArgs;
}

pub struct KeyUp;
impl Event for KeyUp {
    type Args = KeyInputArgs;
}
