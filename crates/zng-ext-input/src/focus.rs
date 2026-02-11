//! Keyboard focus manager.
//!
//! # Events
//!
//! Events this extension provides.
//!
//! * [`FOCUS_CHANGED_EVENT`]
//! * [`RETURN_FOCUS_CHANGED_EVENT`]
//!
//! # Services
//!
//! Services this extension provides.
//!
//! * [`FOCUS`]

pub mod cmd;
pub mod iter;

mod focus_info;
pub use focus_info::*;
use zng_unique_id::{IdEntry, IdMap};

use std::{mem, time::Duration};
use zng_app::{
    APP, DInstant,
    access::{ACCESS_CLICK_EVENT, ACCESS_FOCUS_EVENT, ACCESS_FOCUS_NAV_ORIGIN_EVENT},
    event::event,
    event_args, hn,
    update::UPDATES,
    view_process::raw_events::RAW_KEY_INPUT_EVENT,
    widget::{
        WidgetId,
        info::{InteractionPath, WIDGET_TREE_CHANGED_EVENT, WidgetInfoTree},
    },
    window::WindowId,
};
use zng_app_context::app_local;
use zng_ext_window::{FocusIndicator, WINDOW_FOCUS_CHANGED_EVENT, WINDOWS, WINDOWS_FOCUS, WindowInstanceState};
use zng_layout::unit::TimeUnits as _;
use zng_var::{Var, WeakVar, const_var, var};

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

        /// If is in [`prev_focus`](Self::prev_focus) or [`new_focus`](Self::new_focus).
        fn is_in_target(&self, id: WidgetId) -> bool {
            if let Some(prev) = &self.prev_focus
                && prev.contains(id)
            {
                return true;
            }
            if let Some(new) = &self.new_focus
                && new.contains(id)
            {
                return true;
            }
            false
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

        /// If is in [`prev_return`](Self::prev_return), [`new_return`](Self::new_return)
        /// or [`scope`](Self::scope).
        fn is_in_target(&self, id: WidgetId) -> bool {
            if let Some(scope) = &self.scope
                && scope.contains(id)
            {
                return true;
            }
            if let Some(prev_return) = &self.prev_return
                && prev_return.contains(id)
            {
                return true;
            }
            if let Some(new_return) = &self.new_return
                && new_return.contains(id)
            {
                return true;
            }
            false
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
    pub static FOCUS_CHANGED_EVENT: FocusChangedArgs { let _ = FOCUS_SV.read(); };

    /// Scope return focus widget changed event.
    pub static RETURN_FOCUS_CHANGED_EVENT: ReturnFocusChangedArgs { let _ = FOCUS_SV.read(); };
}

/// Keyboard focus service.
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
        FOCUS_SV.read().navigation_origin.clone()
    }

    /// Current focused widget.
    #[must_use]
    pub fn focused(&self) -> Var<Option<InteractionPath>> {
        FOCUS_SV.read().focused.read_only()
    }

    /// Current return focus of a scope.
    #[must_use]
    pub fn return_focused(&self, scope_id: WidgetId) -> Var<Option<InteractionPath>> {
        FOCUS_SV
            .write()
            .return_focused
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

    /// If the current focused widget is visually indicated.
    #[must_use]
    pub fn is_highlighting(&self) -> Var<bool> {
        FOCUS_SV.read().is_highlighting.read_only()
    }

    /// Current [`return_focused`] for the focused ALT scope, or `None` when not scoped in ALT.
    ///
    /// [`return_focused`]: Self::return_focused
    #[must_use]
    pub fn alt_return(&self) -> Var<Option<InteractionPath>> {
        let mut s = FOCUS_SV.write();
        if let Some(r) = s.alt_return.upgrade() {
            return r;
        }
        let r = s.focused.flat_map(|p| {
            let s = FOCUS_SV.read();
            if let Some(p) = p
                && let Some(tree) = WINDOWS.widget_tree(p.window_id())
                && let Some(wgt) = tree.get(p.widget_id())
                && let Some(wgt) = wgt.into_focusable(s.focus_disabled_widgets.get(), s.focus_hidden_widgets.get())
                && let Some(scope) = wgt.self_and_ancestors().find(|w| w.is_alt_scope())
            {
                drop(s);
                return FOCUS.return_focused(scope.info().id());
            }
            const_var(None)
        });
        s.alt_return = r.downgrade();
        r
    }

    /// Request a focus update.
    ///
    /// All other focus request methods call this method.
    pub fn focus(&self, request: FocusRequest) {
        let mut f = FOCUS_SV.write();
        if f.request.is_none() && f.fallback_request.is_none() {
            UPDATES.once_update("FOCUS.focus", || {
                FOCUS_SV.write().fulfill_request(None, true);
            });
        }
        if request.fallback_only {
            f.fallback_request = Some(request);
        } else {
            f.request = Some(request);
        }
    }

    /// Schedules enabling of [`is_highlighting`] for next update.
    ///
    /// [`is_highlighting`]: Self::is_highlighting
    pub fn highlight(&self) {
        let mut f = FOCUS_SV.write();
        if f.request_highlight {
            return;
        }

        f.request_highlight = true;
        if f.request.is_none() && f.fallback_request.is_none() {
            // `focus` request might not be made
            UPDATES.once_update("FOCUS.highlight", || {
                FOCUS_SV.write().fulfill_highlight_request();
            });
        }
    }

    /// Schedules a [`highlight`] is the latest keyboard event was within the [`auto_highlight`] interval.
    ///
    /// [`highlight`]: Self::highlight
    /// [`auto_highlight`]: Self::auto_highlight
    pub fn highlight_within_auto(&self) {
        let dur = FOCUS_SV.read().auto_highlight.get();
        if let Some(dur) = dur
            && last_keyboard_event().elapsed() <= dur
        {
            self.highlight();
        }
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

    /// Focus the root focusable widget in the given window.
    pub fn focus_window(&self, window_id: impl Into<WindowId>, highlight: bool) {
        self.focus_window_impl(window_id.into(), highlight);
    }
    fn focus_window_impl(&self, window_id: WindowId, highlight: bool) {
        UPDATES.once_update("FOCUS.focus_window", move || {
            if let Some(tree) = WINDOWS.widget_tree(window_id) {
                FOCUS.focus_widget_or_enter(tree.root().id(), false, highlight);
            }
        });
    }

    /// Focus the widget if it is focusable, else focus the first focusable parent, also changes the highlight.
    ///
    /// If the widget and no parent are focusable the focus does not move, in this case the highlight changes
    /// for the current focused widget.
    ///
    /// If `navigation_origin` is `true` the `target` always becomes the [`navigation_origin`] even when it is not focusable.
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
    /// If `navigation_origin` is `true` the `target` becomes the [`navigation_origin`] when it is not focusable
    /// and has no focusable descendant.
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
    /// If `navigation_origin` is `true` the `target` becomes the [`navigation_origin`] when it is not focusable
    /// and has no focusable descendant.
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
        let req = FocusRequest::enter(FOCUS_SV.read().is_highlighting.get());
        self.focus(req);
    }

    /// Focus the first logical ancestor that is focusable from the navigation origin or the current focus
    /// or the return focus from ALT scopes.
    ///
    /// Does nothing if no origin or focus is set. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::exit`].
    pub fn focus_exit(&self) {
        let req = FocusRequest::exit(FOCUS_SV.read().is_highlighting.get());
        self.focus(req)
    }

    /// Focus the logical next widget from the navigation origin or the current focus.
    ///
    /// Does nothing if no origin of focus is set. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::next`].
    pub fn focus_next(&self) {
        let req = FocusRequest::next(FOCUS_SV.read().is_highlighting.get());
        self.focus(req);
    }

    /// Focus the logical previous widget from the navigation origin or the current focus.
    ///
    /// Does nothing if no origin or focus is set. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::prev`].
    pub fn focus_prev(&self) {
        let req = FocusRequest::prev(FOCUS_SV.read().is_highlighting.get());
        self.focus(req);
    }

    /// Focus the nearest upward widget from the navigation origin or the current focus.
    ///
    /// Does nothing if no origin or focus is set. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::up`].
    pub fn focus_up(&self) {
        let req = FocusRequest::up(FOCUS_SV.read().is_highlighting.get());
        self.focus(req);
    }

    /// Focus the nearest widget to the right of the navigation origin or the current focus.
    ///
    /// Does nothing if no origin or focus is set. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::right`].
    pub fn focus_right(&self) {
        let req = FocusRequest::right(FOCUS_SV.read().is_highlighting.get());
        self.focus(req);
    }

    /// Focus the nearest downward widget from the navigation origin or the current focus.
    ///
    /// Does nothing if no origin or focus is set. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::down`].
    pub fn focus_down(&self) {
        let req = FocusRequest::down(FOCUS_SV.read().is_highlighting.get());
        self.focus(req);
    }

    /// Focus the nearest widget to the left of the navigation origin or the current focus.
    ///
    /// Does nothing if no origin or focus is set. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::left`].
    pub fn focus_left(&self) {
        let req = FocusRequest::left(FOCUS_SV.read().is_highlighting.get());
        self.focus(req);
    }

    /// Focus the ALT scope from the navigation origin or the current focus or escapes the current ALT scope.
    ///
    /// Does nothing if no origin or focus is set. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::alt`].
    pub fn focus_alt(&self) {
        let req = FocusRequest::alt(FOCUS_SV.read().is_highlighting.get());
        self.focus(req);
    }
}

