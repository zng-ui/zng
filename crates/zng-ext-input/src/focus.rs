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
use zng_unique_id::IdMap;

use std::{mem, time::Duration};
use zng_app::{
    APP, DInstant, INSTANT,
    event::event,
    event_args, hn,
    update::UPDATES,
    widget::{
        WidgetId,
        info::{InteractionPath, WIDGET_TREE_CHANGED_EVENT, WidgetInfoTree},
    },
    window::WindowId,
};
use zng_app_context::app_local;
use zng_ext_window::{WINDOWS, WINDOWS_FOCUS};
use zng_layout::unit::TimeUnits as _;
use zng_var::{Var, var};

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
    ///
    /// # Provider
    ///
    /// This event is provided by the [`FocusManager`] extension.
    pub static FOCUS_CHANGED_EVENT: FocusChangedArgs { let _ = FOCUS_SV.read(); };

    /// Scope return focus widget changed event.
    ///
    /// # Provider
    ///
    /// This event is provided by the [`FocusManager`] extension.
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

    /// Current ALT return focus.
    #[must_use]
    pub fn alt_return(&self) -> Var<Option<InteractionPath>> {
        FOCUS_SV.read().alt_return.read_only()
    }

    /// If focus is in an ALT scope.
    #[must_use]
    pub fn in_alt(&self) -> Var<bool> {
        FOCUS_SV.read().alt_return.map(|p| p.is_some())
    }

    /// If the current focused widget is visually indicated.
    #[must_use]
    pub fn is_highlighting(&self) -> Var<bool> {
        FOCUS_SV.read().is_highlighting.read_only()
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

fn hooks() {
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
                let pending_request = s.request.take();
                let pending_fallback_request = s.fallback_request.take();
                s.request = Some(FocusRequest::direct_or_related(wgt_id, false, s.is_highlighting.get()));
                s.fulfill_request(Some(&args.tree), false);
                s.request = pending_request;
                s.fallback_request = pending_fallback_request;
            }
            if !args.is_update {
                focus_info::FocusTreeData::consolidate_alt_scopes(&args.prev_tree, &args.tree);
            }
            true
        })
        .perm();
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
    alt_return: Var<Option<InteractionPath>>,
    is_highlighting: Var<bool>,

    commands: cmd::FocusCommands,

    request: Option<FocusRequest>,
    fallback_request: Option<FocusRequest>,
    request_highlight: bool,

    last_keyboard_event: DInstant,
    enabled_nav: FocusNavAction,
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
            alt_return: var(None),
            is_highlighting: var(false),

            commands: cmd::FocusCommands::new(),

            request: None,
            fallback_request: None,
            request_highlight: false,

            last_keyboard_event: DInstant::EPOCH,
            enabled_nav: FocusNavAction::empty(),
        };
        s
    }

    fn fulfill_request(&mut self, tree_hint: Option<&WidgetInfoTree>, is_service_request: bool) {
        // resolve what request to fulfill
        let mut request = self.request.take().or(self.fallback_request.take()).unwrap();

        if mem::take(&mut self.request_highlight) {
            // there was also a highlight request
            request.highlight = true;
        } else if !request.highlight {
            let timestamp = INSTANT.now();
            if let Some(dur) = self.auto_highlight.get()
                && timestamp.duration_since(self.last_keyboard_event) <= dur
            {
                // there was also keyboard interaction within the auto_highlight interval
                request.highlight = true;
            }
        }

        let focus_disabled = self.focus_disabled_widgets.get();
        let focus_hidden = self.focus_hidden_widgets.get();

        // find the current focus info
        let current_info = self
            .focused
            .with(|p| match p {
                Some(p) => {
                    if let Some(t) = tree_hint
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
            match &current_info {
                // try the same window first
                Some(i) => i.info().tree().get(id),
                None => WINDOWS.widget_info(id),
            }
            .map(|i| i.into_focus_info(focus_disabled, focus_hidden))
        };

        // resolve the new focus
        let mut new_info = None;
        let mut new_origin = None;
        match request.target {
            FocusTarget::Direct { target } => match find_wgt(target) {
                Some(w) => {
                    if w.is_focusable() {
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
                        new_info = Some(w);
                    } else {
                        tracing::debug!("cannot focus {target}, not focusable, will try ancestors");
                        match w.ancestors().next() {
                            Some(actual) => {
                                if navigation_origin {
                                    new_origin = Some(w.info().path());
                                }
                                new_info = Some(actual);
                            }
                            None => tracing::debug!("cannot focus {target} or ancestor, none focusable in path"),
                        }
                    }
                }
                None => tracing::debug!("cannot focus {target} or ancestor, not found"),
            },
            FocusTarget::DirectOrEnter { target, navigation_origin } => match find_wgt(target) {
                Some(w) => {
                    if w.is_focusable() {
                        new_info = Some(w);
                    } else {
                        tracing::debug!("cannot focus {target}, not focusable, will try descendants");
                        match w.first_tab_descendant() {
                            Some(actual) => {
                                if navigation_origin {
                                    new_origin = Some(w.info().path());
                                }
                                new_info = Some(actual);
                            }
                            None => tracing::debug!("cannot focus {target} or descendants, none tab focusable in subtree"),
                        }
                    }
                }
                None => tracing::debug!("cannot focus {target} or descendants, not found"),
            },
            FocusTarget::DirectOrRelated { target, navigation_origin } => match find_wgt(target) {
                Some(w) => {
                    if w.is_focusable() {
                        new_info = Some(w);
                    } else {
                        tracing::debug!("cannot focus {target}, not focusable, will try descendants and ancestors");
                        match w
                            .first_tab_descendant()
                            .or_else(|| w.descendants().next())
                            .or_else(|| w.ancestors().next())
                        {
                            Some(actual) => {
                                if navigation_origin {
                                    new_origin = Some(w.info().path());
                                }
                                new_info = Some(actual);
                            }
                            None => tracing::debug!("cannot focus {target} or descendants or ancestors, none focusable"),
                        }
                    }
                }
                None => tracing::debug!("cannot focus {target} or descendants or ancestors, not found"),
            },
            FocusTarget::Enter => match &current_info {
                Some(i) => new_info = i.first_tab_descendant(),
                None => tracing::debug!("cannot enter focused, no current focus"),
            },
            FocusTarget::Exit => match &current_info {
                Some(i) => {
                    if i.is_alt_scope() // is ALT
                        && let Some(r) = self.return_focused.get(&i.info().id())
                        && let Some(r) = r.with(|r| r.as_ref().map(|p| p.widget_id())) // has recorded return
                        && let Some(r) = find_wgt(r)
                        && r.is_focusable()
                    // return is valid
                    {
                        tracing::debug!("exiting from alt scope with return");
                        new_info = Some(r);
                    } else {
                        new_info = i.ancestors().next();
                    }
                }
                None => tracing::debug!("cannot exit focused, no current focus"),
            },
            FocusTarget::Next => match &current_info {
                Some(i) => new_info = i.next_tab(false),
                None => tracing::debug!("cannot focus next, no current focus"),
            },
            FocusTarget::Prev => match &current_info {
                Some(i) => new_info = i.prev_tab(false),
                None => tracing::debug!("cannot focus prev, no current focus"),
            },
            FocusTarget::Up => match &current_info {
                Some(i) => new_info = i.next_up(),
                None => tracing::debug!("cannot focus up, no current focus"),
            },
            FocusTarget::Right => match &current_info {
                Some(i) => new_info = i.next_right(),
                None => tracing::debug!("cannot focus right, no current focus"),
            },
            FocusTarget::Down => match &current_info {
                Some(i) => new_info = i.next_down(),
                None => tracing::debug!("cannot focus down, no current focus"),
            },
            FocusTarget::Left => match &current_info {
                Some(i) => new_info = i.next_left(),
                None => tracing::debug!("cannot focus left, no current focus"),
            },
            FocusTarget::Alt => match &current_info {
                Some(i) => {
                    if let Some(alt) = i.self_and_ancestors().find(|w| w.is_alt_scope()) {
                        // Alt inside ALT scope returns focus
                        if let Some(r) = self.return_focused.get(&alt.info().id())
                            && let Some(r) = r.with(|r| r.as_ref().map(|p| p.widget_id()))
                            && let Some(r) = find_wgt(r)
                            && r.is_focusable()
                        {
                            tracing::trace!("exiting from alt scope with return");
                            new_info = Some(r);
                        } else {
                            tracing::debug!("is in alt scope without return focus, exiting to window root focusable");
                            new_info = i.focus_tree().focusable_root();
                        }
                    } else {
                        new_info = i.alt_scope();
                    }
                }
                None => tracing::debug!("cannot focus up, no current focus"),
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

        let enabled_nav = new_info.enabled_nav();

        let prev_focus = self.focused.get();
        let new_focus = Some(new_info.info().interaction_path());

        if prev_focus == new_focus && current_highlight == new_highlight && self.enabled_nav == enabled_nav {
            return;
        }

        let cause = if is_service_request {
            FocusChangedCause::Request(request)
        } else {
            FocusChangedCause::Recovery
        };

        if current_highlight != new_highlight {
            self.is_highlighting.set(new_highlight);
        }

        if prev_focus != new_focus {
            self.focused.set(new_focus.clone());
        }

        if self.enabled_nav != enabled_nav {
            self.enabled_nav = enabled_nav;
            self.commands.update_enabled(enabled_nav);
        }

        FOCUS_CHANGED_EVENT.notify(FocusChangedArgs::now(prev_focus, new_focus, new_highlight, cause, enabled_nav));

        if new_info.is_scope() {
            // !!: TODO do focus scope action
        }
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
                    self.request = Some(FocusRequest::direct(p.widget_id(), true));
                }
            });
            if self.request.is_some() {
                self.fulfill_request(None, true);
            }
        }
    }
}
