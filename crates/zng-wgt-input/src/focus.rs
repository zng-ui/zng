//! Keyboard focus properties, [`tab_index`](fn@tab_index), [`focusable`](fn@focusable),
//! [`on_focus`](fn@on_focus), [`is_focused`](fn@is_focused) and more.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use zng_app::widget::info::WIDGET_INFO_CHANGED_EVENT;
use zng_ext_input::focus::*;
use zng_ext_input::gesture::{CLICK_EVENT, GESTURES};
use zng_ext_input::mouse::MOUSE_INPUT_EVENT;
use zng_wgt::prelude::*;

/// Makes the widget focusable when set to `true`.
#[property(CONTEXT, default(false), widget_impl(FocusableMix<P>))]
pub fn focusable(child: impl IntoUiNode, focusable: impl IntoVar<bool>) -> UiNode {
    let focusable = focusable.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&focusable);
        }
        UiNodeOp::Info { info } => {
            FocusInfoBuilder::new(info).focusable(focusable.get());
        }
        _ => {}
    })
}

/// Customizes the widget order during TAB navigation.
#[property(CONTEXT, default(TabIndex::default()))]
pub fn tab_index(child: impl IntoUiNode, tab_index: impl IntoVar<TabIndex>) -> UiNode {
    let tab_index = tab_index.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&tab_index);
        }
        UiNodeOp::Info { info } => {
            FocusInfoBuilder::new(info).tab_index(tab_index.get());
        }
        _ => {}
    })
}

/// Makes the widget into a focus scope when set to `true`.
#[property(CONTEXT, default(false))]
pub fn focus_scope(child: impl IntoUiNode, is_scope: impl IntoVar<bool>) -> UiNode {
    focus_scope_impl(child, is_scope, false)
}
/// Widget is the ALT focus scope.
///
/// ALT focus scopes are also, `TabIndex::SKIP`, `skip_directional_nav`, `TabNav::Cycle` and `DirectionalNav::Cycle` by default.
///
/// Also see [`focus_click_behavior`] that can be used to return focus automatically when any widget inside the ALT scope
/// handles a click.
///
/// [`focus_click_behavior`]: fn@focus_click_behavior
#[property(CONTEXT, default(false))]
pub fn alt_focus_scope(child: impl IntoUiNode, is_scope: impl IntoVar<bool>) -> UiNode {
    focus_scope_impl(child, is_scope, true)
}

fn focus_scope_impl(child: impl IntoUiNode, is_scope: impl IntoVar<bool>, is_alt: bool) -> UiNode {
    let is_scope = is_scope.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&is_scope);
        }
        UiNodeOp::Info { info } => {
            let mut info = FocusInfoBuilder::new(info);
            if is_alt {
                info.alt_scope(is_scope.get());
            } else {
                info.scope(is_scope.get());
            }
        }
        UiNodeOp::Deinit => {
            if is_alt && FOCUS.is_focus_within(WIDGET.id()).get() {
                // focus auto recovery can't return focus if the entire scope is missing.
                FOCUS.focus_exit();
            }
        }
        _ => {}
    })
}

/// Behavior of a focus scope when it receives direct focus.
#[property(CONTEXT, default(FocusScopeOnFocus::default()))]
pub fn focus_scope_behavior(child: impl IntoUiNode, behavior: impl IntoVar<FocusScopeOnFocus>) -> UiNode {
    let behavior = behavior.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&behavior);
        }
        UiNodeOp::Info { info } => {
            FocusInfoBuilder::new(info).on_focus(behavior.get());
        }
        _ => {}
    })
}

/// Tab navigation within this focus scope.
#[property(CONTEXT, default(TabNav::Continue))]
pub fn tab_nav(child: impl IntoUiNode, tab_nav: impl IntoVar<TabNav>) -> UiNode {
    let tab_nav = tab_nav.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&tab_nav);
        }
        UiNodeOp::Info { info } => {
            FocusInfoBuilder::new(info).tab_nav(tab_nav.get());
        }
        _ => {}
    })
}

/// Keyboard arrows navigation within this focus scope.
#[property(CONTEXT, default(DirectionalNav::Continue))]
pub fn directional_nav(child: impl IntoUiNode, directional_nav: impl IntoVar<DirectionalNav>) -> UiNode {
    let directional_nav = directional_nav.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&directional_nav);
        }
        UiNodeOp::Info { info } => {
            FocusInfoBuilder::new(info).directional_nav(directional_nav.get());
        }
        _ => {}
    })
}

