//! Mouse events and service.
//!
//! The app extension [`MouseManager`] provides the events and service. It is included in the default application.

use crate::{
    app::{raw_events::*, view_process::VIEW_PROCESS_INITED_EVENT, *},
    context::*,
    event::*,
    keyboard::{ModifiersState, MODIFIERS_CHANGED_EVENT},
    service::*,
    units::*,
    var::{impl_from_and_into_var, var, RcVar, ReadOnlyRcVar, Var},
    widget_info::{HitTestInfo, InteractionPath, WidgetInfoTree, WidgetPath},
    window::{WindowId, Windows},
    widget_instance::WidgetId,
};
use std::{fmt, mem, num::NonZeroU8, time::*};

use linear_map::LinearMap;
pub use zero_ui_view_api::{ButtonState, MouseButton, MouseScrollDelta, MultiClickConfig, TouchForce, TouchPhase};

event_args! {
    /// [`MOUSE_MOVE_EVENT`] arguments.
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
        pub hits: HitTestInfo,

        /// Full path to the top-most hit in [`hits`](MouseMoveArgs::hits).
        pub target: InteractionPath,

        /// Current mouse capture.
        pub capture: Option<CaptureInfo>,

        ..

        /// The [`target`] and [`capture`].
        ///
        /// [`target`]: Self::target
        /// [`capture`]: Self::capture
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_path(&self.target);
            if let Some(c) = &self.capture {
                list.insert_path(&c.target);
            }
        }
    }

    /// [`MOUSE_INPUT_EVENT`] arguments.
    pub struct MouseInputArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: Option<DeviceId>,

        /// Which mouse button generated the event.
        pub button: MouseButton,

        /// Position of the mouse in the window's content area.
        pub position: DipPoint,

        /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        /// The state the [`button`] was changed to.
        ///
        /// [`button`]: Self::button
        pub state: ButtonState,

        /// Hit-test result for the mouse point in the window.
        pub hits: HitTestInfo,

        /// Full path to the top-most hit in [`hits`].
        ///
        /// [`hits`]: Self::hits
        pub target: InteractionPath,

        /// Current mouse capture.
        pub capture: Option<CaptureInfo>,

        /// Last [`target`] pressed by the [`button`] that is now [released].
        ///
        /// [`target`]: Self::target
        /// [`button`]: Self::button
        /// [released]: Self::state
        pub prev_pressed: Option<InteractionPath>,

        ..

        /// The [`target`], [`prev_pressed`] and [`capture`].
        ///
        /// [`target`]: Self::target
        /// [`prev_pressed`]: Self::prev_pressed
        /// [`capture`]: Self::capture
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_path(&self.target);
            if let Some(p) = &self.prev_pressed {
                list.insert_path(p);
            }
            if let Some(c) = &self.capture {
                list.insert_path(&c.target);
            }
        }
    }

    /// [`MOUSE_CLICK_EVENT`] arguments.
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

        /// Sequential click count. Number `1` is single click, `2` is double click, etc.
        pub click_count: NonZeroU8,

        /// Hit-test result for the mouse point in the window, at the moment the click event
        /// was generated.
        pub hits: HitTestInfo,

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
        pub target: InteractionPath,

        ..

        /// The [`target`].
        ///
        /// [`target`]: MouseClickArgs::target
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_path(&self.target)
        }
    }

    /// [`MOUSE_HOVERED_EVENT`] arguments.
    pub struct MouseHoverArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: Option<DeviceId>,

        /// Position of the mouse in the window.
        pub position: DipPoint,

        /// Hit-test result for the mouse point in the window.
        pub hits: HitTestInfo,

        /// Previous top-most hit before the mouse moved.
        pub prev_target: Option<InteractionPath>,

        /// Full path to the top-most hit in [`hits`].
        ///
        /// Is `None` when the mouse moves out of a window or the window closes under the mouse
        /// and there was a previous hovered widget.
        ///
        /// [`hits`]: MouseInputArgs::hits
        pub target: Option<InteractionPath>,

        /// Previous mouse capture.
        pub prev_capture: Option<CaptureInfo>,

        /// Current mouse capture.
        pub capture: Option<CaptureInfo>,

        ..

        /// The [`target`], [`prev_target`] and [`capture`].
        ///
        /// [`target`]: Self::target
        /// [`prev_target`]: Self::prev_target
        /// [`capture`]: Self::capture
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            if let Some(p) = &self.prev_target {
                list.insert_path(p);
            }
            if let Some(p) = &self.target {
                list.insert_path(p);
            }
            if let Some(c) = &self.capture {
                list.insert_path(&c.target);
            }
        }
    }

    /// [`MOUSE_CAPTURE_EVENT`] arguments.
    pub struct MouseCaptureArgs {
        /// Previous mouse capture target and mode.
        pub prev_capture: Option<(WidgetPath, CaptureMode)>,
        /// new mouse capture target and mode.
        pub new_capture: Option<(WidgetPath, CaptureMode)>,

        ..

        /// The [`prev_capture`] and [`new_capture`] paths start with the current path.
        ///
        /// [`prev_capture`]: Self::prev_capture
        /// [`new_capture`]: Self::new_capture
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            if let Some((p, _)) = &self.prev_capture {
                list.insert_path(p);
            }
            if let Some((p, _)) = &self.new_capture {
                list.insert_path(p);
            }
        }
    }

    /// [`MOUSE_WHEEL_EVENT`] arguments.
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
        pub hits: HitTestInfo,

        /// Full path to the widget that got scrolled.
        pub target: InteractionPath,

        ..

        /// The [`target`].
        ///
        /// [`target`]: MouseWheelArgs::target
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_path(&self.target)
        }
    }
}

