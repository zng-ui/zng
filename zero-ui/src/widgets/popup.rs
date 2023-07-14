//! Popup widget.

use crate::core::focus::{DirectionalNav, TabNav};

use crate::{prelude::new_widget::*, widgets::window::layers::LAYERS};

/// An overlay container.
///
/// # LAYERS
///
/// The popup widget is designed to be used as a temporary *flyover* container inserted as a
/// top-most layer using [`LAYERS`]. By default the widget is an [`alt_focus_scope`] that is [`focus_on_init`],
/// cycles [`directional_nav`] and [`tab_nav`], has [`FocusClickBehavior::ExitEnabled`] and removes itself
/// when it loses focus.
///
/// # Context Capture
///
/// This widget captures the context (context vars, locals) at the moment the widget is instantiated,
/// it then loads this context for all node operations. This means that you can instantiate a popup
/// in a context that sets styles that affect the popup contents, even though the popup will not
/// be initialized inside that context.
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

            on_focus_leave = hn!(|_| {
                if CLOSE_ON_FOCUS_LEAVE_VAR.get() {
                    LAYERS.remove(WIDGET.id());
                }
            });
        }
    }

    /// Builds the popup widget, if `context_capture` is enabled the calling context is captured.
    pub fn widget_build(&mut self) -> impl UiNode {
        match self.widget_builder().capture_value_or_default(property_id!(Self::context_capture)) {
            ContextCapture::CaptureBlend { over } => {
                with_context_blend(LocalContext::capture(), over, WidgetBase::widget_build(self)).boxed()
            }
            ContextCapture::DontCapture => WidgetBase::widget_build(self).boxed(),
        }
    }

    widget_impl! {
        /// Popup focus behavior when it or a descendant receives a click.
        ///
        /// Is [`FocusClickBehavior::ExitEnabled`] by default;
        pub focus_click_behavior(behavior: impl IntoVar<FocusClickBehavior>);
    }
}

/// Defines if the popup captures the build/instantiate context and sets it
/// in the node context.
///
/// This is enabled by default and lets the popup use context values from the widget
/// that opens it, not just from the window [`LAYERS`] root where it will actually be inited.
/// There are potential issues with this, see [`ContextCapture`] for more details.
#[property(WIDGET, capture, widget_impl(Popup))]
pub fn context_capture(mode: impl IntoValue<ContextCapture>) {}

context_var! {
    /// Popup style in a context.
    ///
    /// Is the [`DefaultStyle!`] by default.
    ///
    /// [`DefaultStyle!`]: struct@DefaultStyle
    pub static STYLE_VAR: StyleFn = StyleFn::new(|_| DefaultStyle!());

    /// If popup will close when it it is no longer contains the focused widget.
    pub static CLOSE_ON_FOCUS_LEAVE_VAR: bool = true;
}

/// Popup behavior when it loses focus.
///
/// If `true` the popup will remove it self from [`LAYERS`], is `true` by default.
///
/// Sets the [`CLOSE_ON_FOCUS_LEAVE_VAR`].
#[property(CONTEXT, default(CLOSE_ON_FOCUS_LEAVE_VAR))]
pub fn clone_on_focus_leave(child: impl UiNode, close: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, CLOSE_ON_FOCUS_LEAVE_VAR, close)
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
                color: colors::BLACK,
            };
        }
    }
}

/// Defines if a [`Popup!`] captures the build/instantiation context.
///
/// If enabled (default), the popup will build [`with_context_blend`].
///
/// [`Popup!`]: struct@Popup
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ContextCapture {
    /// No context capture or blending, the popup will have
    /// the context it is inited in, like any other widget.
    DontCapture,
    /// Build/instantiation context is captured and blended with the node context during all [`UiNodeOp`].
    CaptureBlend {
        /// If the captured context is blended over or under the node context. If `true` all
        /// context locals and context vars captured replace any set in the node context, otherwise
        /// only captures not in the node context are inserted.
        over: bool,
    },
}
impl Default for ContextCapture {
    /// Is `CaptureBlend { over: true }` by default.
    fn default() -> Self {
        Self::CaptureBlend { over: true }
    }
}
impl_from_and_into_var! {
    fn from(capture_blend_over: bool) -> ContextCapture {
        if capture_blend_over {
            ContextCapture::CaptureBlend { over: true }
        } else {
            ContextCapture::DontCapture
        }
    }
}
