//! Mouse events.

use crate::core::app::*;
use crate::core::context::*;
use crate::core::event::*;
use crate::core::keyboard::ModifiersState;
use crate::core::render::*;
use crate::core::service::*;
use crate::core::window::{WindowEvent, WindowId, Windows};
use std::num::NonZeroU8;
use std::time::*;

use super::units::LayoutPoint;

type WPos = glutin::dpi::PhysicalPosition<f64>;

pub use glutin::event::MouseButton;

event_args! {
    /// [`MouseMove`] event args.
    pub struct MouseMoveArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: DeviceId,

        /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        /// Position of the mouse in the coordinates of [target](MouseMoveArgs::target).
        pub position: LayoutPoint,

        /// Hit-test result for the mouse point in the window.
        pub hits: FrameHitInfo,

        /// Full path to the top-most hit in [`hits`](MouseMoveArgs::hits).
        pub target: WidgetPath,

        ..

        /// If the widget is in [`target`](MouseMoveArgs::target).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.path.widget_id())
         }
    }

    /// [`MouseInput`], [`MouseDown`], [`MouseUp`] event args.
    pub struct MouseInputArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: DeviceId,

        /// Which mouse button generated the event.
        pub button: MouseButton,

        /// Position of the mouse in the coordinates of [`target`](MouseInputArgs::target).
        pub position: LayoutPoint,

        /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        /// The state the [`button`](MouseInputArgs::button) was changed to.
        pub state: ElementState,

        /// Hit-test result for the mouse point in the window.
        pub hits: FrameHitInfo,

        /// Full path to the top-most hit in [`hits`](MouseInputArgs::hits).
        pub target: WidgetPath,

        ..

        /// If the widget is in [`target`](MouseInputArgs::target).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.path.widget_id())
        }
    }

    /// [`MouseClick`] event args.
    pub struct MouseClickArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: DeviceId,

        /// Which mouse button generated the event.
        pub button: MouseButton,

        /// Position of the mouse in the coordinates of [`target`](MouseClickArgs::target).
        pub position: LayoutPoint,

         /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        /// Sequential click count . Number `1` is single click, `2` is double click, etc.
        pub click_count: NonZeroU8,

        /// Hit-test result for the mouse point in the window, at the moment the click event
        /// was generated.
        pub hits: FrameHitInfo,

        /// Full path to the widget that got clicked.
        ///
        /// A widget is clicked if the [`MouseDown`] and [`MouseUp`] happen
        /// in sequence in the same widget. Subsequent clicks (double, triple)
        /// happen on [`MouseDown`].
        ///
        /// If a [`MouseDown`] happen in a child widget and the pointer is dragged
        /// to a larger parent widget and then let go ([`MouseUp`]), the click target
        /// is the parent widget.
        ///
        /// Multi-clicks (`[click_count](MouseClickArgs::click_count) > 1`) only happen to
        /// the same target.
        pub target: WidgetPath,

        ..

        /// If the widget is in [`target`](MouseClickArgs::target).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.path.widget_id())
        }
    }

    /// [`MouseEnter`] and [`MouseLeave`] event args.
    pub struct MouseHoverArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: Option<DeviceId>,

        /// Position of the mouse in the window.
        pub position: LayoutPoint,

        /// Hit-test result for the mouse point in the window.
        pub hits: FrameHitInfo,

        /// Full path to the top-most hit in [`hits`](MouseInputArgs::hits).
        pub target: WidgetPath,

        ..

        /// If the widget is in [`targets`](MouseHoverArgs::targets).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.path.widget_id())
        }
    }
}

impl MouseHoverArgs {
    /// Event caused by the mouse position moving over/out of the widget bounds.
    #[inline]
    pub fn is_mouse_move(&self) -> bool {
        self.device_id.is_some()
    }

    /// Event caused by the widget moving under/out of the mouse position.
    #[inline]
    pub fn is_widget_move(&self) -> bool {
        self.device_id.is_none()
    }
}

event_hp! {
    /// Mouse move event.
    pub MouseMoveEvent: MouseMoveArgs;
}