/// Keyboard shortcuts that focus this widget or its first focusable descendant or its first focusable parent.
#[property(CONTEXT, default(Shortcuts::default()))]
pub fn focus_shortcut(child: impl IntoUiNode, shortcuts: impl IntoVar<Shortcuts>) -> UiNode {
    let shortcuts = shortcuts.into_var();
    let mut _handle = None;
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&shortcuts);
            let s = shortcuts.get();
            _handle = Some(GESTURES.focus_shortcut(s, WIDGET.id()));
        }
        UiNodeOp::Update { .. } => {
            if let Some(s) = shortcuts.get_new() {
                _handle = Some(GESTURES.focus_shortcut(s, WIDGET.id()));
            }
        }
        _ => {}
    })
}

/// If directional navigation from outside this widget skips over it and its descendants.
///
/// Setting this to `true` is the directional navigation equivalent of setting `tab_index` to `SKIP`.
#[property(CONTEXT, default(false))]
pub fn skip_directional(child: impl IntoUiNode, enabled: impl IntoVar<bool>) -> UiNode {
    let enabled = enabled.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&enabled);
        }
        UiNodeOp::Info { info } => {
            FocusInfoBuilder::new(info).skip_directional(enabled.get());
        }
        _ => {}
    })
}

/// Behavior of a widget when a click event is send to it or a descendant.
///
/// See [`focus_click_behavior`] for more details.
///
/// [`focus_click_behavior`]: fn@focus_click_behavior
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FocusClickBehavior {
    /// Click event always ignored.
    ///
    /// The shorthand unit `Ignore!` converts into this.
    Ignore,
    /// Exit focus if a click event was send to the widget or descendant.
    ///
    /// The shorthand unit `Exit!` converts into this.
    Exit,
    /// Exit focus if a click event was send to the enabled widget or enabled descendant.
    ///
    /// The shorthand unit `ExitEnabled!` converts into this.
    ExitEnabled,
    /// Exit focus if the click event was received by the widget or descendant and event propagation was stopped.
    ///
    /// The shorthand unit `ExitHandled!` converts into this.
    ExitHandled,
}
impl_from_and_into_var! {
    fn from(_: ShorthandUnit![Ignore]) -> FocusClickBehavior {
        FocusClickBehavior::Ignore
    }
    fn from(_: ShorthandUnit![Exit]) -> FocusClickBehavior {
        FocusClickBehavior::Exit
    }
    fn from(_: ShorthandUnit![ExitEnabled]) -> FocusClickBehavior {
        FocusClickBehavior::ExitEnabled
    }
    fn from(_: ShorthandUnit![ExitHandled]) -> FocusClickBehavior {
        FocusClickBehavior::ExitHandled
    }
}
impl std::fmt::Debug for FocusClickBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "FocusClickBehavior::")?;
        }
        match self {
            Self::Ignore => write!(f, "Ignore"),
            Self::Exit => write!(f, "Exit"),
            Self::ExitEnabled => write!(f, "ExitEnabled"),
            Self::ExitHandled => write!(f, "ExitHandled"),
        }
    }
}

