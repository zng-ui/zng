//! Mouse events.

use crate::core::app::*;
use crate::core::context::*;
use crate::core::event::*;
use crate::core::render::*;
use crate::core::types::*;
use crate::core::window::Windows;
use std::num::NonZeroU8;
use std::time::*;

event_args! {
    /// [MouseMove] event args.
    pub struct MouseMoveArgs {
        pub window_id: WindowId,
        pub device_id: DeviceId,
        pub modifiers: ModifiersState,
        pub position: LayoutPoint,
        pub hits: FrameHitInfo,

        ..

        /// If the widget is in [hits](MouseMoveArgs::hits).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.hits.contains(ctx.widget_id)
         }
    }

    /// [MouseInput], [MouseDown], [MouseUp] event args.
    pub struct MouseInputArgs {
        pub window_id: WindowId,
        pub device_id: DeviceId,
        pub button: MouseButton,
        pub position: LayoutPoint,
        pub modifiers: ModifiersState,
        pub state: ElementState,
        pub hits: FrameHitInfo,

        ..

        /// If the widget is in [hits](MouseInputArgs::hits).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.hits.contains(ctx.widget_id)
        }
    }

    /// [MouseClick] event args.
    pub struct MouseClickArgs {
        pub window_id: WindowId,
        pub device_id: DeviceId,
        pub button: MouseButton,
        pub position: LayoutPoint,
        pub modifiers: ModifiersState,

        /// Sequential click count . Number `1` is single click, `2` is double click, etc.
        pub click_count: NonZeroU8,

        /// Widgets that got clicked, only the widgets that where clicked are in the hit-info.
        ///
        /// A widget is clicked if the [MouseDown] and [MouseUp] happen
        /// in sequence in the same widget. Subsequent clicks (double, triple)
        /// happen on [MouseDown].
        pub hits: FrameHitInfo,

        ..

        /// If the widget is in [hits](MouseClickArgs::hits).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.hits.contains(ctx.widget_id)
        }
    }
}

/// Mouse move event.
pub struct MouseMove;
impl Event for MouseMove {
    type Args = MouseMoveArgs;
    const IS_HIGH_PRESSURE: bool = true;
}

/// Mouse down or up event.
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

/// Mouse click event, any [click_count](MouseClickArgs::click_count).
pub struct MouseClick;
impl Event for MouseClick {
    type Args = MouseClickArgs;
}

/// Mouse singleclick event ([click_count](MouseClickArgs::click_count) = `1`).
pub struct MouseSingleClick;
impl Event for MouseSingleClick {
    type Args = MouseClickArgs;
}

/// Mouse double-click event ([click_count](MouseClickArgs::click_count) = `2`).
pub struct MouseDoubleClick;
impl Event for MouseDoubleClick {
    type Args = MouseClickArgs;
}

/// Mouse triple-click event ([click_count](MouseClickArgs::click_count) = `3`).
pub struct MouseTripleClick;
impl Event for MouseTripleClick {
    type Args = MouseClickArgs;
}

/// Application extension that provides mouse events.
///
/// # Events
///
/// Events this extension provides.
///
/// * [MouseMove]
/// * [MouseInput]
/// * [MouseDown]
/// * [MouseUp]
/// * [MouseClick]
/// * [MouseSingleClick]
/// * [MouseDoubleClick]
/// * [MouseTripleClick]
pub struct MouseEvents {
    /// last cursor move position.
    pos: LayoutPoint,
    /// last cursor move window.
    pos_window: Option<WindowId>,

    /// last modifiers.
    modifiers: ModifiersState,

    /// when the last mouse_down event happened.
    last_pressed: Instant,
    click_hits: Option<FrameHitInfo>,
    click_count: u8,

    mouse_move: EventEmitter<MouseMoveArgs>,

    mouse_input: EventEmitter<MouseInputArgs>,
    mouse_down: EventEmitter<MouseInputArgs>,
    mouse_up: EventEmitter<MouseInputArgs>,

    mouse_click: EventEmitter<MouseClickArgs>,
    mouse_single_click: EventEmitter<MouseClickArgs>,
    mouse_double_click: EventEmitter<MouseClickArgs>,
    mouse_triple_click: EventEmitter<MouseClickArgs>,
}