zng_env::on_process_start!(|args| {
    if args.yield_until_app() {
        return;
    }

    APP.on_init(hn!(|args| {
        WINDOWS_FOCUS.hook_focus_service(FOCUS.focused());
    }));
});

app_local! {
    static FOCUS_SV: FocusService = FocusService::new();
}
struct FocusService {
    auto_highlight: Var<Option<Duration>>,
    focus_disabled_widgets: Var<bool>,
    focus_hidden_widgets: Var<bool>,

    navigation_origin: Var<Option<WidgetId>>,
    focused: Var<Option<InteractionPath>>,
    return_focused: IdMap<WidgetId, Var<Option<InteractionPath>>>,
    is_highlighting: Var<bool>,
    alt_return: WeakVar<Option<InteractionPath>>,

    commands: cmd::FocusCommands,

    request: Option<FocusRequest>,
    fallback_request: Option<FocusRequest>,
    request_highlight: bool,

    enabled_nav: FocusNavAction,
}
fn last_keyboard_event() -> DInstant {
    RAW_KEY_INPUT_EVENT
        .with(|v| v.latest().map(|a| a.timestamp))
        .unwrap_or(DInstant::EPOCH)
}
impl FocusService {
    fn new() -> Self {
        hooks();
        let s = Self {
            auto_highlight: var(Some(300.ms())),
            focus_disabled_widgets: var(true),
            focus_hidden_widgets: var(true),

            navigation_origin: var(None),
            focused: var(None),
            return_focused: IdMap::default(),
            is_highlighting: var(false),
            alt_return: WeakVar::new(),

            commands: cmd::FocusCommands::new(),

            request: None,
            fallback_request: None,
            request_highlight: false,

            enabled_nav: FocusNavAction::empty(),
        };
        fn refresh(_: &zng_var::VarHookArgs<bool>) -> bool {
            let mut s = FOCUS_SV.write();
            if let Some(id) = s.focused.with(|p| p.as_ref().map(|p| p.widget_id())) {
                tracing::trace!("focus_disabled_widgets or focus_hidden_widgets changed recovery");
                s.focus_direct_recovery(id, None);
            }
            true
        }
        s.focus_disabled_widgets.hook(refresh).perm();
        s.focus_hidden_widgets.hook(refresh).perm();
        s
    }

