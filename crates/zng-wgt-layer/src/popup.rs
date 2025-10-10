//! Popup widget.

use std::time::Duration;

use zng_ext_input::focus::{DirectionalNav, FOCUS_CHANGED_EVENT, FocusScopeOnFocus, TabNav};
use zng_wgt::{modal_included, prelude::*};
use zng_wgt_container::Container;
use zng_wgt_fill::background_color;
use zng_wgt_filter::drop_shadow;
use zng_wgt_input::focus::{
    FocusClickBehavior, FocusableMix, alt_focus_scope, directional_nav, focus_click_behavior, focus_scope_behavior, tab_nav,
};
use zng_wgt_style::{Style, StyleMix, impl_style_fn};

use crate::{AnchorMode, AnchorOffset, LAYERS, LayerIndex};

/// An overlay container.
///
/// # POPUP
///
/// The popup widget is designed to be used as a temporary *flyover* container inserted as a
/// top-most layer using [`POPUP`]. By default the widget is an [`alt_focus_scope`] that is [`focus_on_init`],
/// cycles [`directional_nav`] and [`tab_nav`], and has [`FocusClickBehavior::ExitEnabled`]. It also
/// sets the [`modal_included`] to [`anchor_id`] enabling the popup to be interactive when anchored to modal widgets.
///
/// [`alt_focus_scope`]: fn@alt_focus_scope
/// [`focus_on_init`]: fn@zng_wgt_input::focus::focus_on_init
/// [`directional_nav`]: fn@directional_nav
/// [`tab_nav`]: fn@tab_nav
/// [`modal_included`]: fn@modal_included
/// [`anchor_id`]: POPUP::anchor_id
/// [`FocusClickBehavior::ExitEnabled`]: zng_wgt_input::focus::FocusClickBehavior::ExitEnabled
#[widget($crate::popup::Popup {
    ($child:expr) => {
        child = $child;
    }
})]
pub struct Popup(FocusableMix<StyleMix<Container>>);
impl Popup {
    fn widget_intrinsic(&mut self) {
        self.style_intrinsic(STYLE_FN_VAR, property_id!(self::style_fn));

        widget_set! {
            self;

            alt_focus_scope = true;
            directional_nav = DirectionalNav::Cycle;
            tab_nav = TabNav::Cycle;
            focus_scope_behavior = FocusScopeOnFocus::FirstDescendantIgnoreBounds;
            focus_click_behavior = FocusClickBehavior::ExitEnabled;
            focus_on_init = true;
            modal_included = POPUP.anchor_id();
        }
    }

    widget_impl! {
        /// Popup focus behavior when it or a descendant receives a click.
        ///
        /// Is [`FocusClickBehavior::ExitEnabled`] by default.
        ///
        /// [`FocusClickBehavior::ExitEnabled`]: zng_wgt_input::focus::FocusClickBehavior::ExitEnabled
        pub focus_click_behavior(behavior: impl IntoVar<FocusClickBehavior>);
    }
}
impl_style_fn!(Popup, DefaultStyle);

context_var! {
    /// If popup will close when it no longer contains the focused widget.
    ///
    /// Is `true` by default.
    pub static CLOSE_ON_FOCUS_LEAVE_VAR: bool = true;

    /// Popup anchor mode.
    ///
    /// Is `AnchorMode::popup(AnchorOffset::out_bottom())` by default.
    pub static ANCHOR_MODE_VAR: AnchorMode = AnchorMode::popup(AnchorOffset::out_bottom());

    /// Popup context capture.
    pub static CONTEXT_CAPTURE_VAR: ContextCapture = ContextCapture::default();
}

/// Popup behavior when it loses focus.
///
/// If `true` the popup will close itself, is `true` by default.
///
/// This property must be set on the widget that opens the popup or a parent, not the popup widget itself.
///
/// Sets the [`CLOSE_ON_FOCUS_LEAVE_VAR`].
#[property(CONTEXT, default(CLOSE_ON_FOCUS_LEAVE_VAR))]
pub fn close_on_focus_leave(child: impl IntoUiNode, close: impl IntoVar<bool>) -> UiNode {
    with_context_var(child, CLOSE_ON_FOCUS_LEAVE_VAR, close)
}

