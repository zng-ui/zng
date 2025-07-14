//! Mouse and touch capture.

use std::{
    collections::{HashMap, HashSet},
    fmt, mem,
};

use zng_app::{
    AppExtension,
    event::{event, event_args},
    update::{EventUpdate, UPDATES},
    view_process::{
        VIEW_PROCESS_INITED_EVENT,
        raw_device_events::InputDeviceId,
        raw_events::{
            RAW_FRAME_RENDERED_EVENT, RAW_MOUSE_INPUT_EVENT, RAW_MOUSE_MOVED_EVENT, RAW_TOUCH_EVENT, RAW_WINDOW_CLOSE_EVENT,
            RAW_WINDOW_FOCUS_EVENT,
        },
    },
    widget::{
        WIDGET, WidgetId,
        info::{InteractionPath, WIDGET_INFO_CHANGED_EVENT, WidgetInfoTree, WidgetPath},
    },
    window::{WINDOW, WindowId},
};
use zng_app_context::app_local;
use zng_ext_window::{NestedWindowWidgetInfoExt, WINDOWS};
use zng_layout::unit::{DipPoint, DipToPx};
use zng_var::{ArcVar, ReadOnlyArcVar, Var, impl_from_and_into_var, var};
use zng_view_api::{
    mouse::{ButtonState, MouseButton},
    touch::{TouchId, TouchPhase},
};

/// Application extension that provides mouse and touch capture service.
///
/// # Events
///
/// Events this extension provides.
///
/// * [`POINTER_CAPTURE_EVENT`]
///
/// # Services
///
/// Services this extension provides.
///
/// * [`POINTER_CAPTURE`]
#[derive(Default)]
pub struct PointerCaptureManager {
    mouse_position: HashMap<(WindowId, InputDeviceId), DipPoint>,
    mouse_down: HashSet<(WindowId, InputDeviceId, MouseButton)>,
    touch_down: HashSet<(WindowId, InputDeviceId, TouchId)>,
    capture: Option<CaptureInfo>,
}
impl AppExtension for PointerCaptureManager {
    fn event(&mut self, update: &mut EventUpdate) {
        if let Some(args) = RAW_FRAME_RENDERED_EVENT.on(update) {
            if let Some(c) = &self.capture {
                if c.target.window_id() == args.window_id {
                    if let Ok(info) = WINDOWS.widget_tree(args.window_id) {
                        self.continue_capture(&info);
                    }
                    // else will receive close event.
                }
            }
        } else if let Some(args) = RAW_MOUSE_MOVED_EVENT.on(update) {
            self.mouse_position.insert((args.window_id, args.device_id), args.position);
        } else if let Some(args) = RAW_MOUSE_INPUT_EVENT.on(update) {
            match args.state {
                ButtonState::Pressed => {
                    if self.mouse_down.insert((args.window_id, args.device_id, args.button))
                        && self.mouse_down.len() == 1
                        && self.touch_down.is_empty()
                    {
                        self.on_first_down(
                            args.window_id,
                            self.mouse_position
                                .get(&(args.window_id, args.device_id))
                                .copied()
                                .unwrap_or_default(),
                        );
                    }
                }
                ButtonState::Released => {
                    if self.mouse_down.remove(&(args.window_id, args.device_id, args.button))
                        && self.mouse_down.is_empty()
                        && self.touch_down.is_empty()
                    {
                        self.on_last_up();
                    }
                }
            }
        } else if let Some(args) = RAW_TOUCH_EVENT.on(update) {
            for touch in &args.touches {
                match touch.phase {
                    TouchPhase::Start => {
                        if self.touch_down.insert((args.window_id, args.device_id, touch.touch))
                            && self.touch_down.len() == 1
                            && self.mouse_down.is_empty()
                        {
                            self.on_first_down(args.window_id, touch.position);
                        }
                    }
                    TouchPhase::End | TouchPhase::Cancel => {
                        if self.touch_down.remove(&(args.window_id, args.device_id, touch.touch))
                            && self.touch_down.is_empty()
                            && self.mouse_down.is_empty()
                        {
                            self.on_last_up();
                        }
                    }
                    TouchPhase::Move => {}
                }
            }
        } else if let Some(args) = WIDGET_INFO_CHANGED_EVENT.on(update) {
            if let Some(c) = &self.capture {
                if c.target.window_id() == args.window_id {
                    self.continue_capture(&args.tree);
                }
            }
        } else if let Some(args) = RAW_WINDOW_CLOSE_EVENT.on(update) {
            self.remove_window(args.window_id);
        } else if let Some(args) = RAW_WINDOW_FOCUS_EVENT.on(update) {
            let actual_prev = args.prev_focus.map(|id| WINDOWS.nest_parent(id).map(|(p, _)| p).unwrap_or(id));
            let actual_new = args.new_focus.map(|id| WINDOWS.nest_parent(id).map(|(p, _)| p).unwrap_or(id));

            if actual_prev == actual_new {
                // can happen when focus moves from parent to nested, or malformed event
                return;
            }

            if let Some(w) = actual_prev {
                self.remove_window(w);
            }
        } else if let Some(args) = VIEW_PROCESS_INITED_EVENT.on(update) {
            if args.is_respawn && (!self.mouse_down.is_empty() || !self.touch_down.is_empty()) {
                self.mouse_down.clear();
                self.touch_down.clear();
                self.on_last_up();
            }
        }
    }