/// Behavior of a widget when a click event is send to it or a descendant.
///
/// When a click event targets the widget or descendant the `behavior` closest to the target is applied,
/// that is if `Exit` is set in a parent, but `Ignore` is set on the target than the click is ignored.
/// This can be used to create a effects like a menu that closes on click for command items, but not for clicks
/// in sub-menu items.
///
/// Note that this property does not subscribe to any event, it only observes events flowing trough.
#[property(CONTEXT, default(FocusClickBehavior::Ignore))]
pub fn focus_click_behavior(child: impl IntoUiNode, behavior: impl IntoVar<FocusClickBehavior>) -> UiNode {
    let behavior = behavior.into_var();
    match_node(child, move |c, op| {
        if let UiNodeOp::Event { update } = op {
            let mut delegate = || {
                if let Some(ctx) = &*FOCUS_CLICK_HANDLED_CTX.get() {
                    c.event(update);
                    ctx.swap(true, Ordering::Relaxed)
                } else {
                    let mut ctx = Some(Arc::new(Some(AtomicBool::new(false))));
                    FOCUS_CLICK_HANDLED_CTX.with_context(&mut ctx, || c.event(update));
                    let ctx = ctx.unwrap();
                    (*ctx).as_ref().unwrap().load(Ordering::Relaxed)
                }
            };

            if let Some(args) = CLICK_EVENT.on(update) {
                if !delegate() {
                    let exit = match behavior.get() {
                        FocusClickBehavior::Ignore => false,
                        FocusClickBehavior::Exit => true,
                        FocusClickBehavior::ExitEnabled => args.target.interactivity().is_enabled(),
                        FocusClickBehavior::ExitHandled => args.propagation().is_stopped(),
                    };
                    if exit {
                        FOCUS.focus_exit();
                    }
                }
            } else if let Some(args) = MOUSE_INPUT_EVENT.on_unhandled(update)
                && args.propagation().is_stopped()
                && !delegate()
            {
                // CLICK_EVENT not send if source mouse-input is already handled.

                let exit = match behavior.get() {
                    FocusClickBehavior::Ignore => false,
                    FocusClickBehavior::Exit => true,
                    FocusClickBehavior::ExitEnabled => args.target.interactivity().is_enabled(),
                    FocusClickBehavior::ExitHandled => true,
                };
                if exit {
                    FOCUS.focus_exit();
                }
            }
        }
    })
}
context_local! {
    static FOCUS_CLICK_HANDLED_CTX: Option<AtomicBool> = None;
}

event_property! {
    /// Focus changed in the widget or its descendants.
    pub fn focus_changed {
        event: FOCUS_CHANGED_EVENT,
        args: FocusChangedArgs,
    }

    /// Widget got direct keyboard focus.
    pub fn focus {
        event: FOCUS_CHANGED_EVENT,
        args: FocusChangedArgs,
        filter: |args| args.is_focus(WIDGET.id()),
    }

    /// Widget lost direct keyboard focus.
    pub fn blur {
        event: FOCUS_CHANGED_EVENT,
        args: FocusChangedArgs,
        filter: |args| args.is_blur(WIDGET.id()),
    }

    /// Widget or one of its descendants got focus.
    pub fn focus_enter {
        event: FOCUS_CHANGED_EVENT,
        args: FocusChangedArgs,
        filter: |args| args.is_focus_enter(WIDGET.id()),
    }

    /// Widget or one of its descendants lost focus.
    pub fn focus_leave {
        event: FOCUS_CHANGED_EVENT,
        args: FocusChangedArgs,
        filter: |args| args.is_focus_leave(WIDGET.id()),
    }
}

/// If the widget has keyboard focus.
///
/// This is only `true` if the widget itself is focused.
/// Use [`is_focus_within`] to include focused widgets inside this one.
///
/// # Highlighting
///
/// This property is always `true` when the widget has focus, independent of what device moved the focus,
/// usually when the keyboard is used a special visual indicator is rendered, a dotted line border is common,
/// this state is called *highlighting* and is tracked by the focus manager. To implement such a visual you can use the
/// [`is_focused_hgl`] property.
///
/// # Return Focus
///
/// Usually widgets that have a visual state for this property also have one for [`is_return_focus`], a common example is the
/// *text-input* widget that shows an emphasized border and blinking cursor when focused and still shows the
/// emphasized border without cursor when a menu is open and it is only the return focus.
///
/// [`is_focus_within`]: fn@is_focus_within
/// [`is_focused_hgl`]: fn@is_focused_hgl
/// [`is_return_focus`]: fn@is_return_focus
#[property(EVENT, widget_impl(FocusableMix<P>))]
pub fn is_focused(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    event_state(child, state, false, FOCUS_CHANGED_EVENT, |args| {
        let id = WIDGET.id();
        if args.is_focus(id) {
            Some(true)
        } else if args.is_blur(id) {
            Some(false)
        } else {
            None
        }
    })
}

/// If the widget or one of its descendants has keyboard focus.
///
/// To check if only the widget has keyboard focus use [`is_focused`].
///
/// To track *highlighted* focus within use [`is_focus_within_hgl`] property.
///
/// [`is_focused`]: fn@is_focused
/// [`is_focus_within_hgl`]: fn@is_focus_within_hgl
#[property(EVENT, widget_impl(FocusableMix<P>))]
pub fn is_focus_within(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    event_state(child, state, false, FOCUS_CHANGED_EVENT, |args| {
        let id = WIDGET.id();
        if args.is_focus_enter(id) {
            Some(true)
        } else if args.is_focus_leave(id) {
            Some(false)
        } else {
            None
        }
    })
}

