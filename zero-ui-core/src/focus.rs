//! Keyboard focus manager.
//!
//! The [`FocusManager`] struct is an [app extension](crate::app::AppExtension). It
//! is included in the [default app](crate::app::App::default) and provides the [`Focus`] service
//! and focus events.
//!
//! # Keyboard Focus
//!
//! In a given program only a single widget can receive keyboard input at a time, this widget has the *keyboard focus*.
//! This is an extension of the windows manager own focus management that controls which window is *focused*, receiving
//! keyboard input.
//!
//! You can track the focused widget by listening to the [`FocusChangedEvent`] event or by calling [`focused`](Focus::focused)
//! in the [`Focus`] service.
//!
//! # Navigation
//!
//! The keyboard focus can be moved from one widget to the next using the keyboard or the [`Focus`] service methods.
//! There are two styles of movement: [tabbing](#tab-navigation) that follows the logical order and [directional](#directional-navigation)
//! that follows the visual order.
//!
//! Keyboard navigation behaves different depending on what region of the screen the current focused widget is in, these regions
//! are called [focus scopes](#focus-scopes). Every window is a focus scope that can be subdivided further.
//!
//! ## Tab Navigation
//!
//! Tab navigation follows a logical order, the position of the widget in the [widget tree](FocusInfoTree),
//! optionally overridden with a [custom index](TabIndex).
//!
//! Focus is moved forward by pressing `TAB` or calling [`focus_next`](Focus::focus_next) and backward by pressing `SHIFT+TAB` or
//! calling [`focus_prev`](Focus::focus_prev).
//!
//! ## Directional Navigation
//!
//! Directional navigation follows the visual position of the widget on the screen.
//!
//! Focus is moved by pressing the **arrow keys** or calling the focus direction methods in the [`Focus`](Focus::focus_up) service.
//!
//! ## Focus Scopes
//!
//! Focus scopes are widgets that configure how focus navigation happens inside then. They control what happens
//! when the scope widget is focused, how the navigation flows inside their screen region and even if the navigation
//! can naturally mode out of their region.
//!
//! ### Alt Scopes
//!
//! Alt scopes are specially marked focus scopes that receive focus when the `ALT`
//! key is pressed or [`focus_alt`](Focus::focus_alt) is called in the [`Focus`] service. The alt scope of a widget
//! is selected by [`WidgetFocusInfo::alt_scope`].
//!
//! Alt scopes remember the previously focused widget as a [return focus](#return-focus). The focus returns to the widget
//! when the `ESC` key is pressed, or [`escape_alt`](Focus::escape_alt) is called in the [`Focus`] service.
//!
//! ### Return Focus
//!
//! Focus scopes can be configured to remember the last focused widget inside then, the focus than **returns** to
//! this widget when the scope receives focus. Alt scopes also remember the widget from which the *alt* focus happened
//! and can also return focus back to that widget.
//!
//! Widgets can keep track of this by listening to the [`ReturnFocusChangedEvent`] event or by calling
//! [`return_focused`](Focus::return_focused) in the [`Focus`] service. Usually the window root scope remembers
//! return focus and some widgets, like *text inputs* visually indicate that they will be focused when the window
//! is focused.
//!
//! # Configuring Widgets
//!
//! Focusable configuration is set as render metadata using the [`FocusInfoKey`] that is of a value
//! [`FocusInfoBuilder`]. You can use this type to make a widget focusable or a focus scope and customize
//! how the focus manager interacts with the widget.
//!
//! Note that the main crate already provides properties for configuring focus in widgets, you only need to
//! set the [`FocusInfoKey`] directly if you are developing your own focus defining properties.
//!
//! # Querying
//!
//! Focus information exists as metadata associated with a window widget tree. This metadata can be manually queried by
//! creating a [`FocusInfoTree`] or directly from a widget info by using the [`WidgetInfoFocusExt`] extension methods.

mod focus_info;
pub use focus_info::*;

pub mod commands;
use commands::FocusCommands;

use crate::{
    app::{AppEventSender, AppExtension},
    context::*,
    crate_util::IdMap,
    event::*,
    mouse::MouseInputEvent,
    service::Service,
    units::TimeUnits,
    var::{var, RcVar, ReadOnlyRcVar, Var, Vars},
    widget_info::{InteractionPath, WidgetInfoTree},
    window::{WidgetInfoChangedEvent, WindowFocusChangedEvent, WindowId, Windows},
    WidgetId,
};
use std::{
    collections::hash_map,
    time::{Duration, Instant},
};

