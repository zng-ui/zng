//! Properties that define accessibility metadata.
//!
//! The properties in this module should only be used by widget implementers, they only
//! define metadata for accessibility, this metadata signals the availability of behaviors
//! that are not implemented by these properties, for example an [`AccessRole::Button`] widget
//! must also be focusable and handle click events, an [`AccessRole::TabList`] must contain widgets
//! marked [`AccessRole::Tab`].

use crate::core::widget_info::AccessRole;

use crate::prelude::new_property::*;

/// Sets the widget kind for accessibility services.
///
/// Note that the widget role must be implemented, this property only sets the metadata,
#[property(CONTEXT, default(None))]
pub fn access_role(child: impl UiNode, role: impl IntoVar<Option<AccessRole>>) -> impl UiNode {
    let role = role.into_var();
    let mut handle = VarHandle::dummy();
    match_node(child, move |_, op| match op {
        UiNodeOp::Deinit => {
            handle = VarHandle::dummy();
        }
        UiNodeOp::Info { info } => {
            if let Some(mut info) = info.access() {
                if handle.is_dummy() {
                    handle = role.subscribe(UpdateOp::Info, WIDGET.id());
                }
                if let Some(r) = role.get() {
                    info.set_role(r);
                }
            }
        }
        _ => {}
    })
}