/// If the widget has keyboard focus and the user is using the keyboard to navigate.
///
/// This is only `true` if the widget itself is focused and the focus was acquired by keyboard navigation.
/// You can use [`is_focus_within_hgl`] to include widgets inside this one.
///
/// # Highlighting
///
/// Usually when the keyboard is used to move the focus a special visual indicator is rendered, a dotted line border is common,
/// this state is called *highlighting* and is tracked by the focus manager, this property is only `true`.
///
/// [`is_focus_within_hgl`]: fn@is_focus_within_hgl
/// [`is_focused`]: fn@is_focused
#[property(EVENT, widget_impl(FocusableMix<P>))]
pub fn is_focused_hgl(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    event_state(child, state, false, FOCUS_CHANGED_EVENT, |args| {
        let id = WIDGET.id();
        if args.is_focus(id) {
            Some(args.highlight)
        } else if args.is_blur(id) {
            Some(false)
        } else if args.is_highlight_changed() && args.new_focus.as_ref().map(|p| p.widget_id() == id).unwrap_or(false) {
            Some(args.highlight)
        } else {
            None
        }
    })
}

/// If the widget or one of its descendants has keyboard focus and the user is using the keyboard to navigate.
///
/// To check if only the widget has keyboard focus use [`is_focused_hgl`].
///
/// Also see [`is_focus_within`] to check if the widget has focus within regardless of highlighting.
///
/// [`is_focused_hgl`]: fn@is_focused_hgl
/// [`is_focus_within`]: fn@is_focus_within
#[property(EVENT, widget_impl(FocusableMix<P>))]
pub fn is_focus_within_hgl(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    event_state(child, state, false, FOCUS_CHANGED_EVENT, |args| {
        let id = WIDGET.id();
        if args.is_focus_enter(id) {
            Some(args.highlight)
        } else if args.is_focus_leave(id) {
            Some(false)
        } else if args.is_highlight_changed() && args.new_focus.as_ref().map(|p| p.contains(id)).unwrap_or(false) {
            Some(args.highlight)
        } else {
            None
        }
    })
}

/// If the widget will be focused when a parent scope is focused.
///
/// Focus scopes can remember the last focused widget inside, the focus *returns* to
/// this widget when the scope receives focus. Alt scopes also remember the widget from which the *alt* focus happened
/// and can also return focus back to that widget.
///
/// Usually input widgets that have a visual state for [`is_focused`] also have a visual for this, a common example is the
/// *text-input* widget that shows an emphasized border and blinking cursor when focused and still shows the
/// emphasized border without cursor when a menu is open and it is only the return focus.
///
/// Note that a widget can be [`is_focused`] and `is_return_focus`, this property is `true` if any focus scope considers the
/// widget its return focus, you probably want to declare the widget visual states in such a order that [`is_focused`] overrides
/// the state of this property.
///
/// [`is_focused`]: fn@is_focused_hgl
/// [`is_focused_hgl`]: fn@is_focused_hgl
#[property(EVENT, widget_impl(FocusableMix<P>))]
pub fn is_return_focus(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    event_state(child, state, false, RETURN_FOCUS_CHANGED_EVENT, |args| {
        let id = WIDGET.id();
        if args.is_return_focus(id) {
            Some(true)
        } else if args.was_return_focus(id) {
            Some(false)
        } else {
            None
        }
    })
}

/// If the widget or one of its descendants will be focused when a focus scope is focused.
///
/// To check if only the widget is the return focus use [`is_return_focus`].
///
/// [`is_return_focus`]: fn@is_return_focus
#[property(EVENT, widget_impl(FocusableMix<P>))]
pub fn is_return_focus_within(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    event_state(child, state, false, RETURN_FOCUS_CHANGED_EVENT, |args| {
        let id = WIDGET.id();
        if args.is_return_focus_enter(id) {
            Some(true)
        } else if args.is_return_focus_leave(id) {
            Some(false)
        } else {
            None
        }
    })
}