impl MouseHoverArgs {
    /// If [`capture`] is `None` or [`allows`] the `path` to receive this event.
    ///
    /// [`capture`]: Self::capture
    /// [`allows`]: CaptureInfo::allows
    pub fn capture_allows(&self, path: &WidgetContextPath) -> bool {
        self.capture.as_ref().map(|c| c.allows(path)).unwrap_or(true)
    }

    /// Event caused by the mouse position moving over/out of the widget bounds.
    pub fn is_mouse_move(&self) -> bool {
        self.device_id.is_some()
    }

    /// Event caused by the widget moving under/out of the mouse position.
    pub fn is_widget_move(&self) -> bool {
        self.device_id.is_none()
    }

    /// Event caused by a mouse capture change.
    pub fn is_capture_change(&self) -> bool {
        self.prev_capture != self.capture
    }

    /// Returns `true` if the widget was not hovered, but now is.
    pub fn is_mouse_enter(&self, path: &WidgetContextPath) -> bool {
        !self.was_over(path) && self.is_over(path)
    }

    /// Returns `true` if the widget was hovered, but now isn't.
    pub fn is_mouse_leave(&self, path: &WidgetContextPath) -> bool {
        self.was_over(path) && !self.is_over(path)
    }

    /// Returns `true` if the widget was not hovered or was disabled, but now is hovered and enabled.
    pub fn is_mouse_enter_enabled(&self, path: &WidgetContextPath) -> bool {
        (!self.was_over(path) || self.was_disabled(path.widget_id())) && self.is_over(path) && self.is_enabled(path.widget_id())
    }

    /// Returns `true` if the widget as hovered and enabled, but now is not hovered or is disabled.
    pub fn is_mouse_leave_enabled(&self, path: &WidgetContextPath) -> bool {
        self.was_over(path) && self.was_enabled(path.widget_id()) && (!self.is_over(path) || self.is_disabled(path.widget_id()))
    }

    /// Returns `true` if the widget was not hovered or was enabled, but now is hovered and disabled.
    pub fn is_mouse_enter_disabled(&self, path: &WidgetContextPath) -> bool {
        (!self.was_over(path) || self.was_enabled(path.widget_id())) && self.is_over(path) && self.is_disabled(path.widget_id())
    }

    /// Returns `true` if the widget was hovered and disabled, but now is not hovered or is enabled.
    pub fn is_mouse_leave_disabled(&self, path: &WidgetContextPath) -> bool {
        self.was_over(path) && self.was_disabled(path.widget_id()) && (!self.is_over(path) || self.is_enabled(path.widget_id()))
    }

    /// Returns `true` if the widget is in [`prev_target`] and is allowed by the [`prev_capture`].
    ///
    /// [`prev_target`]: Self::prev_target
    /// [`prev_capture`]: Self::prev_capture
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

    /// Returns `true` if the widget was enabled in [`prev_target`].
    ///
    /// [`prev_target`]: Self::prev_target
    pub fn was_enabled(&self, widget_id: WidgetId) -> bool {
        self.prev_target
            .as_ref()
            .and_then(|t| t.interactivity_of(widget_id))
            .map(|itr| itr.is_enabled())
            .unwrap_or(false)
    }