event_args! {
    /// [`FocusChangedEvent`] arguments.
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
        fn delivery_list(&self) -> EventDeliveryList {
            EventDeliveryList::widgets_opt(self.prev_focus.as_deref()).with_widgets_opt(self.new_focus.as_deref())
        }
    }

    /// [`ReturnFocusChangedEvent`] arguments.
    pub struct ReturnFocusChangedArgs {
        /// The scope that returns the focus when focused directly.
        ///
        /// Is `None` if the previous focus was the return focus of a scope that was removed.
        pub scope : Option<InteractionPath>,

        /// Previous return focus of the widget.
        pub prev_return: Option<InteractionPath>,

        /// New return focus of the widget.
        pub new_return: Option<InteractionPath>,

        ..

        /// The [`prev_return`](Self::prev_return), [`new_return`](Self::new_return)
        /// and [`scope`](Self::scope).
        fn delivery_list(&self) -> EventDeliveryList {
            EventDeliveryList::widgets_opt(self.scope.as_deref())
                .with_widgets_opt(self.prev_return.as_deref())
                .with_widgets_opt(self.new_return.as_deref())
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
    pub fn is_hightlight_changed(&self) -> bool {
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

    /// If `widget_id` is the previous focus and was not before.
    pub fn is_blur(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_focus, &self.new_focus) {
            (Some(prev), Some(new)) => prev.widget_id() == widget_id && new.widget_id() != widget_id,
            (Some(prev), None) => prev.widget_id() == widget_id,
            (None, _) => false,
        }
    }

    /// If `widget_id` is the new focus or a parent of the new focus and was not a parent of the previous focus.
    pub fn is_focus_enter(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_focus, &self.new_focus) {
            (Some(prev), Some(new)) => !prev.contains(widget_id) && new.contains(widget_id),
            (None, Some(new)) => new.contains(widget_id),
            (_, None) => false,
        }
    }

    /// If `widget_id` is the previous focus or a parent of the previous focus and is not a parent of the new focus.
    pub fn is_focus_leave(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_focus, &self.new_focus) {
            (Some(prev), Some(new)) => prev.contains(widget_id) && !new.contains(widget_id),
            (Some(prev), None) => prev.contains(widget_id),
            (None, _) => false,
        }
    }

    /// If the widget is the new focus.
    pub fn is_focused(&self, widget_id: WidgetId) -> bool {
        self.new_focus.as_ref().map(|p| p.widget_id() == widget_id).unwrap_or(false)
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
}

/// The cause of a [`FocusChangedEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// If the change cause by a request to the previous widget, by tab index.
    pub fn is_prev_request(self) -> bool {
        match self {
            FocusChangedCause::Request(r) => matches!(r.target, FocusTarget::Prev),
            _ => false,
        }
    }
}

event! {
    /// Keyboard focused widget changed event.
    ///
    /// # Provider
    ///
    /// This event is provided by the [`FocusManager`] extension.
    pub FocusChangedEvent: FocusChangedArgs;

    /// Scope return focus widget changed event.
    ///
    /// # Provider
    ///
    /// This event is provided by the [`FocusManager`] extension.
    pub ReturnFocusChangedEvent: ReturnFocusChangedArgs;
}

/// Application extension that manages keyboard focus.
///
/// # Events
///
/// Events this extension provides.
///
/// * [FocusChangedEvent]
/// * [ReturnFocusChangedEvent]
///
/// # Services
///
/// Services this extension provides.
///
/// * [Focus]
///
/// # Default
///
/// This extension is included in the [`App::default`].
///
/// # Dependencies
///
/// This extension requires the [`Windows`] service.
///
/// This extension listens to the [`MouseInputEvent`], [`ShortcutEvent`], [`WindowFocusChangedEvent`]
/// and [`WidgetInfoChangedEvent`] events.
///
/// To work properly it should be added to the app after the windows manager extension.
///
/// # About Focus
///
/// See the [module level](crate::focus) documentation for an overview of the keyboard
/// focus concepts implemented by this app extension.
///
/// [`App::default`]: crate::app::App::default
/// [`ShortcutEvent`]: crate::gesture::ShortcutEvent
pub struct FocusManager {
    last_keyboard_event: Instant,
    pending_layout: Option<WidgetInfoTree>,
    pending_render: Option<WidgetInfoTree>,
    commands: Option<FocusCommands>,
}
impl Default for FocusManager {
    fn default() -> Self {
        Self {
            last_keyboard_event: Instant::now() - Duration::from_secs(10),
            pending_layout: None,
            pending_render: None,
            commands: None,
        }
    }
}
impl AppExtension for FocusManager {
    fn init(&mut self, ctx: &mut AppContext) {
        ctx.services.register(Focus::new(ctx.updates.sender()));
        self.commands = Some(FocusCommands::new(ctx.events));
    }

