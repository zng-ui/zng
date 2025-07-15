//! Mouse events and service.
//!
//! The app extension [`MouseManager`] provides the events and service. It is included in the default application.

use std::{collections::HashMap, mem, num::NonZeroU32, time::*};

use zng_app::{
    AppExtension, DInstant, INSTANT,
    event::{EventPropagationHandle, event, event_args},
    shortcut::ModifiersState,
    timer::{DeadlineVar, TIMERS},
    update::EventUpdate,
    view_process::{
        VIEW_PROCESS_INITED_EVENT,
        raw_device_events::InputDeviceId,
        raw_events::{
            RAW_FRAME_RENDERED_EVENT, RAW_MOUSE_INPUT_EVENT, RAW_MOUSE_LEFT_EVENT, RAW_MOUSE_MOVED_EVENT, RAW_MOUSE_WHEEL_EVENT,
            RAW_MULTI_CLICK_CONFIG_CHANGED_EVENT, RAW_WINDOW_FOCUS_EVENT,
        },
    },
    widget::{
        WIDGET, WidgetId,
        info::{HitTestInfo, InteractionPath, WIDGET_INFO_CHANGED_EVENT, WidgetInfo, WidgetInfoBuilder},
    },
    window::WindowId,
};
use zng_app_context::app_local;
use zng_ext_window::{NestedWindowWidgetInfoExt, WINDOWS};
use zng_layout::unit::{Dip, DipPoint, DipToPx, Factor, PxPoint, PxToDip};
use zng_state_map::{StateId, state_map, static_id};
use zng_var::{ArcVar, BoxedVar, IntoVar, LocalVar, ReadOnlyArcVar, Var, impl_from_and_into_var, types::ArcCowVar, var};
use zng_view_api::touch::TouchPhase;
pub use zng_view_api::{
    config::MultiClickConfig,
    mouse::{ButtonState, MouseButton, MouseScrollDelta},
};

use crate::{
    keyboard::{KEYBOARD, MODIFIERS_CHANGED_EVENT},
    pointer_capture::{CaptureInfo, CaptureMode, POINTER_CAPTURE, POINTER_CAPTURE_EVENT},
};

event_args! {
    /// [`MOUSE_MOVE_EVENT`] arguments.
    pub struct MouseMoveArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: InputDeviceId,

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

        /// Current pointer capture.
        pub capture: Option<CaptureInfo>,

        ..

        /// The [`target`] and [`capture`].
        ///
        /// [`target`]: Self::target
        /// [`capture`]: Self::capture
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_wgt(&self.target);
            if let Some(c) = &self.capture {
                list.insert_wgt(&c.target);
            }
        }
    }

    /// [`MOUSE_INPUT_EVENT`] arguments.
    pub struct MouseInputArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: Option<InputDeviceId>,

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

        /// Current pointer capture.
        pub capture: Option<CaptureInfo>,

        /// If [`MOUSE_CLICK_EVENT`] will notify because of this input.
        ///
        /// The click event shares the same propagation handle as this event.
        pub is_click: bool,

        ..

        /// The [`target`], and [`capture`].
        ///
        /// [`target`]: Self::target
        /// [`capture`]: Self::capture
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_wgt(&self.target);
            if let Some(c) = &self.capture {
                list.insert_wgt(&c.target);
            }
        }
    }

    /// [`MOUSE_CLICK_EVENT`] arguments.
    pub struct MouseClickArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: InputDeviceId,

        /// Which mouse button generated the event.
        pub button: MouseButton,

        /// Position of the mouse in the coordinates of [`target`](MouseClickArgs::target).
        pub position: DipPoint,

        /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        /// Count of clicks within the double-click interval. Number `1` is single click, `2` is double click, etc.
        ///
        /// Auto repeated clicks also increment this count.
        pub click_count: NonZeroU32,

        /// If the event was generated by holding the button pressed over a widget with [`ClickMode::repeat`].
        pub is_repeat: bool,

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
        /// Multi-clicks ([`click_count`] > 1) only happen to the same target.
        ///
        /// [mouse down]: MouseInputArgs::is_mouse_down
        /// [mouse up]: MouseInputArgs::is_mouse_up
        /// [`click_count`]: MouseClickArgs::click_count
        pub target: InteractionPath,

        ..

        /// The [`target`].
        ///
        /// [`target`]: MouseClickArgs::target
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_wgt(&self.target)
        }
    }

    /// [`MOUSE_HOVERED_EVENT`] arguments.
    pub struct MouseHoverArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: Option<InputDeviceId>,

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

        /// Previous pointer capture.
        pub prev_capture: Option<CaptureInfo>,

        /// Current pointer capture.
        pub capture: Option<CaptureInfo>,

        ..

        /// The [`target`], [`prev_target`] and [`capture`].
        ///
        /// [`target`]: Self::target
        /// [`prev_target`]: Self::prev_target
        /// [`capture`]: Self::capture
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            if let Some(p) = &self.prev_target {
                list.insert_wgt(p);
            }
            if let Some(p) = &self.target {
                list.insert_wgt(p);
            }
            if let Some(c) = &self.capture {
                list.insert_wgt(&c.target);
            }
        }
    }

    /// [`MOUSE_WHEEL_EVENT`] arguments.
    pub struct MouseWheelArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,
        /// Id of device that generated the event.
        pub device_id: InputDeviceId,

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
            list.insert_wgt(&self.target)
        }
    }
}