    /// Returns `true` if the widget was disabled in [`prev_target`].
    ///
    /// [`prev_target`]: Self::prev_target
    pub fn was_disabled(&self, widget_id: WidgetId) -> bool {
        self.prev_target
            .as_ref()
            .and_then(|t| t.interactivity_of(widget_id))
            .map(|itr| itr.is_disabled())
            .unwrap_or(false)
    }

    /// Returns `true` if the widget is enabled in [`target`].
    ///
    /// [`target`]: Self::target
    pub fn is_enabled(&self, widget_id: WidgetId) -> bool {
        self.target
            .as_ref()
            .and_then(|t| t.interactivity_of(widget_id))
            .map(|itr| itr.is_enabled())
            .unwrap_or(false)
    }

    /// Returns `true` if the widget is disabled in [`target`].
    ///
    /// [`target`]: Self::target
    pub fn is_disabled(&self, widget_id: WidgetId) -> bool {
        self.target
            .as_ref()
            .and_then(|t| t.interactivity_of(widget_id))
            .map(|itr| itr.is_disabled())
            .unwrap_or(false)
    }
}

impl MouseMoveArgs {
    /// If [`capture`] is `None` or [`allows`] the `path` to receive this event.
    ///
    /// [`capture`]: Self::capture
    /// [`allows`]: CaptureInfo::allows
    pub fn capture_allows(&self, path: &WidgetContextPath) -> bool {
        self.capture.as_ref().map(|c| c.allows(path)).unwrap_or(true)
    }
}

impl MouseInputArgs {
    /// If [`capture`] is `None` or [`allows`] the `path` to receive this event.
    ///
    /// [`capture`]: Self::capture
    /// [`allows`]: CaptureInfo::allows
    pub fn capture_allows(&self, path: &WidgetContextPath) -> bool {
        self.capture.as_ref().map(|c| c.allows(path)).unwrap_or(true)
    }

    /// If the `path` is in the [`target`].
    ///
    /// [`target`]: Self::target
    pub fn is_over(&self, widget_id: WidgetId) -> bool {
        self.target.contains(widget_id)
    }

    /// If the `path` is in the [`prev_pressed`].
    ///
    /// [`prev_pressed`]: Self::prev_pressed.
    pub fn was_pressed(&self, widget_id: WidgetId) -> bool {
        self.prev_pressed.as_ref().map(|p| p.contains(widget_id)).unwrap_or(false)
    }

    /// If the `path` in the [`target`] is enabled.
    ///
    /// [`target`]: Self::target
    pub fn is_enabled(&self, widget_id: WidgetId) -> bool {
        self.target.interactivity_of(widget_id).map(|i| i.is_enabled()).unwrap_or(false)
    }

    /// If the `path` in the [`target`] is disabled.
    ///
    /// [`target`]: Self::target
    pub fn is_disabled(&self, widget_id: WidgetId) -> bool {
        self.target.interactivity_of(widget_id).map(|i| i.is_disabled()).unwrap_or(false)
    }

    /// If the [`button`] is the primary.
    ///
    /// [`button`]: Self::button
    pub fn is_primary(&self) -> bool {
        self.button == MouseButton::Left
    }

    /// If the [`button`](Self::button) is the context (right).
    pub fn is_context(&self) -> bool {
        self.button == MouseButton::Right
    }

    /// If the [`state`] is pressed.
    ///
    /// [`state`]: Self::state
    pub fn is_mouse_down(&self) -> bool {
        self.state == ButtonState::Pressed
    }

    /// If the [`state`] is released.
    ///
    /// [`state`]: Self::state
    pub fn is_mouse_up(&self) -> bool {
        self.state == ButtonState::Released
    }
}

impl MouseClickArgs {
    /// Returns `true` if the widget is enabled in [`target`].
    ///
    /// [`target`]: Self::target
    pub fn is_enabled(&self, widget_id: WidgetId) -> bool {
        self.target.interactivity_of(widget_id).map(|i| i.is_enabled()).unwrap_or(false)
    }

    /// Returns `true` if the widget is disabled in [`target`].
    ///
    /// [`target`]: Self::target
    pub fn is_disabled(&self, widget_id: WidgetId) -> bool {
        self.target.interactivity_of(widget_id).map(|i| i.is_disabled()).unwrap_or(false)
    }

