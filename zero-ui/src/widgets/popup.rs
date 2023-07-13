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

    /*
    !!: TODO
    
    /// Builds [`with_local_context`] capturing the current context.
    pub fn widget_build(&mut self) -> impl UiNode {
        let wgt = self.widget_take().build();
        with_local_context(LocalContext::capture(), wgt)
    }
    */

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

// # !!: ISSUES
//
// * If the popup is created in a layer, the context style will not apply.
// * We could try to capture the context, see what happens.
//   - Panic because `LAYOUT` is not available.
//   - This is a general issue, we need to capture only context-vars?
// * We can use a different context tracker for context-vars.
//   - Not impossible to have a context-var that is used like `LAYOUT` and
//     a `context_local!` that affects style.

/// Node that re-contextualizes `child` to `ctx`.
///
/// Context vars will have the values they have inside `ctx`, not where the node is inited, same for
/// most `context_local!` values.
///
/// If `child` is a widget, the returned node will be that widget, that is, [`UiNode::with_context`]
/// will be the same as `child.with_context`, the `ctx` is not loaded for this, only for the node operations.
///
/// # Warning
///
/// Not all contexts will work, in particular, `context_local!` used for immediate communication between
/// parent and child will break if used by `child`. The **only recommended usage** is when `child` is
/// a full widget and it will only have the window for parent ([`LAYERS`]).
///
/// # Panics
///
/// Panics if the `ctx` is from a different app.
pub fn with_local_context(mut ctx: LocalContext, child: impl UiNode) -> impl UiNode {
    match_widget(child, move |c, op| {
        if let UiNodeOp::Init = op {
            let init_app = LocalContext::current_app();
            ctx.with_context(|| {
                let ctx_app = LocalContext::current_app();
                assert_eq!(init_app, ctx_app);
                c.op(op)
            });
        } else {
            ctx.with_context(|| c.op(op));
        }
    })
}