    fn update(&mut self) {
        if let Some(current) = &self.capture {
            let mut cap = POINTER_CAPTURE_SV.write();
            if let Some((widget_id, mode)) = cap.capture_request.take() {
                let is_win_focused = match WINDOWS.is_focused(current.target.window_id()) {
                    Ok(mut f) => {
                        if !f {
                            // nested windows can take two updates to receive focus
                            if let Some(parent) = WINDOWS.nest_parent(current.target.window_id()).map(|(p, _)| p) {
                                f = WINDOWS.is_focused(parent) == Ok(true);
                            }
                        }
                        f
                    }
                    Err(_) => false,
                };
                if is_win_focused {
                    // current window pressed
                    if let Some(widget) = WINDOWS.widget_tree(current.target.window_id()).unwrap().get(widget_id) {
                        // request valid
                        self.set_capture(&mut cap, widget.interaction_path(), mode);
                    }
                }
            } else if mem::take(&mut cap.release_requested) && current.mode != CaptureMode::Window {
                // release capture (back to default capture).
                let target = current.target.root_path();
                self.set_capture(&mut cap, InteractionPath::from_enabled(target.into_owned()), CaptureMode::Window);
            }
        }
    }
}
impl PointerCaptureManager {
    fn remove_window(&mut self, window_id: WindowId) {
        self.mouse_position.retain(|(w, _), _| *w != window_id);

        if !self.mouse_down.is_empty() || !self.touch_down.is_empty() {
            self.mouse_down.retain(|(w, _, _)| *w != window_id);
            self.touch_down.retain(|(w, _, _)| *w != window_id);

            if self.mouse_down.is_empty() && self.touch_down.is_empty() {
                self.on_last_up();
            }
        }
    }

    fn on_first_down(&mut self, window_id: WindowId, point: DipPoint) {
        if let Ok(mut info) = WINDOWS.widget_tree(window_id) {
            let mut cap = POINTER_CAPTURE_SV.write();
            cap.release_requested = false;

            let mut point = point.to_px(info.scale_factor());

            // hit-test for nested window
            if let Some(t) = info.root().hit_test(point).target() {
                if let Some(w) = info.get(t.widget_id) {
                    if let Some(t) = w.nested_window_tree() {
                        info = t;
                        point = w
                            .inner_transform()
                            .inverse()
                            .and_then(|t| t.transform_point(point))
                            .unwrap_or(point);
                    }
                }
            }

            if let Some((widget_id, mode)) = cap.capture_request.take() {
                if let Some(w_info) = info.get(widget_id) {
                    if w_info.hit_test(point).contains(widget_id) {
                        // capture for widget
                        self.set_capture(&mut cap, w_info.interaction_path(), mode);
                        return;
                    }
                }
            }

            // default capture
            self.set_capture(&mut cap, info.root().interaction_path(), CaptureMode::Window);
        }
    }

    fn on_last_up(&mut self) {
        let mut cap = POINTER_CAPTURE_SV.write();
        cap.release_requested = false;
        cap.capture_request = None;
        self.unset_capture(&mut cap);
    }

