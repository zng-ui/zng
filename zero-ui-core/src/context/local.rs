use std::{any::Any, sync::Arc};

pub use zero_ui_app_context::*;

use crate::widget_instance::{match_node, match_widget, UiNode, UiNodeOp};

/// Helper for declaring nodes that sets a context local.
pub fn with_context_local<T: Any + Send + Sync + 'static>(
    child: impl UiNode,
    context: &'static ContextLocal<T>,
    value: impl Into<T>,
) -> impl UiNode {
    let mut value = Some(Arc::new(value.into()));

    match_node(child, move |child, op| {
        context.with_context(&mut value, || child.op(op));
    })
}

/// Helper for declaring nodes that sets a context local with a value generated on init.
///
/// The method calls the `init_value` closure on init to produce a *value* var that is presented as the [`ContextLocal<T>`]
/// in the widget and widget descendants. The closure can be called more than once if the returned node is reinited.
///
/// Apart from the value initialization this behaves just like [`with_context_local`].
pub fn with_context_local_init<T: Any + Send + Sync + 'static>(
    child: impl UiNode,
    context: &'static ContextLocal<T>,
    init_value: impl FnMut() -> T + Send + 'static,
) -> impl UiNode {
    #[cfg(dyn_closure)]
    let init_value: Box<dyn FnMut() -> T + Send> = Box::new(init_value);
    with_context_local_init_impl(child.cfg_boxed(), context, init_value).cfg_boxed()
}
fn with_context_local_init_impl<T: Any + Send + Sync + 'static>(
    child: impl UiNode,
    context: &'static ContextLocal<T>,
    mut init_value: impl FnMut() -> T + Send + 'static,
) -> impl UiNode {
    let mut value = None;

    match_node(child, move |child, op| {
        let mut is_deinit = false;
        match &op {
            UiNodeOp::Init => {
                value = Some(Arc::new(init_value()));
            }
            UiNodeOp::Deinit => {
                is_deinit = true;
            }
            _ => {}
        }

        context.with_context(&mut value, || child.op(op));

        if is_deinit {
            value = None;
        }
    })
}

/// Helper for declaring widgets that are recontextualized to take in some of the context
/// of an *original* parent.
///
/// See [`LocalContext::with_context_blend`] for more details about `over`. The returned
/// node will delegate all node operations to inside the blend. The [`UiNode::with_context`]
/// will delegate to the `child` widget context, but the `ctx` is not blended for this method, only
/// for [`UiNodeOp`] methods.
///
/// # Warning
///
/// Properties, context vars and context locals are implemented with the assumption that all consumers have
/// released the context on return, that is even if the context was shared with worker threads all work was block-waited.
/// This node breaks this assumption, specially with `over: true` you may cause unexpected behavior if you don't consider
/// carefully what context is being captured and what context is being replaced.
///
/// As a general rule, only capture during init or update in [`NestGroup::CHILD`], only wrap full widgets and only place the wrapped
/// widget in a parent's [`NestGroup::CHILD`] for a parent that has no special expectations about the child.
///
/// As an example of things that can go wrong, if you capture during layout, the `LAYOUT` context is captured
/// and replaces `over` the actual layout context during all subsequent layouts in the actual parent.
///
/// # Panics
///
/// Panics during init if `ctx` is not from the same app as the init context.
///
/// [`NestGroup::CHILD`]: crate::widget_builder::NestGroup::CHILD
pub fn with_context_blend(mut ctx: LocalContext, over: bool, child: impl UiNode) -> impl UiNode {
    match_widget(child, move |c, op| {
        if let UiNodeOp::Init = op {
            let init_app = LocalContext::current_app();
            ctx.with_context_blend(over, || {
                let ctx_app = LocalContext::current_app();
                assert_eq!(init_app, ctx_app);
                c.op(op)
            });
        } else {
            ctx.with_context_blend(over, || c.op(op));
        }
    })
}