    fn event_preview<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        if let Some(args) = WidgetInfoChangedEvent.update(args) {
            if ctx
                .services
                .focus()
                .focused
                .as_ref()
                .map(|f| f.window_id() == args.window_id)
                .unwrap_or_default()
            {
                // we need up-to-date visibility and that is affected by both layout and render.
                // so we delay responding to the event if a render or layout was requested when
                // the tree was invalidated.
                if args.pending_render {
                    self.pending_render = Some(args.tree.clone());
                    self.pending_layout = None;
                } else if args.pending_layout {
                    self.pending_render = None;
                    self.pending_layout = Some(args.tree.clone());
                } else {
                    self.pending_render = None;
                    self.pending_layout = None;
                    self.on_info_tree_update(&args.tree, ctx);
                }
            }
        } else {
            self.commands.as_mut().unwrap().event_preview(ctx, args);
        }
    }

    fn layout(&mut self, ctx: &mut AppContext) {
        if let Some(tree) = self.pending_layout.take() {
            self.on_info_tree_update(&tree, ctx);
        }
    }
    fn render(&mut self, ctx: &mut AppContext) {
        if let Some(tree) = self.pending_render.take() {
            self.on_info_tree_update(&tree, ctx);
        } else {
            let (focus, windows) = ctx.services.req_multi::<(Focus, Windows)>();

            // widgets may have changed visibility.
            let args = focus.continue_focus(ctx.vars, windows);
            self.notify(ctx.vars, ctx.events, focus, windows, args);
        }
    }

    fn event<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        let mut request = None;

        if let Some(args) = MouseInputEvent.update(args) {
            if args.is_mouse_down() {
                // click
                request = Some(FocusRequest::direct_or_exit(args.target.widget_id(), false));
            }
        } else if let Some(args) = WindowFocusChangedEvent.update(args) {
            // foreground window maybe changed
            let (focus, windows) = ctx.services.req_multi::<(Focus, Windows)>();
            if let Some((window_id, widget_id, highlight)) = focus.pending_window_focus.take() {
                if args.is_focus(window_id) {
                    request = Some(FocusRequest::direct(widget_id, highlight));
                }
            } else if let Some(args) = focus.continue_focus(ctx.vars, windows) {
                self.notify(ctx.vars, ctx.events, focus, windows, Some(args));
            }

            if let Some(window_id) = args.closed() {
                for args in focus.cleanup_returns_win_closed(ctx.vars, window_id) {
                    ReturnFocusChangedEvent.notify(ctx.events, args);
                }
            }
        } else if let Some(args) = crate::app::raw_events::RawKeyInputEvent.update(args) {
            self.last_keyboard_event = args.timestamp;
        }

        if let Some(request) = request {
            let (focus, windows) = ctx.services.req_multi::<(Focus, Windows)>();
            let args = focus.fulfill_request(ctx.vars, windows, request);
            self.notify(ctx.vars, ctx.events, focus, windows, args);
        }
    }

    fn update(&mut self, ctx: &mut AppContext) {
        let mut request = None;

        let focus = ctx.services.focus();
        if let Some(req) = focus.request.take() {
            // custom
            request = Some(req);
        }

        if let Some(request) = request {
            let (focus, windows) = ctx.services.req_multi::<(Focus, Windows)>();
            let args = focus.fulfill_request(ctx.vars, windows, request);
            self.notify(ctx.vars, ctx.events, focus, windows, args);
        }
    }
}
impl FocusManager {
    fn notify(&mut self, vars: &Vars, events: &mut Events, focus: &mut Focus, windows: &mut Windows, args: Option<FocusChangedArgs>) {
        if let Some(mut args) = args {
            if !args.highlight && args.new_focus.is_some() {
                if let Some(dur) = focus.auto_highlight {
                    if args.timestamp.duration_since(self.last_keyboard_event) <= dur {
                        args.highlight = true;
                        focus.is_highlighting = true;
                        focus.is_highlighting_var.set_ne(vars, true);
                    }
                }
            }

            let commands = self.commands.as_mut().unwrap();
            commands.update_enabled(args.enabled_nav);

            let reverse = args.cause.is_prev_request();
            let prev_focus = args.prev_focus.clone();
            FocusChangedEvent.notify(events, args);

            // may have focused scope.
            while let Some(after_args) = focus.move_after_focus(vars, windows, reverse) {
                commands.update_enabled(after_args.enabled_nav);
                FocusChangedEvent.notify(events, after_args);
            }

            for return_args in focus.update_returns(vars, prev_focus, windows) {
                ReturnFocusChangedEvent.notify(events, return_args);
            }
        }
    }

    fn on_info_tree_update(&mut self, tree: &WidgetInfoTree, ctx: &mut AppContext) {
        let (focus, windows) = ctx.services.req_multi::<(Focus, Windows)>();

        // widget tree rebuilt, check if focus is still valid
        let args = focus.continue_focus(ctx.vars, windows);
        self.notify(ctx.vars, ctx.events, focus, windows, args);

        // cleanup return focuses.
        for args in focus.cleanup_returns(ctx.vars, FocusInfoTree::new(tree, focus.focus_disabled_widgets)) {
            ReturnFocusChangedEvent.notify(ctx.events, args);
        }
    }
}

/// Keyboard focus service.
///
/// # Provider
///
/// This service is provided by the [`FocusManager`] extension.
#[derive(Service)]
pub struct Focus {
    /// If set to a duration, starts highlighting focus when a focus change happen within the duration of
    /// a keyboard input event.
    ///
    /// Default is `300.ms()`.
    pub auto_highlight: Option<Duration>,

    /// If [`DISABLED`] widgets can receive focus.
    ///
    /// This is `true` by default, allowing disabled widgets to receive focus can provide a better experience for users,
    /// as the keyboard navigation stays the same, this is also of special interest for accessibility users, screen readers
    /// tend to only vocalize the focused content.
    ///
    /// Widgets should use a different *focused* visual for disabled focus, it must be clear that the widget has the keyboard focus
    /// only as a navigation waypoint and cannot provide its normal function.
    ///
    /// [`DISABLED`]: crate::widget_info::Interactivity::DISABLED
    pub focus_disabled_widgets: bool,

    request: Option<FocusRequest>,
    app_event_sender: AppEventSender,

    focused_var: RcVar<Option<InteractionPath>>,
    focused: Option<InteractionPath>,

    return_focused_var: IdMap<WidgetId, RcVar<Option<InteractionPath>>>,
    return_focused: IdMap<WidgetId, InteractionPath>,

    alt_return_var: RcVar<Option<InteractionPath>>,
    alt_return: Option<(InteractionPath, InteractionPath)>,

    is_highlighting_var: RcVar<bool>,
    is_highlighting: bool,

    enabled_nav: FocusNavAction,