impl MouseHoverArgs {
    /// If [`capture`] is `None` or [`allows`] the [`WIDGET`] to receive this event.
    ///
    /// [`capture`]: Self::capture
    /// [`allows`]: CaptureInfo::allows
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn capture_allows(&self) -> bool {
        self.capture.as_ref().map(|c| c.allows()).unwrap_or(true)
    }

    /// Event caused by the mouse position moving over/out of the widget bounds.
    pub fn is_mouse_move(&self) -> bool {
        self.device_id.is_some()
    }

    /// Event caused by the widget moving under/out of the mouse position.
    pub fn is_widget_move(&self) -> bool {
        self.device_id.is_none()
    }

    /// Event caused by a pointer capture change.
    pub fn is_capture_change(&self) -> bool {
        self.prev_capture != self.capture
    }

    /// Returns `true` if the [`WIDGET`] was not hovered, but now is.
    ///
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn is_mouse_enter(&self) -> bool {
        !self.was_over() && self.is_over()
    }

    /// Returns `true` if the [`WIDGET`] was hovered, but now isn't.
    ///
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn is_mouse_leave(&self) -> bool {
        self.was_over() && !self.is_over()
    }

    /// Returns `true` if the [`WIDGET`] was not hovered or was disabled, but now is hovered and enabled.
    ///
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn is_mouse_enter_enabled(&self) -> bool {
        (!self.was_over() || self.was_disabled(WIDGET.id())) && self.is_over() && self.is_enabled(WIDGET.id())
    }

    /// Returns `true` if the [`WIDGET`] was hovered and enabled, but now is not hovered or is disabled.
    ///
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn is_mouse_leave_enabled(&self) -> bool {
        self.was_over() && self.was_enabled(WIDGET.id()) && (!self.is_over() || self.is_disabled(WIDGET.id()))
    }

    /// Returns `true` if the [`WIDGET`] was not hovered or was enabled, but now is hovered and disabled.
    ///
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn is_mouse_enter_disabled(&self) -> bool {
        (!self.was_over() || self.was_enabled(WIDGET.id())) && self.is_over() && self.is_disabled(WIDGET.id())
    }

    /// Returns `true` if the [`WIDGET`] was hovered and disabled, but now is not hovered or is enabled.
    ///
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn is_mouse_leave_disabled(&self) -> bool {
        self.was_over() && self.was_disabled(WIDGET.id()) && (!self.is_over() || self.is_enabled(WIDGET.id()))
    }

    /// Returns `true` if the [`WIDGET`] is in [`prev_target`] and is allowed by the [`prev_capture`].
    ///
    /// [`prev_target`]: Self::prev_target
    /// [`prev_capture`]: Self::prev_capture
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn was_over(&self) -> bool {
        if let Some(cap) = &self.prev_capture {
            if !cap.allows() {
                return false;
            }
        }

        if let Some(t) = &self.prev_target {
            return t.contains(WIDGET.id());
        }

        false
    }

    /// Returns `true` if the [`WIDGET`] is in [`target`] and is allowed by the current [`capture`].
    ///
    /// [`target`]: Self::target
    /// [`capture`]: Self::capture
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn is_over(&self) -> bool {
        if let Some(cap) = &self.capture {
            if !cap.allows() {
                return false;
            }
        }

        if let Some(t) = &self.target {
            return t.contains(WIDGET.id());
        }

        false
    }

    /// Returns `true` if the widget was enabled in [`prev_target`].
    ///
    /// [`prev_target`]: Self::prev_target
    pub fn was_enabled(&self, widget_id: WidgetId) -> bool {
        match &self.prev_target {
            Some(t) => t.contains_enabled(widget_id),
            None => false,
        }
    }

    /// Returns `true` if the widget was disabled in [`prev_target`].
    ///
    /// [`prev_target`]: Self::prev_target
    pub fn was_disabled(&self, widget_id: WidgetId) -> bool {
        match &self.prev_target {
            Some(t) => t.contains_disabled(widget_id),
            None => false,
        }
    }

    /// Returns `true` if the widget is enabled in [`target`].
    ///
    /// [`target`]: Self::target
    pub fn is_enabled(&self, widget_id: WidgetId) -> bool {
        match &self.target {
            Some(t) => t.contains_enabled(widget_id),
            None => false,
        }
    }

    /// Returns `true` if the widget is disabled in [`target`].
    ///
    /// [`target`]: Self::target
    pub fn is_disabled(&self, widget_id: WidgetId) -> bool {
        match &self.target {
            Some(t) => t.contains_disabled(widget_id),
            None => false,
        }
    }

    /// Gets position in the widget inner bounds.
    pub fn position_wgt(&self) -> Option<PxPoint> {
        WIDGET.win_point_to_wgt(self.position)
    }
}

