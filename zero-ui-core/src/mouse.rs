//! Mouse events and service.
//!
//! The app extension [`MouseManager`] provides the events and service. It is included in the default application.

use crate::{
    app::{
        raw_events::*,
        view_process::{ViewProcess, ViewProcessRespawnedEvent},
        *,
    },
    context::*,
    event::*,
    keyboard::ModifiersState,
    render::{webrender_api::HitTestResult, *},
    service::*,
    units::*,
    var::{impl_from_and_into_var, var, RcVar, ReadOnlyRcVar, Var},
    widget_info::{WidgetInfoTree, WidgetPath},
    window::{WindowId, Windows, WindowsExt},
    WidgetId,
};
use std::{fmt, mem, num::NonZeroU8, time::*};

pub use zero_ui_view_api::{ButtonState, MouseButton, MouseScrollDelta, MultiClickConfig, TouchPhase};

event_args! {
    /// [`MouseMoveEvent`] event args.
    pub struct MouseMoveArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: DeviceId,

        /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        /// Positions of the cursor in between the previous event and this one.
        ///
        /// Mouse move events can be coalesced, i.e. multiple moves packed into a single event.
        pub coalesced_pos: Vec<DipPoint>,

        /// Position of the mouse in the window's content area.
        pub position: DipPoint,

        /// Hit-test result for the mouse point in the window.
        pub hits: FrameHitInfo,

        /// Full path to the top-most hit in [`hits`](MouseMoveArgs::hits).
        pub target: WidgetPath,

        /// Current mouse capture.
        pub capture: Option<CaptureInfo>,

        ..

        /// If the widget is in [`target`] and is [allowed] by the [`capture`].
        ///
        /// [`target`]: Self::target
        /// [allowed]: CaptureInfo::allows
        /// [`capture`]: Self::capture
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.path.widget_id())
            && self.capture.as_ref().map(|c| c.allows(ctx.path)).unwrap_or(true)
        }
    }

    /// [`MouseInputEvent`] event args.
    pub struct MouseInputArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: DeviceId,

        /// Which mouse button generated the event.
        pub button: MouseButton,

        /// Position of the mouse in the window's content area.
        pub position: DipPoint,

        /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        /// The state the [`button`](MouseInputArgs::button) was changed to.
        pub state: ButtonState,

        /// Hit-test result for the mouse point in the window.
        pub hits: FrameHitInfo,

        /// Full path to the top-most hit in [`hits`](MouseInputArgs::hits).
        pub target: WidgetPath,

        /// Current mouse capture.
        pub capture: Option<CaptureInfo>,

        ..

        /// If the widget is in [`target`], is interactive and is [allowed] by the [`capture`].
        ///
        /// [`target`]: Self::target
        /// [allowed]: CaptureInfo::allows
        /// [`capture`]: Self::capture
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.path.widget_id())
            && self.capture.as_ref().map(|c|c.allows(ctx.path)).unwrap_or(true)
            && ctx.info_tree.find(ctx.path.widget_id()).map(|w|w.allow_interaction()).unwrap_or(false)
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
        pub position: DipPoint,

         /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        /// Sequential click count . Number `1` is single click, `2` is double click, etc.
        pub click_count: NonZeroU8,

        /// Hit-test result for the mouse point in the window, at the moment the click event
        /// was generated.
        pub hits: FrameHitInfo,

        /// Full path to the widget that got clicked.
        ///
        /// A widget is clicked if the [mouse down] and [mouse up] happen
        /// in sequence in the same widget. Subsequent clicks (double, triple)
        /// happen on mouse down.
        ///
        /// If a [mouse down] happen in a child widget and the pointer is dragged
        /// to a larger parent widget and then let go (mouse up), the click target
        /// is the parent widget.
        ///
        /// Multi-clicks (`[click_count]` > 1) only happen to the same target.
        ///
        /// [mouse down]: MouseInputArgs::is_mouse_down
        /// [mouse up]: MouseInputArgs::is_mouse_up
        /// [click_count]: (MouseClickArgs::click_count
        pub target: WidgetPath,

        ..

        /// If the widget is in [`target`] and is interactive.
        ///
        /// [`target`]: MouseClickArgs::target
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.path.widget_id())
            && ctx.info_tree.find(ctx.path.widget_id()).map(|w|w.allow_interaction()).unwrap_or(false)
        }
    }

    /// [`MouseHoveredEvent`] event args.
    pub struct MouseHoverArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: Option<DeviceId>,

        /// Position of the mouse in the window.
        pub position: DipPoint,

        /// Hit-test result for the mouse point in the window.
        pub hits: FrameHitInfo,

        /// Previous top-most hit before the mouse moved.
        pub prev_target: Option<WidgetPath>,

        /// Full path to the top-most hit in [`hits`].
        ///
        /// Is `None` when the mouse moves out of a window or the window closes under the mouse
        /// and there was a previous hovered widget.
        ///
        /// [`hits`]: MouseInputArgs::hits
        pub target: Option<WidgetPath>,

        /// Previous mouse capture.
        pub prev_capture: Option<CaptureInfo>,

        /// Current mouse capture.
        pub capture: Option<CaptureInfo>,

        ..

        /// If the widget is in [`target`] or [`prev_target`] and
        /// if it is [allowed] by the [`capture`].
        ///
        /// [`target`]: Self::target
        /// [`prev_target`]: Self::prev_target
        /// [allowed]: CaptureInfo::allows
        /// [`capture`]: Self::capture
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.capture.as_ref().map(|c|c.allows(ctx.path)).unwrap_or(true)
            && (
                self.target.as_ref().map(|p| p.contains(ctx.path.widget_id())).unwrap_or(false) ||
                self.prev_target.as_ref().map(|p|p.contains(ctx.path.widget_id())).unwrap_or(false)
            )
        }
    }

    /// [`MouseCaptureEvent`] arguments.
    pub struct MouseCaptureArgs {
        /// Previous mouse capture target and mode.
        pub prev_capture: Option<(WidgetPath, CaptureMode)>,
        /// new mouse capture target and mode.
        pub new_capture: Option<(WidgetPath, CaptureMode)>,

        ..

        /// If the [`prev_capture`] or [`new_capture`] contains the widget.
        ///
        /// [`prev_capture`]: Self::prev_capture
        /// [`new_capture`]: Self::new_capture
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

    /// [`MouseWheelEvent`] arguments.
    pub struct MouseWheelArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,
        /// Id of device that generated the event.
        pub device_id: DeviceId,

        /// Position of the mouse in the coordinates of [`target`](MouseWheelArgs::target).
        pub position: DipPoint,
         /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        /// Wheel motion delta, value is in pixels if the *wheel* is a touchpad.
        pub delta: MouseScrollDelta,

        /// Touch state if the device that generated the event is a touchpad.
        pub phase: TouchPhase,

        /// Hit-test result for the mouse point in the window, at the moment the wheel event
        /// was generated.
        pub hits: FrameHitInfo,
        /// Full path to the widget that got scrolled.
        pub target: WidgetPath,

        ..

        /// If the widget is in [`target`] and is interactive.
        ///
        /// [`target`]: MouseWheelArgs::target
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.path.widget_id())
            && ctx.info_tree.find(ctx.path.widget_id()).map(|w|w.allow_interaction()).unwrap_or(false)
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

    /// Event caused by a mouse capture change.
    #[inline]
    pub fn is_capture_change(&self) -> bool {
        self.prev_capture != self.capture
    }

    /// Returns `true` if the widget was not hovered, but now is.
    #[inline]
    pub fn is_mouse_enter(&self, path: &WidgetContextPath) -> bool {
        !self.was_over(path) && self.is_over(path)
    }

    /// Returns `true` if the widget was hovered, but now isn't.
    #[inline]
    pub fn is_mouse_leave(&self, path: &WidgetContextPath) -> bool {
        self.was_over(path) && !self.is_over(path)
    }

    /// Returns `true` if the widget is in [`prev_target`] and is allowed by the [`prev_capture`].
    ///
    /// [`prev_target`]: Self::prev_target
    /// [`prev_capture`]: Self::prev_capture
    #[inline]
    pub fn was_over(&self, path: &WidgetContextPath) -> bool {
        if let Some(cap) = &self.prev_capture {
            if !cap.allows(path) {
                return false;
            }
        }

        if let Some(t) = &self.prev_target {
            return t.contains(path.widget_id());
        }

        false
    }

    /// Returns `true` if the widget is in [`target`] and is allowed by the current [`capture`].
    ///
    /// [`target`]: Self::target
    /// [`capture`]: Self::capture
    #[inline]
    pub fn is_over(&self, path: &WidgetContextPath) -> bool {
        if let Some(cap) = &self.capture {
            if !cap.allows(path) {
                return false;
            }
        }

        if let Some(t) = &self.target {
            return t.contains(path.widget_id());
        }

        false
    }

    /// If the widget is in [`target`], [`prev_target`] or is the [`capture`] holder.
    ///
    /// [`target`]: Self::target
    /// [`prev_target`]: Self::prev_target
    /// [`capture`]: Self::capture
    #[inline]
    pub fn concerns_capture(&self, ctx: &mut WidgetContext) -> bool {
        self.capture
            .as_ref()
            .map(|c| c.target.widget_id() == ctx.path.widget_id())
            .unwrap_or(false)
            || self.target.as_ref().map(|p| p.contains(ctx.path.widget_id())).unwrap_or(false)
            || self.prev_target.as_ref().map(|p| p.contains(ctx.path.widget_id())).unwrap_or(false)
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
    /// If the widget is in [`target`] and allows interaction or is the [`capture`] holder.
    ///
    /// [`target`]: Self::target
    /// [`capture`]: Self::capture
    #[inline]
    pub fn concerns_capture(&self, ctx: &mut WidgetContext) -> bool {
        (self.target.contains(ctx.path.widget_id())
            && ctx
                .info_tree
                .find(ctx.path.widget_id())
                .map(|w| w.allow_interaction())
                .unwrap_or(false))
            || self
                .capture
                .as_ref()
                .map(|c| c.target.widget_id() == ctx.path.widget_id())
                .unwrap_or(false)
    }

    /// If the [`button`] is the primary.
    ///
    /// [`button`]: Self::button
    #[inline]
    pub fn is_primary(&self) -> bool {
        self.button == MouseButton::Left
    }

    /// If the [`button`](Self::button) is the context (right).
    #[inline]
    pub fn is_context(&self) -> bool {
        self.button == MouseButton::Right
    }

    /// If the [`state`] is pressed.
    ///
    /// [`state`]: Self::state
    #[inline]
    pub fn is_mouse_down(&self) -> bool {
        self.state == ButtonState::Pressed
    }

    /// If the [`state`] is released.
    ///
    /// [`state`]: Self::state
    #[inline]
    pub fn is_mouse_up(&self) -> bool {
        self.state == ButtonState::Released
    }
}

