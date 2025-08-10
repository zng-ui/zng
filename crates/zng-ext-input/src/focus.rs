//! Keyboard focus manager.

pub mod iter;

mod focus_info;
pub use focus_info::*;

use zng_app::{
    AppExtension, DInstant, INSTANT,
    access::{ACCESS_CLICK_EVENT, ACCESS_FOCUS_EVENT, ACCESS_FOCUS_NAV_ORIGIN_EVENT},
    event::{event, event_args},
    update::{EventUpdate, InfoUpdates, RenderUpdates, UPDATES},
    view_process::raw_events::RAW_KEY_INPUT_EVENT,
    widget::{
        WidgetId,
        info::{InteractionPath, WIDGET_INFO_CHANGED_EVENT, WidgetBoundsInfo, WidgetInfoTree},
    },
    window::WindowId,
};

pub mod cmd;
use cmd::FocusCommands;
use zng_app_context::app_local;
use zng_ext_window::{WINDOW_FOCUS, WINDOW_FOCUS_CHANGED_EVENT, WINDOWS};
use zng_layout::unit::{Px, PxPoint, PxRect, TimeUnits};
use zng_unique_id::{IdEntry, IdMap};
use zng_var::{Var, var};
use zng_view_api::window::FrameId;

use std::{mem, time::Duration};

use crate::{mouse::MOUSE_INPUT_EVENT, touch::TOUCH_INPUT_EVENT};

event_args! {
    /// [`FOCUS_CHANGED_EVENT`] arguments.
    pub struct FocusChangedArgs {
        /// Previously focused widget.
        pub prev_focus: Option<InteractionPath>,

        /// Newly focused widget.
        pub new_focus: Option<InteractionPath>,

        /// If the focused widget should visually indicate that it is focused.
        ///
        /// This is `true` when the focus change is caused by a key press, `false` when it is caused by a mouse click.
        ///
        /// Some widgets, like *text input*, may ignore this field and always indicate that they are focused.
        pub highlight: bool,

        /// What caused this event.
        pub cause: FocusChangedCause,

        /// Focus navigation actions that can move the focus away from the [`new_focus`].
        ///
        /// [`new_focus`]: Self::new_focus
        pub enabled_nav: FocusNavAction,

        ..

        /// The [`prev_focus`](Self::prev_focus) and [`new_focus`](Self::new_focus).
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            if let Some(prev) = &self.prev_focus {
                list.insert_wgt(prev);
            }
            if let Some(new) = &self.new_focus {
                list.insert_wgt(new);
            }
        }
    }

    /// [`RETURN_FOCUS_CHANGED_EVENT`] arguments.
    pub struct ReturnFocusChangedArgs {
        /// The scope that returns the focus when focused directly.
        ///
        /// Is `None` if the previous focus was the return focus of a scope that was removed.
        pub scope: Option<InteractionPath>,

        /// Previous return focus of the widget.
        pub prev_return: Option<InteractionPath>,

        /// New return focus of the widget.
        pub new_return: Option<InteractionPath>,

        ..

        /// The [`prev_return`](Self::prev_return), [`new_return`](Self::new_return)
        /// and [`scope`](Self::scope).
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            if let Some(scope) = &self.scope {
                list.insert_wgt(scope)
            }
            if let Some(prev_return) = &self.prev_return {
                list.insert_wgt(prev_return)
            }
            if let Some(new_return) = &self.new_return {
                list.insert_wgt(new_return)
            }
        }
    }
}

impl FocusChangedArgs {
    /// If the focus is still in the same widget, but the widget path changed.
    pub fn is_widget_move(&self) -> bool {
        match (&self.prev_focus, &self.new_focus) {
            (Some(prev), Some(new)) => prev.widget_id() == new.widget_id() && prev.as_path() != new.as_path(),
            _ => false,
        }
    }

    /// If the focus is still in the same widget path, but some or all interactivity has changed.
    pub fn is_enabled_change(&self) -> bool {
        match (&self.prev_focus, &self.new_focus) {
            (Some(prev), Some(new)) => prev.as_path() == new.as_path() && prev.disabled_index() != new.disabled_index(),
            _ => false,
        }
    }

    /// If the focus is still in the same widget but [`highlight`](FocusChangedArgs::highlight) changed.
    pub fn is_highlight_changed(&self) -> bool {
        self.prev_focus == self.new_focus
    }

    /// If `widget_id` is the new focus and was not before.
    pub fn is_focus(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_focus, &self.new_focus) {
            (Some(prev), Some(new)) => prev.widget_id() != widget_id && new.widget_id() == widget_id,
            (None, Some(new)) => new.widget_id() == widget_id,
            (_, None) => false,
        }
    }

    /// If `widget_id` is the previous focus and is not now.
    pub fn is_blur(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_focus, &self.new_focus) {
            (Some(prev), Some(new)) => prev.widget_id() == widget_id && new.widget_id() != widget_id,
            (Some(prev), None) => prev.widget_id() == widget_id,
            (None, _) => false,
        }
    }

    /// If `widget_id` is the new focus or a parent of the new focus and was not the focus nor the parent of the previous focus.
    pub fn is_focus_enter(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_focus, &self.new_focus) {
            (Some(prev), Some(new)) => !prev.contains(widget_id) && new.contains(widget_id),
            (None, Some(new)) => new.contains(widget_id),
            (_, None) => false,
        }
    }

    /// If `widget_id` is the new focus or a parent of the new focus and is enabled;
    /// and was not the focus nor the parent of the previous focus or was not enabled.
    pub fn is_focus_enter_enabled(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_focus, &self.new_focus) {
            (Some(prev), Some(new)) => !prev.contains_enabled(widget_id) && new.contains_enabled(widget_id),
            (None, Some(new)) => new.contains_enabled(widget_id),
            (_, None) => false,
        }
    }

    /// If `widget_id` is the previous focus or a parent of the previous focus and is not the new focus nor a parent of the new focus.
    pub fn is_focus_leave(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_focus, &self.new_focus) {
            (Some(prev), Some(new)) => prev.contains(widget_id) && !new.contains(widget_id),
            (Some(prev), None) => prev.contains(widget_id),
            (None, _) => false,
        }
    }

    /// If `widget_id` is the previous focus or a parent of the previous focus and was enabled;
    /// and is not the new focus nor a parent of the new focus or is disabled.
    pub fn is_focus_leave_enabled(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_focus, &self.new_focus) {
            (Some(prev), Some(new)) => prev.contains_enabled(widget_id) && !new.contains_enabled(widget_id),
            (Some(prev), None) => prev.contains_enabled(widget_id),
            (None, _) => false,
        }
    }

    /// If the widget is the new focus.
    pub fn is_focused(&self, widget_id: WidgetId) -> bool {
        self.new_focus.as_ref().map(|p| p.widget_id() == widget_id).unwrap_or(false)
    }

    /// If the widget is in the new focus path.
    pub fn is_focus_within(&self, widget_id: WidgetId) -> bool {
        self.new_focus.as_ref().map(|p| p.contains(widget_id)).unwrap_or(false)
    }
}

impl ReturnFocusChangedArgs {
    /// If the return focus is the same widget but the widget path changed and the widget is still in the same focus scope.
    pub fn is_widget_move(&self) -> bool {
        match (&self.prev_return, &self.new_return) {
            (Some(prev), Some(new)) => prev.widget_id() == new.widget_id() && prev != new,
            _ => false,
        }
    }

    /// If [`scope`](Self::scope) is an ALT scope and `prev_return` or `new_return` if the
    /// widget outside the scope that will be focused back when the user escapes the ALT scope.
    pub fn is_alt_return(&self) -> bool {
        if let Some(scope) = &self.scope {
            match (&self.prev_return, &self.new_return) {
                (Some(prev), None) => !prev.contains(scope.widget_id()),
                (None, Some(new)) => !new.contains(scope.widget_id()),
                _ => false,
            }
        } else {
            false
        }
    }