/// Defines the popup placement and size for popups open by the widget or descendants.
///
/// This property must be set on the widget that opens the popup or a parent, not the popup widget itself.
///
/// This property sets the [`ANCHOR_MODE_VAR`].
#[property(CONTEXT, default(ANCHOR_MODE_VAR))]
pub fn anchor_mode(child: impl IntoUiNode, mode: impl IntoVar<AnchorMode>) -> UiNode {
    with_context_var(child, ANCHOR_MODE_VAR, mode)
}

/// Defines if the popup captures the local context to load in the popup context.
///
/// This is enabled by default and lets the popup use context values from the widget
/// that opens it, not just from the window [`LAYERS`] root where it will actually be inited.
/// There are potential issues with this, see [`ContextCapture`] for more details.
///
/// Note that updates to this property do not affect popups already open, just subsequent popups. This
/// property must be set on the widget that opens the popup or a parent, not the popup widget itself.
///
/// This property sets the [`CONTEXT_CAPTURE_VAR`].
#[property(CONTEXT, default(CONTEXT_CAPTURE_VAR))]
pub fn context_capture(child: impl IntoUiNode, capture: impl IntoVar<ContextCapture>) -> UiNode {
    with_context_var(child, CONTEXT_CAPTURE_VAR, capture)
}

/// Popup service.
pub struct POPUP;
impl POPUP {
    /// Open the `popup` using the current context config vars.
    ///
    /// If the popup node is not a full widget after init it is upgraded to one. Returns
    /// a variable that tracks the popup state and ID.
    pub fn open(&self, popup: impl IntoUiNode) -> Var<PopupState> {
        self.open_impl(popup.into_node(), ANCHOR_MODE_VAR.into(), CONTEXT_CAPTURE_VAR.get())
    }

    /// Open the `popup` using the custom config vars.
    ///
    /// If the popup node is not a full widget after init it is upgraded to one. Returns
    /// a variable that tracks the popup state and ID.
    pub fn open_config(
        &self,
        popup: impl IntoUiNode,
        anchor_mode: impl IntoVar<AnchorMode>,
        context_capture: impl IntoValue<ContextCapture>,
    ) -> Var<PopupState> {
        self.open_impl(popup.into_node(), anchor_mode.into_var(), context_capture.into())
    }

