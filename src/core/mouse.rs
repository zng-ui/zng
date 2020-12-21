//! Mouse events and service.
//!
//! The app extension [`MouseManager`] provides the events and service. It is included in the default application.

use super::units::{LayoutPoint, LayoutRect, LayoutSize};
use super::WidgetId;
use crate::core::app::*;
use crate::core::context::*;
use crate::core::event::*;
use crate::core::keyboard::ModifiersState;
use crate::core::render::*;
use crate::core::service::*;
use crate::core::window::{WindowEvent, WindowId, Windows};
use std::time::*;
use std::{mem, num::NonZeroU8};

type WPos = glutin::dpi::PhysicalPosition<f64>;

pub use glutin::event::MouseButton;

event_args! {
    /// [`MouseMoveEvent`] event args.
    pub struct MouseMoveArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: DeviceId,

        /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        /// Position of the mouse in the coordinates of [`target`](MouseMoveArgs::target).
        pub position: LayoutPoint,

        /// Hit-test result for the mouse point in the window.
        pub hits: FrameHitInfo,

        /// Full path to the top-most hit in [`hits`](MouseMoveArgs::hits).
        pub target: WidgetPath,

        /// Current mouse capture.
        pub capture: Option<CaptureInfo>,

        ..

        /// If the widget is in [`target`](Self::target)
        /// and is [allowed](CaptureInfo::allows) by the [`capture`](Self::capture).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.path.widget_id())
            && self.capture.as_ref().map(|c| c.allows(ctx.path)).unwrap_or(true)
        }
    }

    /// [`MouseInputEvent`], [`MouseDownEvent`], [`MouseUpEvent`] event args.
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

        /// Current mouse capture.
        pub capture: Option<CaptureInfo>,

        ..

        /// If the widget is in [`target`](MouseInputArgs::target)
        /// and is [allowed](CaptureInfo::allows) by the [`capture`](Self::capture).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.path.widget_id())
            && self.capture.as_ref().map(|c|c.allows(ctx.path)).unwrap_or(true)
        }
    }

    /// [`MouseClickEvent`] event args.
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

    /// [`MouseEnterEvent`] and [`MouseLeaveEvent`] event args.
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

        /// Current mouse capture.
        pub capture: Option<CaptureInfo>,

        ..

        /// If the widget is in [`targets`](MouseHoverArgs::targets)
        /// and is [allowed](CaptureInfo::allows) by the [`capture`](Self::capture).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.path.widget_id())
            && self.capture.as_ref().map(|c|c.allows(ctx.path)).unwrap_or(true)
        }
    }

    /// [`MouseCaptureEvent`] arguments.
    pub struct MouseCaptureArgs {
        /// Previous mouse capture target and mode.
        pub prev_capture: Option<(WidgetPath, CaptureMode)>,
        /// new mouse capture target and mode.
        pub new_capture: Option<(WidgetPath, CaptureMode)>,

        ..

        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            if let Some(prev) = &self.prev_capture {
                if prev.0.contains(ctx.path.widget_id()) {
                    return true;
                }
            }
            if let Some(new) = &self.new_capture {
                if new.0.contains(ctx.path.widget_id()) {
                    return true;
                }
            }
            false
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

    /// If the widget is in [`target`](Self::target) or is the [`capture`](Self::capture) holder.
    #[inline]
    pub fn concerns_capture(&self, ctx: &mut WidgetContext) -> bool {
        self.target.contains(ctx.path.widget_id())
            || self
                .capture
                .as_ref()
                .map(|c| c.target.widget_id() == ctx.path.widget_id())
                .unwrap_or(false)
    }
}

impl MouseMoveArgs {
    /// If the widget is in [`target`](Self::target) or is the [`capture`](Self::capture) holder.
    #[inline]
    pub fn concerns_capture(&self, ctx: &mut WidgetContext) -> bool {
        self.target.contains(ctx.path.widget_id())
            || self
                .capture
                .as_ref()
                .map(|c| c.target.widget_id() == ctx.path.widget_id())
                .unwrap_or(false)
    }
}

