//! Popup widget.

use std::time::Duration;

use crate::{
    core::{
        focus::{DirectionalNav, TabNav, FOCUS_CHANGED_EVENT},
        timer::{DeadlineHandle, TIMERS},
    },
    prelude::new_widget::*,
    widgets::window::layers::{AnchorMode, AnchorOffset, LayerIndex, LAYERS},
};

/// An overlay container.
///
/// # POPUP
///
/// The popup widget is designed to be used as a temporary *flyover* container inserted as a
/// top-most layer using [`POPUP`]. By default the widget is an [`alt_focus_scope`] that is [`focus_on_init`],
/// cycles [`directional_nav`] and [`tab_nav`], and has [`FocusClickBehavior::ExitEnabled`].
///
/// [`alt_focus_scope`]: fn@alt_focus_scope
/// [`focus_on_init`]: fn@focus_on_init
/// [`directional_nav`]: fn@directional_nav
/// [`tab_nav`]: fn@tab_nav
#[widget($crate::widgets::popup::Popup {
    ($child:expr) => {
        child = $child;
    }
})]
pub struct Popup(FocusableMix<StyleMix<EnabledMix<Container>>>);
impl Popup {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            style_fn = STYLE_VAR;

            alt_focus_scope = true;
            directional_nav = DirectionalNav::Cycle;
            tab_nav = TabNav::Cycle;
            focus_click_behavior = FocusClickBehavior::ExitEnabled;
            focus_on_init = true;
        }
    }

    widget_impl! {
        /// Popup focus behavior when it or a descendant receives a click.
        ///
        /// Is [`FocusClickBehavior::ExitEnabled`] by default;
        pub focus_click_behavior(behavior: impl IntoVar<FocusClickBehavior>);
    }
}

context_var! {
    /// Popup style in a context.
    ///
    /// Is the [`DefaultStyle!`] by default.
    ///
    /// [`DefaultStyle!`]: struct@DefaultStyle
    pub static STYLE_VAR: StyleFn = StyleFn::new(|_| DefaultStyle!());

    /// If popup will close when it it is no longer contains the focused widget.
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
/// If `true` the popup will remove it self from [`LAYERS`], is `true` by default.
///
/// Sets the [`CLOSE_ON_FOCUS_LEAVE_VAR`].
#[property(CONTEXT, default(CLOSE_ON_FOCUS_LEAVE_VAR))]
pub fn close_on_focus_leave(child: impl UiNode, close: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, CLOSE_ON_FOCUS_LEAVE_VAR, close)
}

/// Defines the popup placement and size for popups open by the widget or descendants.
#[property(CONTEXT, default(ANCHOR_MODE_VAR))]
pub fn anchor_mode(child: impl UiNode, mode: impl IntoVar<AnchorMode>) -> impl UiNode {
    with_context_var(child, ANCHOR_MODE_VAR, mode)
}

/// Defines if the popup captures the build/instantiate context and sets it
/// in the node context.
///
/// This is enabled by default and lets the popup use context values from the widget
/// that opens it, not just from the window [`LAYERS`] root where it will actually be inited.
/// There are potential issues with this, see [`ContextCapture`] for more details.
///
/// Note that updates to this property do not affect popups already open, just subsequent popups.
#[property(CONTEXT, default(CONTEXT_CAPTURE_VAR))]
pub fn context_capture(child: impl UiNode, capture: impl IntoVar<ContextCapture>) -> impl UiNode {
    with_context_var(child, CONTEXT_CAPTURE_VAR, capture)
}

/// Popup service.
pub struct POPUP;
impl POPUP {
    /// Open the `popup` using the current context configuration.
    pub fn open(&self, popup: impl UiNode) -> ReadOnlyArcVar<PopupState> {
        self.open_impl(popup.boxed(), ANCHOR_MODE_VAR, CONTEXT_CAPTURE_VAR.get())
    }

    /// Open the `popup` using the custom config vars.
    pub fn open_config(
        &self,
        popup: impl UiNode,
        anchor_mode: impl IntoVar<AnchorMode>,
        context_capture: impl IntoValue<ContextCapture>,
    ) -> ReadOnlyArcVar<PopupState> {
        self.open_impl(popup.boxed(), anchor_mode.into_var(), context_capture.into())
    }

    fn open_impl(
        &self,
        mut popup: BoxedUiNode,
        anchor_mode: impl Var<AnchorMode>,
        context_capture: ContextCapture,
    ) -> ReadOnlyArcVar<PopupState> {
        let state = var(PopupState::Opening);
        let mut _close_handle = CommandHandle::dummy();

        popup = match_widget(
            popup,
            clmv!(state, |c, op| match op {
                UiNodeOp::Init => {
                    c.init();

                    let id = c.with_context(WidgetUpdateMode::Bubble, || {
                        WIDGET.sub_event(&FOCUS_CHANGED_EVENT);
                        WIDGET.id()
                    });
                    if let Some(id) = id {
                        state.set(PopupState::Open(id));
                        _close_handle = POPUP_CLOSE_CMD.scoped(id).subscribe(true);
                    } else {
                        state.set(PopupState::Closed);
                    }
                }
                UiNodeOp::Deinit => {
                    state.set(PopupState::Closed);
                    _close_handle = CommandHandle::dummy();
                }
                UiNodeOp::Event { update } => {
                    c.with_context(WidgetUpdateMode::Bubble, || {
                        let id = WIDGET.id();

                        if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                            if args.is_focus_leave(id) && CLOSE_ON_FOCUS_LEAVE_VAR.get() {
                                POPUP.close(id);
                            }
                        } else if let Some(args) = POPUP_CLOSE_CMD.scoped(id).on_unhandled(update) {
                            match args.param::<PopupCloseMode>() {
                                Some(s) => match s {
                                    PopupCloseMode::Request => POPUP.close(id),
                                    PopupCloseMode::Force => LAYERS.remove(id),
                                },
                                None => POPUP.close(id),
                            }
                        }
                    });
                }
                _ => {}
            }),
        )
        .boxed();