event! {
    /// Mouse down or up event.
    pub MouseInputEvent: MouseInputArgs;

    /// Mouse down event.
    pub MouseDownEvent: MouseInputArgs;

    /// Mouse up event.
    pub MouseUpEvent: MouseInputArgs;

    /// Mouse click event, any [`click_count`](MouseClickArgs::click_count).
    pub MouseClickEvent: MouseClickArgs;

    /// Mouse single-click event (`[click_count](MouseClickArgs::click_count) == 1`).
    pub MouseSingleClickEvent: MouseClickArgs;

    /// Mouse double-click event (`[click_count](MouseClickArgs::click_count) == 2`).
    pub MouseDoubleClickEvent: MouseClickArgs;

    /// Mouse triple-click event (`[click_count](MouseClickArgs::click_count) == 3`).
    pub MouseTripleClickEvent: MouseClickArgs;

    /// Mouse enters a widget area event.
    pub MouseEnterEvent: MouseHoverArgs;

    /// Mouse leaves a widget area event.
    pub MouseLeaveEvent: MouseHoverArgs;
}

/// Application extension that provides mouse events.
///
/// # Events
///
/// Events this extension provides.
///
/// * [MouseMoveEvent]
/// * [MouseInputEvent]
/// * [MouseDownEvent]
/// * [MouseUpEvent]
/// * [MouseClickEvent]
/// * [MouseSingleClickEvent]
/// * [MouseDoubleClickEvent]
/// * [MouseTripleClickEvent]
/// * [MouseEnterEvent]
/// * [MouseLeaveEvent]
///
/// # Services
///
/// Services this extension provides.
///
/// * [Mouse]
pub struct MouseManager {
    /// last cursor move position (scaled).
    pos: LayoutPoint,
    /// last cursor move over `pos_window`.
    pos_window: Option<WindowId>,
    /// dpi scale of `pos_window`.
    pos_dpi: f32,

    /// last modifiers.
    modifiers: ModifiersState,

    /// when the last mouse_down event happened.
    last_pressed: Instant,
    click_target: Option<WidgetPath>,
    click_count: u8,

    hovered_target: Option<WidgetPath>,

    mouse_move: EventEmitter<MouseMoveArgs>,

    mouse_input: EventEmitter<MouseInputArgs>,
    mouse_down: EventEmitter<MouseInputArgs>,
    mouse_up: EventEmitter<MouseInputArgs>,

    mouse_click: EventEmitter<MouseClickArgs>,
    mouse_single_click: EventEmitter<MouseClickArgs>,
    mouse_double_click: EventEmitter<MouseClickArgs>,
    mouse_triple_click: EventEmitter<MouseClickArgs>,

    mouse_enter: EventEmitter<MouseHoverArgs>,
    mouse_leave: EventEmitter<MouseHoverArgs>,
}

impl Default for MouseManager {
    fn default() -> Self {
        MouseManager {
            pos: LayoutPoint::default(),
            pos_window: None,
            pos_dpi: 1.0,

            modifiers: ModifiersState::default(),

            last_pressed: Instant::now() - Duration::from_secs(60),
            click_target: None,
            click_count: 0,

            hovered_target: None,

            mouse_move: MouseMoveEvent::emitter(),

            mouse_input: MouseInputEvent::emitter(),
            mouse_down: MouseDownEvent::emitter(),
            mouse_up: MouseUpEvent::emitter(),

            mouse_click: MouseClickEvent::emitter(),
            mouse_single_click: MouseSingleClickEvent::emitter(),
            mouse_double_click: MouseDoubleClickEvent::emitter(),
            mouse_triple_click: MouseTripleClickEvent::emitter(),

            mouse_enter: MouseEnterEvent::emitter(),
            mouse_leave: MouseLeaveEvent::emitter(),
        }
    }
}