    /// if the widget was in the [`prev_return`] and is not in the [`new_return`].
    ///
    /// [`prev_return`]: Self::prev_return
    /// [`new_return`]: Self::new_return
    pub fn lost_return_focus(&self, widget_id: WidgetId) -> bool {
        self.prev_return.as_ref().map(|p| p.contains(widget_id)).unwrap_or(false)
            && self.new_return.as_ref().map(|p| !p.contains(widget_id)).unwrap_or(true)
    }

    /// if the widget was not in the [`prev_return`] and is in the [`new_return`].
    ///
    /// [`prev_return`]: Self::prev_return
    /// [`new_return`]: Self::new_return
    pub fn got_return_focus(&self, widget_id: WidgetId) -> bool {
        self.prev_return.as_ref().map(|p| !p.contains(widget_id)).unwrap_or(true)
            && self.new_return.as_ref().map(|p| p.contains(widget_id)).unwrap_or(false)
    }

    /// if the widget was the [`prev_return`] and is the [`new_return`].
    ///
    /// [`prev_return`]: Self::prev_return
    /// [`new_return`]: Self::new_return
    pub fn was_return_focus(&self, widget_id: WidgetId) -> bool {
        self.prev_return.as_ref().map(|p| p.widget_id() == widget_id).unwrap_or(false)
            && self.new_return.as_ref().map(|p| p.widget_id() != widget_id).unwrap_or(true)
    }

    /// if the widget was not the [`prev_return`] and is the [`new_return`].
    ///
    /// [`prev_return`]: Self::prev_return
    /// [`new_return`]: Self::new_return
    pub fn is_return_focus(&self, widget_id: WidgetId) -> bool {
        self.prev_return.as_ref().map(|p| p.widget_id() != widget_id).unwrap_or(true)
            && self.new_return.as_ref().map(|p| p.widget_id() == widget_id).unwrap_or(false)
    }

    /// If `widget_id` is the new return focus or a parent of the new return and was not a parent of the previous return.
    pub fn is_return_focus_enter(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_return, &self.new_return) {
            (Some(prev), Some(new)) => !prev.contains(widget_id) && new.contains(widget_id),
            (None, Some(new)) => new.contains(widget_id),
            (_, None) => false,
        }
    }

    /// If `widget_id` is the previous return focus or a parent of the previous return and is not a parent of the new return.
    pub fn is_return_focus_leave(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_return, &self.new_return) {
            (Some(prev), Some(new)) => prev.contains(widget_id) && !new.contains(widget_id),
            (Some(prev), None) => prev.contains(widget_id),
            (None, _) => false,
        }
    }
}

/// The cause of a [`FOCUS_CHANGED_EVENT`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FocusChangedCause {
    /// The focus changed trying to fulfill the request.
    Request(FocusRequest),

    /// A focus scope got focus causing its [`FocusScopeOnFocus`] action to execute.
    ///
    /// The associated `bool` indicates if the focus was reversed in.
    ScopeGotFocus(bool),

    /// A previously focused widget, was removed or moved.
    Recovery,
}
impl FocusChangedCause {
    /// Get focus request target.
    pub fn request_target(self) -> Option<FocusTarget> {
        match self {
            Self::Request(r) => Some(r.target),
            _ => None,
        }
    }
}

event! {
    /// Keyboard focused widget changed event.
    ///
    /// # Provider
    ///
    /// This event is provided by the [`FocusManager`] extension.
    pub static FOCUS_CHANGED_EVENT: FocusChangedArgs;

    /// Scope return focus widget changed event.
    ///
    /// # Provider
    ///
    /// This event is provided by the [`FocusManager`] extension.
    pub static RETURN_FOCUS_CHANGED_EVENT: ReturnFocusChangedArgs;
}

/// Application extension that manages keyboard focus.
///
/// # Events
///
/// Events this extension provides.
///
/// * [`FOCUS_CHANGED_EVENT`]
/// * [`RETURN_FOCUS_CHANGED_EVENT`]
///
/// # Services
///
/// Services this extension provides.
///
/// * [`FOCUS`]
///
/// # Dependencies
///
/// This extension requires the [`WINDOWS`] service.
///
/// This extension listens to the [`MOUSE_INPUT_EVENT`], [`TOUCH_INPUT_EVENT`], [`SHORTCUT_EVENT`],
/// [`WINDOW_FOCUS_CHANGED_EVENT`] and [`WIDGET_INFO_CHANGED_EVENT`].
///
/// To work properly it should be added to the app after the windows manager extension.
///
/// # About Focus
///
/// See the [module level](../) documentation for an overview of the keyboard
/// focus concepts implemented by this app extension.
///
/// [`SHORTCUT_EVENT`]: crate::gesture::SHORTCUT_EVENT
/// [`WINDOWS`]: zng_ext_window::WINDOWS
/// [`WINDOW_FOCUS_CHANGED_EVENT`]: zng_ext_window::WINDOW_FOCUS_CHANGED_EVENT
/// [`WIDGET_INFO_CHANGED_EVENT`]: zng_app::widget::info::WIDGET_INFO_CHANGED_EVENT
#[derive(Default)]
pub struct FocusManager {
    commands: Option<FocusCommands>,
    pending_render: Option<WidgetInfoTree>,
}

impl AppExtension for FocusManager {
    fn init(&mut self) {
        WINDOW_FOCUS.hook_focus_service(FOCUS.focused());
        self.commands = Some(FocusCommands::new());
    }

    fn event_preview(&mut self, update: &mut EventUpdate) {
        if let Some(args) = WIDGET_INFO_CHANGED_EVENT.on(update) {
            if FOCUS_SV
                .read()
                .focused
                .as_ref()
                .map(|f| f.path.window_id() == args.window_id)
                .unwrap_or_default()
            {
                // we need up-to-date due to visibility or spatial movement and that is affected by both layout and render.
                // so we delay responding to the event if a render or layout was requested when the tree was invalidated.
                if UPDATES.is_pending_render(args.window_id) {
                    self.pending_render = Some(args.tree.clone());
                } else {
                    // no visual change, update interactivity changes.
                    self.pending_render = None;
                    self.on_info_tree_update(args.tree.clone());
                }
            }
            focus_info::FocusTreeData::consolidate_alt_scopes(&args.prev_tree, &args.tree);
        } else if let Some(args) = ACCESS_FOCUS_EVENT.on(update) {
            let is_focused = FOCUS.is_focused(args.widget_id).get();
            if args.focus {
                if !is_focused {
                    FOCUS.focus_widget(args.widget_id, false);
                }
            } else if is_focused {
                FOCUS.focus_exit();
            }
        } else if let Some(args) = ACCESS_FOCUS_NAV_ORIGIN_EVENT.on(update) {
            FOCUS.navigation_origin().set(Some(args.widget_id));
        } else {
            self.commands.as_mut().unwrap().event_preview(update);
        }
    }

    fn render(&mut self, _: &mut RenderUpdates, _: &mut RenderUpdates) {
        if let Some(tree) = self.pending_render.take() {
            self.on_info_tree_update(tree);
        } else {
            // update visibility or enabled commands, they may have changed if the `spatial_frame_id` changed.
            let focus = FOCUS_SV.read();
            let mut invalidated_cmds_or_focused = None;

            if let Some(f) = &focus.focused {
                let w_id = f.path.window_id();
                if let Ok(tree) = WINDOWS.widget_tree(w_id)
                    && focus.enabled_nav.needs_refresh(&tree)
                {
                    invalidated_cmds_or_focused = Some(tree);
                }
            }

            if let Some(tree) = invalidated_cmds_or_focused {
                drop(focus);
                self.on_info_tree_update(tree);
            }
        }
    }