    /// If the [`button`](Self::button) is the primary.
    pub fn is_primary(&self) -> bool {
        self.button == MouseButton::Left
    }

    /// If the [`button`](Self::button) is the context (right).
    pub fn is_context(&self) -> bool {
        self.button == MouseButton::Right
    }

    /// If the [`click_count`](Self::click_count) is `1`.
    pub fn is_single(&self) -> bool {
        self.click_count.get() == 1
    }

    /// If the [`click_count`](Self::click_count) is `2`.
    pub fn is_double(&self) -> bool {
        self.click_count.get() == 2
    }

    /// If the [`click_count`](Self::click_count) is `3`.
    pub fn is_triple(&self) -> bool {
        self.click_count.get() == 3
    }
}

impl MouseCaptureArgs {
    /// If the same widget has mouse capture, but the widget path changed.
    pub fn is_widget_move(&self) -> bool {
        match (&self.prev_capture, &self.new_capture) {
            (Some(prev), Some(new)) => prev.0.widget_id() == new.0.widget_id() && prev.0 != new.0,
            _ => false,
        }
    }

    /// If the same widget has mouse capture, but the capture mode changed.
    pub fn is_mode_change(&self) -> bool {
        match (&self.prev_capture, &self.new_capture) {
            (Some(prev), Some(new)) => prev.0.widget_id() == new.0.widget_id() && prev.1 != new.1,
            _ => false,
        }
    }

    /// If the `widget_id` lost mouse capture with this update.
    pub fn is_lost(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_capture, &self.new_capture) {
            (None, _) => false,
            (Some((path, _)), None) => path.widget_id() == widget_id,
            (Some((prev_path, _)), Some((new_path, _))) => prev_path.widget_id() == widget_id && new_path.widget_id() != widget_id,
        }
    }

    /// If the `widget_id` got mouse capture with this update.
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
        if self.modifiers.has_shift() {
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
        self.modifiers.is_empty() || self.modifiers.is_only(ModifiersState::SHIFT | ModifiersState::ALT)
    }

    /// Returns the delta for a scrolling operation, depending on the [`modifiers`].
    ///
    /// If `ALT` is pressed scales the delta by `alt_factor`, then, if no more modifiers are pressed returns
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
        } else if modifiers.is_only_shift() {
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
        self.modifiers.is_only_ctrl()
    }

    /// Returns the delta for a zoom-in/out operation, depending on the [`modifiers`].
    ///
    /// If only `CTRL` is pressed returns the delta, otherwise returns `None`.
    ///
    /// [`modifiers`]: Self::modifiers
    pub fn zoom_delta(&self) -> Option<MouseScrollDelta> {
        if self.is_zoom() {
            Some(self.delta)
        } else {
            None
        }
    }

    /// Returns `true` if the widget is enabled in [`target`].
    ///
    /// [`target`]: Self::target
    pub fn is_enabled(&self, widget_id: WidgetId) -> bool {
        self.target.interactivity_of(widget_id).map(|i| i.is_enabled()).unwrap_or(false)
    }

    /// Returns `true` if the widget is disabled in [`target`].
    ///
    /// [`target`]: Self::target
    pub fn is_disabled(&self, widget_id: WidgetId) -> bool {
        self.target.interactivity_of(widget_id).map(|i| i.is_disabled()).unwrap_or(false)
    }
}

event! {
    /// Mouse move event.
    pub static MOUSE_MOVE_EVENT: MouseMoveArgs;

    /// Mouse down or up event.
    pub static MOUSE_INPUT_EVENT: MouseInputArgs;

    /// Mouse click event, any [`click_count`](MouseClickArgs::click_count).
    pub static MOUSE_CLICK_EVENT: MouseClickArgs;

    /// The top-most hovered widget changed or mouse capture changed.
    pub static MOUSE_HOVERED_EVENT: MouseHoverArgs;

    /// Mouse capture changed event.
    pub static MOUSE_CAPTURE_EVENT: MouseCaptureArgs;

    /// Mouse wheel scroll event.
    pub static MOUSE_WHEEL_EVENT: MouseWheelArgs;
}

