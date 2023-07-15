//! Popup widget.

use zero_ui_core::focus::FOCUS_CHANGED_EVENT;

use crate::core::focus::{DirectionalNav, TabNav};

use crate::{
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
    /// Open the `popup` with the current context.
    pub fn open(&self, popup: impl UiNode) -> ReadOnlyArcVar<PopupState> {
        self.open_impl(popup.boxed())
    }
    fn open_impl(&self, mut popup: BoxedUiNode) -> ReadOnlyArcVar<PopupState> {
        let state = var(PopupState::Opening);

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
                    } else {
                        state.set(PopupState::Closed);
                    }
                }
                UiNodeOp::Deinit => {
                    state.set(PopupState::Closed);
                }
                UiNodeOp::Event { update } => {
                    c.with_context(WidgetUpdateMode::Bubble, || {
                        if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                            let id = WIDGET.id();
                            if args.is_focus_leave(id) && CLOSE_ON_FOCUS_LEAVE_VAR.get() {
                                POPUP.close(id);
                            }
                        }
                    });
                }
                _ => {}
            }),
        )
        .boxed();

        let capture = CONTEXT_CAPTURE_VAR.get();
        if let ContextCapture::CaptureBlend { filter, over } = capture {
            if filter != CaptureFilter::None {
                popup = with_context_blend(LocalContext::capture_filtered(filter), over, popup).boxed();
            }
        }
        LAYERS.insert_anchored(LayerIndex::TOP_MOST, WIDGET.id(), ANCHOR_MODE_VAR, popup);

        state.read_only()
    }

    /// Deinit and drop the popup widget.
    pub fn close(&self, widget_id: WidgetId) {
        LAYERS.remove(widget_id);
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