    fn open_impl(&self, mut popup: UiNode, anchor_mode: Var<AnchorMode>, context_capture: ContextCapture) -> Var<PopupState> {
        let state = var(PopupState::Opening);
        let mut _close_handle = CommandHandle::dummy();

        let anchor_id = WIDGET.id();

        popup = match_widget(
            popup,
            clmv!(state, |c, op| match op {
                UiNodeOp::Init => {
                    c.init();

                    if let Some(mut wgt) = c.node().as_widget() {
                        wgt.with_context(WidgetUpdateMode::Bubble, || {
                            WIDGET.sub_event(&FOCUS_CHANGED_EVENT);
                        });
                        let id = wgt.id();
                        state.set(PopupState::Open(id));
                        _close_handle = POPUP_CLOSE_CMD.scoped(id).subscribe(true);
                    } else {
                        // not widget after init, generate a widget, but can still become
                        // a widget later, such as a `take_on_init` ArcNode that was already
                        // in use on init, to support `close_delay` in this scenario the not_widget
                        // is wrapped in a node that pumps POPUP_CLOSE_REQUESTED_EVENT to the not_widget
                        // if it is a widget at the time of the event.
                        c.deinit();

                        let not_widget = std::mem::replace(c.node(), UiNode::nil());
                        let not_widget = match_node(not_widget, |c, op| match op {
                            UiNodeOp::Init => {
                                WIDGET.sub_event(&FOCUS_CHANGED_EVENT).sub_event(&POPUP_CLOSE_REQUESTED_EVENT);
                            }
                            UiNodeOp::Event { update } => {
                                if let Some(args) = POPUP_CLOSE_REQUESTED_EVENT.on(update)
                                    && let Some(mut now_is_widget) = c.node().as_widget()
                                {
                                    let now_is_widget = now_is_widget.with_context(WidgetUpdateMode::Ignore, || WIDGET.info().path());
                                    if POPUP_CLOSE_REQUESTED_EVENT.is_subscriber(now_is_widget.widget_id()) {
                                        // node become widget after init, and it expects POPUP_CLOSE_REQUESTED_EVENT.
                                        let mut delivery = UpdateDeliveryList::new_any();
                                        delivery.insert_wgt(&now_is_widget);
                                        let update = POPUP_CLOSE_REQUESTED_EVENT.new_update_custom(args.clone(), delivery);
                                        c.event(&update);
                                    }
                                }
                            }
                            _ => {}
                        });

                        *c.node() = not_widget.into_widget();

                        c.init();
                        let id = c.node().as_widget().unwrap().id();

                        state.set(PopupState::Open(id));
                        _close_handle = POPUP_CLOSE_CMD.scoped(id).subscribe(true);
                    }
                }
                UiNodeOp::Deinit => {
                    state.set(PopupState::Closed);
                    _close_handle = CommandHandle::dummy();
                }
                UiNodeOp::Event { update } => {
                    c.node().as_widget().unwrap().with_context(WidgetUpdateMode::Bubble, || {
                        let id = WIDGET.id();

                        if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                            if args.is_focus_leave(id) && CLOSE_ON_FOCUS_LEAVE_VAR.get() {
                                POPUP.close_id(id);
                            }
                        } else if let Some(args) = POPUP_CLOSE_CMD.scoped(id).on_unhandled(update) {
                            match args.param::<PopupCloseMode>() {
                                Some(s) => match s {
                                    PopupCloseMode::Request => POPUP.close_id(id),
                                    PopupCloseMode::Force => LAYERS.remove(id),
                                },
                                None => POPUP.close_id(id),
                            }
                        }
                    });
                }
                _ => {}
            }),
        );

        let (filter, over) = match context_capture {
            ContextCapture::NoCapture => {
                let filter = CaptureFilter::Include({
                    let mut set = ContextValueSet::new();
                    set.insert(&CLOSE_ON_FOCUS_LEAVE_VAR);
                    set.insert(&ANCHOR_MODE_VAR);
                    set.insert(&CONTEXT_CAPTURE_VAR);
                    set
                });
                (filter, false)
            }
            ContextCapture::CaptureBlend { filter, over } => (filter, over),
        };
        if filter != CaptureFilter::None {
            popup = with_context_blend(LocalContext::capture_filtered(filter), over, popup);
        }
        LAYERS.insert_anchored(LayerIndex::TOP_MOST, anchor_id, anchor_mode, popup);