impl Default for MouseEvents {
    fn default() -> Self {
        MouseEvents {
            pos: LayoutPoint::default(),
            pos_window: None,

            modifiers: ModifiersState::default(),

            last_pressed: Instant::now() - Duration::from_secs(60),
            click_hits: None,
            click_count: 0,

            mouse_move: EventEmitter::new(true),

            mouse_input: EventEmitter::new(false),
            mouse_down: EventEmitter::new(false),
            mouse_up: EventEmitter::new(false),

            mouse_click: EventEmitter::new(false),
            mouse_single_click: EventEmitter::new(false),
            mouse_double_click: EventEmitter::new(false),
            mouse_triple_click: EventEmitter::new(false),
        }
    }
}

impl MouseEvents {
    fn on_mouse_input(&mut self, window_id: WindowId, device_id: DeviceId, state: ElementState, button: MouseButton, ctx: &mut AppContext) {
        let position = if self.pos_window == Some(window_id) {
            self.pos
        } else {
            LayoutPoint::default()
        };

        let hits = ctx.services.req::<Windows>().hit_test(window_id, position).unwrap();
        let args = MouseInputArgs::now(window_id, device_id, button, position, self.modifiers, state, hits.clone());

        // on_mouse_input
        ctx.updates.push_notify(self.mouse_input.clone(), args.clone());

        match state {
            ElementState::Pressed => {
                // on_mouse_down
                ctx.updates.push_notify(self.mouse_down.clone(), args);

                self.click_count = self.click_count.saturating_add(1);
                let now = Instant::now();

                if self.click_count == 1 {
                    self.click_hits = Some(hits);
                } else if self.click_count >= 2 && (now - self.last_pressed) < multi_click_time_ms() {
                    // if click_count >= 2 and the time is in multi-click range.
                    let hits = self.click_hits.as_ref().unwrap().intersection(&hits);

                    let args = MouseClickArgs::new(
                        now,
                        window_id,
                        device_id,
                        button,
                        position,
                        self.modifiers,
                        NonZeroU8::new(self.click_count).unwrap(),
                        hits.clone(),
                    );

                    self.click_hits = Some(hits);

                    // on_mouse_click (click_count > 1)

                    if self.click_count == 2 {
                        if self.mouse_double_click.has_listeners() {
                            ctx.updates.push_notify(self.mouse_double_click.clone(), args.clone());
                        }
                    } else if self.click_count == 3 && self.mouse_triple_click.has_listeners() {
                        ctx.updates.push_notify(self.mouse_triple_click.clone(), args.clone());
                    }

                    ctx.updates.push_notify(self.mouse_click.clone(), args);
                } else {
                    self.click_count = 1;
                    self.click_hits = None;
                }
                self.last_pressed = now;
            }
            ElementState::Released => {
                // on_mouse_up
                ctx.updates.push_notify(self.mouse_up.clone(), args);

                if let Some(click_count) = NonZeroU8::new(self.click_count) {
                    if click_count.get() == 1 {
                        let hits = self.click_hits.as_ref().unwrap().intersection(&hits);

                        let args = MouseClickArgs::now(window_id, device_id, button, position, self.modifiers, click_count, hits.clone());

                        self.click_hits = Some(hits);

                        if self.mouse_single_click.has_listeners() {
                            ctx.updates.push_notify(self.mouse_single_click.clone(), args.clone());
                        }

                        // on_mouse_click
                        ctx.updates.push_notify(self.mouse_click.clone(), args);
                    }
                }
            }
        }
    }

    fn on_cursor_moved(&mut self, window_id: WindowId, device_id: DeviceId, position: LayoutPoint, ctx: &mut AppContext) {
        if position != self.pos || Some(window_id) != self.pos_window {
            self.pos = position;
            self.pos_window = Some(window_id);
            let hits = ctx.services.req::<Windows>().hit_test(window_id, position).unwrap();
            let args = MouseMoveArgs::now(window_id, device_id, self.modifiers, position, hits);

            ctx.updates.push_notify(self.mouse_move.clone(), args);
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
        r.events.register::<MouseClick>(self.mouse_click.listener());
        r.events.register::<MouseDoubleClick>(self.mouse_double_click.listener());
        r.events.register::<MouseTripleClick>(self.mouse_triple_click.listener());
    }

    fn on_device_event(&mut self, _: DeviceId, event: &DeviceEvent, _: &mut AppContext) {
        if let DeviceEvent::ModifiersChanged(m) = event {
            self.modifiers = *m;
        }
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut AppContext) {
        match *event {
            WindowEvent::MouseInput {
                state, device_id, button, ..
            } => self.on_mouse_input(window_id, device_id, state, button, ctx),
            WindowEvent::CursorMoved { device_id, position, .. } => {
                self.on_cursor_moved(window_id, device_id, LayoutPoint::new(position.x as f32, position.y as f32), ctx)
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