impl MouseMoveArgs {
    /// If [`capture`] is `None` or [`allows`] the [`WIDGET`] to receive this event.
    ///
    /// [`capture`]: Self::capture
    /// [`allows`]: CaptureInfo::allows
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn capture_allows(&self) -> bool {
        self.capture.as_ref().map(|c| c.allows()).unwrap_or(true)
    }

    /// Gets position in the widget inner bounds.
    pub fn position_wgt(&self) -> Option<PxPoint> {
        WIDGET.win_point_to_wgt(self.position)
    }
}

impl MouseInputArgs {
    /// If [`capture`] is `None` or [`allows`] the [`WIDGET`] to receive this event.
    ///
    /// [`capture`]: Self::capture
    /// [`allows`]: CaptureInfo::allows
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn capture_allows(&self) -> bool {
        self.capture.as_ref().map(|c| c.allows()).unwrap_or(true)
    }

    /// If the `widget_id` is in the [`target`].
    ///
    /// [`target`]: Self::target
    pub fn is_over(&self, widget_id: WidgetId) -> bool {
        self.target.contains(widget_id)
    }

    /// Deprecated
    #[deprecated = "use `self.target.contains_enabled`"]
    pub fn is_enabled(&self, widget_id: WidgetId) -> bool {
        self.target.contains_enabled(widget_id)
    }

    /// Deprecated
    #[deprecated = "use `self.target.contains_disabled`"]
    pub fn is_disabled(&self, widget_id: WidgetId) -> bool {
        self.target.contains_disabled(widget_id)
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

    /// Gets position in the widget inner bounds.
    pub fn position_wgt(&self) -> Option<PxPoint> {
        WIDGET.win_point_to_wgt(self.position)
    }
}

impl MouseClickArgs {
    /// Deprecated
    #[deprecated = "use `self.target.contains_enabled`"]
    pub fn is_enabled(&self, widget_id: WidgetId) -> bool {
        self.target.contains_enabled(widget_id)
    }

    /// Deprecated
    #[deprecated = "use `self.target.contains_disabled`"]
    pub fn is_disabled(&self, widget_id: WidgetId) -> bool {
        self.target.contains_disabled(widget_id)
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

    /// Gets position in the widget inner bounds.
    pub fn position_wgt(&self) -> Option<PxPoint> {
        WIDGET.win_point_to_wgt(self.position)
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
                _ => unimplemented!(),
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
                _ => return None,
            };
        }

        if modifiers.is_empty() {
            Some(delta)
        } else if modifiers.is_only_shift() {
            Some(match delta {
                MouseScrollDelta::LineDelta(x, y) => MouseScrollDelta::LineDelta(y, x),
                MouseScrollDelta::PixelDelta(x, y) => MouseScrollDelta::PixelDelta(y, x),
                _ => return None,
            })
        } else {
            None
        }
    }

    /// If the modifiers allow the event to be used for zooming.
    ///
    /// Is `true` if only `CTRL` is pressed. If `true` the [`zoom_delta`] method returns a value.
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
        if self.is_zoom() { Some(self.delta) } else { None }
    }

    /// Deprecated
    #[deprecated = "use `self.target.contains_enabled`"]
    pub fn is_enabled(&self, widget_id: WidgetId) -> bool {
        self.target.contains_enabled(widget_id)
    }

    /// Deprecated
    #[deprecated = "use `self.target.contains_disabled`"]
    pub fn is_disabled(&self, widget_id: WidgetId) -> bool {
        self.target.contains_disabled(widget_id)
    }

    /// Gets position in the widget inner bounds.
    pub fn position_wgt(&self) -> Option<PxPoint> {
        WIDGET.win_point_to_wgt(self.position)
    }
}

event! {
    /// Mouse move event.
    pub static MOUSE_MOVE_EVENT: MouseMoveArgs;

    /// Mouse down or up event.
    pub static MOUSE_INPUT_EVENT: MouseInputArgs;

    /// Mouse click event, any [`click_count`](MouseClickArgs::click_count).
    pub static MOUSE_CLICK_EVENT: MouseClickArgs;

    /// The top-most hovered widget changed or pointer capture changed.
    pub static MOUSE_HOVERED_EVENT: MouseHoverArgs;

    /// Mouse wheel scroll event.
    pub static MOUSE_WHEEL_EVENT: MouseWheelArgs;
}

struct ClickingInfo {
    path: InteractionPath,
    press_stop_handle: EventPropagationHandle,

    pressed: bool,
    last_pos: DipPoint,
    last_click: DInstant,
    click_count: u32,

    repeat_timer: Option<DeadlineVar>,
    repeat_count: u32,
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
///
/// # Services
///
/// Services this extension provides.
///
/// * [`MOUSE`]
pub struct MouseManager {
    // last cursor move position (scaled).
    pos: DipPoint,
    // last cursor move over `pos_window` and source device.
    pos_window: Option<WindowId>,
    pos_device: Option<InputDeviceId>,
    // last cursor move hit-test (on the pos_window or a nested window).
    hits: Option<HitTestInfo>,

    /// last modifiers.
    modifiers: ModifiersState,