    fn event(&mut self, update: &mut EventUpdate) {
        let mut request = None;

        if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
            if args.is_mouse_down() {
                // click
                request = Some(FocusRequest::direct_or_exit(args.target.widget_id(), true, false));
            }
        } else if let Some(args) = TOUCH_INPUT_EVENT.on(update) {
            if args.is_touch_start() {
                // start
                request = Some(FocusRequest::direct_or_exit(args.target.widget_id(), true, false));
            }
        } else if let Some(args) = ACCESS_CLICK_EVENT.on(update) {
            // click
            request = Some(FocusRequest::direct_or_exit(args.widget_id, true, false));
        } else if let Some(args) = WINDOW_FOCUS_CHANGED_EVENT.on(update) {
            // foreground window maybe changed
            let mut focus = FOCUS_SV.write();
            if args.new_focus.is_some()
                && let Some(pending) = focus.pending_window_focus.take()
                && args.is_focus(pending.window)
            {
                request = Some(FocusRequest::direct_or_related(
                    pending.target,
                    pending.nav_origin.is_some(),
                    pending.highlight,
                ));
            }
            if request.is_none()
                && let Some(args) = focus.continue_focus()
            {
                self.notify(&mut focus, Some(args));
            }

            if let Some(window_id) = args.closed() {
                for args in focus.cleanup_returns_win_closed(window_id) {
                    RETURN_FOCUS_CHANGED_EVENT.notify(args);
                }
            }
        } else if let Some(args) = RAW_KEY_INPUT_EVENT.on(update) {
            FOCUS_SV.write().last_keyboard_event = args.timestamp;
        }

        if let Some(request) = request {
            let mut focus = FOCUS_SV.write();
            if !matches!(&focus.request, PendingFocusRequest::Update(_)) {
                focus.request = PendingFocusRequest::None;
                focus.pending_highlight = false;
                focus.pending_window_focus = None;
                let args = focus.fulfill_request(request, false);
                self.notify(&mut focus, args);
            }
        }
    }

    fn update(&mut self) {
        let mut focus = FOCUS_SV.write();
        if let Some((request, is_retry)) = focus.request.take_update() {
            focus.pending_highlight = false;
            let args = focus.fulfill_request(request, is_retry);
            self.notify(&mut focus, args);
        } else if mem::take(&mut focus.pending_highlight) {
            let args = focus.continue_focus_highlight(true);
            self.notify(&mut focus, args);
        }

        if let Some(wgt_id) = focus.navigation_origin_var.get_new()
            && wgt_id != focus.navigation_origin
        {
            focus.navigation_origin = wgt_id;
            focus.update_enabled_nav_with_origin();
            let commands = self.commands.as_mut().unwrap();
            commands.update_enabled(focus.enabled_nav.nav);
        }
    }

    fn info(&mut self, _: &mut InfoUpdates) {
        let mut focus = FOCUS_SV.write();
        if let Some(r) = focus.request.take_info() {
            focus.request = PendingFocusRequest::RetryUpdate(r);
            UPDATES.update(None);
        }
    }
}
impl FocusManager {
    fn on_info_tree_update(&mut self, tree: WidgetInfoTree) {
        let mut focus = FOCUS_SV.write();
        let focus = &mut *focus;
        focus.update_focused_center();

        // widget tree rebuilt or visibility may have changed, check if focus is still valid
        let args = focus.continue_focus();
        self.notify(focus, args);

        // cleanup return focuses.
        for args in focus.cleanup_returns(FocusInfoTree::new(
            tree,
            focus.focus_disabled_widgets.get(),
            focus.focus_hidden_widgets.get(),
        )) {
            RETURN_FOCUS_CHANGED_EVENT.notify(args);
        }
    }

    fn notify(&mut self, focus: &mut FocusService, args: Option<FocusChangedArgs>) {
        if let Some(mut args) = args {
            if !args.highlight && args.new_focus.is_some() && focus.auto_highlight(args.timestamp) {
                args.highlight = true;
                focus.is_highlighting = true;
                focus.is_highlighting_var.set(true);
            }

            // reentering single child of parent scope that cycles
            let is_tab_cycle_reentry = matches!(args.cause.request_target(), Some(FocusTarget::Prev | FocusTarget::Next))
                && match (&args.prev_focus, &args.new_focus) {
                    (Some(p), Some(n)) => p.contains(n.widget_id()),
                    _ => false,
                };

            let reverse = matches!(args.cause.request_target(), Some(FocusTarget::Prev));
            let prev_focus = args.prev_focus.clone();
            FOCUS_CHANGED_EVENT.notify(args);

            // may have focused scope.
            while let Some(after_args) = focus.move_after_focus(is_tab_cycle_reentry, reverse) {
                FOCUS_CHANGED_EVENT.notify(after_args);
            }

            for return_args in focus.update_returns(prev_focus) {
                RETURN_FOCUS_CHANGED_EVENT.notify(return_args);
            }
        }

        let commands = self.commands.as_mut().unwrap();
        commands.update_enabled(focus.enabled_nav.nav);
    }
}

app_local! {
    static FOCUS_SV: FocusService = FocusService::new();
}

/// Keyboard focus service.
///
/// # Provider
///
/// This service is provided by the [`FocusManager`] extension.
pub struct FOCUS;
impl FOCUS {
    /// If set to a duration, starts highlighting focus when a focus change happen within the duration of
    /// a keyboard input event.
    ///
    /// Default is `300.ms()`.
    #[must_use]
    pub fn auto_highlight(&self) -> Var<Option<Duration>> {
        FOCUS_SV.read().auto_highlight.clone()
    }

    /// If [`DISABLED`] widgets can receive focus.
    ///
    /// This is `true` by default, allowing disabled widgets to receive focus can provide a better experience for users,
    /// as the keyboard navigation stays the same, this is also of special interest for accessibility users, screen readers
    /// tend to only vocalize the focused content.
    ///
    /// Widgets should use a different *focused* visual for disabled focus, it must be clear that the widget has the keyboard focus
    /// only as a navigation waypoint and cannot provide its normal function.
    ///
    /// [`DISABLED`]: zng_app::widget::info::Interactivity::DISABLED
    #[must_use]
    pub fn focus_disabled_widgets(&self) -> Var<bool> {
        FOCUS_SV.read().focus_disabled_widgets.clone()
    }

    /// If [`Hidden`] widgets can receive focus.
    ///
    /// This is `true` by default, with the expectation that hidden widgets are made visible once they receive focus, this is
    /// particularly important to enable auto-scrolling to view, as widgets inside scroll regions that are far away from the
    /// viewport are auto-hidden.
    ///
    /// Note that widgets can be explicitly made not focusable, so you can disable focus and hide a widget without needing to
    /// disable this feature globally. Note also that this feature does not apply to collapsed widgets.
    ///
    /// [`Hidden`]: zng_app::widget::info::Visibility::Hidden
    #[must_use]
    pub fn focus_hidden_widgets(&self) -> Var<bool> {
        FOCUS_SV.read().focus_hidden_widgets.clone()
    }

    /// Override the starting point of the next focus move.
    ///
    /// Focus requests that move the focus relative to the current focus will move from this widget instead
    /// if it is found in the focused window. This widget does not need to be focusable.
    ///
    /// The variable is cleared every time the focus is moved. Auto focus by click or touch also sets the
    /// navigation origin if the clicked widget is not focusable.
    ///
    /// If not set the [`focused`] widget is the origin.
    ///
    /// [`focused`]: Self::focused
    #[must_use]
    pub fn navigation_origin(&self) -> Var<Option<WidgetId>> {
        FOCUS_SV.read().navigation_origin_var.clone()
    }

