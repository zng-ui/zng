//! Popup widget.

use crate::prelude::new_widget::*;
use crate::widgets::window::layers::LAYERS;

/// An overlay container designed for use in [`LAYERS`].
#[widget($crate::widgets::popup::Popup)]
pub struct Popup(FocusableMix<StyleMix<EnabledMix<Container>>>);
impl Popup {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            style_fn = STYLE_VAR;
        }
    }
}

context_var! {
    /// Popup style in a context.
    ///
    /// Is the [`DefaultStyle!`] by default.
    ///
    /// [`DefaultStyle!`]: struct@DefaultStyle
    pub static STYLE_VAR: StyleFn = StyleFn::new(|_| DefaultStyle!());
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
        }
    }
}

// # !!: ISSUES
//
// * If the popup is created in a layer, the context style will not apply.
// * We could try to capture the context, see what happens.

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
