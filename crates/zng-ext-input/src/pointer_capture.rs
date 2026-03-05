//! Mouse and touch capture.
//!
//! # Events
//!
//! Events this extension provides.
//!
//! * [`POINTER_CAPTURE_EVENT`]
//!
//! # Services
//!
//! Services this extension provides.
//!
//! * [`POINTER_CAPTURE`]

use std::{
    collections::HashSet,
    fmt,
};

use zng_app::{
    event::{event, event_args},
    update::UPDATES,
    view_process::{
        VIEW_PROCESS_INITED_EVENT,
        raw_device_events::InputDeviceId,
        raw_events::{RAW_MOUSE_INPUT_EVENT, RAW_TOUCH_EVENT, RAW_WINDOW_CLOSE_EVENT, RAW_WINDOW_FOCUS_EVENT},
    },
    widget::{
        WidgetId,
        info::{InteractionPath, WIDGET_TREE_CHANGED_EVENT, WidgetInfoTree, WidgetPath},
    },
    window::WindowId,
};
use zng_app_context::app_local;
use zng_ext_window::WINDOWS;
use zng_var::{Var, impl_from_and_into_var, var};
use zng_view_api::{
    mouse::{ButtonState, MouseButton},
    touch::{TouchId, TouchPhase},
};

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
#[expect(non_camel_case_types)]
pub struct POINTER_CAPTURE;
impl POINTER_CAPTURE {
    /// Variable that gets the current capture target and mode.
    pub fn current_capture(&self) -> Var<Option<CaptureInfo>> {
        POINTER_CAPTURE_SV.read().capture.read_only()
    }

    /// Set a widget to redirect all mouse and touch events to.
    ///
    /// The capture will be set only if the widget is pressed.
    pub fn capture_widget(&self, widget_id: WidgetId) {
        self.capture_impl(widget_id, CaptureMode::Widget);
    }

    /// Set a widget to be the root of a capture subtree.
    ///
    /// Mouse and touch events targeting inside the subtree go to target normally. Mouse and touch events outside
    /// the capture root are redirected to the capture root.
    ///
    /// The capture will be set only if the widget is pressed.
    pub fn capture_subtree(&self, widget_id: WidgetId) {
        self.capture_impl(widget_id, CaptureMode::Subtree);
    }

    fn capture_impl(&self, widget_id: WidgetId, mode: CaptureMode) {
        UPDATES.once_update("POINTER_CAPTURE.capture", move || {
            let mut s = POINTER_CAPTURE_SV.write();
            if let Some(cap) = &s.capture_value {
                if let Some(wgt) = WINDOWS.widget_tree(cap.target.window_id()).and_then(|t| t.get(widget_id)) {
                    s.set_capture(wgt.interaction_path(), mode);
                } else {
                    tracing::debug!("ignoring capture request for {widget_id}, no found in pressed window");
                }
            } else {
                tracing::debug!("ignoring capture request for {widget_id}, no window is pressed");
            }
        });
    }