    /// Current focused widget.
    #[must_use]
    pub fn focused(&self) -> Var<Option<InteractionPath>> {
        FOCUS_SV.read().focused_var.read_only()
    }

    /// Current return focus of a scope.
    #[must_use]
    pub fn return_focused(&self, scope_id: WidgetId) -> Var<Option<InteractionPath>> {
        FOCUS_SV
            .write()
            .return_focused_var
            .entry(scope_id)
            .or_insert_with(|| var(None))
            .read_only()
    }

    /// If the [`focused`] path is in the given `window_id`.
    ///
    /// [`focused`]: Self::focused
    pub fn is_window_focused(&self, window_id: WindowId) -> Var<bool> {
        self.focused().map(move |p| matches!(p, Some(p) if p.window_id() == window_id))
    }

    /// If the [`focused`] path contains the given `widget_id`.
    ///
    /// [`focused`]: Self::focused
    pub fn is_focus_within(&self, widget_id: WidgetId) -> Var<bool> {
        self.focused().map(move |p| matches!(p, Some(p) if p.contains(widget_id)))
    }

    /// If the [`focused`] path is to the given `widget_id`.
    ///
    /// [`focused`]: Self::focused
    pub fn is_focused(&self, widget_id: WidgetId) -> Var<bool> {
        self.focused().map(move |p| matches!(p, Some(p) if p.widget_id() == widget_id))
    }

    /// Current ALT return focus.
    #[must_use]
    pub fn alt_return(&self) -> Var<Option<InteractionPath>> {
        FOCUS_SV.read().alt_return_var.read_only()
    }

    /// If focus is in an ALT scope.
    #[must_use]
    pub fn in_alt(&self) -> Var<bool> {
        FOCUS_SV.read().alt_return_var.map(|p| p.is_some())
    }

    /// If the current focused widget is visually indicated.
    #[must_use]
    pub fn is_highlighting(&self) -> Var<bool> {
        FOCUS_SV.read().is_highlighting_var.read_only()
    }

    /// Request a focus update.
    ///
    /// All other focus request methods call this method.
    pub fn focus(&self, mut request: FocusRequest) {
        let mut f = FOCUS_SV.write();
        if !request.highlight && f.auto_highlight(INSTANT.now()) {
            request.highlight = true;
        }
        f.pending_window_focus = None;
        f.request = PendingFocusRequest::Update(request);
        UPDATES.update(None);
    }

    /// Enables focus highlight for the current focus if the key-press allows it.
    fn on_disabled_cmd(&self) {
        let f = FOCUS_SV.read();
        if f.auto_highlight.get().is_some() && !f.is_highlighting {
            drop(f);
            self.highlight();
        }
    }

    /// Schedules enabling of [`is_highlighting`] for next update.
    ///
    /// [`is_highlighting`]: Self::is_highlighting
    pub fn highlight(&self) {
        let mut f = FOCUS_SV.write();
        f.pending_highlight = true;
        UPDATES.update(None);
    }

    /// Focus the widget if it is focusable and change the highlight.
    ///
    /// If the widget is not focusable the focus does not move, in this case the highlight changes
    /// for the current focused widget.
    ///
    /// If the widget is in a window that does not have focus, but is open and not minimized and the app
    /// has keyboard focus in another window; the window is focused and the request is processed when the focus event is received.
    /// The [`FocusRequest`] type has other more advanced window focus configurations.
    ///
    /// This makes a [`focus`](Self::focus) request using [`FocusRequest::direct`].
    pub fn focus_widget(&self, widget_id: impl Into<WidgetId>, highlight: bool) {
        self.focus(FocusRequest::direct(widget_id.into(), highlight));
    }

    /// Focus the widget if it is focusable, else focus the first focusable parent, also changes the highlight.
    ///
    /// If the widget and no parent are focusable the focus does not move, in this case the highlight changes
    /// for the current focused widget.
    ///
    /// If `navigation_origin` is `true` the `target` becomes the [`navigation_origin`] when the first focusable ancestor
    /// is focused because the `target` is not focusable.
    ///
    /// This makes a [`focus`](Self::focus) request using [`FocusRequest::direct_or_exit`].
    ///
    /// [`navigation_origin`]: FOCUS::navigation_origin
    pub fn focus_widget_or_exit(&self, widget_id: impl Into<WidgetId>, navigation_origin: bool, highlight: bool) {
        self.focus(FocusRequest::direct_or_exit(widget_id.into(), navigation_origin, highlight));
    }

    /// Focus the widget if it is focusable, else focus the first focusable descendant, also changes the highlight.
    ///
    /// If the widget and no child are focusable the focus does not move, in this case the highlight changes for
    /// the current focused widget.
    ///
    /// If `navigation_origin` is `true` the `target` becomes the [`navigation_origin`] when the first focusable descendant
    /// is focused because the `target` is not focusable.
    ///
    /// This makes a [`focus`](Self::focus) request [`FocusRequest::direct_or_enter`].
    ///
    /// [`navigation_origin`]: FOCUS::navigation_origin
    pub fn focus_widget_or_enter(&self, widget_id: impl Into<WidgetId>, navigation_origin: bool, highlight: bool) {
        self.focus(FocusRequest::direct_or_enter(widget_id.into(), navigation_origin, highlight));
    }

    /// Focus the widget if it is focusable, else focus the first focusable descendant, else focus the first
    /// focusable ancestor.
    ///
    /// If the widget no focusable widget is found the focus does not move, in this case the highlight changes
    /// for the current focused widget.
    ///
    /// If `navigation_origin` is `true` the `target` becomes the [`navigation_origin`] when the first focusable relative
    /// is focused because the `target` is not focusable.
    ///
    /// This makes a [`focus`](Self::focus) request using [`FocusRequest::direct_or_related`].
    ///
    /// [`navigation_origin`]: FOCUS::navigation_origin
    pub fn focus_widget_or_related(&self, widget_id: impl Into<WidgetId>, navigation_origin: bool, highlight: bool) {
        self.focus(FocusRequest::direct_or_related(widget_id.into(), navigation_origin, highlight));
    }

    /// Focus the first logical descendant that is focusable from the navigation origin or the current focus.
    ///
    /// Does nothing if no origin or focus is set. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::enter`].
    pub fn focus_enter(&self) {
        let req = FocusRequest::enter(FOCUS_SV.read().is_highlighting);
        self.focus(req);
    }

    /// Focus the first logical ancestor that is focusable from the navigation origin or the current focus
    /// or the return focus from ALT scopes.
    ///
    /// Does nothing if no origin or focus is set. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::exit`].
    pub fn focus_exit(&self) {
        let req = FocusRequest::exit(FOCUS_SV.read().is_highlighting);
        self.focus(req)
    }

    /// Focus the logical next widget from the navigation origin or the current focus.
    ///
    /// Does nothing if no origin of focus is set. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::next`].
    pub fn focus_next(&self) {
        let req = FocusRequest::next(FOCUS_SV.read().is_highlighting);
        self.focus(req);
    }

    /// Focus the logical previous widget from the navigation origin or the current focus.
    ///
    /// Does nothing if no origin or focus is set. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::prev`].
    pub fn focus_prev(&self) {
        let req = FocusRequest::prev(FOCUS_SV.read().is_highlighting);
        self.focus(req);
    }

    /// Focus the nearest upward widget from the navigation origin or the current focus.
    ///
    /// Does nothing if no origin or focus is set. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::up`].
    pub fn focus_up(&self) {
        let req = FocusRequest::up(FOCUS_SV.read().is_highlighting);
        self.focus(req);
    }

    /// Focus the nearest widget to the right of the navigation origin or the current focus.
    ///
    /// Does nothing if no origin or focus is set. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::right`].
    pub fn focus_right(&self) {
        let req = FocusRequest::right(FOCUS_SV.read().is_highlighting);
        self.focus(req);
    }