    hovered: Option<InteractionPath>,
    clicking: HashMap<MouseButton, ClickingInfo>,
}
impl Default for MouseManager {
    fn default() -> Self {
        MouseManager {
            pos: DipPoint::zero(),
            pos_window: None,
            pos_device: None,
            hits: None,

            modifiers: ModifiersState::default(),

            hovered: None,
            clicking: HashMap::default(),
        }
    }
}
impl MouseManager {
    fn on_mouse_input(&mut self, mut window_id: WindowId, device_id: InputDeviceId, state: ButtonState, button: MouseButton) {
        let mouse = MOUSE_SV.read();

        let mut position = if self.pos_window == Some(window_id) {
            self.pos
        } else {
            DipPoint::default()
        };

        let hits = self.hits.clone().unwrap_or_else(|| HitTestInfo::no_hits(window_id));

        let wgt_tree = match WINDOWS.widget_tree(hits.window_id()) {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("cannot find clicked window, {e:?}");
                return;
            }
        };

        if hits.window_id() != window_id {
            // is over nested window
            window_id = hits.window_id();
            position = hits.point().to_dip(wgt_tree.scale_factor());
        }

        let (wgt_path, click_mode) = hits
            .target()
            .and_then(|t| wgt_tree.get(t.widget_id).map(|w| (w.interaction_path(), w.click_mode())))
            .unwrap_or_else(|| (wgt_tree.root().interaction_path(), wgt_tree.root().click_mode()));

        let wgt_path = match wgt_path.unblocked() {
            Some(p) => p,
            None => return, // entire window blocked
        };

        match state {
            ButtonState::Pressed => {
                if !mouse.buttons.with(|b| b.contains(&button)) {
                    mouse.buttons.modify(move |btns| btns.to_mut().push(button));
                }
            }
            ButtonState::Released => {
                if mouse.buttons.with(|b| b.contains(&button)) {
                    mouse.buttons.modify(move |btns| {
                        if let Some(i) = btns.as_ref().iter().position(|k| *k == button) {
                            btns.to_mut().swap_remove(i);
                        }
                    });
                }
            }
        }

        let stop_handle = EventPropagationHandle::new();

        let entry = self.clicking.entry(button).or_insert_with(|| ClickingInfo {
            path: wgt_path.clone(),
            press_stop_handle: stop_handle.clone(),
            last_click: DInstant::EPOCH,
            last_pos: position,
            pressed: false,
            click_count: 0,
            repeat_timer: None,
            repeat_count: 0,
        });

        if entry.path != wgt_path {
            let actual_change = entry.path.as_path() != wgt_path.as_path();
            // else only interactivity change mid-click

            entry.path = wgt_path.clone();
            if actual_change {
                entry.press_stop_handle = stop_handle.clone();
                entry.pressed = false;
                entry.click_count = 0;
                entry.repeat_timer = None;
                entry.repeat_count = 0;
            }
        }

        let multi_click_cfg = mouse.multi_click_config.get();

        let double_allowed = entry.last_click.elapsed() <= multi_click_cfg.time && {
            let dist = (entry.last_pos.to_vector() - position.to_vector()).abs();
            let area = multi_click_cfg.area;
            dist.x <= area.width && dist.y <= area.height
        };

        let click_gesture = if entry.click_count == 0 || !double_allowed {
            entry.click_count = 0;
            click_mode.single
        } else {
            click_mode.double
        };

        let click = match state {
            ButtonState::Pressed => {
                entry.pressed = true;
                entry.press_stop_handle = stop_handle.clone();
                matches!(click_gesture, ClickTrigger::Press)
            }
            ButtonState::Released => {
                entry.repeat_count = 0;
                entry.repeat_timer = None;
                if mem::take(&mut entry.pressed) && !entry.press_stop_handle.is_stopped() {
                    matches!(click_gesture, ClickTrigger::PressRelease | ClickTrigger::Release)
                } else {
                    matches!(click_gesture, ClickTrigger::Release)
                }
            }
        };

        if click_mode.repeat {
            if click {
                let t = mouse.repeat_config.get().start_delay;
                entry.repeat_timer = Some(TIMERS.deadline(t));
                entry.repeat_count = 0;
            }
        } else {
            entry.repeat_timer = None;
            entry.repeat_count = 0;
        }

        let capture_info = POINTER_CAPTURE.current_capture_value();

        let now = INSTANT.now();
        let args = MouseInputArgs::new(
            now,
            stop_handle.clone(),
            window_id,
            device_id,
            button,
            position,
            self.modifiers,
            state,
            hits.clone(),
            wgt_path.clone(),
            capture_info,
            click,
        );

        // on_mouse_input
        MOUSE_INPUT_EVENT.notify(args);

        if click {
            if double_allowed {
                entry.click_count += 1;
            } else {
                entry.click_count = 1;
            }

            entry.last_click = now;
            entry.last_pos = position;

            let args = MouseClickArgs::new(
                now,
                stop_handle,
                window_id,
                device_id,
                button,
                position,
                self.modifiers,
                NonZeroU32::new(entry.click_count).unwrap(),
                false,
                hits,
                wgt_path,
            );

            // on_mouse_click
            MOUSE_CLICK_EVENT.notify(args);
        }
    }