impl MouseInputArgs {
    /// If the widget is in [`target`](Self::target) or is the [`capture`](Self::capture) holder.
    #[inline]
    pub fn concerns_capture(&self, ctx: &mut WidgetContext) -> bool {
        self.target.contains(ctx.path.widget_id())
            || self
                .capture
                .as_ref()
                .map(|c| c.target.widget_id() == ctx.path.widget_id())
                .unwrap_or(false)
    }

    /// If the [`button`](Self::button) is the primary.
    #[inline]
    pub fn is_primary(&self) -> bool {
        self.button == MouseButton::Left
    }
}

impl MouseCaptureArgs {
    /// If the same widget has mouse capture, but the widget path changed.
    #[inline]
    pub fn is_widget_move(&self) -> bool {
        match (&self.prev_capture, &self.new_capture) {
            (Some(prev), Some(new)) => prev.0.widget_id() == new.0.widget_id() && prev.0 != new.0,
            _ => false,
        }
    }

    /// If the same widget has mouse capture, but the capture mode changed.
    #[inline]
    pub fn is_mode_change(&self) -> bool {
        match (&self.prev_capture, &self.new_capture) {
            (Some(prev), Some(new)) => prev.0.widget_id() == new.0.widget_id() && prev.1 != new.1,
            _ => false,
        }
    }

    /// If the `widget_id` lost mouse capture with this update.
    #[inline]
    pub fn is_lost(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_capture, &self.new_capture) {
            (None, _) => false,
            (Some((path, _)), None) => path.widget_id() == widget_id,
            (Some((prev_path, _)), Some((new_path, _))) => prev_path.widget_id() == widget_id && new_path.widget_id() != widget_id,
        }
    }

    /// If the `widget_id` got mouse capture with this update.
    #[inline]
    pub fn is_got(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_capture, &self.new_capture) {
            (_, None) => false,
            (None, Some((path, _))) => path.widget_id() == widget_id,
            (Some((prev_path, _)), Some((new_path, _))) => prev_path.widget_id() != widget_id && new_path.widget_id() == widget_id,
        }
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

    /// Mouse capture changed event.
    pub MouseCaptureEvent: MouseCaptureArgs;
}