        state.read_only()
    }

    /// Close the popup widget when `state` is not already closed.
    ///
    /// Notifies [`POPUP_CLOSE_REQUESTED_EVENT`] and then close if no subscriber stops propagation for it.
    pub fn close(&self, state: &Var<PopupState>) {
        match state.get() {
            PopupState::Opening => state
                .hook(|a| {
                    if let PopupState::Open(id) = a.downcast_value::<PopupState>().unwrap() {
                        POPUP_CLOSE_CMD.scoped(*id).notify_param(PopupCloseMode::Request);
                    }
                    false
                })
                .perm(),
            PopupState::Open(id) => self.close_id(id),
            PopupState::Closed => {}
        }
    }

    /// Close the popup widget when `state` is not already closed, without notifying [`POPUP_CLOSE_REQUESTED_EVENT`] first.
    pub fn force_close(&self, state: &Var<PopupState>) {
        match state.get() {
            PopupState::Opening => state
                .hook(|a| {
                    if let PopupState::Open(id) = a.downcast_value::<PopupState>().unwrap() {
                        POPUP_CLOSE_CMD.scoped(*id).notify_param(PopupCloseMode::Force);
                    }
                    false
                })
                .perm(),
            PopupState::Open(id) => self.force_close_id(id),
            PopupState::Closed => {}
        }
    }

    /// Close the popup widget by known ID.
    ///
    /// The `widget_id` must be the same in the [`PopupState::Open`] returned on open.
    ///
    /// You can also use the [`POPUP_CLOSE_CMD`] scoped on the popup to request or force close.    
    pub fn close_id(&self, widget_id: WidgetId) {
        setup_popup_close_service();
        POPUP_CLOSE_REQUESTED_EVENT.notify(PopupCloseRequestedArgs::now(widget_id));
    }

    /// Close the popup widget without notifying the request event.
    pub fn force_close_id(&self, widget_id: WidgetId) {
        POPUP_CLOSE_CMD.scoped(widget_id).notify_param(PopupCloseMode::Force);
    }

    /// Gets a read-only var that tracks the anchor widget in a layered widget context.
    pub fn anchor_id(&self) -> Var<WidgetId> {
        LAYERS.anchor_id().map(|id| id.expect("POPUP layers are always anchored"))
    }
}

/// Identifies the lifetime state of a popup managed by [`POPUP`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PopupState {
    /// Popup will open on the next update.
    Opening,
    /// Popup is open and can close itself, or be closed using the ID.
    Open(WidgetId),
    /// Popup is closed.
    Closed,
}

/// Popup default style.
#[widget($crate::popup::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            replace = true;

            // same as window
            background_color = light_dark(rgb(0.9, 0.9, 0.9), rgb(0.1, 0.1, 0.1));
            drop_shadow = {
                offset: 2,
                blur_radius: 2,
                color: colors::BLACK.with_alpha(50.pct()),
            };
        }
    }
}

/// Defines if a [`Popup!`] captures the build/instantiation context.
///
/// If enabled (default), the popup will build [`with_context_blend`].
///
/// [`Popup!`]: struct@Popup
/// [`with_context_blend`]: zng_wgt::prelude::with_context_blend
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ContextCapture {
    /// No context capture except the popup configuration context.
    ///
    /// The popup will only have the window context as it is open as a layer on the window root.
    ///
    /// Note to filter out even the popup config use [`CaptureFilter::None`] instead.
    NoCapture,
    /// Build/instantiation context is captured and blended with the node context during all [`UiNodeOp`].
    ///
    /// [`UiNodeOp`]: zng_wgt::prelude::UiNodeOp
    CaptureBlend {
        /// What context values are captured.
        filter: CaptureFilter,

        /// If the captured context is blended over or under the node context. If `true` all
        /// context locals and context vars captured replace any set in the node context, otherwise
        /// only captures not in the node context are inserted.
        over: bool,
    },
}
impl Default for ContextCapture {
    /// Captures all context-vars by default, and blend then over the node context.
    fn default() -> Self {
        Self::CaptureBlend {
            filter: CaptureFilter::context_vars(),
            over: true,
        }
    }
}
impl_from_and_into_var! {
    fn from(capture_vars_blend_over: bool) -> ContextCapture {
        if capture_vars_blend_over {
            ContextCapture::CaptureBlend {
                filter: CaptureFilter::ContextVars {
                    exclude: ContextValueSet::new(),
                },
                over: true,
            }
        } else {
            ContextCapture::NoCapture
        }
    }

    fn from(filter_over: CaptureFilter) -> ContextCapture {
        ContextCapture::CaptureBlend {
            filter: filter_over,
            over: true,
        }
    }
}

event_args! {
    /// Arguments for [`POPUP_CLOSE_REQUESTED_EVENT`].
    pub struct PopupCloseRequestedArgs {
        /// The popup that has close requested.
        pub popup: WidgetId,

        ..

        fn delivery_list(&self, delivery_list: &mut UpdateDeliveryList) {
            delivery_list.search_widget(self.popup)
        }
    }
}