impl MouseClickArgs {
    /// If the [`button`](Self::button) is the primary.
    #[inline]
    pub fn is_primary(&self) -> bool {
        self.button == MouseButton::Left
    }

    /// If the [`button`](Self::button) is the context (right).
    #[inline]
    pub fn is_context(&self) -> bool {
        self.button == MouseButton::Right
    }

    /// If the [`click_count`](Self::click_count) is `1`.
    #[inline]
    pub fn is_single(&self) -> bool {
        self.click_count.get() == 1
    }

    /// If the [`click_count`](Self::click_count) is `2`.
    #[inline]
    pub fn is_double(&self) -> bool {
        self.click_count.get() == 2
    }

    /// If the [`click_count`](Self::click_count) is `3`.
    #[inline]
    pub fn is_triple(&self) -> bool {
        self.click_count.get() == 3
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

impl MouseWheelArgs {
    /// Swaps the delta axis if [`modifiers`] contains `SHIFT`.
    ///
    /// [`modifiers`]: Self::modifiers
    pub fn shifted_delta(&self) -> MouseScrollDelta {
        if self.modifiers.shift() {
            match self.delta {
                MouseScrollDelta::LineDelta(x, y) => MouseScrollDelta::LineDelta(y, x),
                MouseScrollDelta::PixelDelta(x, y) => MouseScrollDelta::PixelDelta(y, x),
            }
        } else {
            self.delta
        }
    }

    /// If the modifiers allow the event to be used for scrolling.
    ///
    /// Is `true` if only `SHIFT`, `ALT` or none modifiers are pressed. If `true` the
    /// [`scroll_delta`] method returns a value.
    ///
    /// [`scroll_delta`]: Self::scroll_delta
    pub fn is_scroll(&self) -> bool {
        self.modifiers.is_empty()
            || self.modifiers == ModifiersState::SHIFT
            || self.modifiers == ModifiersState::ALT
            || self.modifiers == ModifiersState::SHIFT | ModifiersState::ALT
    }

    /// Returns the delta for a scrolling operation, depending on the [`modifiers`].
    ///
    /// If `ALT` is pressed scales the delta by `alt_factor`, then, if no more modifers are pressed returns
    /// the scaled delta, if only `SHIFT` is pressed returns the swapped delta, otherwise returns `None`.
    ///
    /// [`modifiers`]: Self::modifiers
    pub fn scroll_delta(&self, alt_factor: impl Into<Factor>) -> Option<MouseScrollDelta> {
        let mut modifiers = self.modifiers;
        let mut delta = self.delta;
        if modifiers.take_alt() {
            let alt_factor = alt_factor.into();
            delta = match delta {
                MouseScrollDelta::LineDelta(x, y) => MouseScrollDelta::LineDelta(x * alt_factor.0, y * alt_factor.0),
                MouseScrollDelta::PixelDelta(x, y) => MouseScrollDelta::PixelDelta(x * alt_factor.0, y * alt_factor.0),
            };
        }

        if modifiers.is_empty() {
            Some(delta)
        } else if modifiers == ModifiersState::SHIFT {
            Some(match delta {
                MouseScrollDelta::LineDelta(x, y) => MouseScrollDelta::LineDelta(y, x),
                MouseScrollDelta::PixelDelta(x, y) => MouseScrollDelta::PixelDelta(y, x),
            })
        } else {
            None
        }
    }

    /// If the modifiers allow the event to be used for zooming.
    ///
    /// Is `true` if only `CTRL` is pressed.  If `true` the [`zoom_delta`] method returns a value.
    ///
    /// [`zoom_delta`]: Self::zoom_delta
    pub fn is_zoom(&self) -> bool {
        self.modifiers == ModifiersState::CTRL
    }

    /// Returns the delta for a zoom-in/out operation, depending on the [`modifiers`].
    ///
    /// If only `CTRL` is pressed returns the delta, otherwise returns `None`.
    ///
    /// [`modifiers`]: Self::modifiers
    pub fn zoom_delta(&self) -> Option<MouseScrollDelta> {
        if self.modifiers == ModifiersState::CTRL {
            Some(self.delta)
        } else {
            None
        }
    }
}

event! {
    /// Mouse move event.
    pub MouseMoveEvent: MouseMoveArgs;

    /// Mouse down or up event.
    pub MouseInputEvent: MouseInputArgs;

    /// Mouse click event, any [`click_count`](MouseClickArgs::click_count).
    pub MouseClickEvent: MouseClickArgs;

    /// The top-most hovered widget changed or mouse capture changed.
    pub MouseHoveredEvent: MouseHoverArgs;

    /// Mouse capture changed event.
    pub MouseCaptureEvent: MouseCaptureArgs;

    /// Mouse wheel scroll event.
    pub MouseWheelEvent: MouseWheelArgs;
}

/// Application extension that provides mouse events and service.
///
/// # Events
///
/// Events this extension provides.
///
/// * [MouseMoveEvent]
/// * [MouseInputEvent]
/// * [MouseClickEvent]
/// * [MouseHoveredEvent]
/// * [MouseCaptureEvent]
///
/// # Services
///
/// Services this extension provides.
///
/// * [Mouse]
pub struct MouseManager {
    // last cursor move position (scaled).
    pos: DipPoint,
    // last cursor move over `pos_window` and source device.
    pos_window: Option<WindowId>,
    pos_device: Option<DeviceId>,
    // last cursor move hit-test.
    pos_hits: (FrameId, PxPoint, HitTestResult),

    /// last modifiers.
    modifiers: ModifiersState,

    click_state: ClickState,

    capture_count: u8,

    hovered: Option<WidgetPath>,

    multi_click_config: RcVar<MultiClickConfig>,
}
impl Default for MouseManager {
    fn default() -> Self {
        MouseManager {
            pos: DipPoint::zero(),
            pos_window: None,
            pos_device: None,
            pos_hits: (FrameId::INVALID, PxPoint::new(Px(-1), Px(-1)), HitTestResult::default()),

            modifiers: ModifiersState::default(),

            click_state: ClickState::None,

            hovered: None,

            capture_count: 0,

            multi_click_config: var(MultiClickConfig::default()),
        }
    }
}
impl MouseManager {
    fn on_mouse_input(&mut self, window_id: WindowId, device_id: DeviceId, state: ButtonState, button: MouseButton, ctx: &mut AppContext) {
        let position = if self.pos_window == Some(window_id) {
            self.pos
        } else {
            DipPoint::default()
        };

        let (windows, mouse) = ctx.services.req_multi::<(Windows, Mouse)>();

        let hits = FrameHitInfo::new(window_id, self.pos_hits.0, self.pos_hits.1, &self.pos_hits.2);

        let frame_info = windows.widget_tree(window_id).unwrap();

        let target = hits
            .target()
            .and_then(|t| frame_info.find(t.widget_id).map(|w| w.path()))
            .unwrap_or_else(|| frame_info.root().path());

        if state == ButtonState::Pressed {
            self.capture_count += 1;
            if self.capture_count == 1 {
                mouse.start_window_capture(target.clone(), ctx.events);
            }

            if !mouse.buttons.get(ctx.vars).contains(&button) {
                mouse.buttons.modify(ctx.vars, move |mut btns| btns.push(button));
            }
        } else {
            self.capture_count = self.capture_count.saturating_sub(1);
            if self.capture_count == 0 {
                mouse.end_window_capture(ctx.events);
            }

            if mouse.buttons.get(ctx.vars).contains(&button) {
                mouse.buttons.modify(ctx.vars, move |mut btns| {
                    if let Some(i) = btns.iter().position(|k| *k == button) {
                        btns.swap_remove(i);
                    }
                });
            }
        }

        let capture_info = if let Some((capture, mode)) = mouse.current_capture() {
            Some(CaptureInfo {
                target: capture.clone(),
                mode,
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
        MouseInputEvent.notify(ctx.events, args);

        match state {
            ButtonState::Pressed => {
                // on_mouse_down

                let now = Instant::now();
                match &mut self.click_state {
                    // maybe a click.
                    ClickState::None => {
                        self.click_state = ClickState::Pressed {
                            btn: button,
                            press_tgt: target,
                        }
                    }
                    // already clicking, maybe multi-click.
                    ClickState::Clicked {
                        start_time,
                        btn,
                        pos,
                        start_tgt,
                        count,
                    } => {
                        debug_assert!(*count >= 1);

                        let cfg = self.multi_click_config.get(ctx.vars);

                        let is_multi_click =
                            // same button
                            *btn == button
                            // within time window
                            && (now - *start_time) <= cfg.time
                            // same widget
                            && start_tgt == &target
                            // within distance of first click
                            && {
                                let dist = (*pos - self.pos).abs();
                                dist.x <= cfg.area.width && dist.y <= cfg.area.height
                            };

                        if is_multi_click {
                            *count = count.saturating_add(1);
                            *start_time = now;

                            let args = MouseClickArgs::new(
                                now,
                                window_id,
                                device_id,
                                button,
                                self.pos,
                                self.modifiers,
                                NonZeroU8::new(*count).unwrap(),
                                hits,
                                target,
                            );
                            MouseClickEvent.notify(ctx.events, args);
                        } else {
                            // start again.
                            self.click_state = ClickState::Pressed {
                                btn: button,
                                press_tgt: target,
                            };
                        }
                    }
                    // more then one Pressed without a Released.
                    ClickState::Pressed { .. } => {
                        self.click_state = ClickState::None;
                    }
                }
            }
            ButtonState::Released => {
                // on_mouse_up

                match &self.click_state {
                    ClickState::Pressed { btn, press_tgt } => {
                        // first click if `Pressed` and `Released` with the same button over the same widget.

                        let mut is_click = false;
                        if *btn == button {
                            if let Some(target) = press_tgt.shared_ancestor(&target) {
                                is_click = true;

                                let now = Instant::now();
                                let args = MouseClickArgs::new(
                                    now,
                                    window_id,
                                    device_id,
                                    button,
                                    position,
                                    self.modifiers,
                                    NonZeroU8::new(1).unwrap(),
                                    hits,
                                    target.clone(),
                                );
                                MouseClickEvent.notify(ctx.events, args);

                                self.click_state = ClickState::Clicked {
                                    start_time: now,
                                    btn: button,
                                    pos: self.pos,
                                    start_tgt: target,
                                    count: 1,
                                };
                            }
                        }

                        if !is_click {
                            self.click_state = ClickState::None;
                        }
                    }
                    ClickState::None => {
                        // Released without Pressed
                    }
                    ClickState::Clicked { btn, start_tgt, .. } => {
                        // is clicking, but can't continue if we are not releasing the same button over the same target.
                        if *btn != button || start_tgt != &target {
                            self.click_state = ClickState::None;
                        }
                    }
                }
            }
        }
    }

    fn on_cursor_moved(
        &mut self,
        window_id: WindowId,
        device_id: DeviceId,
        coalesced_pos: Vec<DipPoint>,
        position: DipPoint,
        hits_res: (FrameId, PxPoint, HitTestResult),
        ctx: &mut AppContext,
    ) {
        let mut moved = Some(window_id) != self.pos_window || Some(device_id) != self.pos_device;

        if moved {
            // if is over another window now.
            self.pos_window = Some(window_id);
            self.pos_device = Some(device_id);
        }

        moved |= position != self.pos;

        if moved {
            // if moved to another window or within the same window.

            self.pos = position;

            let windows = ctx.services.windows();

            // mouse_move data
            let frame_info = match windows.widget_tree(window_id) {
                Ok(f) => f,
                Err(_) => {
                    // window not found
                    if let Some(hovered) = self.hovered.take() {
                        let capture = self.capture_info(ctx.services.mouse());
                        let args = MouseHoverArgs::now(
                            window_id,
                            device_id,
                            position,
                            FrameHitInfo::no_hits(window_id),
                            Some(hovered),
                            None,
                            capture.clone(),
                            capture,
                        );
                        MouseHoveredEvent.notify(ctx, args);
                    }
                    return;
                }
            };

            let hits = FrameHitInfo::new(window_id, hits_res.0, hits_res.1, &hits_res.2);

            let target = if let Some(t) = hits.target() {
                frame_info.find(t.widget_id).map(|w| w.path()).unwrap_or_else(|| {
                    tracing::error!("hits target `{}` not found", t.widget_id);
                    frame_info.root().path()
                })
            } else {
                frame_info.root().path()
            };

            let capture = self.capture_info(ctx.services.mouse());

            // mouse_enter/mouse_leave.
            let hovered_args = if self.hovered.as_ref().map(|h| h != &target).unwrap_or(true) {
                let prev_target = mem::replace(&mut self.hovered, Some(target.clone()));
                let args = MouseHoverArgs::now(
                    window_id,
                    device_id,
                    position,
                    hits.clone(),
                    prev_target,
                    target.clone(),
                    capture.clone(),
                    capture.clone(),
                );
                Some(args)
            } else {
                None
            };

            // mouse_move
            let args = MouseMoveArgs::now(window_id, device_id, self.modifiers, coalesced_pos, position, hits, target, capture);
            MouseMoveEvent.notify(ctx.events, args);

            if let Some(args) = hovered_args {
                MouseHoveredEvent.notify(ctx, args);
            }
        } else if coalesced_pos.is_empty() {
            tracing::warn!("RawCursorMoved did not actually move")
        }

        self.pos_hits = hits_res;
    }

    fn on_scroll(&self, window_id: WindowId, device_id: DeviceId, delta: MouseScrollDelta, phase: TouchPhase, ctx: &mut AppContext) {
        let position = if self.pos_window == Some(window_id) {
            self.pos
        } else {
            DipPoint::default()
        };

        let windows = ctx.services.windows();

        let hits = FrameHitInfo::new(window_id, self.pos_hits.0, self.pos_hits.1, &self.pos_hits.2);

        let frame_info = windows.widget_tree(window_id).unwrap();

        let target = hits
            .target()
            .and_then(|t| frame_info.find(t.widget_id).map(|w| w.path()))
            .unwrap_or_else(|| frame_info.root().path());

        let args = MouseWheelArgs::now(window_id, device_id, position, self.modifiers, delta, phase, hits, target);
        MouseWheelEvent.notify(ctx.events, args);
    }

    fn capture_info(&self, mouse: &mut Mouse) -> Option<CaptureInfo> {
        if let Some((path, mode)) = mouse.current_capture() {
            Some(CaptureInfo {
                target: path.clone(),
                mode,
            })
        } else {
            None
        }
    }

    fn on_cursor_left_window(&mut self, window_id: WindowId, device_id: DeviceId, ctx: &mut AppContext) {
        if Some(window_id) == self.pos_window.take() {
            if let Some(path) = self.hovered.take() {
                let capture = ctx.services.mouse().current_capture().map(|(path, mode)| CaptureInfo {
                    target: path.clone(),
                    mode,
                });
                let args = MouseHoverArgs::now(
                    window_id,
                    device_id,
                    self.pos,
                    FrameHitInfo::no_hits(window_id),
                    Some(path),
                    None,
                    capture.clone(),
                    capture,
                );
                MouseHoveredEvent.notify(ctx.events, args);
            }
        }
    }

    fn on_window_blur(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        self.release_window_capture(window_id, ctx);
    }

    fn on_window_closed(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        self.release_window_capture(window_id, ctx);
    }

    fn release_window_capture(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        let mouse = ctx.services.mouse();
        if let Some((path, _)) = mouse.current_capture() {
            if path.window_id() == window_id {
                mouse.end_window_capture(ctx.events);
                self.capture_count = 0;
            }
        }
    }
}
impl AppExtension for MouseManager {
    fn init(&mut self, ctx: &mut AppContext) {
        if let Some(cfg) = ctx.services.get::<ViewProcess>().and_then(|vp| vp.multi_click_config().ok()) {
            self.multi_click_config.set_ne(ctx.vars, cfg);
        }
        ctx.services
            .register(Mouse::new(ctx.updates.sender(), self.multi_click_config.clone()));
    }

    fn event_preview<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        if let Some(args) = RawFrameRenderedEvent.update(args) {
            // update hovered
            if self.pos_window == Some(args.window_id) {
                let (windows, mouse) = ctx.services.req_multi::<(Windows, Mouse)>();
                let hits = FrameHitInfo::new(args.window_id, args.frame_id, args.cursor_hits.0, &args.cursor_hits.1);
                let target = hits
                    .target()
                    .and_then(|t| windows.widget_tree(args.window_id).unwrap().find(t.widget_id))
                    .map(|w| w.path());

                self.pos_hits = (args.frame_id, args.cursor_hits.0, args.cursor_hits.1.clone());

                if self.hovered != target {
                    let capture = self.capture_info(mouse);
                    let prev = mem::replace(&mut self.hovered, target.clone());
                    let args = MouseHoverArgs::now(args.window_id, None, self.pos, hits, prev, target, capture.clone(), capture);
                    MouseHoveredEvent.notify(ctx.events, args);
                }
            }
            // update capture
            if self.capture_count > 0 {
                let (mouse, windows) = ctx.services.req_multi::<(Mouse, Windows)>();
                if let Ok(frame) = windows.widget_tree(args.window_id) {
                    mouse.continue_capture(frame, ctx.events);
                }
            }
        } else if let Some(args) = RawCursorMovedEvent.update(args) {
            self.on_cursor_moved(
                args.window_id,
                args.device_id,
                args.coalesced_pos.clone(),
                args.position,
                args.hits.clone(),
                ctx,
            );
        } else if let Some(args) = RawMouseWheelEvent.update(args) {
            self.on_scroll(args.window_id, args.device_id, args.delta, args.phase, ctx);
        } else if let Some(args) = RawMouseInputEvent.update(args) {
            self.on_mouse_input(args.window_id, args.device_id, args.state, args.button, ctx);
        } else if let Some(args) = RawModifiersChangedEvent.update(args) {
            self.modifiers = args.modifiers;
        } else if let Some(args) = RawCursorLeftEvent.update(args) {
            self.on_cursor_left_window(args.window_id, args.device_id, ctx);
        } else if let Some(args) = RawWindowFocusEvent.update(args) {
            if !args.focused {
                self.on_window_blur(args.window_id, ctx);
            }
        } else if let Some(args) = RawWindowCloseEvent.update(args) {
            self.on_window_closed(args.window_id, ctx);
        } else if let Some(args) = RawMultiClickConfigChangedEvent.update(args) {
            self.multi_click_config.set_ne(ctx.vars, args.config);
            self.click_state = ClickState::None;
        } else if ViewProcessRespawnedEvent.update(args).is_some() {
            let multi_click_cfg = ctx
                .services
                .get::<ViewProcess>()
                .and_then(|vp| vp.multi_click_config().ok())
                .unwrap_or_default();

            self.multi_click_config.set_ne(ctx.vars, multi_click_cfg);

            let mouse = ctx.services.mouse();

            if let Some(window_id) = self.pos_window.take() {
                if let Some(path) = self.hovered.take() {
                    let args = MouseHoverArgs::now(
                        window_id,
                        None,
                        DipPoint::new(Dip::new(-1), Dip::new(-1)),
                        FrameHitInfo::no_hits(window_id),
                        Some(path),
                        None,
                        None,
                        None,
                    );
                    MouseHoveredEvent.notify(ctx.events, args);
                }
            }
            mouse.current_capture = None;
            mouse.capture_request = None;
            mouse.release_requested = false;
            mouse.buttons.set_ne(ctx.vars, vec![]);
        }
    }

    fn event<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        if let Some(args) = MouseCaptureEvent.update(args) {
            if let Some(path) = &self.hovered {
                if let Some(window_id) = self.pos_window {
                    let hover_args = MouseHoverArgs::now(
                        window_id,
                        self.pos_device.unwrap(),
                        self.pos,
                        FrameHitInfo::new(window_id, self.pos_hits.0, self.pos_hits.1, &self.pos_hits.2),
                        Some(path.clone()),
                        Some(path.clone()),
                        args.prev_capture.as_ref().map(|(path, mode)| CaptureInfo {
                            target: path.clone(),
                            mode: *mode,
                        }),
                        args.new_capture.as_ref().map(|(path, mode)| CaptureInfo {
                            target: path.clone(),
                            mode: *mode,
                        }),
                    );
                    MouseHoveredEvent.notify(ctx.events, hover_args);
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut AppContext) {
        let (mouse, windows) = ctx.services.req_multi::<(Mouse, Windows)>();

        mouse.fulfill_requests(windows, ctx.events);
    }
}

enum ClickState {
    /// Before start.
    None,
    /// Mouse pressed on a widget, if the next event
    /// is a release over the same widget a click event is generated.
    Pressed { btn: MouseButton, press_tgt: WidgetPath },
    /// At least one click completed, as long as subsequent presses happen
    /// within the window of time, widget and distance from the initial press
    /// multi-click events are generated.
    Clicked {
        start_time: Instant,
        btn: MouseButton,
        pos: DipPoint,
        start_tgt: WidgetPath,

        count: u8,
    },
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
#[derive(Service)]
pub struct Mouse {
    current_capture: Option<(WidgetPath, CaptureMode)>,
    capture_request: Option<(WidgetId, CaptureMode)>,
    release_requested: bool,
    update_sender: AppEventSender,
    multi_click_config: RcVar<MultiClickConfig>,
    buttons: RcVar<Vec<MouseButton>>,
}
impl Mouse {
    fn new(update_sender: AppEventSender, multi_click_config: RcVar<MultiClickConfig>) -> Self {
        Mouse {
            current_capture: None,
            capture_request: None,
            release_requested: false,
            update_sender,
            multi_click_config,
            buttons: var(vec![]),
        }
    }

    /// Returns a read-only variable that tracks the [buttons] that are currently pressed.
    ///
    /// [buttons]: MouseButton
    #[inline]
    pub fn buttons(&self) -> ReadOnlyRcVar<Vec<MouseButton>> {
        self.buttons.clone().into_read_only()
    }

    /// Read-only variable that tracks the system click-count increment time and area, a.k.a. the double-click config.
    ///
    /// Repeated clicks with an interval less then this time and within the distance of the first click increment the click count.
    ///
    /// # Value Source
    ///
    /// The value comes from the operating system settings (TODO, only implemented in Windows), the variable
    /// updates with a new value if the system setting is changed.
    ///
    /// In headless apps the default is [`MultiClickConfig::default`] and does not change.
    ///
    /// Internally the [`RawMultiClickConfigChangedEvent`] is listened to update this variable, so you can notify
    /// this event to set this variable, if you really must.
    #[inline]
    pub fn multi_click_config(&self) -> ReadOnlyRcVar<MultiClickConfig> {
        self.multi_click_config.clone().into_read_only()
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
        let _ = self.update_sender.send_ext_update();
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
        let _ = self.update_sender.send_ext_update();
    }

    /// Release the current mouse capture back to window.
    ///
    /// **Note:** The capture is released automatically when the mouse buttons are released
    /// or when the window loses focus.
    #[inline]
    pub fn release_capture(&mut self) {
        self.release_requested = true;
        let _ = self.update_sender.send_ext_update();
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
    pub fn lock_cursor(&mut self, window_id: WindowId, area: DipRect) {
        // https://docs.rs/winit/0.24.0/winit/window/struct.Window.html#method.set_cursor_grab
        // https://github.com/rust-windowing/winit/issues/1677
        todo!("impl lockcursor({window_id:?}, {area:?})")
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
    pub fn lock_cursor_pt(&mut self, window_id: WindowId, point: DipPoint) {
        self.lock_cursor(window_id, DipRect::new(point, DipSize::zero()))
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
        todo!("impl lock_cursor_wgt({window_id:?}, {area:?})")
    }

    /// Locks the cursor to the content area of the window if the window is focused.
    ///
    /// The pointer is moved inside the window to start, the user can only move the cursor inside the window area.
    ///
    /// If the window moves or is resized the cursor gets pushed by the sides of the window area.
    ///
    /// **NOT IMPLEMENTED**
    pub fn lock_cursor_window(&mut self, window_id: WindowId) {
        todo!("impl lock_cursor_window({window_id:?}")
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
    fn start_window_capture(&mut self, mouse_down: WidgetPath, events: &mut Events) {
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
    fn fulfill_requests(&mut self, windows: &Windows, events: &mut Events) {
        if let Some((current_target, current_mode)) = &self.current_capture {
            if let Some((widget_id, mode)) = self.capture_request.take() {
                if let Ok(true) = windows.is_focused(current_target.window_id()) {
                    // current window pressed
                    if let Some(widget) = windows.widget_tree(current_target.window_id()).unwrap().find(widget_id) {
                        // request valid
                        self.set_capture(widget.path(), mode, events);
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
    fn continue_capture(&mut self, frame: &WidgetInfoTree, events: &mut Events) {
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
    fn end_window_capture(&mut self, events: &mut Events) {
        self.release_requested = false;
        self.capture_request = None;
        self.unset_capture(events);
    }

    fn set_capture(&mut self, target: WidgetPath, mode: CaptureMode, events: &mut Events) {
        let new = Some((target, mode));
        if new != self.current_capture {
            let prev = self.current_capture.take();
            self.current_capture = new.clone();
            MouseCaptureEvent.notify(events, MouseCaptureArgs::now(prev, new));
        }
    }

    fn unset_capture(&mut self, events: &mut Events) {
        if self.current_capture.is_some() {
            let prev = self.current_capture.take();
            MouseCaptureEvent.notify(events, MouseCaptureArgs::now(prev, None));
        }
    }
}

/// Mouse capture mode.
#[derive(Copy, Clone, PartialEq, Eq)]
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
impl fmt::Debug for CaptureMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "CaptureMode::")?;
        }
        match self {
            CaptureMode::Window => write!(f, "Window"),
            CaptureMode::Subtree => write!(f, "Subtree"),
            CaptureMode::Widget => write!(f, "Widget"),
        }
    }
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
    /// Capture mode, see [`allows`](Self::allows) for more details.
    pub mode: CaptureMode,
}
impl CaptureInfo {
    /// If the widget is allowed by the current capture.
    ///
    /// | Mode           | Allows                                             |
    /// |----------------|----------------------------------------------------|
    /// | `Window`       | All widgets in the same window.                    |
    /// | `Subtree`      | All widgets that have the `target` in their path.  |
    /// | `Widget`       | Only the `target` widget.                          |
    #[inline]
    pub fn allows(&self, path: &WidgetContextPath) -> bool {
        match self.mode {
            CaptureMode::Window => self.target.window_id() == path.window_id(),
            CaptureMode::Widget => self.target.widget_id() == path.widget_id(),
            CaptureMode::Subtree => path.contains(self.target.widget_id()),
        }
    }
}