impl MouseManager {
    fn on_mouse_input(&mut self, window_id: WindowId, device_id: DeviceId, state: ElementState, button: MouseButton, ctx: &mut AppContext) {
        let position = if self.pos_window == Some(window_id) {
            self.pos
        } else {
            LayoutPoint::default()
        };

        let windows = ctx.services.req::<Windows>();
        let window = windows.window(window_id).unwrap();
        let hits = window.hit_test(position);
        let frame_info = window.frame_info();

        let (target, position) = if let Some(t) = hits.target() {
            (frame_info.find(t.widget_id).unwrap().path(), t.point)
        } else {
            (frame_info.root().path(), position)
        };

        let args = MouseInputArgs::now(
            window_id,
            device_id,
            button,
            position,
            self.modifiers,
            state,
            hits.clone(),
            target.clone(),
        );

        // on_mouse_input
        self.mouse_input.notify(ctx.events, args.clone());

        match state {
            ElementState::Pressed => {
                // on_mouse_down
                self.mouse_down.notify(ctx.events, args);

                self.click_count = self.click_count.saturating_add(1);
                let now = Instant::now();

                if self.click_count >= 2
                    && (now - self.last_pressed) < multi_click_time_ms()
                    && self.click_target.as_ref().unwrap() == &target
                {
                    // if click_count >= 2 AND the time is in multi-click range, AND is the same exact target.

                    let args = MouseClickArgs::new(
                        now,
                        window_id,
                        device_id,
                        button,
                        position,
                        self.modifiers,
                        NonZeroU8::new(self.click_count).unwrap(),
                        hits,
                        target,
                    );

                    // on_mouse_click (click_count > 1)

                    if self.click_count == 2 {
                        if self.mouse_double_click.has_listeners() {
                            self.mouse_double_click.notify(ctx.events, args.clone());
                        }
                    } else if self.click_count == 3 && self.mouse_triple_click.has_listeners() {
                        self.mouse_triple_click.notify(ctx.events, args.clone());
                    }

                    self.mouse_click.notify(ctx.events, args);
                } else {
                    // initial mouse press, could be a click if a Released happened on the same target.
                    self.click_count = 1;
                    self.click_target = Some(target);
                }
                self.last_pressed = now;
            }
            ElementState::Released => {
                // on_mouse_up
                self.mouse_up.notify(ctx.events, args);

                if let Some(click_count) = NonZeroU8::new(self.click_count) {
                    if click_count.get() == 1 {
                        if let Some(target) = self.click_target.as_ref().unwrap().shared_ancestor(&target) {
                            //if MouseDown and MouseUp happened in the same target.

                            let args = MouseClickArgs::now(
                                window_id,
                                device_id,
                                button,
                                position,
                                self.modifiers,
                                click_count,
                                hits,
                                target.clone(),
                            );

                            self.click_target = Some(target);

                            if self.mouse_single_click.has_listeners() {
                                self.mouse_single_click.notify(ctx.events, args.clone());
                            }

                            // on_mouse_click
                            self.mouse_click.notify(ctx.events, args);
                        } else {
                            self.click_count = 0;
                            self.click_target = None;
                        }
                    }
                }
            }
        }
    }

    fn on_cursor_moved(&mut self, window_id: WindowId, device_id: DeviceId, position: WPos, ctx: &mut AppContext) {
        let mut moved = Some(window_id) != self.pos_window;

        if moved {
            // if is over another window now.

            self.pos_window = Some(window_id);

            let windows = ctx.services.req::<Windows>();
            self.pos_dpi = windows.window(window_id).unwrap().scale_factor();
        }

        let pos = LayoutPoint::new(position.x as f32 / self.pos_dpi, position.y as f32 / self.pos_dpi);

        moved |= pos != self.pos;

        if moved {
            // if moved to another window or within the same window.

            self.pos = pos;

            let windows = ctx.services.req::<Windows>();
            let window = windows.window(window_id).unwrap();

            let hits = window.hit_test(pos);

            // mouse_move data
            let frame_info = window.frame_info();
            let (target, position) = if let Some(t) = hits.target() {
                (frame_info.find(t.widget_id).unwrap().path(), t.point)
            } else {
                (frame_info.root().path(), pos)
            };

            // mouse_move
            let args = MouseMoveArgs::now(window_id, device_id, self.modifiers, position, hits.clone(), target.clone());
            self.mouse_move.notify(ctx.events, args);

            // mouse_enter/mouse_leave.
            self.update_hovered(window_id, Some(device_id), hits, Some(target), ctx.events);
        }
    }