    /// Focus the nearest downward widget from the navigation origin or the current focus.
    ///
    /// Does nothing if no origin or focus is set. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::down`].
    pub fn focus_down(&self) {
        let req = FocusRequest::down(FOCUS_SV.read().is_highlighting);
        self.focus(req);
    }

    /// Focus the nearest widget to the left of the navigation origin or the current focus.
    ///
    /// Does nothing if no origin or focus is set. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::left`].
    pub fn focus_left(&self) {
        let req = FocusRequest::left(FOCUS_SV.read().is_highlighting);
        self.focus(req);
    }

    /// Focus the ALT scope from the navigation origin or the current focus or escapes the current ALT scope.
    ///
    /// Does nothing if no origin or focus is set. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::alt`].
    pub fn focus_alt(&self) {
        let req = FocusRequest::alt(FOCUS_SV.read().is_highlighting);
        self.focus(req);
    }
}

enum PendingFocusRequest {
    None,
    InfoRetry(FocusRequest, DInstant),
    Update(FocusRequest),
    RetryUpdate(FocusRequest),
}
impl PendingFocusRequest {
    fn take_update(&mut self) -> Option<(FocusRequest, bool)> {
        match mem::replace(self, PendingFocusRequest::None) {
            PendingFocusRequest::Update(r) => Some((r, false)),
            PendingFocusRequest::RetryUpdate(r) => Some((r, true)),
            r => {
                *self = r;
                None
            }
        }
    }
    fn take_info(&mut self) -> Option<FocusRequest> {
        match mem::replace(self, PendingFocusRequest::None) {
            PendingFocusRequest::InfoRetry(r, i) => {
                if i.elapsed() < 100.ms() {
                    Some(r)
                } else {
                    None
                }
            }
            r => {
                *self = r;
                None
            }
        }
    }
}

struct PendingWindowFocus {
    window: WindowId,
    target: WidgetId,
    highlight: bool,
    nav_origin: Option<WidgetId>,
}

struct FocusService {
    auto_highlight: Var<Option<Duration>>,
    last_keyboard_event: DInstant,

    focus_disabled_widgets: Var<bool>,
    focus_hidden_widgets: Var<bool>,

    request: PendingFocusRequest,

    focused_var: Var<Option<InteractionPath>>,
    focused: Option<FocusedInfo>,
    navigation_origin_var: Var<Option<WidgetId>>,
    navigation_origin: Option<WidgetId>,

    return_focused_var: IdMap<WidgetId, Var<Option<InteractionPath>>>,
    return_focused: IdMap<WidgetId, InteractionPath>,

    alt_return_var: Var<Option<InteractionPath>>,
    alt_return: Option<(InteractionPath, InteractionPath)>,

    is_highlighting_var: Var<bool>,
    is_highlighting: bool,

    enabled_nav: EnabledNavWithFrame,

    pending_window_focus: Option<PendingWindowFocus>,
    pending_highlight: bool,
}
impl FocusService {
    #[must_use]
    fn new() -> Self {
        Self {
            auto_highlight: var(Some(300.ms())),
            last_keyboard_event: DInstant::EPOCH,

            focus_disabled_widgets: var(true),
            focus_hidden_widgets: var(true),

            request: PendingFocusRequest::None,

            focused_var: var(None),
            focused: None,
            navigation_origin_var: var(None),
            navigation_origin: None,

            return_focused_var: IdMap::default(),
            return_focused: IdMap::default(),

            alt_return_var: var(None),
            alt_return: None,

            is_highlighting_var: var(false),
            is_highlighting: false,

            enabled_nav: EnabledNavWithFrame::invalid(),

            pending_window_focus: None,
            pending_highlight: false,
        }
    }

    fn auto_highlight(&self, timestamp: DInstant) -> bool {
        if let Some(dur) = self.auto_highlight.get()
            && timestamp.duration_since(self.last_keyboard_event) <= dur
        {
            return true;
        }
        false
    }

    fn update_enabled_nav_with_origin(&mut self) {
        let mut origin = self
            .focused
            .as_ref()
            .and_then(|f| WINDOWS.widget_tree(f.path.window_id()).ok()?.get(f.path.widget_id()));
        if let Some(id) = self.navigation_origin
            && let Some(focused) = &origin
            && let Some(o) = focused.tree().get(id)
        {
            origin = Some(o);
        }

        if let Some(o) = origin {
            let o = o.into_focus_info(self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get());
            self.enabled_nav = o.enabled_nav_with_frame();
        } else {
            self.enabled_nav.nav = FocusNavAction::empty();
        }
    }

    #[must_use]
    fn fulfill_request(&mut self, request: FocusRequest, is_info_retry: bool) -> Option<FocusChangedArgs> {
        match request.target {
            FocusTarget::Direct { target } => self.focus_direct(target, false, request.highlight, false, false, request),
            FocusTarget::DirectOrExit { target, navigation_origin } => {
                self.focus_direct(target, navigation_origin, request.highlight, false, true, request)
            }
            FocusTarget::DirectOrEnter { target, navigation_origin } => {
                self.focus_direct(target, navigation_origin, request.highlight, true, false, request)
            }
            FocusTarget::DirectOrRelated { target, navigation_origin } => {
                self.focus_direct(target, navigation_origin, request.highlight, true, true, request)
            }
            move_ => {
                let origin;
                let origin_tree;
                if let Some(o) = self.navigation_origin_var.get() {
                    origin = Some(o);
                    origin_tree = WINDOWS.focused_info();
                    self.navigation_origin_var.set(None);
                    self.navigation_origin = None;
                } else if let Some(prev) = &self.focused {
                    origin = Some(prev.path.widget_id());
                    origin_tree = WINDOWS.widget_tree(prev.path.window_id()).ok();
                } else {
                    origin = None;
                    origin_tree = None;
                }

                if let Some(info) = origin_tree
                    && let Some(origin) = origin
                {
                    if let Some(w) = info.get(origin) {
                        let w = w.into_focus_info(self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get());
                        if let Some(new_focus) = match move_ {
                            // tabular
                            FocusTarget::Next => w.next_tab(false),
                            FocusTarget::Prev => w.prev_tab(false),
                            FocusTarget::Enter => w.first_tab_descendant(),
                            FocusTarget::Exit => {
                                if self.alt_return.is_some() && (w.is_alt_scope() || w.ancestors().any(|w| w.is_alt_scope())) {
                                    self.new_focus_for_alt_exit(w, is_info_retry, request.highlight)
                                } else {
                                    w.ancestors().next()
                                }
                            }
                            // directional
                            FocusTarget::Up => w.next_up(),
                            FocusTarget::Right => w.next_right(),
                            FocusTarget::Down => w.next_down(),
                            FocusTarget::Left => w.next_left(),
                            // alt
                            FocusTarget::Alt => {
                                if let Some(alt) = w.alt_scope() {
                                    Some(alt)
                                } else if self.alt_return.is_some() {
                                    // Alt toggles when there is no alt scope.

                                    self.new_focus_for_alt_exit(w, is_info_retry, request.highlight)
                                } else {
                                    None
                                }
                            }
                            // cases covered by parent match
                            FocusTarget::Direct { .. }
                            | FocusTarget::DirectOrExit { .. }
                            | FocusTarget::DirectOrEnter { .. }
                            | FocusTarget::DirectOrRelated { .. } => {
                                unreachable!()
                            }
                        } {
                            // found `new_focus`
                            self.enabled_nav = new_focus.enabled_nav_with_frame();
                            self.move_focus(
                                Some(FocusedInfo::new(new_focus)),
                                None,
                                request.highlight,
                                FocusChangedCause::Request(request),
                            )
                        } else {
                            // no `new_focus`, maybe update highlight and widget path.
                            self.continue_focus_highlight(request.highlight)
                        }
                    } else {
                        // widget not found
                        self.continue_focus_highlight(request.highlight)
                    }
                } else {
                    // window not found
                    self.continue_focus_highlight(request.highlight)
                }
            }
        }
    }