    /// Release the current mouse and touch capture back to window.
    ///
    /// **Note:** The capture is released automatically when the mouse buttons or touch are released
    /// or when the window loses focus.
    pub fn release_capture(&self) {
        UPDATES.once_update("POINTER_CAPTURE.release_capture", move || {
            let mut s = POINTER_CAPTURE_SV.write();
            if let Some(cap) = &s.capture_value
                && cap.mode != CaptureMode::Window
            {
                // release capture (back to default capture).
                let target = cap.target.root_path().into_owned();
                s.set_capture(InteractionPath::from_enabled(target), CaptureMode::Window);
            } else {
                tracing::debug!("ignoring release_capture request, no widget or subtree holding capture");
            }
        });
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
        if widget { CaptureMode::Widget } else { CaptureMode::Window }
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
    /// | Mode           | Allows                                             |
    /// |----------------|----------------------------------------------------|
    /// | `Window`       | All widgets in the same window.                    |
    /// | `Subtree`      | All widgets that have the `target` in their path.  |
    /// | `Widget`       | Only the `target` widget.                          |
    ///
    /// [`WIDGET`]: zng_app::widget::WIDGET
    /// [`WINDOW`]: zng_app::window::WINDOW
    pub fn allows(&self, wgt: (WindowId, WidgetId)) -> bool {
        match self.mode {
            CaptureMode::Window => self.target.window_id() == wgt.0,
            CaptureMode::Widget => self.target.widget_id() == wgt.1,
            CaptureMode::Subtree => {
                if let Some(wgt) = WINDOWS.widget_tree(wgt.0).and_then(|t| t.get(wgt.1)) {
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
    static POINTER_CAPTURE_SV: PointerCaptureService = {
        hooks();
        PointerCaptureService {
            capture_value: None,
            capture: var(None),

            mouse_down: Default::default(),
            touch_down: Default::default(),
        }
    };
}

struct PointerCaptureService {
    capture_value: Option<CaptureInfo>,
    capture: Var<Option<CaptureInfo>>,

    mouse_down: HashSet<(WindowId, InputDeviceId, MouseButton)>,
    touch_down: HashSet<(WindowId, InputDeviceId, TouchId)>,
}

event! {
    /// Mouse and touch capture changed event.
    pub static POINTER_CAPTURE_EVENT: PointerCaptureArgs {
        let _ = POINTER_CAPTURE_SV.read();
    };
}

event_args! {
    /// [`POINTER_CAPTURE_EVENT`] arguments.
    pub struct PointerCaptureArgs {
        /// Previous mouse and touch capture target and mode.
        pub prev_capture: Option<CaptureInfo>,
        /// new mouse and capture target and mode.
        pub new_capture: Option<CaptureInfo>,

        ..

        /// If is in [`prev_capture`] or [`new_capture`] paths start with the current path.
        ///
        /// [`prev_capture`]: Self::prev_capture
        /// [`new_capture`]: Self::new_capture
        fn is_in_target(&self, id: WidgetId) -> bool {
            if let Some(p) = &self.prev_capture
                && p.target.contains(id)
            {
                return true;
            }
            if let Some(p) = &self.new_capture
                && p.target.contains(id)
            {
                return true;
            }
            false
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

fn hooks() {
    WIDGET_TREE_CHANGED_EVENT
        .hook(|args| {
            let mut s = POINTER_CAPTURE_SV.write();
            if let Some(c) = &s.capture_value
                && c.target.window_id() == args.tree.window_id()
            {
                s.continue_capture(&args.tree);
            }
            true
        })
        .perm();

    RAW_MOUSE_INPUT_EVENT
        .hook(|args| {
            let mut s = POINTER_CAPTURE_SV.write();
            match args.state {
                ButtonState::Pressed => {
                    if s.mouse_down.insert((args.window_id, args.device_id, args.button))
                        && s.mouse_down.len() == 1
                        && s.touch_down.is_empty()
                    {
                        s.on_first_down(args.window_id);
                    }
                }
                ButtonState::Released => {
                    if s.mouse_down.remove(&(args.window_id, args.device_id, args.button))
                        && s.mouse_down.is_empty()
                        && s.touch_down.is_empty()
                    {
                        s.on_last_up();
                    }
                }
            }
            true
        })
        .perm();

    RAW_TOUCH_EVENT
        .hook(|args| {
            let mut s = POINTER_CAPTURE_SV.write();
            for touch in &args.touches {
                match touch.phase {
                    TouchPhase::Start => {
                        if s.touch_down.insert((args.window_id, args.device_id, touch.touch))
                            && s.touch_down.len() == 1
                            && s.mouse_down.is_empty()
                        {
                            s.on_first_down(args.window_id);
                        }
                    }
                    TouchPhase::End | TouchPhase::Cancel => {
                        if s.touch_down.remove(&(args.window_id, args.device_id, touch.touch))
                            && s.touch_down.is_empty()
                            && s.mouse_down.is_empty()
                        {
                            s.on_last_up();
                        }
                    }
                    TouchPhase::Move => {}
                }
            }
            true
        })
        .perm();

    RAW_WINDOW_CLOSE_EVENT
        .hook(|args| {
            POINTER_CAPTURE_SV.write().remove_window(args.window_id);
            true
        })
        .perm();

    fn nest_parent(id: WindowId) -> Option<WindowId> {
        WINDOWS
            .vars(id)
            .and_then(|v| if v.nest_parent().get().is_some() { v.parent().get() } else { None })
    }

    RAW_WINDOW_FOCUS_EVENT
        .hook(|args| {
            let actual_prev = args.prev_focus.map(|id| nest_parent(id).unwrap_or(id));
            let actual_new = args.new_focus.map(|id| nest_parent(id).unwrap_or(id));

            if actual_prev == actual_new {
                // can happen when focus moves from parent to nested, or malformed event
                return true;
            }

            if let Some(w) = actual_prev {
                POINTER_CAPTURE_SV.write().remove_window(w);
            }
            true
        })
        .perm();

    VIEW_PROCESS_INITED_EVENT
        .hook(|args| {
            if args.is_respawn {
                let mut s = POINTER_CAPTURE_SV.write();

                if !s.mouse_down.is_empty() || !s.touch_down.is_empty() {
                    s.mouse_down.clear();
                    s.touch_down.clear();
                    s.on_last_up();
                }
            }
            true
        })
        .perm();
}
impl PointerCaptureService {
    fn remove_window(&mut self, window_id: WindowId) {
        if !self.mouse_down.is_empty() || !self.touch_down.is_empty() {
            self.mouse_down.retain(|(w, _, _)| *w != window_id);
            self.touch_down.retain(|(w, _, _)| *w != window_id);

            if self.mouse_down.is_empty() && self.touch_down.is_empty() {
                self.on_last_up();
            }
        }
    }

    fn on_first_down(&mut self, window_id: WindowId) {
        if let Some(info) = WINDOWS.widget_tree(window_id) {
            // default capture
            self.set_capture(info.root().interaction_path(), CaptureMode::Window);
        }
    }

    fn on_last_up(&mut self) {
        self.unset_capture();
    }

    fn continue_capture(&mut self, info: &WidgetInfoTree) {
        let current = self.capture_value.as_ref().unwrap();

        if let Some(widget) = info.get(current.target.widget_id()) {
            if let Some(new_path) = widget.new_interaction_path(&InteractionPath::from_enabled(current.target.clone())) {
                // widget moved inside window tree.
                let mode = current.mode;
                self.set_capture(new_path, mode);
            }
        } else {
            // widget not found. Returns to default capture.
            self.set_capture(info.root().interaction_path(), CaptureMode::Window);
        }
    }

    fn set_capture(&mut self, target: InteractionPath, mode: CaptureMode) {
        let new = target.enabled().map(|target| CaptureInfo { target, mode });
        if new.is_none() {
            self.unset_capture();
            return;
        }
        if new != self.capture_value {
            let prev = self.capture_value.take();
            self.capture_value.clone_from(&new);
            self.capture.set(new.clone());
            POINTER_CAPTURE_EVENT.notify(PointerCaptureArgs::now(prev, new));
        }
    }

    fn unset_capture(&mut self) {
        if self.capture_value.is_some() {
            let prev = self.capture_value.take();
            self.capture_value = None;
            self.capture.set(None);
            POINTER_CAPTURE_EVENT.notify(PointerCaptureArgs::now(prev, None));
        }
    }
}