    fn continue_capture(&mut self, info: &WidgetInfoTree) {
        let current = self.capture.as_ref().unwrap();

        if let Some(widget) = info.get(current.target.widget_id()) {
            if let Some(new_path) = widget.new_interaction_path(&InteractionPath::from_enabled(current.target.clone())) {
                // widget moved inside window tree.
                let mode = current.mode;
                self.set_capture(&mut POINTER_CAPTURE_SV.write(), new_path, mode);
            }
        } else {
            // widget not found. Returns to default capture.
            self.set_capture(&mut POINTER_CAPTURE_SV.write(), info.root().interaction_path(), CaptureMode::Window);
        }
    }

    fn set_capture(&mut self, cap: &mut PointerCaptureService, target: InteractionPath, mode: CaptureMode) {
        let new = target.enabled().map(|target| CaptureInfo { target, mode });
        if new.is_none() {
            self.unset_capture(cap);
            return;
        }
        if new != self.capture {
            let prev = self.capture.take();
            self.capture.clone_from(&new);
            cap.capture_value.clone_from(&new);
            cap.capture.set(new.clone());
            POINTER_CAPTURE_EVENT.notify(PointerCaptureArgs::now(prev, new));
        }
    }

    fn unset_capture(&mut self, cap: &mut PointerCaptureService) {
        if self.capture.is_some() {
            let prev = self.capture.take();
            cap.capture_value = None;
            cap.capture.set(None);
            POINTER_CAPTURE_EVENT.notify(PointerCaptureArgs::now(prev, None));
        }
    }
}

/// Mouse and touch capture service.
///
/// Mouse and touch is **captured** when mouse and touch events are redirected to a specific target. The user
/// can still move the cursor or touch contact outside of the target but the widgets outside do not react to this.
///
/// You can request capture by calling [`capture_widget`](POINTER_CAPTURE::capture_widget) or
/// [`capture_subtree`](POINTER_CAPTURE::capture_subtree) with a widget that was pressed by a mouse button or by touch.
/// The capture will last for as long as any of the mouse buttons or touch contacts are pressed, the widget is visible
/// and the window is focused.
///
/// Windows capture by default, this cannot be disabled. For other widgets this is optional.
///
/// # Provider
///
/// This service is provided by the [`PointerCaptureManager`] extension.
#[expect(non_camel_case_types)]
pub struct POINTER_CAPTURE;
impl POINTER_CAPTURE {
    /// Variable that gets the current capture target and mode.
    pub fn current_capture(&self) -> ReadOnlyArcVar<Option<CaptureInfo>> {
        POINTER_CAPTURE_SV.read().capture.read_only()
    }

    /// Set a widget to redirect all mouse and touch events to.
    ///
    /// The capture will be set only if the widget is pressed.
    pub fn capture_widget(&self, widget_id: WidgetId) {
        let mut m = POINTER_CAPTURE_SV.write();
        m.capture_request = Some((widget_id, CaptureMode::Widget));
        UPDATES.update(None);
    }

    /// Set a widget to be the root of a capture subtree.
    ///
    /// Mouse and touch events targeting inside the subtree go to target normally. Mouse and touch events outside
    /// the capture root are redirected to the capture root.
    ///
    /// The capture will be set only if the widget is pressed.
    pub fn capture_subtree(&self, widget_id: WidgetId) {
        let mut m = POINTER_CAPTURE_SV.write();
        m.capture_request = Some((widget_id, CaptureMode::Subtree));
        UPDATES.update(None);
    }

    /// Release the current mouse and touch capture back to window.
    ///
    /// **Note:** The capture is released automatically when the mouse buttons or touch are released
    /// or when the window loses focus.
    pub fn release_capture(&self) {
        let mut m = POINTER_CAPTURE_SV.write();
        m.release_requested = true;
        UPDATES.update(None);
    }

    /// Latest capture, already valid for the current raw mouse or touch event cycle.
    pub(crate) fn current_capture_value(&self) -> Option<CaptureInfo> {
        POINTER_CAPTURE_SV.read().capture_value.clone()
    }
}

/// Mouse and touch capture mode.
#[derive(Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CaptureMode {
    /// Mouse and touch captured by the window only.
    ///
    /// Default behavior.
    Window,
    /// Mouse and touch events inside the widget sub-tree permitted. Mouse events
    /// outside of the widget redirected to the widget.
    Subtree,

    /// Mouse and touch events redirected to the widget.
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