    /// Return focus from the alt scope, handles cases when the return focus is temporarily blocked.
    #[must_use]
    fn new_focus_for_alt_exit(&mut self, prev_w: WidgetFocusInfo, is_info_retry: bool, highlight: bool) -> Option<WidgetFocusInfo> {
        let (_, return_path) = self.alt_return.as_ref().unwrap();

        let return_int = return_path.interactivity();
        let return_id = return_path.widget_id();
        let info = prev_w.focus_tree();

        let r = info.get_or_parent(return_path);
        if let Some(w) = &r
            && w.info().id() != return_id
            && !is_info_retry
            && return_int.is_blocked()
        {
            // blocked return may not have unblocked yet

            if let Some(exists) = info.tree().get(return_id) {
                let exists = exists.into_focus_info(info.focus_disabled_widgets(), info.focus_hidden_widgets());
                if !exists.is_focusable() && exists.info().interactivity().is_blocked() {
                    // Still blocked. A common pattern is to set a `modal` filter on the alt-scope
                    // then remove the modal filter when alt-scope loses focus.
                    //
                    // Here we know that the return focus was blocked after the alt got focus, because
                    // blocked widgets can't before return focus, and we know that we are moving focus
                    // to some `r`. So we setup an info retry, the focus will move to `r` momentarily,
                    // exiting the alt-scope, and if it removes the modal filter the focus will return.
                    self.request =
                        PendingFocusRequest::InfoRetry(FocusRequest::direct_or_related(return_id, false, highlight), INSTANT.now());
                }
            }
        }

        r
    }

    /// Checks if `focused()` is still valid, if not moves focus to nearest valid.
    #[must_use]
    fn continue_focus(&mut self) -> Option<FocusChangedArgs> {
        if let Some(focused) = &self.focused
            && let Ok(true) = WINDOWS.is_focused(focused.path.window_id())
        {
            let info = WINDOWS.widget_tree(focused.path.window_id()).unwrap();
            if let Some(widget) = info
                .get(focused.path.widget_id())
                .map(|w| w.into_focus_info(self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get()))
            {
                if widget.is_focusable() {
                    // :-) probably in the same place, maybe moved inside same window.
                    self.enabled_nav = widget.enabled_nav_with_frame();
                    return self.move_focus(
                        Some(FocusedInfo::new(widget)),
                        self.navigation_origin_var.get(),
                        self.is_highlighting,
                        FocusChangedCause::Recovery,
                    );
                } else {
                    // widget no longer focusable
                    if let Some(parent) = widget.parent() {
                        // move to nearest inside focusable parent, or parent
                        let new_focus = parent.nearest(focused.center, Px::MAX).unwrap_or(parent);
                        self.enabled_nav = new_focus.enabled_nav_with_frame();
                        return self.move_focus(
                            Some(FocusedInfo::new(new_focus)),
                            self.navigation_origin_var.get(),
                            self.is_highlighting,
                            FocusChangedCause::Recovery,
                        );
                    } else {
                        // no focusable parent or root
                        return self.focus_focused_window(self.is_highlighting);
                    }
                }
            } else {
                // widget not found
                for &parent in focused.path.ancestors().iter().rev() {
                    if let Some(parent) = info
                        .get(parent)
                        .and_then(|w| w.into_focusable(self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get()))
                    {
                        // move to nearest inside focusable parent, or parent
                        let new_focus = parent.nearest(focused.center, Px::MAX).unwrap_or(parent);
                        self.enabled_nav = new_focus.enabled_nav_with_frame();
                        return self.move_focus(
                            Some(FocusedInfo::new(new_focus)),
                            self.navigation_origin_var.get(),
                            self.is_highlighting,
                            FocusChangedCause::Recovery,
                        );
                    }
                }
            }
        } // else window not found or not focused
        // else no current focus
        self.focus_focused_window(false)
    }

    #[must_use]
    fn continue_focus_highlight(&mut self, highlight: bool) -> Option<FocusChangedArgs> {
        if let Some(mut args) = self.continue_focus() {
            args.highlight = highlight;
            self.is_highlighting = highlight;
            self.is_highlighting_var.set(highlight);
            Some(args)
        } else if self.is_highlighting != highlight {
            self.is_highlighting = highlight;
            self.is_highlighting_var.set(highlight);
            let focused = self.focused.as_ref().map(|p| p.path.clone());
            Some(FocusChangedArgs::now(
                focused.clone(),
                focused,
                highlight,
                FocusChangedCause::Recovery,
                self.enabled_nav.nav,
            ))
        } else {
            None
        }
    }

    #[must_use]
    fn focus_direct(
        &mut self,
        widget_id: WidgetId,
        navigation_origin: bool,
        highlight: bool,
        fallback_to_children: bool,
        fallback_to_parents: bool,
        request: FocusRequest,
    ) -> Option<FocusChangedArgs> {
        let mut next_origin = None;
        let mut target = None;
        if let Some(w) = WINDOWS
            .widget_trees()
            .iter()
            .find_map(|info| info.get(widget_id))
            .map(|w| w.into_focus_info(self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get()))
        {
            if w.is_focusable() {
                let enable = w.enabled_nav_with_frame();
                target = Some((FocusedInfo::new(w), enable));
            } else if fallback_to_children {
                let enable = if navigation_origin {
                    next_origin = Some(widget_id);
                    Some(w.enabled_nav_with_frame())
                } else {
                    None
                };
                if let Some(w) = w.descendants().next() {
                    let enable = enable.unwrap_or_else(|| w.enabled_nav_with_frame());
                    target = Some((FocusedInfo::new(w), enable));
                }
            } else if fallback_to_parents {
                let enable = if navigation_origin {
                    next_origin = Some(widget_id);
                    Some(w.enabled_nav_with_frame())
                } else {
                    None
                };
                if let Some(w) = w.parent() {
                    let enable = enable.unwrap_or_else(|| w.enabled_nav_with_frame());
                    target = Some((FocusedInfo::new(w), enable));
                }
            }
        }

        if let Some((target, enabled_nav)) = target {
            if let Ok(false) = WINDOWS.is_focused(target.path.window_id()) {
                if request.force_window_focus || WINDOWS.focused_window_id().is_some() {
                    // if can steal focus from other apps or focus is already in another window of the app.
                    WINDOWS.focus(target.path.window_id()).unwrap();
                } else if request.window_indicator.is_some() {
                    // if app does not have focus, focus stealing is not allowed, but a request indicator can be set.
                    WINDOWS
                        .vars(target.path.window_id())
                        .unwrap()
                        .focus_indicator()
                        .set(request.window_indicator);
                }

                // will focus when the window is focused
                self.pending_window_focus = Some(PendingWindowFocus {
                    window: target.path.window_id(),
                    target: target.path.widget_id(),
                    highlight,
                    nav_origin: next_origin,
                });
                self.navigation_origin = next_origin;
                self.navigation_origin_var.set(next_origin);
                None
            } else {
                self.enabled_nav = enabled_nav;
                self.move_focus(Some(target), next_origin, highlight, FocusChangedCause::Request(request))
            }
        } else {
            self.navigation_origin = next_origin;
            self.navigation_origin_var.set(next_origin);
            self.change_highlight(highlight, request)
        }
    }

    #[must_use]
    fn change_highlight(&mut self, highlight: bool, request: FocusRequest) -> Option<FocusChangedArgs> {
        if self.is_highlighting != highlight {
            self.is_highlighting = highlight;
            self.is_highlighting_var.set(highlight);
            let focused = self.focused.as_ref().map(|p| p.path.clone());
            Some(FocusChangedArgs::now(
                focused.clone(),
                focused,
                highlight,
                FocusChangedCause::Request(request),
                self.enabled_nav.nav,
            ))
        } else {
            None
        }
    }