/// If the widget is focused on info init.
///
/// When the widget is inited and present in the info tree a [`FOCUS.focus_widget_or_related`] request is made for the widget.
///
/// [`FOCUS.focus_widget_or_related`]: FOCUS::focus_widget_or_related
#[property(EVENT, default(false), widget_impl(FocusableMix<P>))]
pub fn focus_on_init(child: impl IntoUiNode, enabled: impl IntoVar<bool>) -> UiNode {
    let enabled = enabled.into_var();

    enum State {
        WaitInfo,
        InfoInited,
        Done,
    }
    let mut state = State::WaitInfo;

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            if enabled.get() {
                state = State::WaitInfo;
            } else {
                state = State::Done;
            }
        }
        UiNodeOp::Info { .. } => {
            if let State::WaitInfo = &state {
                state = State::InfoInited;
                // next update will be after the info is in tree.
                WIDGET.update();
            }
        }
        UiNodeOp::Update { .. } => {
            if let State::InfoInited = &state {
                state = State::Done;
                FOCUS.focus_widget_or_related(WIDGET.id(), false, false);
            }
        }
        _ => {}
    })
}

/// If the widget return focus to the previous focus when it inited.
///
/// This can be used with the [`modal`] property to declare *modal dialogs* that return the focus
/// to the widget that opens the dialog.
///
/// Consider using [`focus_click_behavior`] if the widget is also an ALT focus scope.
///
/// [`modal`]: fn@zng_wgt::modal
/// [`focus_click_behavior`]: fn@focus_click_behavior
#[property(EVENT, default(false), widget_impl(FocusableMix<P>))]
pub fn return_focus_on_deinit(child: impl IntoUiNode, enabled: impl IntoVar<bool>) -> UiNode {
    let enabled = enabled.into_var();
    let mut return_focus = None;
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            return_focus = FOCUS.focused().with(|p| p.as_ref().map(|p| p.widget_id()));
        }
        UiNodeOp::Deinit => {
            if let Some(id) = return_focus.take()
                && enabled.get()
            {
                if let Some(w) = zng_ext_window::WINDOWS.widget_info(id)
                    && w.into_focusable(false, false).is_some()
                {
                    // can focus on the next update
                    FOCUS.focus_widget(id, false);
                    return;
                }
                // try focus after info rebuild.
                WIDGET_INFO_CHANGED_EVENT
                    .on_pre_event(hn_once!(|_| {
                        FOCUS.focus_widget(id, false);
                    }))
                    .perm();
                // ensure info rebuilds to clear the event at least
                WIDGET.update_info();
            }
        }
        _ => {}
    })
}

/// Focusable widget mixin. Enables keyboard focusing on the widget and adds a focused highlight visual.
#[widget_mixin]
pub struct FocusableMix<P>(P);
impl<P: WidgetImpl> FocusableMix<P> {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            focusable = true;
            when *#is_focused_hgl {
                zng_wgt_fill::foreground_highlight = {
                    offsets: FOCUS_HIGHLIGHT_OFFSETS_VAR,
                    widths: FOCUS_HIGHLIGHT_WIDTHS_VAR,
                    sides: FOCUS_HIGHLIGHT_SIDES_VAR,
                };
            }
        }
    }
}

context_var! {
    /// Padding offsets of the foreground highlight when the widget is focused.
    pub static FOCUS_HIGHLIGHT_OFFSETS_VAR: SideOffsets = 1;
    /// Border widths of the foreground highlight when the widget is focused.
    pub static FOCUS_HIGHLIGHT_WIDTHS_VAR: SideOffsets = 0.5;
    /// Border sides of the foreground highlight when the widget is focused.
    pub static FOCUS_HIGHLIGHT_SIDES_VAR: BorderSides = BorderSides::dashed(rgba(200, 200, 200, 1.0));
}

/// Sets the foreground highlight values used when the widget is focused and highlighted.
#[property(
    CONTEXT,
    default(FOCUS_HIGHLIGHT_OFFSETS_VAR, FOCUS_HIGHLIGHT_WIDTHS_VAR, FOCUS_HIGHLIGHT_SIDES_VAR),
    widget_impl(FocusableMix<P>)
)]
pub fn focus_highlight(
    child: impl IntoUiNode,
    offsets: impl IntoVar<SideOffsets>,
    widths: impl IntoVar<SideOffsets>,
    sides: impl IntoVar<BorderSides>,
) -> UiNode {
    let child = with_context_var(child, FOCUS_HIGHLIGHT_WIDTHS_VAR, offsets);
    let child = with_context_var(child, FOCUS_HIGHLIGHT_OFFSETS_VAR, widths);
    with_context_var(child, FOCUS_HIGHLIGHT_SIDES_VAR, sides)
}
