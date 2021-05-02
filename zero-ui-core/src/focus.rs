//! Keyboard focus manager.
//!
//! The [`FocusManager`] struct is an [app extension](crate::app::AppExtension). It
//! is included in the [default app](crate::app::App::default) and provides the [`Focus`] service
//! and focus events.
//!
//! # Keyboard Focus
//!
//! In a given program only a single widget can receive keyboard input at a time, this widget has the *keyboard focus*.
//! This is an extension of the operating system own focus manager that controls which window is *active*, receiving
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
//! Tab navigation follows a logical order, the position of the widget in the [widget tree](FrameFocusInfo),
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
//! Focus scopes can be configured to remember the last focused widget inside then, the focus then **returns** to
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
//! Focus information exists as metadata associated with a window frame. This metadata can be manually queried by
//! creating a [`FrameFocusInfo`] or directly from a widget info by using the [`WidgetInfoFocusExt`] extension methods.

use crate::app::{AppExtension, DeviceEvent, DeviceId};
use crate::context::*;
use crate::event::*;
use crate::gesture::{shortcut, ShortcutArgs, ShortcutEvent};
use crate::mouse::{MouseDownEvent, MouseInputArgs};
use crate::render::DescendantFilter;
use crate::render::{FrameInfo, WidgetInfo, WidgetPath};
use crate::service::AppService;
use crate::units::LayoutPoint;
use crate::widget_base::WidgetEnabledExt;
use crate::window::{WindowId, WindowIsActiveArgs, WindowIsActiveChangedEvent, Windows};
use crate::WidgetId;
use fnv::FnvHashMap;
use std::time::{Duration, Instant};

event_args! {
    /// [`FocusChangedEvent`] arguments.
    pub struct FocusChangedArgs {
        /// Previously focused widget.
        pub prev_focus: Option<WidgetPath>,

        /// Newly focused widget.
        pub new_focus: Option<WidgetPath>,

        /// If the focused widget should visually indicate that it is focused.
        ///
        /// This is `true` when the focus change is caused by a key press, `false` when it is caused by a mouse click.
        ///
        /// Some widgets, like *text input*, may ignore this field and always indicate that they are focused.
        pub highlight: bool,

        /// What caused this event.
        pub cause: FocusChangedCause,

        ..

        /// If the widget is [`prev_focus`](Self::prev_focus) or
        /// [`new_focus`](Self::new_focus).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            if let Some(prev) = &self.prev_focus {
                if prev.contains(ctx.path.widget_id()) {
                    return true
                }
            }

            if let Some(new) = &self.new_focus {
                if new.contains(ctx.path.widget_id()) {
                    return true
                }
            }

            false
        }
    }

    /// [`ReturnFocusChangedEvent`] arguments.
    pub struct ReturnFocusChangedArgs {
        /// The scope that returns the focus when focused directly.
        pub scope_id : WidgetId,

        /// Previous return focus of the widget.
        pub prev_return: Option<WidgetPath>,

        /// New return focus of the widget.
        pub new_return: Option<WidgetPath>,

        ..

        /// If the widget is [`prev_return`](Self::prev_return), [`new_return`](Self::new_return)
        /// or [`scope_id`](Self::scope_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            if let Some(prev) = &self.prev_return {
                if prev.widget_id() == ctx.path.widget_id() {
                    return true
                }
            }

            if let Some(new) = &self.new_return {
                if new.widget_id() == ctx.path.widget_id() {
                    return true
                }

            }
            self.scope_id == ctx.path.widget_id()
        }
    }
}

impl FocusChangedArgs {
    /// If the focus is still in the same widget but the widget path changed.
    #[inline]
    pub fn is_widget_move(&self) -> bool {
        match (&self.prev_focus, &self.new_focus) {
            (Some(prev), Some(new)) => prev.widget_id() == new.widget_id() && prev != new,
            _ => false,
        }
    }

    /// If the focus is still in the same widget but [`highlight`](FocusChangedArgs::highlight) changed.
    #[inline]
    pub fn is_hightlight_changed(&self) -> bool {
        self.prev_focus == self.new_focus
    }

    /// If `widget_id` is the new focus.
    #[inline]
    pub fn is_focus(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_focus, &self.new_focus) {
            (Some(prev), Some(new)) => prev.widget_id() != widget_id && new.widget_id() == widget_id,
            (None, Some(new)) => new.widget_id() == widget_id,
            (_, None) => false,
        }
    }

    /// If `widget_id` is the previous focus.
    #[inline]
    pub fn is_blur(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_focus, &self.new_focus) {
            (Some(prev), Some(new)) => prev.widget_id() == widget_id && new.widget_id() != widget_id,
            (Some(prev), None) => prev.widget_id() == widget_id,
            (None, _) => false,
        }
    }

    /// If `widget_id` is the new focus or a parent of the new focus and was not a parent of the previous focus.
    #[inline]
    pub fn is_focus_enter(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_focus, &self.new_focus) {
            (Some(prev), Some(new)) => !prev.contains(widget_id) && new.contains(widget_id),
            (None, Some(new)) => new.contains(widget_id),
            (_, None) => false,
        }
    }

    /// If `widget_id` is the previous focus or a parent of the previous focus and is not a parent of the new focus.
    #[inline]
    pub fn is_focus_leave(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_focus, &self.new_focus) {
            (Some(prev), Some(new)) => prev.contains(widget_id) && !new.contains(widget_id),
            (Some(prev), None) => prev.contains(widget_id),
            (None, _) => false,
        }
    }
}

impl ReturnFocusChangedArgs {
    /// If the return focus is the same widget but the widget path changed and the widget is still in the same focus scope.
    #[inline]
    pub fn is_widget_move(&self) -> bool {
        match (&self.prev_return, &self.new_return) {
            (Some(prev), Some(new)) => prev.widget_id() == new.widget_id() && prev != new,
            _ => false,
        }
    }

    /// If [`scope_id`](Self::scope_id) is an ALT scope and `prev_return` or `new_return` if the
    /// widget outside the scope that will be focused back when the user escapes the ALT scope.
    #[inline]
    pub fn is_alt_return(&self) -> bool {
        match (&self.prev_return, &self.new_return) {
            (Some(prev), None) => !prev.contains(self.scope_id),
            (None, Some(new)) => !new.contains(self.scope_id),
            _ => false,
        }
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
    #[inline]
    pub fn is_prev_request(self) -> bool {
        match self {
            FocusChangedCause::Request(r) => matches!(r.target, FocusTarget::Prev),
            _ => false,
        }
    }
}

state_key! {
    /// Reference to the [`FocusInfoBuilder`] in the widget state of a frame.
    pub struct FocusInfoKey: FocusInfoBuilder;
}

/// Widget tab navigation position within a focus scope.
///
/// The index is zero based, zero first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct TabIndex(pub u32);
impl TabIndex {
    /// Widget is skipped during tab navigation.
    ///
    /// The integer value is `u32::MAX`.
    pub const SKIP: TabIndex = TabIndex(u32::MAX);

    /// Default focusable widget index.
    ///
    /// Tab navigation uses the widget position in the widget tree when multiple widgets have the same index
    /// so if no widget index is explicitly set they get auto-sorted by their position.
    ///
    /// The integer value is `u32::MAX / 2`.
    pub const AUTO: TabIndex = TabIndex(u32::MAX / 2);

    /// If is [`SKIP`](TabIndex::SKIP).
    #[inline]
    pub fn is_skip(self) -> bool {
        self == Self::SKIP
    }

    /// If is [`AUTO`](TabIndex::AUTO).
    #[inline]
    pub fn is_auto(self) -> bool {
        self == Self::AUTO
    }

    /// Create a new tab index that is guaranteed to not be [`SKIP`](Self::SKIP).
    ///
    /// Returns `SKIP - 1` if `index` is `SKIP`.
    #[inline]
    pub fn not_skip(index: u32) -> Self {
        TabIndex(if index == Self::SKIP.0 { Self::SKIP.0 - 1 } else { index })
    }

    /// Create a new tab index that is guaranteed to be before [`AUTO`](Self::AUTO).
    ///
    /// Returns `AUTO - 1` if `index` is equal to or greater then `AUTO`.
    #[inline]
    pub fn before_auto(index: u32) -> Self {
        TabIndex(if index >= Self::AUTO.0 { Self::AUTO.0 - 1 } else { index })
    }

    /// Create a new tab index that is guaranteed to be after [`AUTO`](Self::AUTO) and not [`SKIP`](Self::SKIP).
    ///
    /// The `index` argument is zero based here.
    ///
    /// Returns `not_skip(AUTO + 1 + index)`.
    #[inline]
    pub fn after_auto(index: u32) -> Self {
        Self::not_skip((Self::AUTO.0 + 1).saturating_add(index))
    }
}
impl Default for TabIndex {
    /// `AUTO`
    fn default() -> Self {
        TabIndex::AUTO
    }
}
impl_from_and_into_var! {
    /// Calls [`TabIndex::not_skip`].
    fn from(index: u32) -> TabIndex {
        TabIndex::not_skip(index)
    }
}

/// Tab navigation configuration of a focus scope.
///
/// See the [module level](crate::focus#tab-navigation) for an overview of tab navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TabNav {
    /// Tab moves into the scope but does not move the focus inside the scope.
    None,
    /// Tab moves the focus through the scope continuing out after the last item.
    Continue,
    /// Tab is contained in the scope, does not move after the last item.
    Contained,
    /// Tab is contained in the scope, after the last item moves to the first item in the scope.
    Cycle,
    /// Tab moves into the scope once but then moves out of the scope.
    Once,
}