    #[must_use]
    fn focus_focused_window(&mut self, highlight: bool) -> Option<FocusChangedArgs> {
        if let Some(info) = WINDOWS.focused_info() {
            let info = FocusInfoTree::new(info, self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get());
            if let Some(root) = info.focusable_root() {
                // found focused window and it is focusable.
                self.enabled_nav = root.enabled_nav_with_frame();
                self.move_focus(Some(FocusedInfo::new(root)), None, highlight, FocusChangedCause::Recovery)
            } else {
                // has focused window but it is not focusable.
                self.enabled_nav = EnabledNavWithFrame::invalid();
                self.move_focus(None, None, false, FocusChangedCause::Recovery)
            }
        } else {
            // no focused window
            self.enabled_nav = EnabledNavWithFrame::invalid();
            self.move_focus(None, None, false, FocusChangedCause::Recovery)
        }
    }

    #[must_use]
    fn move_focus(
        &mut self,
        new_focus: Option<FocusedInfo>,
        new_origin: Option<WidgetId>,
        highlight: bool,
        cause: FocusChangedCause,
    ) -> Option<FocusChangedArgs> {
        let prev_highlight = std::mem::replace(&mut self.is_highlighting, highlight);
        self.is_highlighting_var.set(highlight);

        self.navigation_origin = new_origin;
        if self.navigation_origin_var.get() != new_origin {
            self.navigation_origin_var.set(new_origin);
        }

        let r = if self.focused.as_ref().map(|p| &p.path) != new_focus.as_ref().map(|p| &p.path) {
            let new_focus = new_focus.as_ref().map(|p| p.path.clone());
            let args = FocusChangedArgs::now(
                self.focused.take().map(|p| p.path),
                new_focus.clone(),
                self.is_highlighting,
                cause,
                self.enabled_nav.nav,
            );
            self.focused_var.set(new_focus);
            Some(args)
        } else if prev_highlight != highlight {
            let new_focus = new_focus.as_ref().map(|p| p.path.clone());
            Some(FocusChangedArgs::now(
                new_focus.clone(),
                new_focus,
                highlight,
                cause,
                self.enabled_nav.nav,
            ))
        } else {
            None
        };

        // can be just a center update.
        self.focused = new_focus;

        r
    }

    #[must_use]
    fn move_after_focus(&mut self, is_tab_cycle_reentry: bool, reverse: bool) -> Option<FocusChangedArgs> {
        if let Some(focused) = &self.focused
            && let Some(info) = WINDOWS.focused_info()
            && let Some(widget) =
                FocusInfoTree::new(info, self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get()).get(focused.path.widget_id())
        {
            if let Some(nested) = widget.nested_window() {
                tracing::debug!("focus nested window {nested:?}");
                let _ = WINDOWS.focus(nested);
            } else if widget.is_scope() {
                let last_focused = |id| self.return_focused.get(&id).map(|p| p.as_path());
                if let Some(widget) = widget.on_focus_scope_move(last_focused, is_tab_cycle_reentry, reverse) {
                    self.enabled_nav = widget.enabled_nav_with_frame();
                    return self.move_focus(
                        Some(FocusedInfo::new(widget)),
                        self.navigation_origin,
                        self.is_highlighting,
                        FocusChangedCause::ScopeGotFocus(reverse),
                    );
                }
            }
        }
        None
    }

    /// Updates `return_focused` and `alt_return` after `focused` changed.
    #[must_use]
    fn update_returns(&mut self, prev_focus: Option<InteractionPath>) -> Vec<ReturnFocusChangedArgs> {
        let mut r = vec![];

        if let Some((scope, _)) = &mut self.alt_return {
            // if we have an `alt_return` check if is still inside an ALT.

            let mut retain_alt = false;
            if let Some(new_focus) = &self.focused {
                if let Some(s) = new_focus.path.ancestor_path(scope.widget_id()) {
                    retain_alt = true; // just a focus move inside the ALT.
                    *scope = s.into_owned();
                } else if let Ok(info) = WINDOWS.widget_tree(new_focus.path.window_id())
                    && let Some(widget) = FocusInfoTree::new(info, self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get())
                        .get(new_focus.path.widget_id())
                {
                    let alt_scope = if widget.is_alt_scope() {
                        Some(widget)
                    } else {
                        widget.scopes().find(|s| s.is_alt_scope())
                    };

                    if let Some(alt_scope) = alt_scope {
                        // entered another ALT
                        retain_alt = true;
                        *scope = alt_scope.info().interaction_path();
                    }
                }
            }

            if !retain_alt {
                let (scope, widget_path) = self.alt_return.take().unwrap();
                self.alt_return_var.set(None);
                r.push(ReturnFocusChangedArgs::now(scope, Some(widget_path), None));
            }
        } else if let Some(new_focus) = &self.focused {
            // if we don't have an `alt_return` but focused something, check if focus
            // moved inside an ALT.

            if let Ok(info) = WINDOWS.widget_tree(new_focus.path.window_id())
                && let Some(widget) = FocusInfoTree::new(info, self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get())
                    .get(new_focus.path.widget_id())
            {
                let alt_scope = if widget.is_alt_scope() {
                    Some(widget)
                } else {
                    widget.scopes().find(|s| s.is_alt_scope())
                };
                if let Some(alt_scope) = alt_scope {
                    let scope = alt_scope.info().interaction_path();
                    // entered an alt_scope.

                    if let Some(prev) = &prev_focus {
                        // previous focus is the return.
                        r.push(ReturnFocusChangedArgs::now(scope.clone(), None, Some(prev.clone())));
                        self.alt_return = Some((scope, prev.clone()));
                        self.alt_return_var.set(prev.clone());
                    } else if let Some(parent) = alt_scope.parent() {
                        // no previous focus, ALT parent is the return.
                        let parent_path = parent.info().interaction_path();
                        r.push(ReturnFocusChangedArgs::now(scope.clone(), None, Some(parent_path.clone())));
                        self.alt_return = Some((scope, parent_path.clone()));
                        self.alt_return_var.set(parent_path);
                    }
                }
            }
        }

        /*
         *   Update `return_focused`
         */

        if let Some(new_focus) = &self.focused
            && let Ok(info) = WINDOWS.widget_tree(new_focus.path.window_id())
            && let Some(widget) =
                FocusInfoTree::new(info, self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get()).get(new_focus.path.widget_id())
            && !widget.is_alt_scope()
            && widget.scopes().all(|s| !s.is_alt_scope())
        {
            // if not inside ALT, update return for each LastFocused parent scopes.

            for scope in widget
                .scopes()
                .filter(|s| s.focus_info().scope_on_focus() == FocusScopeOnFocus::LastFocused)
            {
                let scope = scope.info().interaction_path();
                let path = widget.info().interaction_path();
                if let Some(current) = self.return_focused.get_mut(&scope.widget_id()) {
                    if current != &path {
                        let prev = std::mem::replace(current, path);
                        self.return_focused_var.get(&scope.widget_id()).unwrap().set(current.clone());
                        r.push(ReturnFocusChangedArgs::now(scope, Some(prev), Some(current.clone())));
                    }
                } else {
                    self.return_focused.insert(scope.widget_id(), path.clone());
                    match self.return_focused_var.entry(scope.widget_id()) {
                        IdEntry::Occupied(e) => e.get().set(Some(path.clone())),
                        IdEntry::Vacant(e) => {
                            e.insert(var(Some(path.clone())));
                        }
                    }
                    r.push(ReturnFocusChangedArgs::now(scope, None, Some(path)));
                }
            }
        }

        r
    }