    fn fulfill_request(&mut self, tree_hint: Option<&WidgetInfoTree>, is_service_request: bool) {
        // resolve what request to fulfill
        let mut request = self.request.take().or(self.fallback_request.take()).unwrap();

        if mem::take(&mut self.request_highlight) {
            // there was also a highlight request
            request.highlight = true;
        } else if !request.highlight
            && let Some(dur) = self.auto_highlight.get()
            && last_keyboard_event().elapsed() <= dur
        {
            // there was also keyboard interaction within the auto_highlight interval
            tracing::trace!("last keyboard event within {dur:?}, highlight");
            request.highlight = true;
        }

        let focus_disabled = self.focus_disabled_widgets.get();
        let focus_hidden = self.focus_hidden_widgets.get();

        // find the current focus info
        let current_info = self
            .focused
            .with(|p| match p {
                Some(p) => {
                    if let Some(t) = &tree_hint
                        && t.window_id() == p.window_id()
                    {
                        t.get(p.widget_id())
                    } else {
                        WINDOWS.widget_tree(p.window_id()).and_then(|t| t.get(p.widget_id()))
                    }
                }
                None => None,
            })
            .map(|i| i.into_focus_info(focus_disabled, focus_hidden));

        // search for widget
        let find_wgt = |id| {
            if let Some(c) = &current_info
                && let Some(r) = c.info().tree().get(id)
            {
                return Some(r.into_focus_info(focus_disabled, focus_hidden));
            }
            if let Some(t) = &tree_hint
                && let Some(r) = t.get(id)
            {
                return Some(r.into_focus_info(focus_disabled, focus_hidden));
            }
            WINDOWS.widget_info(id).map(|r| r.into_focus_info(focus_disabled, focus_hidden))
        };

        // navigation origin
        let origin_info = self
            .navigation_origin
            .get()
            .and_then(|id| current_info.as_ref().and_then(|i| i.info().tree().get(id)))
            .map(|i| i.into_focus_info(focus_disabled, focus_hidden))
            .or_else(|| current_info.clone());

        // resolve the new focus
        let mut new_info = None;
        let mut new_origin = None;
        match request.target {
            FocusTarget::Direct { target } => match find_wgt(target) {
                Some(w) => {
                    if w.is_focusable() {
                        tracing::trace!("focus {:?}", w.info().id());
                        new_info = Some(w);
                    } else {
                        tracing::debug!("cannot focus {target}, not focusable")
                    }
                }
                None => tracing::debug!("cannot focus {target}, not found"),
            },
            FocusTarget::DirectOrExit { target, navigation_origin } => match find_wgt(target) {
                Some(w) => {
                    if w.is_focusable() {
                        tracing::trace!("focus {:?}", w.info().id());
                        new_info = Some(w);
                    } else {
                        tracing::debug!("cannot focus {target}, not focusable, will try ancestors");
                        match w.ancestors().next() {
                            Some(actual) => {
                                tracing::trace!("focusing ancestor {:?}", actual.info().id());
                                new_info = Some(actual);
                            }
                            None => {
                                tracing::debug!("cannot focus {target} or ancestor, none focusable in path");
                            }
                        }
                        if navigation_origin {
                            new_origin = Some(w.info().id());
                        }
                    }
                }
                None => tracing::debug!("cannot focus {target} or ancestor, not found"),
            },
            FocusTarget::DirectOrEnter { target, navigation_origin } => match find_wgt(target) {
                Some(w) => {
                    if w.is_focusable() {
                        tracing::trace!("focus {:?}", w.info().id());
                        new_info = Some(w);
                    } else {
                        tracing::debug!("cannot focus {target}, not focusable, will try descendants");
                        match w.first_tab_descendant() {
                            Some(actual) => {
                                tracing::trace!("focusing descendant {:?}", actual.info().id());
                                new_info = Some(actual);
                            }
                            None => {
                                if navigation_origin {
                                    new_origin = Some(w.info().id());
                                }
                                tracing::debug!("cannot focus {target} or descendants, none tab focusable in subtree");
                            }
                        }
                    }
                }
                None => tracing::debug!("cannot focus {target} or descendants, not found"),
            },
            FocusTarget::DirectOrRelated { target, navigation_origin } => match find_wgt(target) {
                Some(w) => {
                    if w.is_focusable() {
                        tracing::trace!("focus {:?}", w.info().id());
                        new_info = Some(w);
                    } else {
                        tracing::debug!("cannot focus {target}, not focusable, will try descendants and ancestors");
                        match w
                            .first_tab_descendant()
                            .map(|w| (w, false))
                            .or_else(|| w.descendants().next().map(|w| (w, false)))
                            .or_else(|| w.ancestors().next().map(|w| (w, true)))
                        {
                            Some((actual, is_ancestor)) => {
                                if navigation_origin && is_ancestor {
                                    new_origin = Some(w.info().id());
                                }
                                tracing::trace!(
                                    "focusing {} {:?}",
                                    if is_ancestor { "ancestor" } else { "descendant" },
                                    actual.info().id()
                                );
                                new_info = Some(actual);
                            }
                            None => {
                                if navigation_origin {
                                    new_origin = Some(w.info().id());
                                }
                                tracing::debug!("cannot focus {target} or descendants or ancestors, none focusable")
                            }
                        }
                    }
                }
                None => tracing::debug!("cannot focus {target} or descendants or ancestors, not found"),
            },
            FocusTarget::Enter => match &origin_info {
                Some(i) => {
                    new_info = i.first_tab_descendant();
                    tracing::trace!("enter {:?}, focus {:?}", i.info().id(), new_info.as_ref().map(|w| w.info().id()));
                }
                None => tracing::debug!("cannot enter focused, no current focus"),
            },
            FocusTarget::Exit => match &origin_info {
                Some(i) => {
                    if let Some(alt) = i.self_and_ancestors().find(|s| s.is_alt_scope()) // is in alt
                    && let Some(r) = self.return_focused.get(&alt.info().id())
                    && let Some(r) = r.with(|p| p.as_ref().map(|p| p.widget_id()))  // has recorded return
                    && let Some(r) = find_wgt(r)
                    && r.is_focusable()
                    {
                        // return is valid
                        tracing::trace!("exiting from alt scope {:?} to return {:?}", i.info().id(), r.info().id());
                        new_info = Some(r);
                    } else {
                        new_info = i.ancestors().next();
                        tracing::trace!("exit {:?}, focus {:?}", i.info().id(), new_info.as_ref().map(|w| w.info().id()));
                    }
                }
                None => tracing::debug!("cannot exit focused, no current focus"),
            },
            FocusTarget::Next => match &origin_info {
                Some(i) => {
                    new_info = i.next_tab(false);
                    tracing::trace!(
                        "next from {:?}, focus {:?}",
                        i.info().id(),
                        new_info.as_ref().map(|w| w.info().id())
                    );
                }
                None => tracing::debug!("cannot focus next, no current focus"),
            },
            FocusTarget::Prev => match &origin_info {
                Some(i) => {
                    new_info = i.prev_tab(false);
                    tracing::trace!(
                        "prev from {:?}, focus {:?}",
                        i.info().id(),
                        new_info.as_ref().map(|w| w.info().id())
                    );
                }
                None => tracing::debug!("cannot focus prev, no current focus"),
            },
            FocusTarget::Up => match &origin_info {
                Some(i) => {
                    new_info = i.next_up();
                    tracing::trace!("up from {:?}, focus {:?}", i.info().id(), new_info.as_ref().map(|w| w.info().id()));
                }
                None => tracing::debug!("cannot focus up, no current focus"),
            },
            FocusTarget::Right => match &origin_info {
                Some(i) => {
                    new_info = i.next_right();
                    tracing::trace!(
                        "right from {:?}, focus {:?}",
                        i.info().id(),
                        new_info.as_ref().map(|w| w.info().id())
                    );
                }
                None => tracing::debug!("cannot focus right, no current focus"),
            },
            FocusTarget::Down => match &origin_info {
                Some(i) => {
                    new_info = i.next_down();
                    tracing::trace!(
                        "down from {:?}, focus {:?}",
                        i.info().id(),
                        new_info.as_ref().map(|w| w.info().id())
                    );
                }
                None => tracing::debug!("cannot focus down, no current focus"),
            },
            FocusTarget::Left => match &origin_info {
                Some(i) => {
                    new_info = i.next_left();
                    tracing::trace!(
                        "left from {:?}, focus {:?}",
                        i.info().id(),
                        new_info.as_ref().map(|w| w.info().id())
                    );
                }
                None => tracing::debug!("cannot focus left, no current focus"),
            },
            FocusTarget::Alt => match &origin_info {
                Some(i) => {
                    if let Some(alt) = i.self_and_ancestors().find(|w| w.is_alt_scope()) {
                        // Alt inside ALT scope returns focus
                        if let Some(r) = self.return_focused.get(&alt.info().id())
                            && let Some(r) = r.with(|p| p.as_ref().map(|p| p.widget_id()))
                            && let Some(r) = find_wgt(r)
                            && r.is_focusable()
                        {
                            tracing::trace!("toggle alt from alt scope, exit to return {:?}", r.info().id());
                            new_info = Some(r);
                        } else {
                            tracing::trace!("is in alt scope without return focus, exiting to window root focusable");
                            new_info = i.focus_tree().focusable_root();
                            tracing::trace!(
                                "toggle alt from alt scope, to window root {:?}",
                                new_info.as_ref().map(|w| w.info().id())
                            );
                        }
                    } else {
                        new_info = i.alt_scope();
                        tracing::trace!("alt into alt scope {:?}", new_info.as_ref().map(|w| w.info().id()));
                    }
                }
                None => tracing::debug!("cannot focus alt, no current focus"),
            },
        }

        let new_info = match new_info {
            Some(i) => i, // new_info was selected
            None => match current_info.clone() {
                // no new_info was selected, continue with current_info but check highlight and enabled_nav changes
                Some(i) => i,
                // has no focus and continues without
                None => return,
            },
        };

        let current_highlight = self.is_highlighting.get();
        let new_highlight = request.highlight;

        let mut new_enabled_nav = new_info.enabled_nav();

        let prev_focus = self.focused.get();
        let mut new_focus = Some(new_info.info().interaction_path());

        if prev_focus == new_focus && current_highlight == new_highlight && self.enabled_nav == new_enabled_nav {
            // no change
            tracing::trace!("no focus change");
            return;
        }

        if let Some(prev_info) = &current_info {
            // update return focus
            let mut update_return = |scope_path: InteractionPath| -> bool {
                match self.return_focused.entry(scope_path.widget_id()) {
                    IdEntry::Occupied(e) => {
                        let e = e.get();
                        if e.with(|p| *p != prev_focus) {
                            e.set(prev_focus.clone());
                            RETURN_FOCUS_CHANGED_EVENT.notify(ReturnFocusChangedArgs::now(scope_path, e.get(), prev_focus.clone()));
                            return true;
                        }
                    }
                    IdEntry::Vacant(e) => {
                        e.insert(var(prev_focus.clone()));
                        RETURN_FOCUS_CHANGED_EVENT.notify(ReturnFocusChangedArgs::now(scope_path, None, prev_focus.clone()));
                        return true;
                    }
                }
                false
            };

            let prev_scope = prev_info.self_and_ancestors().find(|w| w.is_scope());
            let new_scope = new_info.self_and_ancestors().find(|w| w.is_scope());

            if prev_scope != new_scope {
                if let Some(scope) = new_scope
                    && scope.is_alt_scope()
                {
                    // focus entered ALT scope, previous focus outside is return
                    let set = update_return(scope.info().interaction_path());
                    if set {
                        tracing::trace!(
                            "set alt scope {:?} return focus to {:?}",
                            scope.info().id(),
                            prev_focus.as_ref().map(|f| f.widget_id())
                        );
                    }
                }
                if let Some(scope) = prev_scope
                    && !scope.is_alt_scope()
                    && matches!(
                        scope.focus_info().scope_on_focus(),
                        FocusScopeOnFocus::LastFocused | FocusScopeOnFocus::LastFocusedIgnoreBounds
                    )
                {
                    // focus exited scope that remembers last focused
                    let set = update_return(scope.info().interaction_path());
                    if set {
                        tracing::trace!(
                            "set scope {:?} return focus to {:?}",
                            scope.info().id(),
                            prev_focus.as_ref().map(|f| f.widget_id())
                        );
                    }
                }
            }
        }

        let cause = if is_service_request {
            FocusChangedCause::Request(request)
        } else {
            FocusChangedCause::Recovery
        };

        if current_highlight != new_highlight {
            self.is_highlighting.set(new_highlight);
        }

        let mut focused_changed = prev_focus != new_focus;

        let prev_focus_window = prev_focus.as_ref().map(|p| p.window_id());
        let new_focus_window = new_focus.as_ref().map(|p| p.window_id());

        let args = FocusChangedArgs::now(prev_focus, new_focus.clone(), new_highlight, cause, new_enabled_nav);

        if new_info.is_scope() {
            // scopes moves focus to a child
            let last_focused = |id| {
                self.return_focused
                    .get(&id)
                    .and_then(|p| p.with(|p| p.as_ref().map(|p| p.widget_id())))
            };

            // reentering single child of parent scope that cycles
            let is_tab_cycle_reentry = matches!(args.cause.request_target(), Some(FocusTarget::Prev | FocusTarget::Next))
                && match (&args.prev_focus, &args.new_focus) {
                    (Some(p), Some(n)) => p.contains(n.widget_id()),
                    _ => false,
                };

            // reversed into the scope, first is last
            let reverse = matches!(args.cause.request_target(), Some(FocusTarget::Prev));

            if let Some(w) = new_info.on_focus_scope_move(last_focused, is_tab_cycle_reentry, reverse) {
                // scope moves focus to child

                let prev_focus = args.new_focus.clone();
                new_focus = Some(w.info().interaction_path());

                focused_changed = args.prev_focus != new_focus;
                FOCUS_CHANGED_EVENT.notify(args);

                if prev_focus != new_focus {
                    new_enabled_nav = w.enabled_nav();

                    tracing::trace!("on focus scope move to {:?}", new_focus.as_ref().map(|w| w.widget_id()));

                    FOCUS_CHANGED_EVENT.notify(FocusChangedArgs::now(
                        prev_focus,
                        new_focus.clone(),
                        new_highlight,
                        FocusChangedCause::ScopeGotFocus(reverse),
                        new_enabled_nav,
                    ));
                }
            } else {
                FOCUS_CHANGED_EVENT.notify(args);
            }
        } else {
            FOCUS_CHANGED_EVENT.notify(args);
        }

        if focused_changed {
            self.focused.set(new_focus);

            if prev_focus_window != new_focus_window
                && let Some(w) = new_focus_window
                && let Some(mode) = WINDOWS.mode(w)
                && mode.is_headed()
                && let Some(vars) = WINDOWS.vars(w)
                && matches!(vars.instance_state().get(), WindowInstanceState::Loaded { has_view: true })
            {
                tracing::trace!("focus changed to another window, from {prev_focus_window:?} to {new_focus_window:?}");

                if prev_focus_window.is_some() {
                    WINDOWS_FOCUS.focus(w);
                } else if request.force_window_focus {
                    tracing::trace!("attempting to steal focus from other app");
                    // try to steal focus, or set critical indicator if system does not allow focus stealing
                    vars.focus_indicator().set(Some(FocusIndicator::Critical));
                    WINDOWS_FOCUS.focus(w);
                } else if let Some(i) = request.window_indicator {
                    tracing::trace!("set focus indicator {i:?}");
                    vars.focus_indicator().set(i);
                } else {
                    tracing::debug!("app does not have focus and request did not force focus or set indicator");
                }
            }
        }

        if self.enabled_nav != new_enabled_nav {
            self.enabled_nav = new_enabled_nav;
            tracing::trace!("update cmds {:?}", new_enabled_nav);
            self.commands.update_enabled(new_enabled_nav);
        }

        self.navigation_origin.set(new_origin);
    }