/// Directional navigation configuration of a focus scope.
///
/// See the [module level](crate::focus#directional-navigation) for an overview of directional navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DirectionalNav {
    /// Arrows does not move the focus inside the scope.
    None,
    /// Arrows move the focus through the scope continuing out of the edges.
    Continue,
    /// Arrows move the focus inside the scope only, stops at the edges.
    Contained,
    /// Arrows move the focus inside the scope only, cycles back to oppose edges.
    Cycle,
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
/// This extension is included in the [default app](crate::app::App::default).
///
/// # Dependencies
///
/// This extension requires the [`MouseDownEvent`],
/// [`ShortcutEvent`] and [`WindowIsActiveChangedEvent`]
///  events during initialization to function.
///
/// # About Focus
///
/// See the [module level](crate::focus) documentation for an overview of the keyboard
/// focus concepts implemented by this app extension.
pub struct FocusManager {
    focus_changed: EventEmitter<FocusChangedArgs>,
    return_focus_changed: EventEmitter<ReturnFocusChangedArgs>,
    windows_activation: EventListener<WindowIsActiveArgs>,
    mouse_down: EventListener<MouseInputArgs>,
    shortcut: EventListener<ShortcutArgs>,
    focused: Option<WidgetPath>,
    last_keyboard_event: Instant,
}
impl Default for FocusManager {
    fn default() -> Self {
        Self {
            focus_changed: FocusChangedEvent::emitter(),
            return_focus_changed: ReturnFocusChangedEvent::emitter(),
            windows_activation: WindowIsActiveChangedEvent::never(),
            mouse_down: MouseDownEvent::never(),
            shortcut: ShortcutEvent::never(),
            focused: None,
            last_keyboard_event: Instant::now() - Duration::from_secs(10),
        }
    }
}
impl AppExtension for FocusManager {
    fn init(&mut self, ctx: &mut AppInitContext) {
        self.windows_activation = ctx.events.listen::<WindowIsActiveChangedEvent>();
        self.mouse_down = ctx.events.listen::<MouseDownEvent>();
        self.shortcut = ctx.events.listen::<ShortcutEvent>();

        ctx.services.register(Focus::new(ctx.updates.notifier().clone()));

        ctx.events.register::<FocusChangedEvent>(self.focus_changed.listener());
        ctx.events.register::<ReturnFocusChangedEvent>(self.return_focus_changed.listener());
    }

    fn update(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        if update.update_hp {
            return;
        }

        let mut request = None;

        let focus = ctx.services.req::<Focus>();
        if let Some(req) = focus.request.take() {
            // custom
            request = Some(req);
        } else if let Some(args) = self.mouse_down.updates(ctx.events).last() {
            // click
            request = Some(FocusRequest::direct_or_parent(args.target.widget_id(), false));
        } else if let Some(args) = self.shortcut.updates(ctx.events).last() {
            // keyboard
            if args.shortcut == shortcut!(Tab) {
                request = Some(FocusRequest::next(true))
            } else if args.shortcut == shortcut!(SHIFT + Tab) {
                request = Some(FocusRequest::prev(true))
            } else if args.shortcut == shortcut!(Alt) {
                request = Some(FocusRequest::alt(true))
            } else if args.shortcut == shortcut!(Escape) {
                request = Some(FocusRequest::escape_alt(true))
            } else if args.shortcut == shortcut!(Up) {
                request = Some(FocusRequest::up(true))
            } else if args.shortcut == shortcut!(Right) {
                request = Some(FocusRequest::right(true))
            } else if args.shortcut == shortcut!(Down) {
                request = Some(FocusRequest::down(true))
            } else if args.shortcut == shortcut!(Left) {
                request = Some(FocusRequest::left(true))
            }
        }

        if let Some(request) = request {
            let (focus, windows) = ctx.services.req_multi::<(Focus, Windows)>();
            self.notify(focus.fulfill_request(request, windows), focus, windows, ctx.events);
        } else if self.windows_activation.has_updates(ctx.events) {
            // foreground window maybe changed
            let (focus, windows) = ctx.services.req_multi::<(Focus, Windows)>();
            if let Some(mut args) = focus.continue_focus(windows) {
                if !args.highlight && args.new_focus.is_some() && (Instant::now() - self.last_keyboard_event) < Duration::from_millis(300) {
                    // window probably activated using keyboard.
                    args.highlight = true;
                    focus.is_highlighting = true;
                }
                self.notify(Some(args), focus, windows, ctx.events);
                // TODO alt scope focused just before ALT+TAB clears the focus.
            }

            for close in self.windows_activation.updates(ctx.events).iter().filter(|a| a.closed) {
                for args in focus.cleanup_returns_win_closed(close.window_id) {
                    self.return_focus_changed.notify(ctx.events, args);
                }
            }
        }
    }

    fn on_device_event(&mut self, _: DeviceId, event: &DeviceEvent, _: &mut AppContext) {
        if let DeviceEvent::Key(_) = event {
            self.last_keyboard_event = Instant::now();
        }
    }

    fn on_new_frame_ready(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        let (focus, windows) = ctx.services.req_multi::<(Focus, Windows)>();

        if self.focused.as_ref().map(|f| f.window_id() == window_id).unwrap_or_default() {
            // new window frame, check if focus is still valid
            self.notify(focus.continue_focus(windows), focus, windows, ctx.events);
        }

        for args in focus.cleanup_returns(FrameFocusInfo::new(
            windows.window(window_id).expect("window in on_new_frame_ready").frame_info(),
        )) {
            self.return_focus_changed.notify(&ctx.events, args);
        }
    }
}
impl FocusManager {
    fn notify(&mut self, args: Option<FocusChangedArgs>, focus: &mut Focus, windows: &mut Windows, events: &Events) {
        if let Some(args) = args {
            let reverse = args.cause.is_prev_request();
            let prev_focus = args.prev_focus.clone();
            self.focused = args.new_focus.clone();
            self.focus_changed.notify(events, args);

            // may have focused scope.
            while let Some(after_args) = focus.move_after_focus(windows, reverse) {
                self.focused = after_args.new_focus.clone();
                self.focus_changed.notify(events, after_args);
            }

            for return_args in focus.update_returns(prev_focus, windows) {
                self.return_focus_changed.notify(events, return_args);
            }
        }
    }
}

/// Keyboard focus service.
///
/// # Provider
///
/// This service is provided by the [`FocusManager`] extension.
#[derive(AppService)]
pub struct Focus {
    request: Option<FocusRequest>,
    update_notifier: UpdateNotifier,
    focused: Option<WidgetPath>,
    return_focused: FnvHashMap<WidgetId, WidgetPath>,
    alt_return: Option<(WidgetId, WidgetPath)>,
    is_highlighting: bool,
}
impl Focus {
    /// New focus service, the `update_notifier` is used to flag an update after a focus change request.
    #[inline]
    #[must_use]
    pub fn new(update_notifier: UpdateNotifier) -> Self {
        Focus {
            request: None,
            update_notifier,
            focused: None,
            is_highlighting: false,
            return_focused: FnvHashMap::default(),
            alt_return: None,
        }
    }

    /// Current focused widget.
    #[inline]
    #[must_use]
    pub fn focused(&self) -> Option<&WidgetPath> {
        self.focused.as_ref()
    }

    /// Current return focus of a scope.
    #[inline]
    #[must_use]
    pub fn return_focused(&self, scope_id: WidgetId) -> Option<&WidgetPath> {
        self.return_focused.get(&scope_id)
    }

    /// Current ALT return focus.
    #[inline]
    #[must_use]
    pub fn alt_return(&self) -> Option<&WidgetPath> {
        self.alt_return.as_ref().map(|(_, p)| p)
    }

    /// If focus is in an ALT scope.
    #[inline]
    #[must_use]
    pub fn in_alt(&self) -> bool {
        self.alt_return.is_some()
    }

    /// If the current focused widget is visually indicated.
    #[inline]
    #[must_use]
    pub fn is_highlighting(&self) -> bool {
        self.is_highlighting
    }

    /// Request a focus update.
    #[inline]
    pub fn focus(&mut self, request: FocusRequest) {
        self.request = Some(request);
        self.update_notifier.update();
    }

    /// Focus the widget if it is focusable and change the highlight.
    ///
    /// If the widget is not focusable the focus does not move, in this case the highlight changes
    /// for the current focused widget.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::direct`].
    #[inline]
    pub fn focus_widget(&mut self, widget_id: WidgetId, highlight: bool) {
        self.focus(FocusRequest::direct(widget_id, highlight))
    }

    /// Focus the widget if it is focusable, else focus the first focusable parent, also changes the highlight.
    ///
    /// If the widget and no parent if focusable the focus does not move, in this case the highlight changes
    /// for the current focused widget.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::direct_or_parent`].
    #[inline]
    pub fn focus_widget_or_parent(&mut self, widget_id: WidgetId, highlight: bool) {
        self.focus(FocusRequest::direct_or_parent(widget_id, highlight))
    }

    /// Focus the logical next widget from the current focus.
    ///
    /// Does nothing if no widget is focused. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::next`].
    #[inline]
    pub fn focus_next(&mut self) {
        self.focus(FocusRequest::next(self.is_highlighting));
    }

    /// Focus the logical previous widget from the current focus.
    ///
    /// Does nothing if no widget is focused. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::prev`].
    #[inline]
    pub fn focus_prev(&mut self) {
        self.focus(FocusRequest::prev(self.is_highlighting));
    }