/// Application extension that provides mouse events and service.
///
/// # Events
///
/// Events this extension provides.
///
/// * [`MOUSE_MOVE_EVENT`]
/// * [`MOUSE_INPUT_EVENT`]
/// * [`MOUSE_CLICK_EVENT`]
/// * [`MOUSE_HOVERED_EVENT`]
/// * [`MOUSE_CAPTURE_EVENT`]
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
    pos_hits: Option<HitTestInfo>,

    /// last modifiers.
    modifiers: ModifiersState,

    click_state: ClickState,

    capture_count: u8,

    hovered: Option<InteractionPath>,
    pressed: LinearMap<MouseButton, InteractionPath>,

    multi_click_config: RcVar<MultiClickConfig>,
}
impl Default for MouseManager {
    fn default() -> Self {
        MouseManager {
            pos: DipPoint::zero(),
            pos_window: None,
            pos_device: None,
            pos_hits: None,

            modifiers: ModifiersState::default(),

            click_state: ClickState::None,

            hovered: None,
            pressed: LinearMap::default(),

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
        let (windows, mouse) = <(Windows, Mouse)>::req(ctx.services);

        let hits = self.pos_hits.clone().unwrap_or_else(|| HitTestInfo::no_hits(window_id));

        let frame_info = windows.widget_tree(window_id).unwrap();

        let target = hits
            .target()
            .and_then(|t| frame_info.get(t.widget_id).map(|w| w.interaction_path()))
            .unwrap_or_else(|| frame_info.root().interaction_path());

        let target = match target.unblocked() {
            Some(t) => t,
            None => return,
        };

        let prev_pressed;

        if state == ButtonState::Pressed {
            self.capture_count += 1;
            if self.capture_count == 1 {
                mouse.start_window_capture(target.clone(), ctx.events);
            }

            if !mouse.buttons.with(|b| b.contains(&button)) {
                mouse.buttons.modify(ctx.vars, move |btns| btns.get_mut().push(button));
            }

            prev_pressed = self.pressed.insert(button, target.clone());
        } else {
            // ButtonState::Released
            self.capture_count = self.capture_count.saturating_sub(1);
            if self.capture_count == 0 {
                mouse.end_window_capture(ctx.events);
            }

            if mouse.buttons.with(|b| b.contains(&button)) {
                mouse.buttons.modify(ctx.vars, move |btns| {
                    if let Some(i) = btns.get().iter().position(|k| *k == button) {
                        btns.get_mut().swap_remove(i);
                    }
                });
            }

            prev_pressed = self.pressed.remove(&button);
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
            prev_pressed,
        );

        // on_mouse_input
        MOUSE_INPUT_EVENT.notify(ctx.events, args);

        match state {
            ButtonState::Pressed => {
                // on_mouse_down

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

                        let cfg = self.multi_click_config.get();
                        let now = Instant::now();

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
                                Default::default(),
                                window_id,
                                device_id,
                                button,
                                self.pos,
                                self.modifiers,
                                NonZeroU8::new(*count).unwrap(),
                                hits,
                                target,
                            );
                            MOUSE_CLICK_EVENT.notify(ctx.events, args);
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
                                let target = target.into_owned();

                                is_click = true;

                                let now = Instant::now();
                                let args = MouseClickArgs::new(
                                    now,
                                    Default::default(),
                                    window_id,
                                    device_id,
                                    button,
                                    position,
                                    self.modifiers,
                                    NonZeroU8::new(1).unwrap(),
                                    hits,
                                    target.clone(),
                                );
                                MOUSE_CLICK_EVENT.notify(ctx.events, args);

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

            let windows = Windows::req(ctx.services);

            // mouse_move data
            let frame_info = match windows.widget_tree(window_id) {
                Ok(f) => f,
                Err(_) => {
                    // window not found
                    if let Some(hovered) = self.hovered.take() {
                        let capture = self.capture_info(Mouse::req(ctx.services));
                        let args = MouseHoverArgs::now(
                            window_id,
                            device_id,
                            position,
                            HitTestInfo::no_hits(window_id),
                            Some(hovered),
                            None,
                            capture.clone(),
                            capture,
                        );
                        MOUSE_HOVERED_EVENT.notify(ctx, args);
                    }
                    return;
                }
            };

            let pos_hits = frame_info.root().hit_test(position.to_px(frame_info.scale_factor().0));
            self.pos_hits = Some(pos_hits.clone());

            let target = if let Some(t) = pos_hits.target() {
                frame_info.get(t.widget_id).map(|w| w.interaction_path()).unwrap_or_else(|| {
                    tracing::error!("hits target `{}` not found", t.widget_id);
                    frame_info.root().interaction_path()
                })
            } else {
                frame_info.root().interaction_path()
            }
            .unblocked();

            let capture = self.capture_info(Mouse::req(ctx.services));

            // mouse_enter/mouse_leave.
            let hovered_args = if self.hovered != target {
                let prev_target = mem::replace(&mut self.hovered, target.clone());
                let args = MouseHoverArgs::now(
                    window_id,
                    device_id,
                    position,
                    pos_hits.clone(),
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
            if let Some(target) = target {
                let args = MouseMoveArgs::now(
                    window_id,
                    device_id,
                    self.modifiers,
                    coalesced_pos,
                    position,
                    pos_hits,
                    target,
                    capture,
                );
                MOUSE_MOVE_EVENT.notify(ctx.events, args);
            }

            if let Some(args) = hovered_args {
                MOUSE_HOVERED_EVENT.notify(ctx, args);
            }
        } else if coalesced_pos.is_empty() {
            tracing::warn!("RawCursorMoved did not actually move")
        }
    }

    fn on_scroll(&self, window_id: WindowId, device_id: DeviceId, delta: MouseScrollDelta, phase: TouchPhase, ctx: &mut AppContext) {
        let position = if self.pos_window == Some(window_id) {
            self.pos
        } else {
            DipPoint::default()
        };

        let windows = Windows::req(ctx.services);

        let hits = self.pos_hits.clone().unwrap_or_else(|| HitTestInfo::no_hits(window_id));

        let frame_info = windows.widget_tree(window_id).unwrap();

        let target = hits
            .target()
            .and_then(|t| frame_info.get(t.widget_id).map(|w| w.interaction_path()))
            .unwrap_or_else(|| frame_info.root().interaction_path());

        if let Some(target) = target.unblocked() {
            let args = MouseWheelArgs::now(window_id, device_id, position, self.modifiers, delta, phase, hits, target);
            MOUSE_WHEEL_EVENT.notify(ctx.events, args);
        }
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
                let capture = Mouse::req(ctx.services).current_capture().map(|(path, mode)| CaptureInfo {
                    target: path.clone(),
                    mode,
                });
                let args = MouseHoverArgs::now(
                    window_id,
                    device_id,
                    self.pos,
                    HitTestInfo::no_hits(window_id),
                    Some(path),
                    None,
                    capture.clone(),
                    capture,
                );
                MOUSE_HOVERED_EVENT.notify(ctx.events, args);
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
        let mouse = Mouse::req(ctx.services);
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
        ctx.services
            .register(Mouse::new(ctx.updates.sender(), self.multi_click_config.clone()));
    }

    fn event_preview(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        if let Some(args) = RAW_FRAME_RENDERED_EVENT.on(update) {
            // update hovered
            if self.pos_window == Some(args.window_id) {
                let (windows, mouse) = <(Windows, Mouse)>::req(ctx.services);
                let frame_info = windows.widget_tree(args.window_id).unwrap();
                let pos_hits = frame_info.root().hit_test(self.pos.to_px(frame_info.scale_factor().0));
                self.pos_hits = Some(pos_hits.clone());
                let target = pos_hits
                    .target()
                    .and_then(|t| frame_info.get(t.widget_id))
                    .and_then(|w| w.interaction_path().unblocked());

                if self.hovered != target {
                    let capture = self.capture_info(mouse);
                    let prev = mem::replace(&mut self.hovered, target.clone());
                    let args = MouseHoverArgs::now(args.window_id, None, self.pos, pos_hits, prev, target, capture.clone(), capture);
                    MOUSE_HOVERED_EVENT.notify(ctx.events, args);
                }
            }
            // update capture
            if self.capture_count > 0 {
                let (mouse, windows) = <(Mouse, Windows)>::req(ctx.services);
                if let Ok(frame) = windows.widget_tree(args.window_id) {
                    mouse.continue_capture(frame, ctx.events);
                }
            }
        } else if let Some(args) = RAW_CURSOR_MOVED_EVENT.on(update) {
            self.on_cursor_moved(args.window_id, args.device_id, args.coalesced_pos.clone(), args.position, ctx);
        } else if let Some(args) = RAW_MOUSE_WHEEL_EVENT.on(update) {
            self.on_scroll(args.window_id, args.device_id, args.delta, args.phase, ctx);
        } else if let Some(args) = RAW_MOUSE_INPUT_EVENT.on(update) {
            self.on_mouse_input(args.window_id, args.device_id, args.state, args.button, ctx);
        } else if let Some(args) = MODIFIERS_CHANGED_EVENT.on(update) {
            self.modifiers = args.modifiers;
        } else if let Some(args) = RAW_CURSOR_LEFT_EVENT.on(update) {
            self.on_cursor_left_window(args.window_id, args.device_id, ctx);
        } else if let Some(args) = RAW_WINDOW_FOCUS_EVENT.on(update) {
            if let Some(window_id) = args.prev_focus {
                self.on_window_blur(window_id, ctx);
            }
        } else if let Some(args) = RAW_WINDOW_CLOSE_EVENT.on(update) {
            self.on_window_closed(args.window_id, ctx);
        } else if let Some(args) = RAW_MULTI_CLICK_CONFIG_CHANGED_EVENT.on(update) {
            self.multi_click_config.set_ne(ctx.vars, args.config);
            self.click_state = ClickState::None;
        } else if let Some(args) = VIEW_PROCESS_INITED_EVENT.on(update) {
            self.multi_click_config.set_ne(ctx.vars, args.multi_click_config);

            if args.is_respawn {
                let mouse = Mouse::req(ctx.services);

                if let Some(window_id) = self.pos_window.take() {
                    if let Some(path) = self.hovered.take() {
                        mouse.buttons.with(|b| {
                            for btn in b {
                                let args = MouseInputArgs::now(
                                    window_id,
                                    None,
                                    *btn,
                                    DipPoint::new(Dip::new(-1), Dip::new(-1)),
                                    ModifiersState::empty(),
                                    ButtonState::Released,
                                    HitTestInfo::no_hits(window_id),
                                    path.clone(),
                                    None,
                                    None,
                                );
                                MOUSE_INPUT_EVENT.notify(ctx.events, args);
                            }
                        });
                        let args = MouseHoverArgs::now(
                            window_id,
                            None,
                            DipPoint::new(Dip::new(-1), Dip::new(-1)),
                            HitTestInfo::no_hits(window_id),
                            Some(path),
                            None,
                            None,
                            None,
                        );
                        MOUSE_HOVERED_EVENT.notify(ctx.events, args);
                    }
                }
                if let Some(cap) = mouse.current_capture.take() {
                    let args = MouseCaptureArgs::now(Some(cap), None);
                    MOUSE_CAPTURE_EVENT.notify(ctx.events, args);
                }
                mouse.capture_request = None;
                mouse.release_requested = false;
                self.click_state = ClickState::None;
                self.capture_count = 0;
                self.pressed.clear();
                self.pos_device = None;
                self.pos_window = None;
                self.pos_hits = None;
            }
        }
    }

    fn event(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        if let Some(args) = MOUSE_CAPTURE_EVENT.on(update) {
            if let Some(path) = &self.hovered {
                if let Some(window_id) = self.pos_window {
                    let hover_args = MouseHoverArgs::now(
                        window_id,
                        self.pos_device.unwrap(),
                        self.pos,
                        self.pos_hits.clone().unwrap_or_else(|| HitTestInfo::no_hits(window_id)),
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
                    MOUSE_HOVERED_EVENT.notify(ctx.events, hover_args);
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut AppContext) {
        let (mouse, windows) = <(Mouse, Windows)>::req(ctx.services);

        mouse.fulfill_requests(windows, ctx.events);
    }
}

enum ClickState {
    /// Before start.
    None,
    /// Mouse pressed on a widget, if the next event
    /// is a release over the same widget a click event is generated.
    Pressed { btn: MouseButton, press_tgt: InteractionPath },
    /// At least one click completed, as long as subsequent presses happen
    /// within the window of time, widget and distance from the initial press
    /// multi-click events are generated.
    Clicked {
        start_time: Instant,
        btn: MouseButton,
        pos: DipPoint,
        start_tgt: InteractionPath,

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
    pub fn buttons(&self) -> ReadOnlyRcVar<Vec<MouseButton>> {
        self.buttons.read_only()
    }

    /// Read-only variable that tracks the system click-count increment time and area, a.k.a. the double-click config.
    ///
    /// Repeated clicks with an interval less then this time and within the distance of the first click increment the click count.
    ///
    /// # Value Source
    ///
    /// The value comes from the operating system settings, the variable
    /// updates with a new value if the system setting is changed.
    ///
    /// In headless apps the default is [`MultiClickConfig::default`] and does not change.
    ///
    /// Internally the [`RAW_MULTI_CLICK_CONFIG_CHANGED_EVENT`] is listened to update this variable, so you can notify
    /// this event to set this variable, if you really must.
    pub fn multi_click_config(&self) -> ReadOnlyRcVar<MultiClickConfig> {
        self.multi_click_config.read_only()
    }

    /// The current capture target and mode.
    pub fn current_capture(&self) -> Option<(&WidgetPath, CaptureMode)> {
        self.current_capture.as_ref().map(|(p, c)| (p, *c))
    }

    /// Set a widget to redirect all mouse events to.
    ///
    /// The capture will be set only if the pointer is currently pressed over the widget.
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
    pub fn capture_subtree(&mut self, widget_id: WidgetId) {
        self.capture_request = Some((widget_id, CaptureMode::Subtree));
        let _ = self.update_sender.send_ext_update();
    }

    /// Release the current mouse capture back to window.
    ///
    /// **Note:** The capture is released automatically when the mouse buttons are released
    /// or when the window loses focus.
    pub fn release_capture(&mut self) {
        self.release_requested = true;
        let _ = self.update_sender.send_ext_update();
    }

    /// Call when the mouse starts pressing on the window.
    fn start_window_capture(&mut self, mouse_down: InteractionPath, events: &mut Events) {
        self.release_requested = false;

        if let Some((target, mode)) = self.capture_request.take() {
            if let Some(target) = mouse_down.ancestor_path(target) {
                self.set_capture(target.into_owned(), mode, events);
                return; // fulfilled request at start.
            }
        }

        // set default capture.
        self.set_capture(mouse_down.root_path().into_owned(), CaptureMode::Window, events);
    }

    /// Call after UI update.
    fn fulfill_requests(&mut self, windows: &Windows, events: &mut Events) {
        if let Some((current_target, current_mode)) = &self.current_capture {
            if let Some((widget_id, mode)) = self.capture_request.take() {
                if let Ok(true) = windows.is_focused(current_target.window_id()) {
                    // current window pressed
                    if let Some(widget) = windows.widget_tree(current_target.window_id()).unwrap().get(widget_id) {
                        // request valid
                        self.set_capture(widget.interaction_path(), mode, events);
                    }
                }
            } else if mem::take(&mut self.release_requested) && *current_mode != CaptureMode::Window {
                // release capture (back to default capture).
                let target = current_target.root_path();
                self.set_capture(InteractionPath::from_enabled(target.into_owned()), CaptureMode::Window, events);
            }
        }
    }

    /// Call after a frame is generated.
    fn continue_capture(&mut self, frame: &WidgetInfoTree, events: &mut Events) {
        if let Some((target, mode)) = &self.current_capture {
            if frame.window_id() == target.window_id() {
                // is a frame from the capturing window.
                if let Some(widget) = frame.get(target.widget_id()) {
                    if let Some(new_path) = widget.new_interaction_path(&InteractionPath::from_enabled(target.clone())) {
                        // widget moved inside window tree.
                        let mode = *mode;
                        self.set_capture(new_path, mode, events);
                    }
                } else {
                    // widget not found. Returns to default capture.
                    self.set_capture(frame.root().interaction_path(), CaptureMode::Window, events);
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

    fn set_capture(&mut self, target: InteractionPath, mode: CaptureMode, events: &mut Events) {
        let new = target.enabled().map(|target| (target, mode));
        if new.is_none() {
            self.unset_capture(events);
            return;
        }

        if new != self.current_capture {
            let prev = self.current_capture.take();
            self.current_capture = new.clone();
            MOUSE_CAPTURE_EVENT.notify(events, MouseCaptureArgs::now(prev, new));
        }
    }

    fn unset_capture(&mut self, events: &mut Events) {
        if self.current_capture.is_some() {
            let prev = self.current_capture.take();
            MOUSE_CAPTURE_EVENT.notify(events, MouseCaptureArgs::now(prev, None));
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureInfo {
    /// Widget that is capturing all mouse events. The widget and all ancestors are [`ENABLED`].
    ///
    /// This is the window root widget for capture mode `Window`.
    ///
    /// [`ENABLED`]: crate::widget_info::Interactivity::ENABLED
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
    pub fn allows(&self, path: &WidgetContextPath) -> bool {
        match self.mode {
            CaptureMode::Window => self.target.window_id() == path.window_id(),
            CaptureMode::Widget => self.target.widget_id() == path.widget_id(),
            CaptureMode::Subtree => path.contains(self.target.widget_id()),
        }
    }
}