    pending_window_focus: Option<(WindowId, WidgetId, bool)>,
}
impl Focus {
    /// New focus service, the `update_sender` is used to flag an update after a focus change request.
    #[must_use]
    pub fn new(app_event_sender: AppEventSender) -> Self {
        Focus {
            focus_disabled_widgets: true,
            auto_highlight: Some(300.ms()),

            request: None,
            app_event_sender,

            focused_var: var(None),
            focused: None,

            return_focused_var: IdMap::default(),
            return_focused: IdMap::default(),

            alt_return_var: var(None),
            alt_return: None,

            is_highlighting_var: var(false),
            is_highlighting: false,

            enabled_nav: FocusNavAction::empty(),

            pending_window_focus: None,
        }
    }

    /// Current focused widget.
    #[must_use]
    pub fn focused(&self) -> ReadOnlyRcVar<Option<InteractionPath>> {
        self.focused_var.clone().into_read_only()
    }

    /// Current return focus of a scope.
    #[must_use]
    pub fn return_focused(&mut self, scope_id: WidgetId) -> ReadOnlyRcVar<Option<InteractionPath>> {
        self.return_focused_var
            .entry(scope_id)
            .or_insert_with(|| var(None))
            .clone()
            .into_read_only()
    }

    /// If the [`focused`] path is in the given `window_id`.
    ///
    /// [`focused`]: Self::focused
    pub fn is_window_focused(&self, window_id: WindowId) -> impl Var<bool> {
        self.focused().map(move |p| matches!(p, Some(p) if p.window_id() == window_id))
    }

    /// Current ALT return focus.
    #[must_use]
    pub fn alt_return(&self) -> ReadOnlyRcVar<Option<InteractionPath>> {
        self.alt_return_var.clone().into_read_only()
    }

    /// If focus is in an ALT scope.
    #[must_use]
    pub fn in_alt(&self) -> impl Var<bool> {
        self.alt_return_var.map(|p| p.is_some())
    }

    /// If the current focused widget is visually indicated.
    #[must_use]
    pub fn is_highlighting(&self) -> ReadOnlyRcVar<bool> {
        self.is_highlighting_var.clone().into_read_only()
    }

    /// Request a focus update.
    ///
    /// All other focus request methods call this method.
    pub fn focus(&mut self, request: FocusRequest) {
        self.pending_window_focus = None;
        self.request = Some(request);
        let _ = self.app_event_sender.send_ext_update();
    }

    /// Focus the widget if it is focusable and change the highlight.
    ///
    /// If the widget is not focusable the focus does not move, in this case the highlight changes
    /// for the current focused widget.
    ///
    /// If the widget widget is in a window that is not focused, but is open and not minimized and the app
    /// has keyboard focus in another window; the window is focused and the request processed when the focus event is received.
    /// The [`FocusRequest`] type has other more advanced window focus configurations.
    ///
    /// This makes a [`focus`](Self::focus) request using [`FocusRequest::direct`].
    pub fn focus_widget(&mut self, widget_id: WidgetId, highlight: bool) {
        self.focus(FocusRequest::direct(widget_id, highlight))
    }

    /// Focus the widget if it is focusable, else focus the first focusable parent, also changes the highlight.
    ///
    /// If the widget and no parent are focusable the focus does not move, in this case the highlight changes
    /// for the current focused widget.
    ///
    /// This makes a [`focus`](Self::focus) request using [`FocusRequest::direct_or_exit`].
    pub fn focus_widget_or_exit(&mut self, widget_id: WidgetId, highlight: bool) {
        self.focus(FocusRequest::direct_or_exit(widget_id, highlight))
    }

    /// Focus the widget if it is focusable, else focus the first focusable descendant, also changes the highlight.
    ///
    /// If the widget and no child are focusable the focus does not move, in this case the highlight changes for
    /// the current focused widget.
    ///
    /// This makes a [`focus`](Self::focus) request [`FocusRequest::direct_or_enter`].
    pub fn focus_widget_or_enter(&mut self, widget_id: WidgetId, highlight: bool) {
        self.focus(FocusRequest::direct_or_enter(widget_id, highlight))
    }

    /// Focus the widget if it is focusable, else focus the first focusable descendant, else focus the first
    /// focusable ancestor.
    ///
    /// If the widget no focusable widget is found the focus does not move, in this case the highlight changes
    /// for the current focused widget.
    ///
    /// This makes a [`focus`](Self::focus) request using [`FocusRequest::direct_or_related`].
    pub fn focus_widget_or_related(&mut self, widget_id: WidgetId, highlight: bool) {
        self.focus(FocusRequest::direct_or_related(widget_id, highlight))
    }

    /// Focus the first logical descendant that is focusable from the current focus.
    ///
    /// Does nothing if no widget is focused. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::child`].
    pub fn focus_child(&mut self) {
        self.focus(FocusRequest::child(self.is_highlighting))
    }

    /// Focus the first logical ancestor that is focusable from the current focus.
    ///
    /// Does nothing if no widget is focused. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::parent`].
    pub fn focus_parent(&mut self) {
        self.focus(FocusRequest::parent(self.is_highlighting))
    }

    /// Focus the logical next widget from the current focus.
    ///
    /// Does nothing if no widget is focused. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::next`].
    pub fn focus_next(&mut self) {
        self.focus(FocusRequest::next(self.is_highlighting));
    }

    /// Focus the logical previous widget from the current focus.
    ///
    /// Does nothing if no widget is focused. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::prev`].
    pub fn focus_prev(&mut self) {
        self.focus(FocusRequest::prev(self.is_highlighting));
    }

    /// Focus the closest upward widget from the current focus.
    ///
    /// Does nothing if no widget is focused. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::up`].
    pub fn focus_up(&mut self) {
        self.focus(FocusRequest::up(self.is_highlighting));
    }