    /// Focus the closest upward widget from the current focus.
    ///
    /// Does nothing if no widget is focused. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::up`].
    #[inline]
    pub fn focus_up(&mut self) {
        self.focus(FocusRequest::up(self.is_highlighting));
    }

    /// Focus the closest widget to the right of the current focus.
    ///
    /// Does nothing if no widget is focused. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::right`].
    #[inline]
    pub fn focus_right(&mut self) {
        self.focus(FocusRequest::right(self.is_highlighting));
    }

    /// Focus the closest downward widget from the current focus.
    ///
    /// Does nothing if no widget is focused. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::down`].
    #[inline]
    pub fn focus_down(&mut self) {
        self.focus(FocusRequest::down(self.is_highlighting));
    }

    /// Focus the closest widget to the left of the current focus.
    ///
    /// Does nothing if no widget is focused. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::left`].
    #[inline]
    pub fn focus_left(&mut self) {
        self.focus(FocusRequest::left(self.is_highlighting));
    }

    /// Focus the ALT scope from the current focused widget.
    ///
    /// Does nothing if no widget is focused. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::alt`].
    #[inline]
    pub fn focus_alt(&mut self) {
        self.focus(FocusRequest::alt(self.is_highlighting));
    }

    /// Focus the previous focused widget before the focus was moved to the ALT scope.
    ///
    /// Does nothing if no widget is focused. Continues highlighting the new focus if the current is highlighted.
    ///
    /// This is makes a [`focus`](Self::focus) request using [`FocusRequest::escape_alt`].
    #[inline]
    pub fn escape_alt(&mut self) {
        self.focus(FocusRequest::escape_alt(self.is_highlighting));
    }

    #[must_use]
    fn fulfill_request(&mut self, request: FocusRequest, windows: &Windows) -> Option<FocusChangedArgs> {
        match (&self.focused, request.target) {
            (_, FocusTarget::Direct(widget_id)) => self.focus_direct(widget_id, request.highlight, false, windows, request),
            (_, FocusTarget::DirectOrParent(widget_id)) => self.focus_direct(widget_id, request.highlight, true, windows, request),
            (Some(prev), move_) => {
                if let Ok(w) = windows.window(prev.window_id()) {
                    let frame = FrameFocusInfo::new(w.frame_info());
                    if let Some(w) = frame.find(prev.widget_id()) {
                        let mut can_only_highlight = true;
                        if let Some(new_focus) = match move_ {
                            // tabular
                            FocusTarget::Next => w.next_tab(false),
                            FocusTarget::Prev => w.prev_tab(false),
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
                                    return self.fulfill_request(FocusRequest::escape_alt(request.highlight), windows);
                                } else {
                                    None
                                }
                            }
                            FocusTarget::EscapeAlt => {
                                // Esc does not enable highlight without moving focus.
                                can_only_highlight = false;
                                self.alt_return.as_ref().and_then(|(_, p)| frame.get_or_parent(&p))
                            }
                            // cases covered by parent match
                            FocusTarget::Direct { .. } | FocusTarget::DirectOrParent { .. } => unreachable!(),
                        } {
                            // found `new_focus`
                            self.move_focus(Some(new_focus.info.path()), request.highlight, FocusChangedCause::Request(request))
                        } else {
                            // no `new_focus`, maybe update highlight and widget path.
                            self.continue_focus_highlight(
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
                        self.continue_focus_highlight(windows, request.highlight)
                    }
                } else {
                    // window not found
                    self.continue_focus_highlight(windows, request.highlight)
                }
            }
            _ => None,
        }
    }

    /// Checks if `focused()` is still valid, if not moves focus to nearest valid.
    #[must_use]
    fn continue_focus(&mut self, windows: &Windows) -> Option<FocusChangedArgs> {
        if let Some(focused) = &self.focused {
            if let Ok(window) = windows.window(focused.window_id()) {
                if window.is_active() {
                    if let Some(widget) = window.frame_info().find(focused.widget_id()).map(|w| w.as_focus_info()) {
                        if widget.is_focusable() {
                            // :-) probably in the same place, maybe moved inside same window.
                            self.move_focus(Some(widget.info.path()), self.is_highlighting, FocusChangedCause::Recovery)
                        } else {
                            // widget no longer focusable
                            if let Some(parent) = widget.parent() {
                                // move to focusable parent
                                self.move_focus(Some(parent.info.path()), self.is_highlighting, FocusChangedCause::Recovery)
                            } else {
                                // no focusable parent, is this an error?
                                self.move_focus(None, false, FocusChangedCause::Recovery)
                            }
                        }
                    } else {
                        // widget not found
                        self.continue_focus_moved_widget(windows)
                    }
                } else {
                    // window not active anymore
                    self.continue_focus_moved_widget(windows)
                }
            } else {
                // window not found
                self.continue_focus_moved_widget(windows)
            }
        } else {
            // no previous focus
            self.focus_active_window(windows, false)
        }
    }

    #[must_use]
    fn continue_focus_moved_widget(&mut self, windows: &Windows) -> Option<FocusChangedArgs> {
        let focused = self.focused.as_ref().unwrap();
        for window in windows.windows() {
            if let Some(widget) = window.frame_info().find(focused.widget_id()).map(|w| w.as_focus_info()) {
                // found the widget in another window
                if window.is_active() {
                    return if widget.is_focusable() {
                        // same widget, moved to another window
                        self.move_focus(Some(widget.info.path()), self.is_highlighting, FocusChangedCause::Recovery)
                    } else {
                        // widget no longer focusable
                        if let Some(parent) = widget.parent() {
                            // move to focusable parent
                            self.move_focus(Some(parent.info.path()), self.is_highlighting, FocusChangedCause::Recovery)
                        } else {
                            // no focusable parent, is this an error?
                            self.move_focus(None, false, FocusChangedCause::Recovery)
                        }
                    };
                }
                break;
            }
        }
        // did not find the widget in a focusable context, was removed or is inside an inactive window.
        self.focus_active_window(windows, self.is_highlighting)
    }

    #[must_use]
    fn continue_focus_highlight(&mut self, windows: &Windows, highlight: bool) -> Option<FocusChangedArgs> {
        if let Some(mut args) = self.continue_focus(windows) {
            args.highlight = highlight;
            self.is_highlighting = highlight;
            Some(args)
        } else if self.is_highlighting != highlight {
            self.is_highlighting = highlight;
            Some(FocusChangedArgs::now(
                self.focused.clone(),
                self.focused.clone(),
                highlight,
                FocusChangedCause::Recovery,
            ))
        } else {
            None
        }
    }

    #[must_use]
    fn focus_direct(
        &mut self,
        widget_id: WidgetId,
        highlight: bool,
        fallback_to_parents: bool,
        windows: &Windows,
        request: FocusRequest,
    ) -> Option<FocusChangedArgs> {
        for w in windows.windows() {
            let frame = w.frame_info();
            if let Some(w) = frame.find(widget_id).map(|w| w.as_focus_info()) {
                if w.is_focusable() {
                    return self.move_focus(Some(w.info.path()), highlight, FocusChangedCause::Request(request));
                } else if fallback_to_parents {
                    if let Some(w) = w.parent() {
                        return self.move_focus(Some(w.info.path()), highlight, FocusChangedCause::Request(request));
                    } else {
                        // no focusable parent, just activate window?
                        //TODO
                    }
                }
                break;
            }
        }

        self.change_highlight(highlight, request)
    }

    #[must_use]
    fn change_highlight(&mut self, highlight: bool, request: FocusRequest) -> Option<FocusChangedArgs> {
        if self.is_highlighting != highlight {
            self.is_highlighting = highlight;
            Some(FocusChangedArgs::now(
                self.focused.clone(),
                self.focused.clone(),
                highlight,
                FocusChangedCause::Request(request),
            ))
        } else {
            None
        }
    }

    #[must_use]
    fn focus_active_window(&mut self, windows: &Windows, highlight: bool) -> Option<FocusChangedArgs> {
        if let Some(active) = windows.windows().find(|w| w.is_active()) {
            let frame = FrameFocusInfo::new(active.frame_info());
            let root = frame.root();
            if root.is_focusable() {
                // found active window and it is focusable.
                self.move_focus(Some(root.info.path()), highlight, FocusChangedCause::Recovery)
            } else {
                // has active window but it is not focusable
                self.move_focus(None, false, FocusChangedCause::Recovery)
            }
        } else {
            // no active window
            self.move_focus(None, false, FocusChangedCause::Recovery)
        }
    }

    #[must_use]
    fn move_focus(&mut self, new_focus: Option<WidgetPath>, highlight: bool, cause: FocusChangedCause) -> Option<FocusChangedArgs> {
        let prev_highlight = std::mem::replace(&mut self.is_highlighting, highlight);

        if self.focused != new_focus {
            let args = FocusChangedArgs::now(self.focused.take(), new_focus.clone(), self.is_highlighting, cause);
            self.focused = new_focus;
            Some(args)
        } else if prev_highlight != highlight {
            Some(FocusChangedArgs::now(new_focus.clone(), new_focus, highlight, cause))
        } else {
            None
        }
    }

