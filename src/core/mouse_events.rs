use super::*;
use context::{AppContext, AppInitContext};
pub use glutin::event::ElementState;
use std::time::{Duration, Instant};
pub use webrender::api::units::LayoutPoint;

/// [MouseMove] event args.
#[derive(Debug, Clone)]
pub struct MouseMoveArgs {
    pub timestamp: Instant,
    pub window_id: WindowId,
    pub device_id: DeviceId,
    pub modifiers: ModifiersState,
    pub position: LayoutPoint,
}
impl EventArgs for MouseMoveArgs {
    fn timestamp(&self) -> Instant {
        self.timestamp
    }
}

/// [MouseInput], [MouseDown], [MouseUp] event args.
#[derive(Debug, Clone)]
pub struct MouseInputArgs {
    pub timestamp: Instant,
    pub window_id: WindowId,
    pub device_id: DeviceId,
    pub button: MouseButton,
    pub position: LayoutPoint,
    pub modifiers: ModifiersState,
    pub state: ElementState,
}
impl EventArgs for MouseInputArgs {
    fn timestamp(&self) -> Instant {
        self.timestamp
    }
}

/// [MouseClick] event args.
#[derive(Debug, Clone)]
pub struct MouseClickArgs {
    pub timestamp: Instant,
    pub window_id: WindowId,
    pub device_id: DeviceId,
    pub button: MouseButton,
    pub position: LayoutPoint,
    pub modifiers: ModifiersState,
    pub click_count: u8,
}
impl EventArgs for MouseClickArgs {
    fn timestamp(&self) -> Instant {
        self.timestamp
    }
}

pub struct MouseEvents {
    pos: LayoutPoint,
    modifiers: ModifiersState,
    pos_window: Option<WindowId>,
    last_pressed: Instant,
    click_count: u8,
    mouse_move: EventEmitter<MouseMoveArgs>,
    mouse_input: EventEmitter<MouseInputArgs>,
    mouse_down: EventEmitter<MouseInputArgs>,
    mouse_up: EventEmitter<MouseInputArgs>,
    mouse_click: EventEmitter<MouseClickArgs>,
}

impl Default for MouseEvents {
    fn default() -> Self {
        MouseEvents {
            pos: LayoutPoint::default(),
            modifiers: ModifiersState::default(),
            pos_window: None,
            last_pressed: Instant::now() - Duration::from_secs(60),
            click_count: 0,
            mouse_move: EventEmitter::new(true),
            mouse_input: EventEmitter::new(false),
            mouse_down: EventEmitter::new(false),
            mouse_up: EventEmitter::new(false),
            mouse_click: EventEmitter::new(false),
        }
    }
}

impl AppExtension for MouseEvents {
    fn init(&mut self, r: &mut AppInitContext) {
        r.events.register::<MouseMove>(self.mouse_move.listener());

        r.events.register::<MouseInput>(self.mouse_input.listener());
        r.events.register::<MouseDown>(self.mouse_down.listener());
        r.events.register::<MouseUp>(self.mouse_up.listener());
        r.events.register::<MouseClick>(self.mouse_click.listener());
    }

    fn on_device_event(&mut self, _: DeviceId, event: &DeviceEvent, _: &mut AppContext) {
        if let DeviceEvent::ModifiersChanged(m) = event {
            self.modifiers = *m;
        }
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut AppContext) {
        match *event {
            WindowEvent::MouseInput {
                state,
                device_id,
                button,
                ..
            } => {
                let position = if self.pos_window == Some(window_id) {
                    self.pos
                } else {
                    LayoutPoint::default()
                };
                let args = MouseInputArgs {
                    timestamp: Instant::now(),
                    window_id,
                    device_id,
                    button,
                    position,
                    modifiers: self.modifiers,
                    state,
                };
                ctx.updates.push_notify(self.mouse_input.clone(), args.clone());
                match state {
                    ElementState::Pressed => {
                        ctx.updates.push_notify(self.mouse_down.clone(), args);

                        self.click_count = self.click_count.saturating_add(1);

                        let now = Instant::now();

                        if self.click_count > 1 {
                            if (now - self.last_pressed) < multi_click_time_ms() {
                                let args = MouseClickArgs {
                                    timestamp: Instant::now(),
                                    window_id,
                                    device_id,
                                    button,
                                    position,
                                    modifiers: self.modifiers,
                                    click_count: self.click_count,
                                };
                                ctx.updates.push_notify(self.mouse_click.clone(), args);
                            } else {
                                self.click_count = 1;
                            }
                        }
                        self.last_pressed = now;

                        todo!(r"src\properties\events.rs");
                    }
                    ElementState::Released => {
                        ctx.updates.push_notify(self.mouse_up.clone(), args);

                        if self.click_count == 1 {
                            let args = MouseClickArgs {
                                timestamp: Instant::now(),
                                window_id,
                                device_id,
                                button,
                                position,
                                modifiers: self.modifiers,
                                click_count: 1,
                            };
                            ctx.updates.push_notify(self.mouse_click.clone(), args);
                        }

                        todo!(r"src\properties\events.rs");
                    }
                }
            }
            WindowEvent::CursorMoved {
                device_id, position, ..
            } => {
                let position = LayoutPoint::new(position.x as f32, position.y as f32);
                if position != self.pos || Some(window_id) != self.pos_window {
                    self.pos = position;
                    self.pos_window = Some(window_id);

                    let args = MouseMoveArgs {
                        timestamp: Instant::now(),
                        window_id,
                        device_id,
                        modifiers: self.modifiers,
                        position,
                    };

                    ctx.updates.push_notify(self.mouse_move.clone(), args);
                }
            }
            _ => {}
        }
    }
}

#[cfg(target_os = "windows")]
fn multi_click_time_ms() -> Duration {
    Duration::from_millis(u64::from(unsafe { winapi::um::winuser::GetDoubleClickTime() }))
}

#[cfg(not(target_os = "windows"))]
fn multi_click_time_ms() -> u32 {
    // https://stackoverflow.com/questions/50868129/how-to-get-double-click-time-interval-value-programmatically-on-linux
    // https://developer.apple.com/documentation/appkit/nsevent/1532495-mouseevent
    Duration::from_millis(500)
}

/// Mouse move event.
pub struct MouseMove;

impl Event for MouseMove {
    type Args = MouseMoveArgs;
}

/// Mouse input event.
pub struct MouseInput;

impl Event for MouseInput {
    type Args = MouseInputArgs;
}

/// Mouse down event.
pub struct MouseDown;

impl Event for MouseDown {
    type Args = MouseInputArgs;
}

/// Mouse up event.
pub struct MouseUp;

impl Event for MouseUp {
    type Args = MouseInputArgs;
}

/// Mouse click event.
pub struct MouseClick;

impl Event for MouseClick {
    type Args = MouseClickArgs;
}