    fn on_cursor_left(&mut self, window_id: WindowId, device_id: DeviceId, ctx: &mut AppContext) {
        if Some(window_id) == self.pos_window.take() {
            if let Some(target) = self.hovered_target.take() {
                let args = MouseHoverArgs::now(
                    window_id,
                    device_id,
                    LayoutPoint::new(-1., -1.),
                    FrameHitInfo::no_hits(window_id),
                    target,
                );
                self.mouse_leave.notify(ctx.events, args);
            }
        }
    }

    fn on_new_frame(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        if self.pos_window == Some(window_id) {
            let window = ctx.services.req::<Windows>().window(window_id).unwrap();
            let hits = window.hit_test(self.pos);
            let target = hits.target().and_then(|t| window.frame_info().find(t.widget_id)).map(|w| w.path());
            self.update_hovered(window_id, None, hits, target, ctx.events);
        }
    }

    fn update_hovered(
        &mut self,
        window_id: WindowId,
        device_id: Option<DeviceId>,
        hits: FrameHitInfo,
        new_target: Option<WidgetPath>,
        events: &Events,
    ) {
        if self.hovered_target != new_target {
            if let Some(old_target) = self.hovered_target.take() {
                let args = MouseHoverArgs::now(window_id, device_id, self.pos, hits.clone(), old_target);
                self.mouse_leave.notify(events, args);
            }
            self.hovered_target = new_target.clone();
            if let Some(new_target) = new_target {
                let args = MouseHoverArgs::now(window_id, device_id, self.pos, hits, new_target);
                self.mouse_enter.notify(events, args);
            }
        }
    }
}

impl AppExtension for MouseManager {
    fn init(&mut self, r: &mut AppInitContext) {
        r.events.register::<MouseMoveEvent>(self.mouse_move.listener());

        r.events.register::<MouseInputEvent>(self.mouse_input.listener());
        r.events.register::<MouseDownEvent>(self.mouse_down.listener());
        r.events.register::<MouseUpEvent>(self.mouse_up.listener());

        r.events.register::<MouseClickEvent>(self.mouse_click.listener());
        r.events.register::<MouseSingleClickEvent>(self.mouse_single_click.listener());
        r.events.register::<MouseDoubleClickEvent>(self.mouse_double_click.listener());
        r.events.register::<MouseTripleClickEvent>(self.mouse_triple_click.listener());

        r.events.register::<MouseEnterEvent>(self.mouse_enter.listener());
        r.events.register::<MouseLeaveEvent>(self.mouse_leave.listener());

        r.services.register(Mouse::new());
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut AppContext) {
        match *event {
            WindowEvent::CursorMoved { device_id, position, .. } => self.on_cursor_moved(window_id, device_id, position, ctx),
            WindowEvent::MouseInput {
                state, device_id, button, ..
            } => self.on_mouse_input(window_id, device_id, state, button, ctx),
            WindowEvent::ModifiersChanged(m) => self.modifiers = m,
            WindowEvent::CursorLeft { device_id } => self.on_cursor_left(window_id, device_id, ctx),
            _ => {}
        }
    }

    fn on_new_frame_ready(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        self.on_new_frame(window_id, ctx);
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

/// Mouse service.
///
/// # Provider
///
/// This service is provided by the [`MouseManager`] extension.
#[derive(AppService)]
pub struct Mouse {
    current_capture: Option<(WidgetPath, CaptureMode)>,
    capture_request: Option<(WidgetPath, CaptureMode)>,
}
impl Mouse {
    fn new() -> Self {
        Mouse {
            current_capture: None,
            capture_request: None,
        }
    }

    /// Gets the current capture target and mode.
    pub fn current_capture(&self) -> Option<(&WidgetPath, CaptureMode)> {
        self.current_capture.as_ref().map(|(p, c)| (p, *c))
    }

    /// Set a `target` to redirect all mouse events to.
    pub fn capture(&mut self, target: WidgetPath, mode: CaptureMode) -> EventListener<bool> {
        todo!()
    }

    /// Release the current mouse capture.
    pub fn release_capture(&mut self) {
        todo!()
    }
}

/// Mouse capture mode.
#[derive(Copy, Clone, Debug)]
pub enum CaptureMode {
    /// All mouse events redirected to the widget.
    Widget,
    /// All mouse events inside the widget sub-tree permitted. All mouse events
    /// outside of the widget redirected to the widget.
    Subtree,
}