    #[must_use]
    fn move_after_focus(&mut self, windows: &Windows, reverse: bool) -> Option<FocusChangedArgs> {
        if let Some(focused) = &self.focused {
            if let Ok(window) = windows.window(focused.window_id()) {
                if let Some(widget) = FrameFocusInfo::new(window.frame_info()).get(focused) {
                    if widget.is_scope() {
                        if let Some(widget) = widget.on_focus_scope_move(|id| self.return_focused(id), reverse) {
                            return self.move_focus(
                                Some(widget.info.path()),
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
    fn update_returns(&mut self, prev_focus: Option<WidgetPath>, windows: &Windows) -> Vec<ReturnFocusChangedArgs> {
        let mut r = vec![];

        if let Some((scope_id, _)) = &self.alt_return {
            // if we have an `alt_return` check if is still inside the ALT.

            let scope_id = *scope_id;

            let mut retain_alt = false;
            if let Some(new_focus) = &self.focused {
                if new_focus.contains(scope_id) {
                    retain_alt = true; // just a focus move inside the ALT.
                }
            }

            if !retain_alt {
                let (scope_id, widget_path) = self.alt_return.take().unwrap();
                r.push(ReturnFocusChangedArgs::now(scope_id, Some(widget_path), None));
            }
        } else if let Some(new_focus) = &self.focused {
            // if we don't have an `alt_return` but focused something, check if focus
            // moved inside an ALT.

            if let Ok(window) = windows.window(new_focus.window_id()) {
                if let Some(widget) = FrameFocusInfo::new(window.frame_info()).get(new_focus) {
                    let alt_scope = if widget.is_alt_scope() {
                        Some(widget)
                    } else if let Some(alt_scope) = widget.scopes().find(|s| s.is_alt_scope()) {
                        Some(alt_scope)
                    } else {
                        None
                    };
                    if let Some(alt_scope) = alt_scope {
                        let scope_id = alt_scope.info.widget_id();
                        // entered an alt_scope.

                        if let Some(prev) = &prev_focus {
                            // previous focus is the return.
                            r.push(ReturnFocusChangedArgs::now(scope_id, None, Some(prev.clone())));
                            self.alt_return = Some((scope_id, prev.clone()));
                        } else if let Some(parent) = alt_scope.parent() {
                            // no previous focus, ALT parent is the return.
                            let parent_path = parent.info.path();
                            r.push(ReturnFocusChangedArgs::now(scope_id, None, Some(parent_path.clone())));
                            self.alt_return = Some((scope_id, parent_path));
                        }
                    }
                }
            }
        }

        /*
         *   Update `return_focused`
         */

        if let Some(new_focus) = &self.focused {
            if let Ok(window) = windows.window(new_focus.window_id()) {
                if let Some(widget) = FrameFocusInfo::new(window.frame_info()).get(new_focus) {
                    if widget.scopes().all(|s| !s.is_alt_scope()) {
                        // if not inside ALT, update return for each LastFocused parent scopes.

                        for scope in widget
                            .scopes()
                            .filter(|s| s.focus_info().scope_on_focus() == FocusScopeOnFocus::LastFocused)
                        {
                            let scope_id = scope.info.widget_id();
                            let path = widget.info.path();
                            if let Some(current) = self.return_focused.get_mut(&scope_id) {
                                if current != &path {
                                    let prev = std::mem::replace(current, path);
                                    r.push(ReturnFocusChangedArgs::now(scope_id, Some(prev), Some(current.clone())));
                                }
                            } else {
                                r.push(ReturnFocusChangedArgs::now(scope_id, None, Some(path.clone())));
                                self.return_focused.insert(scope_id, path);
                            }
                        }
                    }
                }
            }
        }

        r
    }

    /// Cleanup `return_focused` and `alt_return` after new `frame`.
    #[must_use]
    fn cleanup_returns(&mut self, frame: FrameFocusInfo) -> Vec<ReturnFocusChangedArgs> {
        let mut r = vec![];

        self.return_focused.retain(|&scope_id, widget_path| {
            if widget_path.window_id() != frame.info.window_id() {
                return true; // retain, not same window.
            }

            let mut retain = false;

            if let Some(widget) = frame.get(widget_path) {
                if let Some(scope) = widget.scopes().find(|s| s.info.widget_id() == scope_id) {
                    if scope.focus_info().scope_on_focus() == FocusScopeOnFocus::LastFocused {
                        retain = true; // retain, widget still exists in same scope and scope still is LastFocused.

                        let path = widget.info.path();
                        if &path != widget_path {
                            // widget moved inside scope.
                            r.push(ReturnFocusChangedArgs::now(scope_id, Some(widget_path.clone()), Some(path.clone())));
                            *widget_path = path;
                        }
                    }
                } else if let Some(scope) = frame.find(scope_id) {
                    if scope.focus_info().scope_on_focus() == FocusScopeOnFocus::LastFocused {
                        // widget not inside scope anymore, but scope still exists and is valid.
                        if let Some(first) = scope.first_tab_descendant() {
                            // LastFocused goes to the first descendant as fallback.
                            retain = true;

                            let path = first.info.path();
                            r.push(ReturnFocusChangedArgs::now(scope_id, Some(widget_path.clone()), Some(path.clone())));
                            *widget_path = path;
                        }
                    }
                }
            } else if let Some(parent) = frame.get_or_parent(widget_path) {
                // widget not in window anymore, but a focusable parent is..
                if let Some(scope) = parent.scopes().find(|s| s.info.widget_id() == scope_id) {
                    if scope.focus_info().scope_on_focus() == FocusScopeOnFocus::LastFocused {
                        // ..and the parent is inside the scope, and the scope is still valid.
                        retain = true;

                        let path = parent.info.path();
                        r.push(ReturnFocusChangedArgs::now(scope_id, Some(widget_path.clone()), Some(path.clone())));
                        *widget_path = path;
                    }
                }
            }

            if !retain {
                r.push(ReturnFocusChangedArgs::now(scope_id, Some(widget_path.clone()), None));
            }
            retain
        });

        let mut retain_alt = true;
        if let Some((scope_id, widget_path)) = &mut self.alt_return {
            let scope_id = *scope_id;
            if widget_path.window_id() == frame.info.window_id() {
                // we needs to update alt_return

                retain_alt = false; // will retain only if still valid

                if let Some(widget) = frame.get(widget_path) {
                    if !widget.scopes().any(|s| s.info.widget_id() == scope_id) {
                        retain_alt = true; // retain, widget still exists outside of the ALT scope.

                        let path = widget.info.path();
                        if &path != widget_path {
                            // widget moved outside ALT scope.
                            r.push(ReturnFocusChangedArgs::now(scope_id, Some(widget_path.clone()), Some(path)));
                        }
                    }
                } else if let Some(parent) = frame.get_or_parent(widget_path) {
                    // widget not in window anymore, but a focusable parent is..
                    if parent.scopes().any(|s| s.info.widget_id() == scope_id) {
                        // ..and the parent is not inside the ALT scope.
                        retain_alt = true;

                        let path = parent.info.path();
                        r.push(ReturnFocusChangedArgs::now(scope_id, Some(widget_path.clone()), Some(path.clone())));
                        *widget_path = path;
                    }
                }
            }
        }
        if !retain_alt {
            let (scope_id, widget_path) = self.alt_return.take().unwrap();
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
            let (scope_id, widget_path) = self.alt_return.take().unwrap();
            r.push(ReturnFocusChangedArgs::now(scope_id, Some(widget_path), None));
        }

        self.return_focused.retain(|&scope_id, widget_path| {
            let retain = widget_path.window_id() != window_id;

            if !retain {
                r.push(ReturnFocusChangedArgs::now(scope_id, Some(widget_path.clone()), None));
            }

            retain
        });

        r
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Focus change request.
pub struct FocusRequest {
    /// Where to move the focus.
    pub target: FocusTarget,
    /// If the widget should visually indicate that it is focused.
    pub highlight: bool,
}

impl FocusRequest {
    #[inline]
    #[allow(missing_docs)]
    pub fn new(target: FocusTarget, highlight: bool) -> Self {
        Self { target, highlight }
    }

    /// New [`FocusTarget::Direct`] request.
    #[inline]
    pub fn direct(widget_id: WidgetId, highlight: bool) -> Self {
        Self::new(FocusTarget::Direct(widget_id), highlight)
    }
    /// New [`FocusTarget::DirectOrParent`] request.
    #[inline]
    pub fn direct_or_parent(widget_id: WidgetId, highlight: bool) -> Self {
        Self::new(FocusTarget::DirectOrParent(widget_id), highlight)
    }
    /// New [`FocusTarget::Next`] request.
    #[inline]
    pub fn next(highlight: bool) -> Self {
        Self::new(FocusTarget::Next, highlight)
    }
    /// New [`FocusTarget::Prev`] request.
    #[inline]
    pub fn prev(highlight: bool) -> Self {
        Self::new(FocusTarget::Prev, highlight)
    }
    /// New [`FocusTarget::Up`] request.
    #[inline]
    pub fn up(highlight: bool) -> Self {
        Self::new(FocusTarget::Up, highlight)
    }
    /// New [`FocusTarget::Right`] request.
    #[inline]
    pub fn right(highlight: bool) -> Self {
        Self::new(FocusTarget::Right, highlight)
    }
    /// New [`FocusTarget::Down`] request.
    #[inline]
    pub fn down(highlight: bool) -> Self {
        Self::new(FocusTarget::Down, highlight)
    }
    /// New [`FocusTarget::Left`] request.
    #[inline]
    pub fn left(highlight: bool) -> Self {
        Self::new(FocusTarget::Left, highlight)
    }
    /// New [`FocusTarget::Alt`] request.
    #[inline]
    pub fn alt(highlight: bool) -> Self {
        Self::new(FocusTarget::Alt, highlight)
    }
    /// New [`FocusTarget::EscapeAlt`] request.
    #[inline]
    pub fn escape_alt(highlight: bool) -> Self {
        Self::new(FocusTarget::EscapeAlt, highlight)
    }
}

/// Focus request target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusTarget {
    /// Move focus to widget.
    Direct(WidgetId),
    /// Move focus to the widget if it is focusable or to a focusable parent.
    DirectOrParent(WidgetId),

    /// Move focus to next from current in screen, or to first in screen.
    Next,
    /// Move focus to previous from current in screen, or to last in screen.
    Prev,

    /// Move focus above current.
    Up,
    /// Move focus to the right of current.
    Right,
    /// Move focus bellow current.
    Down,
    /// Move focus to the left of current.
    Left,

    /// Move focus to the current widget ALT scope.
    Alt,
    /// Move focus back from ALT scope.
    EscapeAlt,
}

/// A [`FrameInfo`] wrapper for querying focus info out of the widget tree.
#[derive(Copy, Clone)]
pub struct FrameFocusInfo<'a> {
    /// Full frame info.
    pub info: &'a FrameInfo,
}
impl<'a> FrameFocusInfo<'a> {
    /// Wrap a `frame_info` reference to enable focus info querying.
    #[inline]
    pub fn new(frame_info: &'a FrameInfo) -> Self {
        FrameFocusInfo { info: frame_info }
    }

    /// Reference to the root widget in the frame.
    ///
    /// The root is usually a focusable focus scope but it may not be. This
    /// is the only method that returns a [`WidgetFocusInfo`] that may not be focusable.
    #[inline]
    pub fn root(&self) -> WidgetFocusInfo {
        WidgetFocusInfo::new(self.info.root())
    }

    /// Reference to the widget in the frame, if it is present and is focusable.
    #[inline]
    pub fn find(&self, widget_id: WidgetId) -> Option<WidgetFocusInfo> {
        self.info.find(widget_id).and_then(|i| i.as_focusable())
    }

    /// Reference to the widget in the frame, if it is present and is focusable.
    ///
    /// Faster then [`find`](Self::find) if the widget path was generated by the same frame.
    #[inline]
    pub fn get(&self, path: &WidgetPath) -> Option<WidgetFocusInfo> {
        self.info.get(path).and_then(|i| i.as_focusable())
    }

    /// Reference to the first focusable widget or parent in the frame.
    #[inline]
    pub fn get_or_parent(&self, path: &WidgetPath) -> Option<WidgetFocusInfo> {
        self.get(path)
            .or_else(|| path.ancestors().iter().rev().find_map(|&id| self.find(id)))
    }

    /// If the frame info contains the widget and it is focusable.
    #[inline]
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.find(widget_id).is_some()
    }
}

/// [`WidgetInfo`] extensions that build a [`WidgetFocusInfo`].
pub trait WidgetInfoFocusExt<'a> {
    /// Wraps the [`WidgetInfo`] in a [`WidgetFocusInfo`] even if it is not focusable.
    #[allow(clippy::wrong_self_convention)] // WidgetFocusInfo is a reference wrapper.
    fn as_focus_info(self) -> WidgetFocusInfo<'a>;

    /// Returns a wrapped [`WidgetFocusInfo`] if the [`WidgetInfo`] is focusable.
    #[allow(clippy::wrong_self_convention)] // WidgetFocusInfo is a reference wrapper.
    fn as_focusable(self) -> Option<WidgetFocusInfo<'a>>;
}
impl<'a> WidgetInfoFocusExt<'a> for WidgetInfo<'a> {
    fn as_focus_info(self) -> WidgetFocusInfo<'a> {
        WidgetFocusInfo::new(self)
    }
    fn as_focusable(self) -> Option<WidgetFocusInfo<'a>> {
        let r = self.as_focus_info();
        if r.is_focusable() {
            Some(r)
        } else {
            None
        }
    }
}

/// [`WidgetInfo`] wrapper that adds focus information for each widget.
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct WidgetFocusInfo<'a> {
    /// Full widget info.
    pub info: WidgetInfo<'a>,
}
macro_rules! DirectionFn {
    (impl) => { impl Fn(LayoutPoint, LayoutPoint) -> (f32, f32, f32, f32) };
    (up) => { |from_pt, cand_c| (cand_c.y, from_pt.y, cand_c.x, from_pt.x) };
    (down) => { |from_pt, cand_c| (from_pt.y, cand_c.y, cand_c.x, from_pt.x) };
    (left) => { |from_pt, cand_c| (cand_c.x, from_pt.x, cand_c.y, from_pt.y) };
    (right) => { |from_pt, cand_c| (from_pt.x, cand_c.x, cand_c.y, from_pt.y) };
}
impl<'a> WidgetFocusInfo<'a> {
    /// Wrap a `widget_info` reference to enable focus info querying.
    #[inline]
    pub fn new(widget_info: WidgetInfo<'a>) -> Self {
        WidgetFocusInfo { info: widget_info }
    }