event! {
    /// Closing popup event.
    ///
    /// Requesting [`propagation().stop()`] on this event cancels the popup close.
    ///
    /// [`propagation().stop()`]: zng_app::event::EventPropagationHandle::stop
    pub static POPUP_CLOSE_REQUESTED_EVENT: PopupCloseRequestedArgs;
}
event_property! {
    /// Closing popup event.
    ///
    /// Requesting [`propagation().stop()`] on this event cancels the popup close.
    ///
    /// [`propagation().stop()`]: zng_app::event::EventPropagationHandle::stop
    pub fn popup_close_requested {
        event: POPUP_CLOSE_REQUESTED_EVENT,
        args: PopupCloseRequestedArgs,
    }
}

command! {
    /// Close the popup.
    ///
    /// # Param
    ///
    /// The parameter can be [`PopupCloseMode`]. If not set the normal
    /// [`POPUP.close`] behavior is invoked.
    ///
    /// [`POPUP.close`]: POPUP::close
    pub static POPUP_CLOSE_CMD;
}

/// Optional parameter for [`POPUP_CLOSE_CMD`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopupCloseMode {
    /// Calls [`POPUP.close`].
    ///
    /// [`POPUP.close`]: POPUP::close
    #[default]
    Request,
    /// Calls [`POPUP.force_close`].
    ///
    /// [`POPUP.force_close`]: POPUP::force_close
    Force,
}

fn setup_popup_close_service() {
    app_local! {
        static POPUP_SETUP: bool = false;
    }

    if !std::mem::replace(&mut *POPUP_SETUP.write(), true) {
        POPUP_CLOSE_REQUESTED_EVENT
            .on_event(hn!(|args| {
                if !args.propagation().is_stopped() {
                    POPUP_CLOSE_CMD.scoped(args.popup).notify_param(PopupCloseMode::Force);
                }
            }))
            .perm();
    }
}

/// Delay awaited before actually closing when popup close is requested.
///
/// You can use this delay to await a closing animation for example. This property sets [`is_close_delaying`]
/// while awaiting the `delay`.
///
/// [`is_close_delaying`]: fn@is_close_delaying
#[property(EVENT, default(Duration::ZERO), widget_impl(Popup))]
pub fn close_delay(child: impl IntoUiNode, delay: impl IntoVar<Duration>) -> UiNode {
    let delay = delay.into_var();
    let mut timer = None::<DeadlineHandle>;

    let child = match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_event(&POPUP_CLOSE_REQUESTED_EVENT);
        }
        UiNodeOp::Deinit => {
            timer = None;
        }
        UiNodeOp::Event { update } => {
            c.event(update);
            if let Some(args) = POPUP_CLOSE_REQUESTED_EVENT.on_unhandled(update) {
                if args.popup != WIDGET.id() {
                    return;
                }

                if let Some(timer) = &timer {
                    if timer.has_executed() {
                        // allow
                        return;
                    } else {
                        args.propagation().stop();
                        // timer already running.
                        return;
                    }
                }

                let delay = delay.get();
                if delay != Duration::ZERO {
                    args.propagation().stop();

                    IS_CLOSE_DELAYED_VAR.set(true);
                    let cmd = POPUP_CLOSE_CMD.scoped(args.popup);
                    timer = Some(TIMERS.on_deadline(
                        delay,
                        hn_once!(|_| {
                            cmd.notify_param(PopupCloseMode::Force);
                        }),
                    ));
                }
            }
        }
        _ => {}
    });
    with_context_var(child, IS_CLOSE_DELAYED_VAR, var(false))
}

/// If close was requested for this layered widget and it is just awaiting for the [`close_delay`].
///
/// [`close_delay`]: fn@close_delay
#[property(EVENT+1, widget_impl(Popup))]
pub fn is_close_delaying(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    bind_state(child, IS_CLOSE_DELAYED_VAR, state)
}

context_var! {
    static IS_CLOSE_DELAYED_VAR: bool = false;
}