    fn on_cursor_moved(&mut self, window_id: WindowId, device_id: InputDeviceId, coalesced_pos: Vec<DipPoint>, mut position: DipPoint) {
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

            // mouse_move data
            let mut frame_info = match WINDOWS.widget_tree(window_id) {
                Ok(f) => f,
                Err(_) => {
                    // window not found
                    if let Some(hovered) = self.hovered.take() {
                        let capture = POINTER_CAPTURE.current_capture_value();
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
                        MOUSE_HOVERED_EVENT.notify(args);
                    }
                    return;
                }
            };

            let mut pos_hits = frame_info.root().hit_test(position.to_px(frame_info.scale_factor()));

            let target = if let Some(t) = pos_hits.target() {
                if let Some(w) = frame_info.get(t.widget_id) {
                    if let Some(f) = w.nested_window_tree() {
                        // nested window hit
                        frame_info = f;
                        let factor = frame_info.scale_factor();
                        let pos = position.to_px(factor);
                        let pos = w.inner_transform().inverse().and_then(|t| t.transform_point(pos)).unwrap_or(pos);
                        pos_hits = frame_info.root().hit_test(pos);
                        position = pos.to_dip(factor);
                        pos_hits
                            .target()
                            .and_then(|h| frame_info.get(h.widget_id))
                            .map(|w| w.interaction_path())
                            .unwrap_or_else(|| frame_info.root().interaction_path())
                    } else {
                        w.interaction_path()
                    }
                } else {
                    tracing::error!("hits target `{}` not found", t.widget_id);
                    frame_info.root().interaction_path()
                }
            } else {
                frame_info.root().interaction_path()
            }
            .unblocked();

            MOUSE_SV.read().position.set(Some(MousePosition {
                window_id: frame_info.window_id(),
                position,
                timestamp: INSTANT.now(),
            }));

            self.hits = Some(pos_hits.clone());

            let capture = POINTER_CAPTURE.current_capture_value();

            // mouse_enter/mouse_leave.
            let hovered_args = if self.hovered != target {
                MOUSE_SV.read().hovered.set(target.clone());
                let prev_target = mem::replace(&mut self.hovered, target.clone());
                let args = MouseHoverArgs::now(
                    frame_info.window_id(),
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
                    frame_info.window_id(),
                    device_id,
                    self.modifiers,
                    coalesced_pos,
                    position,
                    pos_hits,
                    target,
                    capture,
                );
                MOUSE_MOVE_EVENT.notify(args);
            }

            if let Some(args) = hovered_args {
                MOUSE_HOVERED_EVENT.notify(args);
            }
        } else if coalesced_pos.is_empty() {
            tracing::debug!("RawCursorMoved did not actually move")
        }
    }

    fn on_scroll(&self, window_id: WindowId, device_id: InputDeviceId, delta: MouseScrollDelta, phase: TouchPhase) {
        let position = if self.pos_window == Some(window_id) {
            self.pos
        } else {
            DipPoint::default()
        };

        let hits = self.hits.clone().unwrap_or_else(|| HitTestInfo::no_hits(window_id));

        let frame_info = WINDOWS.widget_tree(hits.window_id()).unwrap();

        let target = hits
            .target()
            .and_then(|t| frame_info.get(t.widget_id).map(|w| w.interaction_path()))
            .unwrap_or_else(|| frame_info.root().interaction_path());

        if let Some(target) = target.unblocked() {
            let args = MouseWheelArgs::now(hits.window_id(), device_id, position, self.modifiers, delta, phase, hits, target);
            MOUSE_WHEEL_EVENT.notify(args);
        }
    }

    fn on_cursor_left_window(&mut self, window_id: WindowId, device_id: InputDeviceId) {
        if Some(window_id) == self.pos_window.take() {
            MOUSE_SV.read().position.set(None);
            if let Some(path) = self.hovered.take() {
                MOUSE_SV.read().hovered.set(None);
                let capture = POINTER_CAPTURE.current_capture_value();
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
                MOUSE_HOVERED_EVENT.notify(args);
            }
        }
    }

    fn on_window_blur(&mut self, prev_window: WindowId, new_window: Option<WindowId>) {
        if new_window.is_some() {
            if let Some(p) = self.pos_window {
                // last hovered window losing focus, and is not just focusing a nested window
                if p == prev_window && (new_window.is_none() || new_window != self.hits.as_ref().map(|h| h.window_id())) {
                    self.clean_all_state();
                }
            }
        } else {
            self.clean_all_state();
        }
    }

    /// Call after a frame or info rebuild.
    fn continue_hovered(&mut self, mut window_id: WindowId) {
        if self.pos_window == Some(window_id) {
            // update hovered if widgets moved under the cursor position.
            let mut frame_info = match WINDOWS.widget_tree(window_id) {
                Ok(f) => f,
                Err(_) => {
                    self.clean_all_state();
                    return;
                }
            };
            let mut pos_hits = frame_info.root().hit_test(self.pos.to_px(frame_info.scale_factor()));
            let mut position = self.pos;
            let target = if let Some(t) = pos_hits.target() {
                if let Some(w) = frame_info.get(t.widget_id) {
                    if let Some(f) = w.nested_window_tree() {
                        frame_info = f;
                        let factor = frame_info.scale_factor();
                        let pos = self.pos.to_px(factor);
                        let pos = w.inner_transform().inverse().and_then(|t| t.transform_point(pos)).unwrap_or(pos);
                        pos_hits = frame_info.root().hit_test(pos);
                        window_id = frame_info.window_id();
                        position = pos.to_dip(factor);
                        pos_hits
                            .target()
                            .and_then(|h| frame_info.get(h.widget_id))
                            .map(|w| w.interaction_path())
                            .unwrap_or_else(|| frame_info.root().interaction_path())
                    } else {
                        w.interaction_path()
                    }
                } else {
                    tracing::error!("hits target `{}` not found", t.widget_id);
                    frame_info.root().interaction_path()
                }
            } else {
                frame_info.root().interaction_path()
            }
            .unblocked();
            self.hits = Some(pos_hits.clone());

            if self.hovered != target {
                let capture = POINTER_CAPTURE.current_capture_value();
                let prev = mem::replace(&mut self.hovered, target.clone());
                let args = MouseHoverArgs::now(window_id, None, position, pos_hits, prev, target, capture.clone(), capture);
                MOUSE_HOVERED_EVENT.notify(args);
            }
        }
    }

    fn clean_all_state(&mut self) {
        let mouse = MOUSE_SV.read();
        if self.pos_window.take().is_some() {
            if let Some(path) = self.hovered.take() {
                let window_id = path.window_id();
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
                            false,
                        );
                        MOUSE_INPUT_EVENT.notify(args);
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
                MOUSE_HOVERED_EVENT.notify(args);
            }
        }
        mouse.buttons.set(vec![]);
        self.clicking.clear();
        self.pos_device = None;
        self.pos_window = None;
        self.hits = None;
        mouse.position.set(None);
        mouse.hovered.set(None);
    }
}
impl AppExtension for MouseManager {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        if let Some(args) = RAW_FRAME_RENDERED_EVENT.on(update) {
            self.continue_hovered(args.window_id);
        } else if let Some(args) = RAW_MOUSE_MOVED_EVENT.on(update) {
            self.on_cursor_moved(args.window_id, args.device_id, args.coalesced_pos.clone(), args.position);
        } else if let Some(args) = RAW_MOUSE_WHEEL_EVENT.on(update) {
            self.on_scroll(args.window_id, args.device_id, args.delta, args.phase);
        } else if let Some(args) = RAW_MOUSE_INPUT_EVENT.on(update) {
            self.on_mouse_input(args.window_id, args.device_id, args.state, args.button);
        } else if let Some(args) = MODIFIERS_CHANGED_EVENT.on(update) {
            self.modifiers = args.modifiers;
        } else if let Some(args) = RAW_MOUSE_LEFT_EVENT.on(update) {
            self.on_cursor_left_window(args.window_id, args.device_id);
        } else if let Some(args) = RAW_WINDOW_FOCUS_EVENT.on(update) {
            if let Some(window_id) = args.prev_focus {
                self.on_window_blur(window_id, args.new_focus);
            }
        } else if let Some(args) = WIDGET_INFO_CHANGED_EVENT.on(update) {
            self.continue_hovered(args.window_id);
        } else if let Some(args) = RAW_MULTI_CLICK_CONFIG_CHANGED_EVENT.on(update) {
            MOUSE_SV.read().multi_click_config.set(args.config);
            self.clicking.clear();
        } else if let Some(args) = VIEW_PROCESS_INITED_EVENT.on(update) {
            MOUSE_SV.read().multi_click_config.set(args.multi_click_config);

            if args.is_respawn {
                self.clean_all_state();
            }
        }
    }

    fn event(&mut self, update: &mut EventUpdate) {
        if let Some(args) = POINTER_CAPTURE_EVENT.on(update) {
            if let Some(path) = &self.hovered {
                if self.pos_window.is_some() {
                    let window_id = path.window_id();
                    let hover_args = MouseHoverArgs::now(
                        window_id,
                        self.pos_device.unwrap(),
                        self.pos,
                        self.hits.clone().unwrap_or_else(|| HitTestInfo::no_hits(window_id)),
                        Some(path.clone()),
                        Some(path.clone()),
                        args.prev_capture.clone(),
                        args.new_capture.clone(),
                    );
                    MOUSE_HOVERED_EVENT.notify(hover_args);
                }
            }
        }
    }

    fn update_preview(&mut self) {
        // update pressed repeat clicks
        for (btn, info) in self.clicking.iter_mut() {
            if let Some(timer) = info.repeat_timer.take() {
                // is repeat mode
                if timer.with_new(|t| t.has_elapsed()).unwrap_or(false) {
                    // time to repeat
                    info.repeat_count = info.repeat_count.saturating_add(1);

                    if let (Some(dv), Ok(tree)) = (self.pos_device, WINDOWS.widget_tree(info.path.window_id())) {
                        // probably still valid

                        let hit_test = tree.root().hit_test(self.pos.to_px(tree.scale_factor()));

                        // get the hit target, constrained by capture
                        let mut target = None;
                        if let Some(hit) = hit_test.target().map(|t| tree.get(t.widget_id).unwrap()) {
                            target = hit.path().shared_ancestor(info.path.as_path()).map(|c| c.into_owned());
                        }
                        if let Some(c) = POINTER_CAPTURE.current_capture_value() {
                            match c.mode {
                                CaptureMode::Window => {
                                    if let Some(t) = &target {
                                        if t.window_id() != c.target.window_id() {
                                            target = None; // captured in other window, cancel repeat
                                        }
                                    } else {
                                        // no hit, but window capture
                                        target = Some(tree.root().path());
                                    }
                                }
                                CaptureMode::Subtree => {
                                    if let Some(t) = &target {
                                        target = c.target.shared_ancestor(t).map(|c| c.into_owned());
                                    } else {
                                        target = Some(c.target);
                                    }
                                }
                                CaptureMode::Widget => {
                                    target = Some(c.target);
                                }
                            }
                        }

                        if let Some(target) = target {
                            // if still has a target
                            if let Some(target) = tree.get(target.widget_id()).and_then(|w| w.interaction_path().unblocked()) {
                                // and it is unblocked

                                // notify repeat
                                let args = MouseClickArgs::now(
                                    target.window_id(),
                                    dv,
                                    *btn,
                                    self.pos,
                                    self.modifiers,
                                    NonZeroU32::new(info.repeat_count).unwrap(),
                                    true,
                                    hit_test,
                                    target,
                                );
                                MOUSE_CLICK_EVENT.notify(args);

                                // continue timer
                                let t = MOUSE.repeat_config().get().interval;
                                info.repeat_timer = Some(TIMERS.deadline(t));
                            }
                        }
                    }
                } else {
                    // not time to repeat
                    info.repeat_timer = Some(timer);
                }
            }
        }
    }
}