    /// Root focusable.
    #[inline]
    pub fn root(self) -> Self {
        self.ancestors().last().unwrap_or(self)
    }

    /// If the widget is focusable.
    ///
    /// ## Note
    ///
    /// This is probably `true`, the only way to get a [`WidgetFocusInfo`] for a non-focusable widget is by
    /// calling [`as_focus_info`](WidgetInfoFocusExt::as_focus_info) or explicitly constructing one.
    ///
    /// Focus scopes are also focusable.
    #[inline]
    pub fn is_focusable(self) -> bool {
        self.focus_info().is_focusable()
    }

    /// Is focus scope.
    #[inline]
    pub fn is_scope(self) -> bool {
        self.focus_info().is_scope()
    }

    /// Is ALT focus scope.
    #[inline]
    pub fn is_alt_scope(self) -> bool {
        self.focus_info().is_alt_scope()
    }

    /// Widget focus metadata.
    #[inline]
    pub fn focus_info(self) -> FocusInfo {
        if !self.info.enabled() {
            FocusInfo::NotFocusable
        } else if let Some(builder) = self.info.meta().get::<FocusInfoKey>() {
            builder.build()
        } else {
            FocusInfo::NotFocusable
        }
    }

    /// Iterator over focusable parent -> grandparent -> .. -> root.
    #[inline]
    pub fn ancestors(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        self.info.ancestors().focusable()
    }

    /// Iterator over focus scopes parent -> grandparent -> .. -> root.
    #[inline]
    pub fn scopes(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        self.info.ancestors().filter_map(|i| {
            let i = i.as_focus_info();
            if i.is_scope() {
                Some(i)
            } else {
                None
            }
        })
    }

    /// Reference to the focusable parent that contains this widget.
    #[inline]
    pub fn parent(self) -> Option<WidgetFocusInfo<'a>> {
        self.ancestors().next()
    }