        if let ContextCapture::CaptureBlend { filter, over } = context_capture {
            if filter != CaptureFilter::None {
                popup = with_context_blend(LocalContext::capture_filtered(filter), over, popup).boxed();
            }
        }
        LAYERS.insert_anchored(LayerIndex::TOP_MOST, WIDGET.id(), anchor_mode, popup);

        state.read_only()
    }

    /// Close the popup widget.
    ///
    /// Notifies [`POPUP_CLOSE_REQUESTED_EVENT`] and then close if no subscriber stops propagation for it.
    ///
    /// You can also use the [`POPUP_CLOSE_CMD`] to request or force close.
    pub fn close(&self, widget_id: WidgetId) {
        setup_popup_close_service();
        POPUP_CLOSE_REQUESTED_EVENT.notify(PopupCloseRequestedArgs::now(widget_id));
    }

    /// Close the popup widget without notifying the request event.
    pub fn force_close(&self, widget_id: WidgetId) {
        POPUP_CLOSE_CMD.scoped(widget_id).notify_param(PopupCloseMode::Force);
    }
}

/// Identifies the lifetime state of a popup managed by [`POPUP`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PopupState {
    /// Popup will open on the next update.
    Opening,
    /// Popup is open and can close it self, or be closed using the ID.
    Open(WidgetId),
    /// Popup is closed.
    Closed,
}

/// Sets the popup style in a context, the parent style is fully replaced.
#[property(CONTEXT, default(STYLE_VAR))]
pub fn replace_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    with_context_var(child, STYLE_VAR, style)
}

/// Extends the popup style in a context, the parent style is used, properties of the same name set in
/// `style` override the parent style.
#[property(CONTEXT, default(StyleFn::nil()))]
pub fn extend_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    style::with_style_extension(child, STYLE_VAR, style)
}

/// Popup default style.
#[widget($crate::widgets::popup::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            // same as window
            background_color = color_scheme_map(rgb(0.1, 0.1, 0.1), rgb(0.9, 0.9, 0.9));
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
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ContextCapture {
    /// No context capture or blending, the popup will have
    /// the context it is inited in, like any other widget.
    DontCapture,
    /// Build/instantiation context is captured and blended with the node context during all [`UiNodeOp`].
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
            filter: CaptureFilter::ContextVars {
                exclude: ContextValueSet::new(),
            },
            over: true,
        }
    }
}
impl_from_and_into_var! {
    fn from(capture_vars_blend_over: bool) -> ContextCapture {
        if capture_vars_blend_over {
            ContextCapture::CaptureBlend { filter: CaptureFilter::ContextVars { exclude: ContextValueSet::new() }, over: true }
        } else {
            ContextCapture::DontCapture
        }
    }

    fn from(filter_over: CaptureFilter) -> ContextCapture {
        ContextCapture::CaptureBlend { filter: filter_over, over: true }
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
    /// [`propagation().stop()`]: crate::core::event::EventPropagationHandle::stop
    pub static POPUP_CLOSE_REQUESTED_EVENT: PopupCloseRequestedArgs;
}
event_property! {
    /// Closing popup event.
    ///
    /// Requesting [`propagation().stop()`] on this event cancels the popup close.
    ///
    /// [`propagation().stop()`]: crate::core::event::EventPropagationHandle::stop
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
            .on_event(app_hn!(|args: &PopupCloseRequestedArgs, _| {
                if !args.propagation().is_stopped() {
                    POPUP_CLOSE_CMD.scoped(args.popup).notify_param(PopupCloseMode::Force);
                }
            }))
            .perm();
    }
}

/// Awaits `delay` before requesting a direct close for the popup widget after close is requested.
///
/// You can use this delay to await a closing animation for example. This property sets [`is_popup_close_delaying`]
/// while awaiting the `delay`.
///
/// [`is_popup_close_delaying`]: fn@is_popup_close_delaying
#[property(EVENT, default(Duration::ZERO), widget_impl(Popup))]
pub fn close_delay(child: impl UiNode, delay: impl IntoVar<Duration>) -> impl UiNode {
    let delay = delay.into_var();
    let mut timer = None::<DeadlineHandle>;

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_event(&POPUP_CLOSE_REQUESTED_EVENT);
        }
        UiNodeOp::Deinit => {
            timer = None;
        }
        UiNodeOp::Event { update } => {
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

                    let _ = IS_CLOSE_DELAYED_VAR.set(true);
                    let cmd = POPUP_CLOSE_CMD.scoped(args.popup);
                    timer = Some(TIMERS.on_deadline(
                        delay,
                        app_hn_once!(|_| {
                            cmd.notify_param(PopupCloseMode::Force);
                        }),
                    ));
                }
            }
        }
        _ => {}
    })
}

/// If close was requested for this layered widget and it is just awaiting for the [`popup_close_delay`].
///
/// [`popup_close_delay`]: fn@popup_close_delay
#[property(CONTEXT, widget_impl(Popup))]
pub fn is_close_delaying(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    // reverse context var, is set by `popup_close_delay`.
    with_context_var(child, IS_CLOSE_DELAYED_VAR, state)
}

context_var! {
    static IS_CLOSE_DELAYED_VAR: bool = false;
}