/// Represents mouse gestures that can initiate a click.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ClickTrigger {
    /// Widget is clicked when the same mouse button is pressed and released on it.
    PressRelease,
    /// Widget is clicked when a mouse button is pressed on it.
    Press,
    /// Widget is clicked when a mouse button is released on it, even if not pressed on it.
    Release,
}

/// Defines how click events are generated for a widget.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ClickMode {
    /// Gesture that causes the *first* click, that is a click that is not in the double-click interval.
    pub single: ClickTrigger,

    /// Gesture that causes the subsequent clicks, if done within the double-click interval.
    pub double: ClickTrigger,

    /// If a mouse button is held pressed after a delay generate repeat clicks on an interval.
    pub repeat: bool,
}
impl Default for ClickMode {
    /// First `PressRelease`, double `Press` and no repeat.
    fn default() -> Self {
        Self {
            single: ClickTrigger::PressRelease,
            double: ClickTrigger::Press,
            repeat: false,
        }
    }
}
impl IntoVar<Option<ClickMode>> for ClickTrigger {
    type Var = LocalVar<Option<ClickMode>>;

    fn into_var(self) -> Self::Var {
        Some(ClickMode::from(self)).into_var()
    }
}
impl_from_and_into_var! {
    fn from(gesture: ClickTrigger) -> ClickMode {
        ClickMode {
            single: gesture,
            double: gesture,
            repeat: false,
        }
    }

    fn from(some: ClickMode) -> Option<ClickMode>;
}
impl ClickMode {
    /// Click on [`ClickTrigger::Press`].
    pub fn press() -> Self {
        Self {
            single: ClickTrigger::Press,
            double: ClickTrigger::Press,
            repeat: false,
        }
    }