/// Information about mouse and touch capture in a mouse or touch event argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureInfo {
    /// Widget that is capturing all mouse and touch events. The widget and all ancestors are [`ENABLED`].
    ///
    /// This is the window root widget for capture mode `Window`.
    ///
    /// [`ENABLED`]: zng_app::widget::info::Interactivity::ENABLED
    pub target: WidgetPath,
    /// Capture mode, see [`allows`](Self::allows) for more details.
    pub mode: CaptureMode,
}
impl CaptureInfo {
    /// If the widget is allowed by the current capture.
    ///
    /// This method uses [`WINDOW`] and [`WIDGET`] to identify the widget context.
    ///
    /// | Mode           | Allows                                             |
    /// |----------------|----------------------------------------------------|
    /// | `Window`       | All widgets in the same window.                    |
    /// | `Subtree`      | All widgets that have the `target` in their path.  |
    /// | `Widget`       | Only the `target` widget.                          |
    ///
    /// [`WIDGET`]: zng_app::widget::WIDGET
    /// [`WINDOW`]: zng_app::window::WINDOW
    pub fn allows(&self) -> bool {
        match self.mode {
            CaptureMode::Window => self.target.window_id() == WINDOW.id(),
            CaptureMode::Widget => self.target.widget_id() == WIDGET.id(),
            CaptureMode::Subtree => {
                let tree = WINDOW.info();
                if let Some(wgt) = tree.get(WIDGET.id()) {
                    for wgt in wgt.self_and_ancestors() {
                        if wgt.id() == self.target.widget_id() {
                            return true;
                        }
                    }
                }
                false
            }
        }
    }
}

app_local! {
    static POINTER_CAPTURE_SV: PointerCaptureService = PointerCaptureService {
        capture_value: None,
        capture: var(None),
        capture_request: None,
        release_requested: false,
    };
}

struct PointerCaptureService {
    capture_value: Option<CaptureInfo>,
    capture: ArcVar<Option<CaptureInfo>>,
    capture_request: Option<(WidgetId, CaptureMode)>,
    release_requested: bool,
}

event! {
    /// Mouse and touch capture changed event.
    pub static POINTER_CAPTURE_EVENT: PointerCaptureArgs;
}

event_args! {
    /// [`POINTER_CAPTURE_EVENT`] arguments.
    pub struct PointerCaptureArgs {
        /// Previous mouse and touch capture target and mode.
        pub prev_capture: Option<CaptureInfo>,
        /// new mouse and capture target and mode.
        pub new_capture: Option<CaptureInfo>,

        ..

        /// The [`prev_capture`] and [`new_capture`] paths start with the current path.
        ///
        /// [`prev_capture`]: Self::prev_capture
        /// [`new_capture`]: Self::new_capture
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            if let Some(p) = &self.prev_capture {
                list.insert_wgt(&p.target);
            }
            if let Some(p) = &self.new_capture {
                list.insert_wgt(&p.target);
            }
        }
    }
}

impl PointerCaptureArgs {
    /// If the same widget has pointer capture, but the widget path changed.
    pub fn is_widget_move(&self) -> bool {
        match (&self.prev_capture, &self.new_capture) {
            (Some(prev), Some(new)) => prev.target.widget_id() == new.target.widget_id() && prev.target != new.target,
            _ => false,
        }
    }

    /// If the same widget has pointer capture, but the capture mode changed.
    pub fn is_mode_change(&self) -> bool {
        match (&self.prev_capture, &self.new_capture) {
            (Some(prev), Some(new)) => prev.target.widget_id() == new.target.widget_id() && prev.mode != new.mode,
            _ => false,
        }
    }

    /// If the `widget_id` lost pointer capture with this update.
    pub fn is_lost(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_capture, &self.new_capture) {
            (None, _) => false,
            (Some(p), None) => p.target.widget_id() == widget_id,
            (Some(prev), Some(new)) => prev.target.widget_id() == widget_id && new.target.widget_id() != widget_id,
        }
    }

    /// If the `widget_id` got pointer capture with this update.
    pub fn is_got(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_capture, &self.new_capture) {
            (_, None) => false,
            (None, Some(p)) => p.target.widget_id() == widget_id,
            (Some(prev), Some(new)) => prev.target.widget_id() != widget_id && new.target.widget_id() == widget_id,
        }
    }
}