    /// Cleanup `return_focused` and `alt_return` after new widget tree.
    #[must_use]
    fn cleanup_returns(&mut self, info: FocusInfoTree) -> Vec<ReturnFocusChangedArgs> {
        let mut r = vec![];

        if self.return_focused_var.len() > 20 {
            self.return_focused_var
                .retain(|_, var| var.strong_count() > 1 || var.with(Option::is_some))
        }

        self.return_focused.retain(|&scope_id, widget_path| {
            if widget_path.window_id() != info.tree().window_id() {
                return true; // retain, not same window.
            }

            let mut retain = false;

            if let Some(widget) = info.tree().get(widget_path.widget_id()) {
                if let Some(scope) = widget
                    .clone()
                    .into_focus_info(info.focus_disabled_widgets(), info.focus_hidden_widgets())
                    .scopes()
                    .find(|s| s.info().id() == scope_id)
                {
                    if scope.focus_info().scope_on_focus() == FocusScopeOnFocus::LastFocused {
                        retain = true; // retain, widget still exists in same scope and scope still is LastFocused.

                        let path = widget.interaction_path();
                        if &path != widget_path {
                            // widget moved inside scope.
                            r.push(ReturnFocusChangedArgs::now(
                                scope.info().interaction_path(),
                                Some(widget_path.clone()),
                                Some(path.clone()),
                            ));
                            *widget_path = path;
                        }
                    }
                } else if let Some(scope) = info.get(scope_id)
                    && scope.focus_info().scope_on_focus() == FocusScopeOnFocus::LastFocused
                {
                    // widget not inside scope anymore, but scope still exists and is valid.
                    if let Some(first) = scope.first_tab_descendant() {
                        // LastFocused goes to the first descendant as fallback.
                        retain = true;

                        let path = first.info().interaction_path();
                        r.push(ReturnFocusChangedArgs::now(
                            scope.info().interaction_path(),
                            Some(widget_path.clone()),
                            Some(path.clone()),
                        ));
                        *widget_path = path;
                    }
                }
            } else if let Some(parent) = info.get_or_parent(widget_path) {
                // widget not in window anymore, but a focusable parent is..
                if let Some(scope) = parent.scopes().find(|s| s.info().id() == scope_id)
                    && scope.focus_info().scope_on_focus() == FocusScopeOnFocus::LastFocused
                {
                    // ..and the parent is inside the scope, and the scope is still valid.
                    retain = true;

                    let path = parent.info().interaction_path();
                    r.push(ReturnFocusChangedArgs::now(
                        scope.info().interaction_path(),
                        Some(widget_path.clone()),
                        Some(path.clone()),
                    ));
                    *widget_path = path;
                }
            }

            if !retain {
                let scope_path = info.get(scope_id).map(|i| i.info().interaction_path());

                if scope_path.is_some() {
                    match self.return_focused_var.entry(scope_id) {
                        IdEntry::Occupied(e) => {
                            if e.get().strong_count() == 1 {
                                e.remove();
                            } else {
                                e.get().set(None);
                            }
                        }
                        IdEntry::Vacant(_) => {}
                    }
                } else if let Some(var) = self.return_focused_var.remove(&scope_id)
                    && var.strong_count() > 1
                {
                    var.set(None);
                }

                r.push(ReturnFocusChangedArgs::now(scope_path, Some(widget_path.clone()), None));
            }
            retain
        });

        let mut retain_alt = true;
        if let Some((scope, widget_path)) = &mut self.alt_return
            && widget_path.window_id() == info.tree().window_id()
        {
            // we need to update alt_return

            retain_alt = false; // will retain only if still valid

            if let Some(widget) = info.tree().get(widget_path.widget_id()) {
                if !widget
                    .clone()
                    .into_focus_info(info.focus_disabled_widgets(), info.focus_hidden_widgets())
                    .scopes()
                    .any(|s| s.info().id() == scope.widget_id())
                {
                    retain_alt = true; // retain, widget still exists outside of the ALT scope.

                    let path = widget.interaction_path();
                    if &path != widget_path {
                        // widget moved without entering the ALT scope.
                        r.push(ReturnFocusChangedArgs::now(
                            scope.clone(),
                            Some(widget_path.clone()),
                            Some(path.clone()),
                        ));
                        *widget_path = path;
                    }
                }
            } else if let Some(parent) = info.get_or_parent(widget_path) {
                // widget not in window anymore, but a focusable parent is..
                if !parent.scopes().any(|s| s.info().id() == scope.widget_id()) {
                    // ..and the parent is not inside the ALT scope.
                    retain_alt = true;

                    let path = parent.info().interaction_path();
                    r.push(ReturnFocusChangedArgs::now(
                        scope.clone(),
                        Some(widget_path.clone()),
                        Some(path.clone()),
                    ));
                    *widget_path = path.clone();
                    self.alt_return_var.set(path);
                }
            }
        }
        if !retain_alt {
            let (scope_id, widget_path) = self.alt_return.take().unwrap();
            self.alt_return_var.set(None);
            r.push(ReturnFocusChangedArgs::now(scope_id, Some(widget_path), None));
        }

        r
    }

    /// Cleanup `return_focused` and `alt_return` after a window closed.
    #[must_use]
    fn cleanup_returns_win_closed(&mut self, window_id: WindowId) -> Vec<ReturnFocusChangedArgs> {
        let mut r = vec![];

        if self
            .alt_return
            .as_ref()
            .map(|(_, w)| w.window_id() == window_id)
            .unwrap_or_default()
        {
            let (_, widget_path) = self.alt_return.take().unwrap();
            self.alt_return_var.set(None);
            r.push(ReturnFocusChangedArgs::now(None, Some(widget_path), None));
        }

        self.return_focused.retain(|&scope_id, widget_path| {
            let retain = widget_path.window_id() != window_id;

            if !retain {
                let var = self.return_focused_var.remove(&scope_id).unwrap();
                var.set(None);

                r.push(ReturnFocusChangedArgs::now(None, Some(widget_path.clone()), None));
            }

            retain
        });

        r
    }

    fn update_focused_center(&mut self) {
        if let Some(f) = &mut self.focused {
            let bounds = f.bounds_info.inner_bounds();
            if bounds != PxRect::zero() {
                f.center = bounds.center();
            }
        }
    }
}

#[derive(Debug)]
struct FocusedInfo {
    path: InteractionPath,
    bounds_info: WidgetBoundsInfo,
    center: PxPoint,
}
impl FocusedInfo {
    pub fn new(focusable: WidgetFocusInfo) -> Self {
        FocusedInfo {
            path: focusable.info().interaction_path(),
            bounds_info: focusable.info().bounds_info(),
            center: focusable.info().center(),
        }
    }
}

struct EnabledNavWithFrame {
    nav: FocusNavAction,
    spatial_frame_id: FrameId,
    visibility_id: FrameId,
}
impl EnabledNavWithFrame {
    fn invalid() -> Self {
        Self {
            nav: FocusNavAction::empty(),
            spatial_frame_id: FrameId::INVALID,
            visibility_id: FrameId::INVALID,
        }
    }
    fn needs_refresh(&self, tree: &WidgetInfoTree) -> bool {
        let stats = tree.stats();
        stats.bounds_updated_frame != self.spatial_frame_id || stats.vis_updated_frame != self.visibility_id
    }
}
trait EnabledNavWithFrameExt {
    fn enabled_nav_with_frame(&self) -> EnabledNavWithFrame;
}
impl EnabledNavWithFrameExt for WidgetFocusInfo {
    fn enabled_nav_with_frame(&self) -> EnabledNavWithFrame {
        let stats = self.info().tree().stats();
        EnabledNavWithFrame {
            nav: self.enabled_nav(),
            spatial_frame_id: stats.bounds_updated_frame,
            visibility_id: stats.vis_updated_frame,
        }
    }
}