    /// Click on release.
    pub fn release() -> Self {
        Self {
            single: ClickTrigger::Release,
            double: ClickTrigger::Release,
            repeat: false,
        }
    }

    /// Click on press and repeat.
    pub fn repeat() -> Self {
        Self {
            single: ClickTrigger::Press,
            double: ClickTrigger::Press,
            repeat: true,
        }
    }

    /// Click on press+release or repeat.
    pub fn mixed_repeat() -> Self {
        Self {
            single: ClickTrigger::PressRelease,
            double: ClickTrigger::Press,
            repeat: true,
        }
    }
}

/// Mouse config methods.
pub trait WidgetInfoMouseExt {
    /// Gets the click mode of the widget.
    fn click_mode(&self) -> ClickMode;
}
impl WidgetInfoMouseExt for WidgetInfo {
    fn click_mode(&self) -> ClickMode {
        for w in self.self_and_ancestors() {
            if let Some(m) = w.meta().get_clone(*CLICK_MODE_ID).flatten() {
                return m;
            }
        }
        ClickMode::default()
    }
}

/// Mouse config builder methods.
pub trait WidgetInfoBuilderMouseExt {
    /// Sets the click mode of the widget.
    ///
    /// Setting this to `None` will cause the widget to inherit the click mode.
    fn set_click_mode(&mut self, mode: Option<ClickMode>);
}
impl WidgetInfoBuilderMouseExt for WidgetInfoBuilder {
    fn set_click_mode(&mut self, mode: Option<ClickMode>) {
        self.with_meta(|mut m| match m.entry(*CLICK_MODE_ID) {
            state_map::StateMapEntry::Occupied(mut e) => *e.get_mut() = mode,
            state_map::StateMapEntry::Vacant(e) => {
                if mode.is_some() {
                    e.insert(mode);
                }
            }
        })
    }
}