    /// Focus the closest widget to the right of the current focus.
    ///
    /// Does nothing if no widget is focused. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::right`].
    pub fn focus_right(&mut self) {
        self.focus(FocusRequest::right(self.is_highlighting));
    }

    /// Focus the closest downward widget from the current focus.
    ///
    /// Does nothing if no widget is focused. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::down`].
    pub fn focus_down(&mut self) {
        self.focus(FocusRequest::down(self.is_highlighting));
    }

    /// Focus the closest widget to the left of the current focus.
    ///
    /// Does nothing if no widget is focused. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::left`].
    pub fn focus_left(&mut self) {
        self.focus(FocusRequest::left(self.is_highlighting));
    }

    /// Focus the ALT scope from the current focused widget.
    ///
    /// Does nothing if no widget is focused. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::alt`].
    pub fn focus_alt(&mut self) {
        self.focus(FocusRequest::alt(self.is_highlighting));
    }

    /// Focus the previous focused widget before the focus was moved to the ALT scope.
    ///
    /// Does nothing if no widget is focused. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::escape_alt`].
    pub fn escape_alt(&mut self) {
        self.focus(FocusRequest::escape_alt(self.is_highlighting));
    }

    #[must_use]
    fn fulfill_request(&mut self, vars: &Vars, windows: &mut Windows, request: FocusRequest) -> Option<FocusChangedArgs> {
        match (&self.focused, request.target) {
            (_, FocusTarget::Direct(widget_id)) => self.focus_direct(vars, windows, widget_id, request.highlight, false, false, request),
            (_, FocusTarget::DirectOrExit(widget_id)) => {
                self.focus_direct(vars, windows, widget_id, request.highlight, false, true, request)
            }
            (_, FocusTarget::DirectOrEnder(widget_id)) => {
                self.focus_direct(vars, windows, widget_id, request.highlight, true, false, request)
            }
            (_, FocusTarget::DirectOrRelated(widget_id)) => {
                self.focus_direct(vars, windows, widget_id, request.highlight, true, true, request)
            }
            (Some(prev), move_) => {
                if let Ok(info) = windows.widget_tree(prev.window_id()) {
                    let info = FocusInfoTree::new(info, self.focus_disabled_widgets);
                    if let Some(w) = info.find(prev.widget_id()) {
                        let mut can_only_highlight = true;
                        if let Some(new_focus) = match move_ {
                            // tabular
                            FocusTarget::Next => w.next_tab(false),
                            FocusTarget::Prev => w.prev_tab(false),
                            FocusTarget::Child => w.tab_descendants().first().copied(),
                            FocusTarget::Parent => w.ancestors().next(),
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
                                    return self.fulfill_request(vars, windows, FocusRequest::escape_alt(request.highlight));
                                } else {
                                    None
                                }
                            }
                            FocusTarget::EscapeAlt => {
                                // Esc does not enable highlight without moving focus.
                                can_only_highlight = false;
                                self.alt_return.as_ref().and_then(|(_, p)| info.get_or_parent(p))
                            }
                            // cases covered by parent match
                            FocusTarget::Direct { .. }
                            | FocusTarget::DirectOrExit { .. }
                            | FocusTarget::DirectOrEnder { .. }
                            | FocusTarget::DirectOrRelated { .. } => {
                                unreachable!()
                            }
                        } {
                            // found `new_focus`
                            self.enabled_nav = new_focus.enabled_nav();
                            self.move_focus(
                                vars,
                                Some(new_focus.info.interaction_path()),
                                request.highlight,
                                FocusChangedCause::Request(request),
                            )
                        } else {
                            // no `new_focus`, maybe update highlight and widget path.
                            self.continue_focus_highlight(
                                vars,
                                windows,
                                if can_only_highlight {
                                    request.highlight
                                } else {
                                    self.is_highlighting
                                },
                            )
                        }
                    } else {
                        // widget not found
                        self.continue_focus_highlight(vars, windows, request.highlight)
                    }
                } else {
                    // window not found
                    self.continue_focus_highlight(vars, windows, request.highlight)
                }
            }
            _ => None,
        }
    }

    /// Checks if `focused()` is still valid, if not moves focus to nearest valid.
    #[must_use]
    fn continue_focus(&mut self, vars: &Vars, windows: &Windows) -> Option<FocusChangedArgs> {
        if let Some(focused) = &self.focused {
            if let Ok(true) = windows.is_focused(focused.window_id()) {
                let info = windows.widget_tree(focused.window_id()).unwrap();
                if let Some(widget) = info.find(focused.widget_id()).map(|w| w.as_focus_info(self.focus_disabled_widgets)) {
                    if widget.is_focusable() {
                        // :-) probably in the same place, maybe moved inside same window.
                        self.enabled_nav = widget.enabled_nav();
                        return self.move_focus(
                            vars,
                            Some(widget.info.interaction_path()),
                            self.is_highlighting,
                            FocusChangedCause::Recovery,
                        );
                    } else {
                        // widget no longer focusable
                        if let Some(parent) = widget.parent() {
                            // move to focusable parent
                            self.enabled_nav = widget.enabled_nav();
                            return self.move_focus(
                                vars,
                                Some(parent.info.interaction_path()),
                                self.is_highlighting,
                                FocusChangedCause::Recovery,
                            );
                        } else {
                            // no focusable parent or root
                            return self.focus_focused_window(vars, windows, self.is_highlighting);
                        }
                    }
                } else {
                    // widget not found, move to focusable known parent
                    for &parent in focused.ancestors().iter().rev() {
                        if let Some(parent) = info.find(parent).and_then(|w| w.as_focusable(self.focus_disabled_widgets)) {
                            // move to focusable parent
                            self.enabled_nav = parent.enabled_nav();
                            return self.move_focus(
                                vars,
                                Some(parent.info.interaction_path()),
                                self.is_highlighting,
                                FocusChangedCause::Recovery,
                            );
                        }
                    }
                }
            } // else window not found or not focused
        } // else no current focus
        self.focus_focused_window(vars, windows, false)
    }

    #[must_use]
    fn continue_focus_highlight(&mut self, vars: &Vars, windows: &Windows, highlight: bool) -> Option<FocusChangedArgs> {
        if let Some(mut args) = self.continue_focus(vars, windows) {
            args.highlight = highlight;
            self.is_highlighting = highlight;
            self.is_highlighting_var.set_ne(vars, highlight);
            Some(args)
        } else if self.is_highlighting != highlight {
            self.is_highlighting = highlight;
            self.is_highlighting_var.set_ne(vars, highlight);
            Some(FocusChangedArgs::now(
                self.focused.clone(),
                self.focused.clone(),
                highlight,
                FocusChangedCause::Recovery,
                self.enabled_nav,
            ))
        } else {
            None
        }
    }

    #[must_use]
    #[allow(clippy::too_many_arguments)]
    fn focus_direct(
        &mut self,
        vars: &Vars,
        windows: &mut Windows,
        widget_id: WidgetId,
        highlight: bool,
        fallback_to_childs: bool,
        fallback_to_parents: bool,
        request: FocusRequest,
    ) -> Option<FocusChangedArgs> {
        let mut target = None;
        if let Some(w) = windows
            .widget_trees()
            .find_map(|info| info.find(widget_id))
            .map(|w| w.as_focus_info(self.focus_disabled_widgets))
        {
            if w.is_focusable() {
                target = Some((w.info.interaction_path(), w.enabled_nav()));
            } else if fallback_to_childs {
                if let Some(w) = w.descendants().next() {
                    target = Some((w.info.interaction_path(), w.enabled_nav()));
                }
            } else if fallback_to_parents {
                if let Some(w) = w.parent() {
                    target = Some((w.info.interaction_path(), w.enabled_nav()));
                }
            }
        }

        if let Some((target, enabled_nav)) = target {
            if let Ok(false) = windows.is_focused(target.window_id()) {
                let requested_win_focus;
                if request.force_window_focus || windows.focused_window_id().is_some() {
                    // if can steal focus from other apps or focus is already in another window of the app.
                    windows.focus(target.window_id()).unwrap();
                    requested_win_focus = true;
                } else if request.window_indicator.is_some() {
                    // if app does not have focus, focus stealing is not allowed, but a request indicator can be set.
                    windows
                        .vars(target.window_id())
                        .unwrap()
                        .focus_indicator()
                        .set(vars, request.window_indicator);
                    requested_win_focus = true;
                } else {
                    requested_win_focus = false;
                }

                if requested_win_focus {
                    self.pending_window_focus = Some((target.window_id(), target.widget_id(), highlight));
                }
                None
            } else {
                self.enabled_nav = enabled_nav;
                self.move_focus(vars, Some(target), highlight, FocusChangedCause::Request(request))
            }
        } else {
            self.change_highlight(vars, highlight, request)
        }
    }

    #[must_use]
    fn change_highlight(&mut self, vars: &Vars, highlight: bool, request: FocusRequest) -> Option<FocusChangedArgs> {
        if self.is_highlighting != highlight {
            self.is_highlighting = highlight;
            self.is_highlighting_var.set_ne(vars, highlight);
            Some(FocusChangedArgs::now(
                self.focused.clone(),
                self.focused.clone(),
                highlight,
                FocusChangedCause::Request(request),
                self.enabled_nav,
            ))
        } else {
            None
        }
    }

    #[must_use]
    fn focus_focused_window(&mut self, vars: &Vars, windows: &Windows, highlight: bool) -> Option<FocusChangedArgs> {
        if let Some(info) = windows.focused_info() {
            let info = FocusInfoTree::new(info, self.focus_disabled_widgets);
            if let Some(root) = info.focusable_root() {
                // found focused window and it is focusable.
                self.enabled_nav = root.enabled_nav();
                self.move_focus(vars, Some(root.info.interaction_path()), highlight, FocusChangedCause::Recovery)
            } else {
                // has focused window but it is not focusable.
                self.enabled_nav = FocusNavAction::empty();
                self.move_focus(vars, None, false, FocusChangedCause::Recovery)
            }
        } else {
            // no focused window
            self.enabled_nav = FocusNavAction::empty();
            self.move_focus(vars, None, false, FocusChangedCause::Recovery)
        }
    }

    #[must_use]
    fn move_focus(
        &mut self,
        vars: &Vars,
        new_focus: Option<InteractionPath>,
        highlight: bool,
        cause: FocusChangedCause,
    ) -> Option<FocusChangedArgs> {
        let prev_highlight = std::mem::replace(&mut self.is_highlighting, highlight);
        self.is_highlighting_var.set_ne(vars, highlight);

        if self.focused != new_focus {
            let args = FocusChangedArgs::now(
                self.focused.take(),
                new_focus.clone(),
                self.is_highlighting,
                cause,
                self.enabled_nav,
            );
            self.focused = new_focus.clone();
            self.focused_var.set(vars, new_focus); // this can happen more than once per update, so we can't use set_ne.
            Some(args)
        } else if prev_highlight != highlight {
            Some(FocusChangedArgs::now(
                new_focus.clone(),
                new_focus,
                highlight,
                cause,
                self.enabled_nav,
            ))
        } else {
            None
        }
    }

    #[must_use]
    fn move_after_focus(&mut self, vars: &Vars, windows: &Windows, reverse: bool) -> Option<FocusChangedArgs> {
        if let Some(focused) = &self.focused {
            if let Some(info) = windows.focused_info() {
                if let Some(widget) = FocusInfoTree::new(info, self.focus_disabled_widgets).get(focused) {
                    if widget.is_scope() {
                        if let Some(widget) = widget.on_focus_scope_move(|id| self.return_focused.get(&id).map(|p| p.as_path()), reverse) {
                            self.enabled_nav = widget.enabled_nav();
                            return self.move_focus(
                                vars,
                                Some(widget.info.interaction_path()),
                                self.is_highlighting,
                                FocusChangedCause::ScopeGotFocus(reverse),
                            );
                        }
                    }
                }
            }
        }
        None
    }

    /// Updates `return_focused` and `alt_return` after `focused` changed.
    #[must_use]
    fn update_returns(&mut self, vars: &Vars, prev_focus: Option<InteractionPath>, windows: &Windows) -> Vec<ReturnFocusChangedArgs> {
        let mut r = vec![];

        if let Some((scope, _)) = &self.alt_return {
            // if we have an `alt_return` check if is still inside the ALT.

            let mut retain_alt = false;
            if let Some(new_focus) = &self.focused {
                if new_focus.contains(scope.widget_id()) {
                    retain_alt = true; // just a focus move inside the ALT.
                }
            }

            if !retain_alt {
                let (scope, widget_path) = self.alt_return.take().unwrap();
                self.alt_return_var.set_ne(vars, None);
                r.push(ReturnFocusChangedArgs::now(scope, Some(widget_path), None));
            }
        } else if let Some(new_focus) = &self.focused {
            // if we don't have an `alt_return` but focused something, check if focus
            // moved inside an ALT.

            if let Ok(info) = windows.widget_tree(new_focus.window_id()) {
                if let Some(widget) = FocusInfoTree::new(info, self.focus_disabled_widgets).get(new_focus) {
                    let alt_scope = if widget.is_alt_scope() {
                        Some(widget)
                    } else {
                        widget.scopes().find(|s| s.is_alt_scope())
                    };
                    if let Some(alt_scope) = alt_scope {
                        let scope = alt_scope.info.interaction_path();
                        // entered an alt_scope.

                        if let Some(prev) = &prev_focus {
                            // previous focus is the return.
                            r.push(ReturnFocusChangedArgs::now(scope.clone(), None, Some(prev.clone())));
                            self.alt_return = Some((scope, prev.clone()));
                            self.alt_return_var.set(vars, prev.clone());
                        } else if let Some(parent) = alt_scope.parent() {
                            // no previous focus, ALT parent is the return.
                            let parent_path = parent.info.interaction_path();
                            r.push(ReturnFocusChangedArgs::now(scope.clone(), None, Some(parent_path.clone())));
                            self.alt_return = Some((scope, parent_path.clone()));
                            self.alt_return_var.set(vars, parent_path);
                        }
                    }
                }
            }
        }

        /*
         *   Update `return_focused`
         */

        if let Some(new_focus) = &self.focused {
            if let Ok(info) = windows.widget_tree(new_focus.window_id()) {
                if let Some(widget) = FocusInfoTree::new(info, self.focus_disabled_widgets).get(new_focus) {
                    if widget.scopes().all(|s| !s.is_alt_scope()) {
                        // if not inside ALT, update return for each LastFocused parent scopes.

                        for scope in widget
                            .scopes()
                            .filter(|s| s.focus_info().scope_on_focus() == FocusScopeOnFocus::LastFocused)
                        {
                            let scope = scope.info.interaction_path();
                            let path = widget.info.interaction_path();
                            if let Some(current) = self.return_focused.get_mut(&scope.widget_id()) {
                                if current != &path {
                                    let prev = std::mem::replace(current, path);
                                    self.return_focused_var.get(&scope.widget_id()).unwrap().set(vars, current.clone());
                                    r.push(ReturnFocusChangedArgs::now(scope, Some(prev), Some(current.clone())));
                                }
                            } else {
                                self.return_focused.insert(scope.widget_id(), path.clone());
                                match self.return_focused_var.entry(scope.widget_id()) {
                                    hash_map::Entry::Occupied(e) => e.get().set(vars, Some(path.clone())),
                                    hash_map::Entry::Vacant(e) => {
                                        e.insert(var(Some(path.clone())));
                                    }
                                }
                                r.push(ReturnFocusChangedArgs::now(scope, None, Some(path)));
                            }
                        }
                    }
                }
            }
        }

        r
    }

    /// Cleanup `return_focused` and `alt_return` after new widget tree.
    #[must_use]
    fn cleanup_returns(&mut self, vars: &Vars, info: FocusInfoTree) -> Vec<ReturnFocusChangedArgs> {
        let mut r = vec![];

        if self.return_focused_var.len() > 20 {
            self.return_focused_var
                .retain(|_, var| var.strong_count() > 1 || var.get(vars).is_some())
        }

        self.return_focused.retain(|&scope_id, widget_path| {
            if widget_path.window_id() != info.tree.window_id() {
                return true; // retain, not same window.
            }

            let mut retain = false;

            if let Some(widget) = info.get(widget_path) {
                if let Some(scope) = widget.scopes().find(|s| s.info.widget_id() == scope_id) {
                    if scope.focus_info().scope_on_focus() == FocusScopeOnFocus::LastFocused {
                        retain = true; // retain, widget still exists in same scope and scope still is LastFocused.

                        let path = widget.info.interaction_path();
                        if &path != widget_path {
                            // widget moved inside scope.
                            r.push(ReturnFocusChangedArgs::now(
                                scope.info.interaction_path(),
                                Some(widget_path.clone()),
                                Some(path.clone()),
                            ));
                            *widget_path = path;
                        }
                    }
                } else if let Some(scope) = info.find(scope_id) {
                    if scope.focus_info().scope_on_focus() == FocusScopeOnFocus::LastFocused {
                        // widget not inside scope anymore, but scope still exists and is valid.
                        if let Some(first) = scope.first_tab_descendant() {
                            // LastFocused goes to the first descendant as fallback.
                            retain = true;

                            let path = first.info.interaction_path();
                            r.push(ReturnFocusChangedArgs::now(
                                scope.info.interaction_path(),
                                Some(widget_path.clone()),
                                Some(path.clone()),
                            ));
                            *widget_path = path;
                        }
                    }
                }
            } else if let Some(parent) = info.get_or_parent(widget_path) {
                // widget not in window anymore, but a focusable parent is..
                if let Some(scope) = parent.scopes().find(|s| s.info.widget_id() == scope_id) {
                    if scope.focus_info().scope_on_focus() == FocusScopeOnFocus::LastFocused {
                        // ..and the parent is inside the scope, and the scope is still valid.
                        retain = true;

                        let path = parent.info.interaction_path();
                        r.push(ReturnFocusChangedArgs::now(
                            scope.info.interaction_path(),
                            Some(widget_path.clone()),
                            Some(path.clone()),
                        ));
                        *widget_path = path;
                    }
                }
            }

            if !retain {
                let scope_path = info.find(scope_id).map(|i| i.info.interaction_path());

                if scope_path.is_some() {
                    match self.return_focused_var.entry(scope_id) {
                        hash_map::Entry::Occupied(e) => {
                            if e.get().strong_count() == 1 {
                                e.remove();
                            } else {
                                e.get().set(vars, None);
                            }
                        }
                        hash_map::Entry::Vacant(_) => {}
                    }
                } else if let Some(var) = self.return_focused_var.remove(&scope_id) {
                    if var.strong_count() > 1 {
                        var.set(vars, None);
                    }
                }

                r.push(ReturnFocusChangedArgs::now(scope_path, Some(widget_path.clone()), None));
            }
            retain
        });

        let mut retain_alt = true;
        if let Some((scope, widget_path)) = &mut self.alt_return {
            if widget_path.window_id() == info.tree.window_id() {
                // we need to update alt_return

                retain_alt = false; // will retain only if still valid

                if let Some(widget) = info.get(widget_path) {
                    if !widget.scopes().any(|s| s.info.widget_id() == scope.widget_id()) {
                        retain_alt = true; // retain, widget still exists outside of the ALT scope.

                        let path = widget.info.interaction_path();
                        if &path != widget_path {
                            // widget moved outside ALT scope.
                            r.push(ReturnFocusChangedArgs::now(scope.clone(), Some(widget_path.clone()), Some(path)));
                        }
                    }
                } else if let Some(parent) = info.get_or_parent(widget_path) {
                    // widget not in window anymore, but a focusable parent is..
                    if !parent.scopes().any(|s| s.info.widget_id() == scope.widget_id()) {
                        // ..and the parent is not inside the ALT scope.
                        retain_alt = true;

                        let path = parent.info.interaction_path();
                        r.push(ReturnFocusChangedArgs::now(
                            scope.clone(),
                            Some(widget_path.clone()),
                            Some(path.clone()),
                        ));
                        *widget_path = path.clone();
                        self.alt_return_var.set(vars, path)
                    }
                }
            }
        }
        if !retain_alt {
            let (scope_id, widget_path) = self.alt_return.take().unwrap();
            self.alt_return_var.set(vars, None);
            r.push(ReturnFocusChangedArgs::now(scope_id, Some(widget_path), None));
        }

        r
    }

    /// Cleanup `return_focused` and `alt_return` after a window closed.
    #[must_use]
    fn cleanup_returns_win_closed(&mut self, vars: &Vars, window_id: WindowId) -> Vec<ReturnFocusChangedArgs> {
        let mut r = vec![];

        if self
            .alt_return
            .as_ref()
            .map(|(_, w)| w.window_id() == window_id)
            .unwrap_or_default()
        {
            let (_, widget_path) = self.alt_return.take().unwrap();
            self.alt_return_var.set_ne(vars, None);
            r.push(ReturnFocusChangedArgs::now(None, Some(widget_path), None));
        }

        self.return_focused.retain(|&scope_id, widget_path| {
            let retain = widget_path.window_id() != window_id;

            if !retain {
                let var = self.return_focused_var.remove(&scope_id).unwrap();
                var.set(vars, None);

                r.push(ReturnFocusChangedArgs::now(None, Some(widget_path.clone()), None));
            }

            retain
        });

        r
    }
}