    /// Reference the focus scope parent that contains the widget.
    #[inline]
    pub fn scope(self) -> Option<WidgetFocusInfo<'a>> {
        self.scopes().next()
    }

    /// Gets the [`scope`](Self::scope) and the widgets from the scope to `self`.
    fn scope_with_path(self) -> Option<(WidgetFocusInfo<'a>, Vec<WidgetFocusInfo<'a>>)> {
        let mut path = vec![];
        for i in self.info.ancestors() {
            let i = i.as_focus_info();
            if i.is_scope() {
                path.reverse();
                return Some((i, path));
            } else {
                path.push(i);
            }
        }
        None
    }

    /// Reference the ALT focus scope *closest* with the current widget.
    ///
    /// # Closest Alt Scope
    ///
    /// a - If `self` is already an ALT scope or is in one, moves to a sibling ALT scope, nested ALT scopes are ignored.
    /// b - If `self` is a normal scope, moves to the first descendant ALT scope, otherwise..
    /// c - Recursively searches for an ALT scope sibling up the scope tree.
    #[inline]
    pub fn alt_scope(self) -> Option<WidgetFocusInfo<'a>> {
        if self.in_alt_scope() {
            // We do not allow nested alt scopes, search for sibling focus scope.
            return self.scopes().find(|s| !s.is_alt_scope()).and_then(|s| s.alt_scope_query(s));
        } else if self.is_scope() {
            // if we are a normal scope, search for an inner ALT scope first.
            let r = self.descendants().find(|w| w.focus_info().is_alt_scope());
            if r.is_some() {
                return r;
            }
        }

        // search for a sibling alt scope and up the scopes tree.
        self.alt_scope_query(self)
    }
    fn alt_scope_query(self, skip: WidgetFocusInfo<'a>) -> Option<WidgetFocusInfo<'a>> {
        if let Some(scope) = self.scope() {
            // search for an ALT scope in our previous scope siblings.
            scope
                .filter_descendants(|w| {
                    if w == skip {
                        DescendantFilter::SkipTree
                    } else {
                        DescendantFilter::Include
                    }
                })
                .find(|w| w.focus_info().is_alt_scope())
                // if found no sibling ALT scope, do the same search for our scope.
                .or_else(|| scope.alt_scope_query(skip))
        } else {
            // we reached root, no ALT found.
            None
        }
    }

    /// Widget is in a ALT scope or is an ALT scope.
    #[inline]
    pub fn in_alt_scope(self) -> bool {
        self.is_alt_scope() || self.scopes().any(|s| s.is_alt_scope())
    }

    /// Widget the focus needs to move to when `self` gets focused.
    ///
    /// # Input
    ///
    /// * `last_focused`: A function that returns the last focused widget within a focus scope identified by `WidgetId`.
    /// * `reverse`: If the focus is *reversing* into `self`.
    ///
    /// # Returns
    ///
    /// Returns the different widget the focus must move to after focusing in `self` that is a focus scope.
    ///
    /// If `self` is not a [`FocusScope`](FocusInfo::FocusScope) always returns `None`.
    #[inline]
    pub fn on_focus_scope_move<'p>(
        self,
        last_focused: impl FnOnce(WidgetId) -> Option<&'p WidgetPath>,
        reverse: bool,
    ) -> Option<WidgetFocusInfo<'a>> {
        match self.focus_info() {
            FocusInfo::FocusScope { on_focus, .. } => match on_focus {
                FocusScopeOnFocus::FirstDescendant => {
                    if reverse {
                        self.last_tab_descendant()
                    } else {
                        self.first_tab_descendant()
                    }
                }
                FocusScopeOnFocus::LastFocused => last_focused(self.info.widget_id())
                    .and_then(|path| self.info.frame().get(path))
                    .and_then(|w| w.as_focusable())
                    .and_then(|f| {
                        if f.ancestors().any(|a| a == self) {
                            Some(f) // valid last focused
                        } else {
                            None
                        }
                    })
                    .or_else(|| {
                        if reverse {
                            self.last_tab_descendant()
                        } else {
                            self.first_tab_descendant()
                        }
                    }), // fallback
                FocusScopeOnFocus::Self_ => None,
            },
            FocusInfo::NotFocusable | FocusInfo::Focusable { .. } => None,
        }
    }

    /// Iterator over the focusable widgets contained by this widget.
    #[inline]
    pub fn descendants(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        self.info.descendants().focusable()
    }

    /// Iterator over all focusable widgets contained by this widget filtered by the `filter` closure.
    #[inline]
    pub fn filter_descendants(
        self,
        mut filter: impl FnMut(WidgetFocusInfo<'a>) -> DescendantFilter,
    ) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        self.info
            .filter_descendants(move |info| {
                if let Some(focusable) = info.as_focusable() {
                    filter(focusable)
                } else {
                    DescendantFilter::Skip
                }
            })
            .map(|info| info.as_focus_info())
    }

    fn descendants_skip_tab(self, ignore_skip: impl Fn(WidgetId) -> bool) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        self.filter_descendants(move |f| {
            if f.focus_info().tab_index() == TabIndex::SKIP && !ignore_skip(f.info.widget_id()) {
                DescendantFilter::SkipTree
            } else {
                DescendantFilter::Include
            }
        })
    }

    /// Descendants sorted by TAB index.
    ///
    /// [`SKIP`](TabIndex::SKIP) items and its descendants are not included.
    #[inline]
    pub fn tab_descendants(self) -> Vec<WidgetFocusInfo<'a>> {
        self.tab_descendants_ignore_skip(|_| false)
    }

    /// Like [`tab_descendants`](Self::tab_descendants) but you can suppress skipping some items.
    pub fn tab_descendants_ignore_skip(self, ignore_skip: impl Fn(WidgetId) -> bool) -> Vec<WidgetFocusInfo<'a>> {
        let mut vec: Vec<_> = self.descendants_skip_tab(ignore_skip).collect();

        vec.sort_by_key(|f| f.focus_info().tab_index());

        vec
    }

    /// First descendant considering TAB index.
    #[inline]
    pub fn first_tab_descendant(self) -> Option<WidgetFocusInfo<'a>> {
        let mut r = None;
        let mut r_index = TabIndex::SKIP;

        for d in self.descendants_skip_tab(|_| false) {
            let ti = d.focus_info().tab_index();
            if ti < r_index {
                r = Some(d);
                r_index = ti;
            }
        }

        r
    }

    /// Last descendant considering TAB index.
    #[inline]
    pub fn last_tab_descendant(self) -> Option<WidgetFocusInfo<'a>> {
        let mut descendants = self.descendants_skip_tab(|_| false);

        if let Some(d) = descendants.next() {
            let mut r_index = d.focus_info().tab_index();
            let mut r = Some(d);
            for d in descendants {
                let ti = d.focus_info().tab_index();

                if ti >= r_index {
                    r = Some(d);
                    r_index = ti;
                }
            }
            r
        } else {
            None
        }
    }

    /// Iterator over all focusable widgets in the same scope after this widget.
    #[inline]
    pub fn next_focusables(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        let self_id = self.info.widget_id();
        self.scope()
            .into_iter()
            .flat_map(|s| s.descendants())
            .skip_while(move |f| f.info.widget_id() != self_id)
            .skip(1)
    }

    /// Next focusable in the same scope after this widget.
    #[inline]
    pub fn next_focusable(self) -> Option<WidgetFocusInfo<'a>> {
        self.next_focusables().next()
    }

    /// Next focusable in the same scope after this widget respecting the TAB index.
    ///
    /// If `self` is `TabIndex::SKIP` or `skip_self` is set returns the next non-skip
    /// focusable in the same scope after this widget.
    ///
    /// If `self` is the last item in scope returns the sorted descendants of the parent scope.
    pub fn next_tab_focusable(self, skip_self: bool) -> Result<WidgetFocusInfo<'a>, Vec<WidgetFocusInfo<'a>>> {
        let self_index = self.focus_info().tab_index();

        // TAB siblings in the scope, excluding `TabIndex::SKIP` branches, unless its our own branch.
        let mut siblings = self
            .scope_with_path()
            .map(|(scope, path)| {
                let ignored_skips: Vec<_> = path
                    .into_iter()
                    .filter(|w| w.focus_info().tab_index() == TabIndex::SKIP)
                    .map(|w| w.info.widget_id())
                    .collect();

                scope.tab_descendants_ignore_skip(|i| ignored_skips.contains(&i))
            })
            .unwrap_or_default();

        if self_index == TabIndex::SKIP || skip_self {
            // TAB from skip, goes to next in widget tree.

            while let Some(next) = self.info.next_sibling().map(|s| s.as_focus_info()) {
                if next.focus_info().tab_index() != TabIndex::SKIP {
                    return Ok(next);
                }
            }
            return Err(siblings);
        }

        // binary search the same tab index gets any of the items with the same tab index.
        let i_same = siblings.binary_search_by_key(&self_index, |f| f.focus_info().tab_index()).unwrap();
        // so we do a linear search before and after to find `self`.

        // before
        for i in (0..=i_same).rev() {
            if siblings[i] == self {
                return if i == siblings.len() - 1 {
                    // we are the last item.
                    Err(siblings)
                } else {
                    // next
                    Ok(siblings.swap_remove(i + 1))
                };
            } else if siblings[i].focus_info().tab_index() != self_index {
                // did not find `self` before `i_same`
                break;
            }
        }

        // after
        for i in i_same..siblings.len() {
            if siblings[i] == self {
                return if i == siblings.len() - 1 {
                    // we are the last item.
                    Err(siblings)
                } else {
                    // next
                    Ok(siblings.swap_remove(i + 1))
                };
            }
        }

        Err(siblings)
    }

    /// Iterator over all focusable widgets in the same scope before this widget in reverse.
    #[inline]
    pub fn prev_focusables(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        let self_id = self.info.widget_id();

        let mut prev: Vec<_> = self
            .scope()
            .into_iter()
            .flat_map(|s| s.descendants())
            .take_while(move |f| f.info.widget_id() != self_id)
            .collect();

        prev.reverse();

        prev.into_iter()
    }

    /// Previous focusable in the same scope before this widget.
    #[inline]
    pub fn prev_focusable(self) -> Option<WidgetFocusInfo<'a>> {
        let self_id = self.info.widget_id();

        self.scope()
            .and_then(move |s| s.descendants().take_while(move |f| f.info.widget_id() != self_id).last())
    }

    /// Previous focusable in the same scope before this widget respecting the TAB index.
    ///
    /// If `self` is `TabIndex::SKIP` or `skip_self` is set returns the previous non-skip
    /// focusable in the same scope before this widget.
    ///
    /// If `self` is the first item in scope returns the sorted descendants of the parent scope.
    pub fn prev_tab_focusable(self, skip_self: bool) -> Result<WidgetFocusInfo<'a>, Vec<WidgetFocusInfo<'a>>> {
        let self_index = self.focus_info().tab_index();

        // TAB siblings in the scope, excluding `TabIndex::SKIP` branches, unless its our own branch.
        let mut siblings = self
            .scope_with_path()
            .map(|(scope, path)| {
                let ignored_skips: Vec<_> = path
                    .into_iter()
                    .filter(|w| w.focus_info().tab_index() == TabIndex::SKIP)
                    .map(|w| w.info.widget_id())
                    .collect();

                scope.tab_descendants_ignore_skip(|i| ignored_skips.contains(&i))
            })
            .unwrap_or_default();

        if self_index == TabIndex::SKIP || skip_self {
            // TAB from skip, goes to prev in widget tree.
            while let Some(prev) = self.info.prev_sibling().map(|s| s.as_focus_info()) {
                if prev.focus_info().tab_index() != TabIndex::SKIP {
                    return Ok(prev);
                }
            }
            return Err(siblings);
        }

        // binary search the same tab index gets any of the items with the same tab index.
        let i_same = siblings.binary_search_by_key(&self_index, |f| f.focus_info().tab_index()).unwrap();

        // before
        for i in (0..=i_same).rev() {
            if siblings[i] == self {
                return if i == 0 {
                    // we are the first item.
                    Err(siblings)
                } else {
                    // prev
                    Ok(siblings.swap_remove(i - 1))
                };
            } else if siblings[i].focus_info().tab_index() != self_index {
                // did not find `self` before `i_same`
                break;
            }
        }

        // after
        for i in i_same..siblings.len() {
            if siblings[i] == self {
                return if i == 0 {
                    // we are the first item.
                    Err(siblings)
                } else {
                    // prev
                    Ok(siblings.swap_remove(i - 1))
                };
            }
        }

        Err(siblings)
    }

    /// Widget to focus when pressing TAB from this widget.
    ///
    /// Set `skip_self` to not enter `self`, that is, the focus goes to the next sibling or next sibling descendant.
    ///
    /// Returns `None` if the focus does not move to another widget.
    #[inline]
    pub fn next_tab(self, skip_self: bool) -> Option<WidgetFocusInfo<'a>> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.tab_nav() {
                TabNav::None => None,
                TabNav::Continue => self.next_tab_focusable(skip_self).ok().or_else(|| scope.next_tab(true)),
                TabNav::Contained => self.next_tab_focusable(skip_self).ok(),
                TabNav::Cycle => self
                    .next_tab_focusable(skip_self)
                    .or_else(|sorted_siblings| {
                        if let Some(first) = sorted_siblings.into_iter().find(|f| f.focus_info().tab_index() != TabIndex::SKIP) {
                            if first == self {
                                Err(())
                            } else {
                                Ok(first)
                            }
                        } else {
                            Err(())
                        }
                    })
                    .ok(),
                TabNav::Once => scope.next_tab(true),
            }
        } else {
            None
        }
    }

    /// Widget to focus when pressing SHIFT+TAB from this widget.
    ///
    /// Set `skip_self` to not enter `self`, that is, the focus goes to the previous sibling or previous sibling descendant.
    ///
    /// Returns `None` if the focus does not move to another widget.
    #[inline]
    pub fn prev_tab(self, skip_self: bool) -> Option<WidgetFocusInfo<'a>> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.tab_nav() {
                TabNav::None => None,
                TabNav::Continue => self.prev_tab_focusable(skip_self).ok().or_else(|| scope.prev_tab(true)),
                TabNav::Contained => self.prev_tab_focusable(skip_self).ok(),
                TabNav::Cycle => self
                    .prev_tab_focusable(skip_self)
                    .or_else(|sorted_siblings| {
                        if let Some(last) = sorted_siblings.into_iter().rfind(|f| f.focus_info().tab_index() != TabIndex::SKIP) {
                            if last == self {
                                Err(())
                            } else {
                                Ok(last)
                            }
                        } else {
                            Err(())
                        }
                    })
                    .ok(),
                TabNav::Once => scope.prev_tab(true),
            }
        } else {
            None
        }
    }

    fn descendants_skip_directional(self, also_skip: Option<WidgetFocusInfo<'a>>) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        self.filter_descendants(move |f| {
            if also_skip == Some(f) || f.focus_info().skip_directional() {
                DescendantFilter::SkipTree
            } else {
                DescendantFilter::Include
            }
        })
    }

    fn directional_from_pt(
        self,
        scope: WidgetFocusInfo<'a>,
        from_pt: LayoutPoint,
        direction: DirectionFn![impl],
        skip_descendants: bool,
    ) -> Option<WidgetFocusInfo<'a>> {
        let skip_id = self.info.widget_id();

        let distance = move |other_pt: LayoutPoint| {
            let a = (other_pt.x - from_pt.x).powf(2.);
            let b = (other_pt.y - from_pt.y).powf(2.);
            a + b
        };

        let mut candidate_dist = f32::MAX;
        let mut candidate = None;

        for w in scope.descendants_skip_directional(if skip_descendants { Some(self) } else { None }) {
            if w.info.widget_id() != skip_id {
                let candidate_center = w.info.center();

                let (a, b, c, d) = direction(from_pt, candidate_center);
                let mut is_in_direction = false;

                // for 'up' this is:
                // is above line?
                if a <= b {
                    // is to the right?
                    if c >= d {
                        // is in the 45º 'frustum'
                        // │?╱
                        // │╱__
                        is_in_direction = c <= d + (b - a);
                    } else {
                        //  ╲?│
                        // __╲│
                        is_in_direction = c >= d - (b - a);
                    }
                }

                if is_in_direction {
                    let dist = distance(candidate_center);
                    if dist < candidate_dist {
                        candidate = Some(w);
                        candidate_dist = dist;
                    }
                }
            }
        }

        candidate
    }

    fn directional_next(self, direction_vals: DirectionFn![impl]) -> Option<WidgetFocusInfo<'a>> {
        self.scope()
            .and_then(|s| self.directional_from_pt(s, self.info.center(), direction_vals, true))
    }

    /// Closest focusable in the same scope above this widget.
    #[inline]
    pub fn focusable_up(self) -> Option<WidgetFocusInfo<'a>> {
        self.directional_next(DirectionFn![up])
    }

    /// Closest focusable in the same scope below this widget.
    #[inline]
    pub fn focusable_down(self) -> Option<WidgetFocusInfo<'a>> {
        self.directional_next(DirectionFn![down])
    }

    /// Closest focusable in the same scope to the left of this widget.
    #[inline]
    pub fn focusable_left(self) -> Option<WidgetFocusInfo<'a>> {
        self.directional_next(DirectionFn![left])
    }

    /// Closest focusable in the same scope to the right of this widget.
    #[inline]
    pub fn focusable_right(self) -> Option<WidgetFocusInfo<'a>> {
        self.directional_next(DirectionFn![right])
    }

    /// Widget to focus when pressing the arrow up key from this widget.
    #[inline]
    pub fn next_up(self) -> Option<WidgetFocusInfo<'a>> {
        self.next_up_from(self.info.center())
    }
    fn next_up_from(self, point: LayoutPoint) -> Option<WidgetFocusInfo<'a>> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.directional_nav() {
                DirectionalNav::None => None,
                DirectionalNav::Continue => self.focusable_up().or_else(|| scope.next_up_from(point)),
                DirectionalNav::Contained => self.focusable_up(),
                DirectionalNav::Cycle => {
                    self.focusable_up().or_else(|| {
                        // next up from the same X but from the bottom segment of scope.
                        let mut from_pt = point;
                        from_pt.y = scope.info.bounds().max_y();
                        self.directional_from_pt(scope, from_pt, DirectionFn![up], false)
                    })
                }
            }
        } else {
            None
        }
    }

    /// Widget to focus when pressing the arrow right key from this widget.
    #[inline]
    pub fn next_right(self) -> Option<WidgetFocusInfo<'a>> {
        self.next_right_from(self.info.center())
    }
    fn next_right_from(self, point: LayoutPoint) -> Option<WidgetFocusInfo<'a>> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.directional_nav() {
                DirectionalNav::None => None,
                DirectionalNav::Continue => self.focusable_right().or_else(|| scope.next_right_from(point)),
                DirectionalNav::Contained => self.focusable_right(),
                DirectionalNav::Cycle => self.focusable_right().or_else(|| {
                    // next right from the same Y but from the left segment of scope.
                    let mut from_pt = point;
                    from_pt.x = scope.info.bounds().min_x();
                    self.directional_from_pt(scope, from_pt, DirectionFn![right], false)
                }),
            }
        } else {
            None
        }
    }

    /// Widget to focus when pressing the arrow down key from this widget.
    #[inline]
    pub fn next_down(self) -> Option<WidgetFocusInfo<'a>> {
        self.next_down_from(self.info.center())
    }
    fn next_down_from(self, point: LayoutPoint) -> Option<WidgetFocusInfo<'a>> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.directional_nav() {
                DirectionalNav::None => None,
                DirectionalNav::Continue => self.focusable_down().or_else(|| scope.next_down_from(point)),
                DirectionalNav::Contained => self.focusable_down(),
                DirectionalNav::Cycle => self.focusable_down().or_else(|| {
                    // next down from the same X but from the top segment of scope.
                    let mut from_pt = point;
                    from_pt.y = scope.info.bounds().min_y();
                    self.directional_from_pt(scope, from_pt, DirectionFn![down], false)
                }),
            }
        } else {
            None
        }
    }

    /// Widget to focus when pressing the arrow left key from this widget.
    #[inline]
    pub fn next_left(self) -> Option<WidgetFocusInfo<'a>> {
        self.next_left_from(self.info.center())
    }
    fn next_left_from(self, point: LayoutPoint) -> Option<WidgetFocusInfo<'a>> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.directional_nav() {
                DirectionalNav::None => None,
                DirectionalNav::Continue => self.focusable_left().or_else(|| scope.next_left_from(point)),
                DirectionalNav::Contained => self.focusable_left(),
                DirectionalNav::Cycle => self.focusable_left().or_else(|| {
                    // next left from the same Y but from the right segment of scope.
                    let mut from_pt = point;
                    from_pt.x = scope.info.bounds().max_x();
                    self.directional_from_pt(scope, from_pt, DirectionFn![left], false)
                }),
            }
        } else {
            None
        }
    }
}