static_id! {
    static ref CLICK_MODE_ID: StateId<Option<ClickMode>>;
}

/// Settings that define the mouse button pressed repeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ButtonRepeatConfig {
    /// Delay before repeat starts.
    pub start_delay: Duration,
    /// Delay before each repeat event after the first.
    pub interval: Duration,
}
impl Default for ButtonRepeatConfig {
    /// 600ms, 100ms.
    fn default() -> Self {
        Self {
            start_delay: Duration::from_millis(600),
            interval: Duration::from_millis(100),
        }
    }
}

/// Mouse service.
///
/// # Mouse Capture
///
/// Mouse capture is integrated with touch capture in the [`POINTER_CAPTURE`] service.
///
/// # Provider
///
/// This service is provided by the [`MouseManager`] extension.
///
/// [`POINTER_CAPTURE`]: crate::pointer_capture::POINTER_CAPTURE
pub struct MOUSE;
impl MOUSE {
    /// Returns a read-only variable that tracks the [buttons] that are currently pressed.
    ///
    /// [buttons]: MouseButton
    pub fn buttons(&self) -> ReadOnlyArcVar<Vec<MouseButton>> {
        MOUSE_SV.read().buttons.read_only()
    }

    /// Variable that defines the click-count increment time and area, a.k.a. the double-click config.
    ///
    /// Repeated clicks with an interval less then this time and within the distance of the first click increment the click count.
    ///
    /// The value is the same as [`sys_multi_click_config`], if set the variable disconnects from system config.
    ///
    /// [`sys_multi_click_config`]: Self::sys_multi_click_config
    pub fn multi_click_config(&self) -> ArcCowVar<MultiClickConfig, ArcVar<MultiClickConfig>> {
        MOUSE_SV.read().multi_click_config.clone()
    }

    /// Variable that tracks the system click-count increment time and area, a.k.a. the double-click config.
    ///
    /// # Value Source
    ///
    /// The value comes from the operating system settings, the variable
    /// updates with a new value if the system setting is changed and on view-process (re)init.
    ///
    /// In headless apps the default is [`MultiClickConfig::default`] and does not change.
    pub fn sys_multi_click_config(&self) -> ReadOnlyArcVar<MultiClickConfig> {
        MOUSE_SV.read().sys_multi_click_config.read_only()
    }

    /// Variable that gets and sets the config for [`ClickMode::repeat`] clicks.
    ///
    /// Note that this variable is linked with [`KEYBOARD.repeat_config`] until it is set, so if it is never set
    /// it will update with the keyboard value.
    ///
    /// [`KEYBOARD.repeat_config`]: KEYBOARD::repeat_config
    pub fn repeat_config(&self) -> BoxedVar<ButtonRepeatConfig> {
        MOUSE_SV.read().repeat_config.clone()
    }

    /// Variable that gets current hovered window and cursor point over that window.
    pub fn position(&self) -> ReadOnlyArcVar<Option<MousePosition>> {
        MOUSE_SV.read().position.read_only()
    }

    /// Variable that gets the current hovered window and widgets.
    pub fn hovered(&self) -> ReadOnlyArcVar<Option<InteractionPath>> {
        MOUSE_SV.read().hovered.read_only()
    }
}

/// Mouse cursor position.
///
/// Tracked in [`MOUSE.position`].
///
/// [`MOUSE.position`]: MOUSE::position
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MousePosition {
    /// Window the mouse is hovering.
    pub window_id: WindowId,
    /// Mouse position in the window.
    pub position: DipPoint,
    /// Timestamp of the mouse move.
    pub timestamp: DInstant,
}

app_local! {
    static MOUSE_SV: MouseService = {
        let sys_multi_click_config = var(MultiClickConfig::default());
        MouseService {
            multi_click_config: sys_multi_click_config.cow(),
            sys_multi_click_config,
            repeat_config: KEYBOARD
                .repeat_config()
                .map(|c| ButtonRepeatConfig {
                    start_delay: c.start_delay,
                    interval: c.interval,
                })
                .cow()
                .boxed(),
            buttons: var(vec![]),
            hovered: var(None),
            position: var(None),
        }
    };
}
struct MouseService {
    multi_click_config: ArcCowVar<MultiClickConfig, ArcVar<MultiClickConfig>>,
    sys_multi_click_config: ArcVar<MultiClickConfig>,
    repeat_config: BoxedVar<ButtonRepeatConfig>,
    buttons: ArcVar<Vec<MouseButton>>,
    hovered: ArcVar<Option<InteractionPath>>,
    position: ArcVar<Option<MousePosition>>,
}