/// Application extension that provides mouse events and service.
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
/// * [MouseCaptureEvent]
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

    capture_count: u8,

    hover_enter_args: Option<MouseHoverArgs>,

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

            hover_enter_args: None,

            capture_count: 0,

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

        let (windows, mouse) = ctx.services.req_multi::<(Windows, Mouse)>();
        let window = windows.window(window_id).unwrap();
        let hits = window.hit_test(position);
        let frame_info = window.frame_info();

        let (target, position) = if let Some(t) = hits.target() {
            (frame_info.find(t.widget_id).unwrap().path(), t.point)
        } else {
            (frame_info.root().path(), position)
        };

        if state == ElementState::Pressed {
            self.capture_count += 1;
            if self.capture_count == 1 {
                mouse.start_window_capture(target.clone(), ctx.events);
            }
        } else {
            self.capture_count = self.capture_count.saturating_sub(1);
            if self.capture_count == 0 {
                mouse.end_window_capture(ctx.events);
            }
        }

        let capture_info = if let Some((capture, mode)) = mouse.current_capture() {
            Some(CaptureInfo {
                target: capture.clone(),
                mode,
                position, // TODO
            })
        } else {
            None
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
            capture_info,
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

            let mouse = ctx.services.req::<Mouse>();
            let capture = if let Some((path, mode)) = mouse.current_capture() {
                Some(CaptureInfo {
                    position: self.pos, // TODO must be related to capture.
                    target: path.clone(),
                    mode,
                })
            } else {
                None
            };

            // mouse_move
            let args = MouseMoveArgs::now(
                window_id,
                device_id,
                self.modifiers,
                position,
                hits.clone(),
                target.clone(),
                capture,
            );
            self.mouse_move.notify(ctx.events, args);

            // mouse_enter/mouse_leave.
            self.update_hovered(
                window_id,
                Some(device_id),
                hits,
                Some(target),
                ctx.events,
                ctx.services.req::<Mouse>(),
            );
        }
    }

    fn on_cursor_left(&mut self, window_id: WindowId, device_id: DeviceId, ctx: &mut AppContext) {
        if Some(window_id) == self.pos_window.take() {
            if let Some(args) = self.hover_enter_args.take() {
                let capture = ctx.services.req::<Mouse>().current_capture().map(|(path, mode)| CaptureInfo {
                    target: path.clone(),
                    mode,
                    position: self.pos, // TODO
                });
                let args = MouseHoverArgs::now(
                    window_id,
                    device_id,
                    LayoutPoint::new(-1., -1.),
                    FrameHitInfo::no_hits(window_id),
                    args.target,
                    capture,
                );
                self.mouse_leave.notify(ctx.events, args);
            }
        }
    }

    fn on_update(&mut self, ctx: &mut AppContext) {
        let (mouse, windows) = ctx.services.req_multi::<(Mouse, Windows)>();

        if let Some(cap_args) = mouse.capture_event.updates(ctx.events).iter().last() {
            if let Some(hover_args) = self.hover_enter_args.take() {
                let hover_args = MouseHoverArgs::now(
                    hover_args.window_id,
                    hover_args.device_id,
                    hover_args.position,
                    hover_args.hits,
                    hover_args.target,
                    cap_args.new_capture.as_ref().map(|(path, mode)| CaptureInfo {
                        target: path.clone(),
                        mode: *mode,
                        position: LayoutPoint::zero(), //TODO
                    }),
                );
                self.mouse_enter.notify(ctx.events, hover_args.clone());
                self.hover_enter_args = Some(hover_args);
            }
        }

        mouse.fulfill_requests(windows, ctx.events);
    }

    fn on_new_frame(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        // update hovered
        if self.pos_window == Some(window_id) {
            let (windows, mouse) = ctx.services.req_multi::<(Windows, Mouse)>();
            let window = windows.window(window_id).unwrap();
            let hits = window.hit_test(self.pos);
            let target = hits.target().and_then(|t| window.frame_info().find(t.widget_id)).map(|w| w.path());
            self.update_hovered(window_id, None, hits, target, ctx.events, mouse);
        }
        // update capture
        if self.capture_count > 0 {
            let (mouse, windows) = ctx.services.req_multi::<(Mouse, Windows)>();
            if let Ok(window) = windows.window(window_id) {
                mouse.continue_capture(window.frame_info(), ctx.events);
            }
        }
    }

    fn update_hovered(
        &mut self,
        window_id: WindowId,
        device_id: Option<DeviceId>,
        hits: FrameHitInfo,
        new_target: Option<WidgetPath>,
        events: &Events,
        mouse: &Mouse,
    ) {
        if let Some(new_target) = new_target {
            if let Some(last_enter_args) = self.hover_enter_args.take() {
                if last_enter_args.target != new_target {
                    // widget under mouse changed.
                    self.notify_mouse_leave(window_id, device_id, last_enter_args.target, hits.clone(), events, mouse);
                    self.notify_mouse_enter(window_id, device_id, new_target, hits, events, mouse);
                } else {
                    // widget did not change.
                    self.hover_enter_args = Some(last_enter_args);
                }
            } else {
                // mouse entered first widget.
                self.notify_mouse_enter(window_id, device_id, new_target, hits, events, mouse);
            }
        } else if let Some(old_enter_args) = self.hover_enter_args.take() {
            // mouse left all widgets.
            self.notify_mouse_leave(window_id, device_id, old_enter_args.target, hits, events, mouse);
        }
    }
    fn notify_mouse_leave(
        &self,
        window_id: WindowId,
        device_id: Option<DeviceId>,
        old_target: WidgetPath,
        hits: FrameHitInfo,
        events: &Events,
        mouse: &Mouse,
    ) {
        let capture = mouse.current_capture().map(|(path, mode)| CaptureInfo {
            target: path.clone(),
            mode,
            position: self.pos, // TODO
        });
        let args = MouseHoverArgs::now(window_id, device_id, self.pos, hits, old_target, capture);
        self.mouse_leave.notify(events, args);
    }
    fn notify_mouse_enter(
        &mut self,
        window_id: WindowId,
        device_id: Option<DeviceId>,
        new_target: WidgetPath,
        hits: FrameHitInfo,
        events: &Events,
        mouse: &Mouse,
    ) {
        let capture = mouse.current_capture().map(|(path, mode)| CaptureInfo {
            target: path.clone(),
            mode,
            position: self.pos, // TODO
        });
        let args = MouseHoverArgs::now(window_id, device_id, self.pos, hits, new_target, capture);
        self.mouse_enter.notify(events, args.clone());
        self.hover_enter_args = Some(args);
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

        r.services.register(Mouse::new(r.updates.notifier().clone(), r.events));
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

    fn update(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        if update.update_hp {
            return;
        }

        self.on_update(ctx);
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
/// # Mouse Capture
///
/// A mouse is **captured** when mouse events are redirected to a specific target. The user
/// can still move the cursor outside of the target but the widgets outside do not interact with the cursor.
///
/// You can request capture by calling [`capture_widget`](Mouse::capture_widget) or
/// [`capture_subtree`](Mouse::capture_subtree) with a widget that was pressed by a mouse button. The capture
/// will last for as long as any of the mouse buttons are pressed, the widget is visible and the window is focused.
///
/// Windows capture the mouse by default, this cannot be disabled. For other widgets this is optional.
///
/// # Cursor Lock
///
/// The cursor is **locked** when it cannot be moved outside of an area. The user can still move the cursor inside
/// the area but it visually stops at the area boundaries.
///
/// You can request lock by calling [`lock_cursor_pt`](Mouse::lock_cursor_pt), [`lock_cursor_widget`](Mouse::lock_cursor_widget),
/// or one of the other `lock_cursor*` methods.
///
/// The cursor will stay locked for as long the target is visible and the window is focused.
///
/// # Provider
///
/// This service is provided by the [`MouseManager`] extension.
#[derive(AppService)]
pub struct Mouse {
    capture_event: EventEmitter<MouseCaptureArgs>,
    current_capture: Option<(WidgetPath, CaptureMode)>,
    capture_request: Option<(WidgetId, CaptureMode)>,
    release_requested: bool,
    notifier: UpdateNotifier,
}
impl Mouse {
    fn new(notifier: UpdateNotifier, events: &mut Events) -> Self {
        let capture_event = MouseCaptureEvent::emitter();
        events.register::<MouseCaptureEvent>(capture_event.listener());

        Mouse {
            capture_event,
            current_capture: None,
            capture_request: None,
            release_requested: false,
            notifier,
        }
    }

    /// The current capture target and mode.
    #[inline]
    pub fn current_capture(&self) -> Option<(&WidgetPath, CaptureMode)> {
        self.current_capture.as_ref().map(|(p, c)| (p, *c))
    }

    /// Set a widget to redirect all mouse events to.
    ///
    /// The capture will be set only if the pointer is currently pressed over the widget.
    #[inline]
    pub fn capture_widget(&mut self, widget_id: WidgetId) {
        self.capture_request = Some((widget_id, CaptureMode::Widget));
        self.notifier.update();
    }

    /// Set a widget to be the root of a capture subtree.
    ///
    /// Mouse events targeting inside the subtree go to target normally. Mouse events outside
    /// the capture root are redirected to the capture root.
    ///
    /// The capture will be set only if the pointer is currently pressed over the widget.
    #[inline]
    pub fn capture_subtree(&mut self, widget_id: WidgetId) {
        self.capture_request = Some((widget_id, CaptureMode::Subtree));
        self.notifier.update();
    }

    /// Release the current mouse capture back to window.
    ///
    /// **Note:** The capture is released automatically when the mouse buttons are released
    /// or when the window loses focus.
    #[inline]
    pub fn release_capture(&mut self) {
        self.release_requested = true;
        self.notifier.update();
    }

    /// The current cursor lock active.
    pub fn current_lock(&self) {
        todo!()
    }

    /// Locks the cursor in an `area` of the window if the it is focused.
    ///
    /// The pointer is moved inside the area to start, the user can only move the cursor inside the area.
    /// Mouse move events are generated only by move inside the area, you can use the mouse device events
    /// to monitor attempts to move outside the area.
    ///
    /// The area is relative to the window, if the window moves the cursor gets pushed by the sides of the area.
    ///
    /// **NOT IMPLEMENTED**
    pub fn lock_cursor(&mut self, window_id: WindowId, area: LayoutRect) {
        // https://docs.rs/winit/0.24.0/winit/window/struct.Window.html#method.set_cursor_grab
        // https://github.com/rust-windowing/winit/issues/1677
        todo!("impl lockcursor({:?}, {:?})", window_id, area)
    }

    /// Locks the cursor to a `point` of the window if it is focused.
    ///
    /// The pointer is moved to the point to start, the user cannot move the cursor. No mouse move events are
    /// generated, you can use the mouse device events to monitor attempts to move the mouse.
    ///
    /// The point is relative to the window, if the window moves the cursor moves to match the point.
    ///
    /// **NOT IMPLEMENTED**
    #[inline]
    pub fn lock_cursor_pt(&mut self, window_id: WindowId, point: LayoutPoint) {
        self.lock_cursor(window_id, LayoutRect::new(point, LayoutSize::new(0.0, 0.0)))
    }

    /// Locks the cursor to the `area` of a widget in a window that is focused.
    ///
    /// The pointer is moved to the point to start, the user can only move the cursor inside the widget area.
    /// Mouse move events are generated only for the widget, you can use the mouse device events to monitor attempts to move
    /// outside the area.
    ///
    /// If the widget moves the cursor gets pushed by the sides of the area.
    ///
    /// **NOT IMPLEMENTED**
    pub fn lock_cursor_widget(&mut self, window_id: WindowId, area: WidgetId) {
        todo!("impl lock_cursor_wgt({:?}, {:?})", window_id, area)
    }

    /// Locks the cursor to the content area of the window if the window is focused.
    ///
    /// The pointer is moved inside the window to start, the user can only move the cursor inside the window area.
    ///
    /// If the window moves or is resized the cursor gets pushed by the sides of the window area.
    ///
    /// **NOT IMPLEMENTED**
    pub fn lock_cursor_window(&mut self, window_id: WindowId) {
        todo!("impl lock_cursor_window({:?}", window_id)
    }

    /// Release the cursor lock.
    ///
    /// **Note:** the cursor lock is released automatically when the window loses focus.
    ///
    /// **NOT IMPLEMENTED**
    pub fn release_lock(&mut self) {
        todo!()
    }

    /// Call when the mouse starts pressing on the window.
    fn start_window_capture(&mut self, mouse_down: WidgetPath, events: &Events) {
        self.release_requested = false;

        if let Some((target, mode)) = self.capture_request.take() {
            if let Some(target) = mouse_down.ancestor_path(target) {
                self.set_capture(target, mode, events);
                return; // fulfilled request at start.
            }
        }

        // set default capture.
        self.set_capture(mouse_down.root_path(), CaptureMode::Window, events);
    }

    /// Call after UI update.
    fn fulfill_requests(&mut self, windows: &Windows, events: &Events) {
        if let Some((current_target, current_mode)) = &self.current_capture {
            if let Some((widget_id, mode)) = self.capture_request.take() {
                if let Ok(window) = windows.window(current_target.window_id()) {
                    if window.is_active() {
                        // current window pressed
                        if let Some(widget) = window.frame_info().find(widget_id) {
                            // request valid
                            self.set_capture(widget.path(), mode, events);
                        }
                    }
                }
            } else if mem::take(&mut self.release_requested) && *current_mode != CaptureMode::Window {
                // release capture (back to default capture).
                let target = current_target.root_path();
                self.set_capture(target, CaptureMode::Window, events);
            }
        }
    }

    /// Call after a frame is generated.
    fn continue_capture(&mut self, frame: &FrameInfo, events: &Events) {
        if let Some((target, mode)) = &self.current_capture {
            if frame.window_id() == target.window_id() {
                // is a frame from the capturing window.
                if let Some(widget) = frame.get(target) {
                    if let Some(new_path) = widget.new_path(target) {
                        // widget moved inside window tree.
                        let mode = *mode;
                        self.set_capture(new_path, mode, events);
                    }
                } else {
                    // widget not found. Returns to default capture.
                    self.set_capture(frame.root().path(), CaptureMode::Window, events);
                }
            }
        }
    }

    /// Call when the mouse stops pressing on the window, or the window loses focus or is closed.
    fn end_window_capture(&mut self, events: &Events) {
        self.release_requested = false;
        self.capture_request = None;
        self.unset_capture(events);
    }

    fn set_capture(&mut self, target: WidgetPath, mode: CaptureMode, events: &Events) {
        let new = Some((target, mode));
        if new != self.current_capture {
            println!("capture change to: {:?}", new.as_ref().map(|(p, m)| (p.to_string(), m)));
            let prev = self.current_capture.take();
            self.current_capture = new.clone();
            self.capture_event.notify(events, MouseCaptureArgs::now(prev, new));
        }
    }

    fn unset_capture(&mut self, events: &Events) {
        if self.current_capture.is_some() {
            println!("capture change to: None");
            let prev = self.current_capture.take();
            self.capture_event.notify(events, MouseCaptureArgs::now(prev, None));
        }
    }
}

/// Mouse capture mode.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CaptureMode {
    /// Mouse captured by the window only.
    ///
    /// Default behavior.
    Window,
    /// Mouse events inside the widget sub-tree permitted. Mouse events
    /// outside of the widget redirected to the widget.
    Subtree,

    /// Mouse events redirected to the widget.
    Widget,
}
impl Default for CaptureMode {
    /// [`CaptureMode::Window`]
    #[inline]
    fn default() -> Self {
        CaptureMode::Window
    }
}
impl_from_and_into_var! {
    /// Convert `true` to [`CaptureMode::Widget`] and `false` to [`CaptureMode::Window`].
    fn from(widget: bool) -> CaptureMode {
        if widget {
            CaptureMode::Widget
        } else {
            CaptureMode::Window
        }
    }
}

/// Information about mouse capture in a mouse event argument.
#[derive(Debug, Clone, PartialEq)]
pub struct CaptureInfo {
    /// Widget that is capturing all mouse events.
    ///
    /// This is the window root widget for capture mode `Window`.
    pub target: WidgetPath,
    pub mode: CaptureMode,
    /// Position of the pointer related to the `target` area.
    pub position: LayoutPoint,
}
impl CaptureInfo {
    /// If the widget is allowed by the current capture.
    ///
    /// | Mode           | Allows                                             |
    /// |----------------|----------------------------------------------------|
    /// | `Window`       | All widgets in the same window.                    |
    /// | `Subtree`      | All widgets that have the `target` in their path. |
    /// | `Widget`       | Only the `target` widget.                         |
    #[inline]
    pub fn allows(&self, path: &WidgetContextPath) -> bool {
        match self.mode {
            CaptureMode::Window => self.target.window_id() == path.window_id(),
            CaptureMode::Widget => self.target.widget_id() == path.widget_id(),
            CaptureMode::Subtree => path.contains(self.target.widget_id()),
        }
    }
}