/// Filter-maps an iterator of [`WidgetInfo`] to [`WidgetFocusInfo`].
pub trait IterFocusable<'a, I: Iterator<Item = WidgetInfo<'a>>> {
    /// Returns an iterator of only the focusable widgets.
    fn focusable(self) -> std::iter::FilterMap<I, fn(WidgetInfo<'a>) -> Option<WidgetFocusInfo<'a>>>;
}
impl<'a, I: Iterator<Item = WidgetInfo<'a>>> IterFocusable<'a, I> for I {
    fn focusable(self) -> std::iter::FilterMap<I, fn(WidgetInfo<'a>) -> Option<WidgetFocusInfo<'a>>> {
        self.filter_map(|i| i.as_focusable())
    }
}

/// Focus metadata associated with a widget in a frame.
#[derive(Debug, Clone, Copy)]
pub enum FocusInfo {
    /// The widget is not focusable.
    NotFocusable,
    /// The widget is focusable as a single item.
    Focusable {
        /// Tab index of the widget.
        tab_index: TabIndex,
        /// If the widget is skipped during directional navigation from outside.
        skip_directional: bool,
    },
    /// The widget is a focusable focus scope.
    FocusScope {
        /// Tab index of the widget.
        tab_index: TabIndex,
        /// If the widget is skipped during directional navigation from outside.
        skip_directional: bool,
        /// Tab navigation inside the focus scope.
        tab_nav: TabNav,
        /// Directional navigation inside the focus scope.
        directional_nav: DirectionalNav,
        /// Behavior of the widget when receiving direct focus.
        on_focus: FocusScopeOnFocus,
        /// If this scope is focused when the ALT key is pressed.
        alt: bool,
    },
}

/// Behavior of a focus scope when it receives direct focus.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FocusScopeOnFocus {
    /// Just focus the scope widget.
    Self_,
    /// Focus the first descendant considering the TAB index, if the scope has no descendants
    /// behaves like [`Self_`](Self::Self_).
    ///
    /// Focus the last descendant if the focus is *reversing* in, e.g. in a SHIFT+TAB action.
    FirstDescendant,
    /// Focus the descendant that was last focused before focus moved out of the scope. If the
    /// scope cannot return focus, behaves like [`FirstDescendant`](Self::FirstDescendant).
    LastFocused,
}
impl Default for FocusScopeOnFocus {
    /// [`FirstDescendant`](Self::FirstDescendant)
    fn default() -> Self {
        FocusScopeOnFocus::FirstDescendant
    }
}

