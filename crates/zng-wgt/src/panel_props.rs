use zng_app::widget::node::Z_INDEX;

use crate::prelude::*;

/// Defines the render order of a widget in a layout panel.
///
/// When set the widget will still update and layout according to their *logical* position in the list but
/// they will render according to the order defined by the [`ZIndex`] value.
///
/// An error is logged on init if the widget is not a direct child of a Z-sorting panel.
///
/// [`ZIndex`]: zng_app::widget::node::ZIndex
#[property(CONTEXT, default(ZIndex::DEFAULT))]
pub fn z_index(child: impl IntoUiNode, index: impl IntoVar<ZIndex>) -> UiNode {
    let index = index.into_var();
    let mut valid = false;

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            valid = Z_INDEX.set(index.get());

            if valid {
                WIDGET.sub_var(&index);
            } else {
                tracing::error!(
                    "property `z_index` set for `{}` but it is not the direct child of a Z-sorting panel",
                    WIDGET.trace_id()
                );
            }
        }
        UiNodeOp::Update { .. } => {
            if valid {
                if let Some(i) = index.get_new() {
                    assert!(Z_INDEX.set(i));
                }
            }
        }
        _ => {}
    })
}