    fn fulfill_highlight_request(&mut self) {
        if self.request.is_some() || self.fallback_request.is_some() {
            // `FOCUS.focus` was requested after the highlight request in the same update pass
            debug_assert!(self.request_highlight);
            return;
        }

        if !self.is_highlighting.get() {
            self.focused.with(|f| {
                if let Some(p) = f {
                    // does a request to focused, with highlight now
                    tracing::trace!("highlight request to {:?}", p.widget_id());
                    self.request = Some(FocusRequest::direct(p.widget_id(), true));
                }
            });
            if self.request.is_some() {
                self.fulfill_request(None, true);
            }
        }
    }

    fn focus_direct_recovery(&mut self, wgt_id: WidgetId, tree_hint: Option<&WidgetInfoTree>) {
        let pending_request = self.request.take();
        let pending_fallback_request = self.fallback_request.take();
        self.request = Some(FocusRequest::direct_or_related(wgt_id, false, self.is_highlighting.get()));
        self.fulfill_request(tree_hint, false);
        self.request = pending_request;
        self.fallback_request = pending_fallback_request;
    }
}

fn hooks() {
    ACCESS_FOCUS_EVENT
        .hook(|args| {
            let is_focused = FOCUS_SV.read().focused.with(|p| p.as_ref().map(|p| p.widget_id())) == Some(args.target.widget_id());
            if args.focus {
                if !is_focused {
                    tracing::trace!("access focus request {}", args.target.widget_id());
                    FOCUS.focus_widget(args.target.widget_id(), false);
                } else {
                    tracing::debug!("access focus request {} ignored, already focused", args.target.widget_id());
                }
            } else if is_focused {
                tracing::trace!("access focus exit request {}", args.target.widget_id());
                FOCUS.focus_exit();
            } else {
                tracing::debug!("access focus exit request {} ignored, not focused", args.target.widget_id());
            }
            true
        })
        .perm();

    ACCESS_FOCUS_NAV_ORIGIN_EVENT
        .hook(|args| {
            let is_window_focused = FOCUS_SV.read().focused.with(|p| p.as_ref().map(|p| p.window_id())) == Some(args.target.window_id());
            if is_window_focused {
                tracing::trace!("access focus nav origin request {}", args.target.widget_id());
                FOCUS.navigation_origin().set(Some(args.target.widget_id()));
            } else {
                tracing::debug!(
                    "access focus nav origin request {} ignored, not in focused window",
                    args.target.widget_id()
                );
            }
            true
        })
        .perm();

    MOUSE_INPUT_EVENT
        .hook(|args| {
            if args.is_mouse_down() {
                tracing::trace!("mouse press focus request {}", args.target.widget_id());
                FOCUS.focus(FocusRequest::direct_or_exit(args.target.widget_id(), true, false));
            }
            true
        })
        .perm();

    TOUCH_INPUT_EVENT
        .hook(|args| {
            if args.is_touch_start() {
                tracing::trace!("touch start focus request {}", args.target.widget_id());
                FOCUS.focus(FocusRequest::direct_or_exit(args.target.widget_id(), true, false));
            }
            true
        })
        .perm();

    ACCESS_CLICK_EVENT
        .hook(|args| {
            tracing::trace!("access click focus request {}", args.target.widget_id());
            FOCUS.focus(FocusRequest::direct_or_exit(args.target.widget_id(), true, false));
            true
        })
        .perm();

    WIDGET_TREE_CHANGED_EVENT
        .hook(|args| {
            let mut s = FOCUS_SV.write();
            if let Some((win_id, wgt_id)) = s.focused.with(|f| f.as_ref().map(|f| (f.window_id(), f.widget_id())))
                && args.tree.window_id() == win_id
            {
                // on tree rebuild the focused widget can move to another slot or stop being focusable
                // on tree update (render) the widget can stop being focusable due to visibility change
                //
                // to correct we do a focus request and fulfill on the focused widget id.
                //
                // this is also needed to update the commands enabled status, for example, the previous widget from focused
                // is now collapsed, so the previous command must now be disabled
                tracing::trace!("tree changed recovery");
                s.focus_direct_recovery(wgt_id, Some(&args.tree));

                // and check all return focus
                s.return_focused.retain(|scope_id, ret| {
                    if let Some((win_id, wgt_id)) = ret.with(|f| f.as_ref().map(|f| (f.window_id(), f.widget_id())))
                        && win_id == args.tree.window_id()
                    {
                        if win_id != args.tree.window_id() {
                            // not relevant to this event
                            return true;
                        }

                        if let Some(scope) = args.tree.get(*scope_id) {
                            // scope still exists

                            if let Some(wgt) = args.tree.get(wgt_id) {
                                // widget still exists

                                let wgt_path = wgt.interaction_path();
                                if ret.with(|p| p.as_ref() != Some(&wgt_path)) {
                                    // changed
                                    tracing::trace!("return_focus of {scope_id} ({wgt_id}) changed");

                                    let was_inside_scope = ret.with(|p| p.as_ref().unwrap().contains(*scope_id));
                                    let is_inside_scope = scope.is_ancestor(&wgt);

                                    if was_inside_scope == is_inside_scope {
                                        ret.set(Some(wgt_path.clone()));
                                        RETURN_FOCUS_CHANGED_EVENT.notify(ReturnFocusChangedArgs::now(
                                            scope.interaction_path(),
                                            ret.get(),
                                            Some(wgt_path),
                                        ));

                                        // retain record
                                        return true;
                                    } else {
                                        tracing::trace!("return_focus of {scope_id} ({wgt_id}) cannot be return anymore");

                                        ret.set(None);
                                        RETURN_FOCUS_CHANGED_EVENT.notify(ReturnFocusChangedArgs::now(
                                            scope.interaction_path(),
                                            ret.get(),
                                            None,
                                        ));
                                    }
                                } else {
                                    // retain record
                                    return true;
                                }
                            } else {
                                tracing::trace!("return_focus of {scope_id} ({wgt_id}) no longer in focus tree");
                                ret.set(None);
                                RETURN_FOCUS_CHANGED_EVENT.notify(ReturnFocusChangedArgs::now(scope.interaction_path(), ret.get(), None));
                            }
                        }
                    }

                    // retain only if has observers
                    ret.strong_count() > 1
                });
            }
            if !args.is_update {
                focus_info::FocusTreeData::consolidate_alt_scopes(&args.prev_tree, &args.tree);
            }
            true
        })
        .perm();

    WINDOW_FOCUS_CHANGED_EVENT
        .hook(|args| {
            let mut s = FOCUS_SV.write();
            let current_focus = s.focused.with(|p| p.as_ref().map(|p| p.window_id()));
            if current_focus != args.new_focus {
                if let Some(id) = args.new_focus
                    && let Some(tree) = WINDOWS.widget_tree(id)
                {
                    tracing::trace!("window focus changed to {id:?}");
                    let tree = FocusInfoTree::new(tree, s.focus_disabled_widgets.get(), s.focus_hidden_widgets.get());
                    if let Some(root) = tree.focusable_root() {
                        let pending_request = s.request.take();
                        let pending_fallback_request = s.fallback_request.take();
                        tracing::trace!("window focus changed focus request {:?}", root.info().id());
                        s.request = Some(FocusRequest::direct_or_related(root.info().id(), false, s.is_highlighting.get()));
                        s.fulfill_request(Some(tree.tree()), false);
                        s.request = pending_request;
                        s.fallback_request = pending_fallback_request;
                        return true;
                    } else {
                        tracing::debug!("focused window does not have any focusable widget");
                    }
                } else {
                    tracing::debug!("all windows lost focus");
                }

                if let Some(win_id) = current_focus {
                    let wgt_id = s.focused.with(|p| p.as_ref().map(|p| p.widget_id())).unwrap();

                    // notify blur
                    s.focused.set(None);
                    s.is_highlighting.set(false);
                    if !s.enabled_nav.is_empty() {
                        s.enabled_nav = FocusNavAction::empty();
                        s.commands.update_enabled(FocusNavAction::empty());
                    }

                    FOCUS_CHANGED_EVENT.notify(FocusChangedArgs::now(
                        s.focused.get(),
                        None,
                        false,
                        FocusChangedCause::Recovery,
                        s.enabled_nav,
                    ));

                    if let Some(prev_tree) = WINDOWS.widget_tree(win_id)
                        && let Some(wgt) = prev_tree.get(wgt_id)
                        && let Some(wgt) = wgt.into_focusable(s.focus_disabled_widgets.get(), s.focus_hidden_widgets.get())
                        && let Some(root_scope) = wgt.self_and_ancestors().filter(|w| w.is_scope()).last()
                        && !root_scope.is_alt_scope()
                        && matches!(
                            root_scope.focus_info().scope_on_focus(),
                            FocusScopeOnFocus::LastFocused | FocusScopeOnFocus::LastFocusedIgnoreBounds
                        )
                    {
                        // window still open, update return_focused for window root scope

                        let mut return_change = None;
                        if let Some(alt_scope) = wgt.self_and_ancestors().find(|w| w.is_alt_scope()) {
                            // was inside alt, the return focus for the root is the alt return, does not return inside alt
                            if let Some(ret) = s.return_focused.get(&alt_scope.info().id())
                                && let Some(path) = ret.get()
                            {
                                return_change = Some(path);
                            }
                        } else {
                            // normal return, last focused
                            return_change = s.focused.get();
                        }

                        if return_change.is_some() {
                            let mut prev = None;
                            match s.return_focused.entry(root_scope.info().id()) {
                                IdEntry::Occupied(e) => {
                                    prev = e.get().get();
                                    e.get().set(return_change.clone());
                                }
                                IdEntry::Vacant(e) => {
                                    e.insert(var(return_change.clone()));
                                }
                            }
                            RETURN_FOCUS_CHANGED_EVENT.notify(ReturnFocusChangedArgs::now(
                                Some(root_scope.info().interaction_path()),
                                prev,
                                return_change,
                            ));
                        }
                    } else {
                        // window closed, cleanup return_focused
                        s.return_focused.retain(|scope_id, v| {
                            if let Some(p_win_id) = v.with(|p| p.as_ref().map(|p| p.window_id()))
                                && p_win_id == win_id
                            {
                                // was return in closed window, can assume the scope was dropped because if it
                                // had moved to another window the WIDGET_TREE_CHANGED_EVENT handler would have
                                // updates this by the time this event happens
                                #[cfg(debug_assertions)]
                                if WINDOWS.widget_info(*scope_id).is_some() {
                                    tracing::error!("expected focus scope {scope_id} to not exist after window close");
                                }
                                #[cfg(not(debug_assertions))]
                                let _ = scope_id;

                                return false;
                            }
                            true
                        });
                    }
                }
            }
            true
        })
        .perm();
}