impl FocusInfo {
    /// If is focusable or a focus scope.
    #[inline]
    pub fn is_focusable(self) -> bool {
        !matches!(self, FocusInfo::NotFocusable)
    }

    /// If is a focus scope.
    #[inline]
    pub fn is_scope(self) -> bool {
        matches!(self, FocusInfo::FocusScope { .. })
    }

    /// If is an ALT focus scope.
    #[inline]
    pub fn is_alt_scope(self) -> bool {
        match self {
            FocusInfo::FocusScope { alt, .. } => alt,
            _ => false,
        }
    }

    /// Tab navigation mode.
    ///
    /// | Variant                   | Returns                                 |
    /// |---------------------------|-----------------------------------------|
    /// | Focus scope               | Associated value, default is `Continue` |
    /// | Focusable                 | `TabNav::Continue`                      |
    /// | Not-Focusable             | `TabNav::None`                          |
    #[inline]
    pub fn tab_nav(self) -> TabNav {
        match self {
            FocusInfo::FocusScope { tab_nav, .. } => tab_nav,
            FocusInfo::Focusable { .. } => TabNav::Continue,
            FocusInfo::NotFocusable => TabNav::None,
        }
    }

    /// Directional navigation mode.
    ///
    /// | Variant                   | Returns                             |
    /// |---------------------------|-------------------------------------|
    /// | Focus scope               | Associated value, default is `None` |
    /// | Focusable                 | `DirectionalNav::Continue`          |
    /// | Not-Focusable             | `DirectionalNav::None`              |
    #[inline]
    pub fn directional_nav(self) -> DirectionalNav {
        match self {
            FocusInfo::FocusScope { directional_nav, .. } => directional_nav,
            FocusInfo::Focusable { .. } => DirectionalNav::Continue,
            FocusInfo::NotFocusable => DirectionalNav::None,
        }
    }

    /// Tab navigation index.
    ///
    /// | Variant           | Returns                                       |
    /// |-------------------|-----------------------------------------------|
    /// | Focusable & Scope | Associated value, default is `TabIndex::AUTO` |
    /// | Not-Focusable     | `TabIndex::SKIP`                              |
    #[inline]
    pub fn tab_index(self) -> TabIndex {
        match self {
            FocusInfo::Focusable { tab_index, .. } => tab_index,
            FocusInfo::FocusScope { tab_index, .. } => tab_index,
            FocusInfo::NotFocusable => TabIndex::SKIP,
        }
    }

    /// If directional navigation skips over this widget.
    ///
    /// | Variant           | Returns                                       |
    /// |-------------------|-----------------------------------------------|
    /// | Focusable & Scope | Associated value, default is `false`          |
    /// | Not-Focusable     | `true`                                        |
    #[inline]
    pub fn skip_directional(self) -> bool {
        match self {
            FocusInfo::Focusable { skip_directional, .. } => skip_directional,
            FocusInfo::FocusScope { skip_directional, .. } => skip_directional,
            FocusInfo::NotFocusable => true,
        }
    }

    /// Focus scope behavior when it receives direct focus.
    ///
    /// | Variant                   | Returns                                                           |
    /// |---------------------------|-------------------------------------------------------------------|
    /// | Scope                     | Associated value, default is `FocusScopeOnFocus::FirstDescendant` |
    /// | Focusable & Not-Focusable | `FocusScopeOnFocus::Self_`                                        |
    #[inline]
    pub fn scope_on_focus(self) -> FocusScopeOnFocus {
        match self {
            FocusInfo::FocusScope { on_focus, .. } => on_focus,
            _ => FocusScopeOnFocus::Self_,
        }
    }
}

/// Builder for [`FocusInfo`] accessible in a [`FrameBuilder`](crate::render::FrameBuilder).
///
/// Use the [`FocusInfoKey`] to access the builder for the widget state in a frame.
#[derive(Default)]
pub struct FocusInfoBuilder {
    /// If the widget is focusable and the value was explicitly set.
    pub focusable: Option<bool>,

    /// If the widget is a focus scope and the value was explicitly set.
    pub scope: Option<bool>,
    /// If the widget is an ALT focus scope when it is a focus scope.
    pub alt_scope: bool,
    /// When the widget is a focus scope, its behavior on receiving direct focus.
    pub on_focus: FocusScopeOnFocus,

    /// Widget TAB index and if the index was explicitly set.
    pub tab_index: Option<TabIndex>,

    /// TAB navigation within this widget, if set turns the widget into a focus scope.
    pub tab_nav: Option<TabNav>,
    /// Directional navigation within this widget, if set turns the widget into a focus scope.
    pub directional_nav: Option<DirectionalNav>,

    /// If directional navigation skips over this widget.
    pub skip_directional: Option<bool>,
}
impl FocusInfoBuilder {
    /// Build a [`FocusInfo`] from the collected configuration in `self`.
    ///
    /// The widget is not focusable nor a focus scope if it set [`focusable`](Self::focusable) to `false`.
    ///
    /// The widget is a *focus scope* if it set [`scope`](Self::scope) to `true` **or** if it set [`tab_nav`](Self::tab_nav) or
    /// [`directional_nav`](Self::directional_nav) and did not set [`scope`](Self::scope) to `false`.
    ///
    /// The widget is *focusable* if it set [`focusable`](Self::focusable) to `true` **or** if it set the [`tab_index`](Self::tab_index).
    ///
    /// The widget is not focusable if it did not set any of the members mentioned.
    ///
    /// ## Tab Index
    ///
    /// If the [`tab_index`](Self::tab_index) was not set but the widget is focusable or a focus scope, the [`TabIndex::AUTO`]
    /// is used for the widget.
    ///
    /// ## Skip Directional
    ///
    /// If the [`skip_directional`](Self::skip_directional) was not set but the widget is focusable or a focus scope, it is
    /// set to `false` for the widget.
    ///
    /// ## Focus Scope
    ///
    /// If the widget is a focus scope, it is configured using [`alt_scope`](Self::alt_scope) and [`on_focus`](Self::on_focus).
    /// If the widget is not a scope these members are ignored.
    ///
    /// ### Tab Navigation
    ///
    /// If [`tab_nav`](Self::tab_nav) is not set but the widget is a focus scope, [`TabNav::Continue`] is used.
    ///
    /// ### Directional Navigation
    ///
    /// If [`directional_nav`](Self::directional_nav) is not set but the widget is a focus scope, [`DirectionalNav::Continue`] is used.
    #[inline]
    pub fn build(&self) -> FocusInfo {
        match (self.focusable, self.scope, self.tab_index, self.tab_nav, self.directional_nav) {
            // Set as not focusable.
            (Some(false), _, _, _, _) => FocusInfo::NotFocusable,

            // Set as focus scope and not set as not focusable
            // or set tab navigation and did not set as not focus scope
            // or set directional navigation and did not set as not focus scope.
            (_, Some(true), idx, tab, dir) | (_, None, idx, tab @ Some(_), dir) | (_, None, idx, tab, dir @ Some(_)) => {
                FocusInfo::FocusScope {
                    tab_index: idx.unwrap_or(TabIndex::AUTO),
                    skip_directional: self.skip_directional.unwrap_or_default(),
                    tab_nav: tab.unwrap_or(TabNav::Continue),
                    directional_nav: dir.unwrap_or(DirectionalNav::Continue),
                    alt: self.alt_scope,
                    on_focus: self.on_focus,
                }
            }

            // Set as focusable and was not focus scope
            // or set tab index and was not focus scope and did not set as not focusable.
            (Some(true), _, idx, _, _) | (_, _, idx @ Some(_), _, _) => FocusInfo::Focusable {
                tab_index: idx.unwrap_or(TabIndex::AUTO),
                skip_directional: self.skip_directional.unwrap_or_default(),
            },

            _ => FocusInfo::NotFocusable,
        }
    }
}
