//! Properties that define accessibility metadata.
//!
//! The properties in this module should only be used by widget implementers, they only
//! define metadata for accessibility, this metadata signals the availability of behaviors
//! that are not implemented by these properties, for example an [`AccessRole::Button`] widget
//! must also be focusable and handle click events, an [`AccessRole::TabList`] must contain widgets
//! marked [`AccessRole::Tab`].

use crate::core::widget_info::{AccessRole, AccessState}; // !!: TODO, re-export in core info?

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
            if info.access_enabled() {
                if handle.is_dummy() {
                    handle = role.subscribe(UpdateOp::Info, WIDGET.id());
                }
                if let Some(r) = role.get() {
                    info.set_access_role(r);
                }
            }
        }
        _ => {}
    })
}

/// Sets accessibility metadata indicating if this widget is checked.
///
/// Values are `Some(true)` for checked, `Some(false)` for unchecked and `None` for mixed.
///
/// Note that the widget "check" operation must be implemented, this property only sets the metadata. Ideally
/// this property is set by default by the widget implementation.
#[property(CONTEXT, default(Some(false)))]
pub fn access_checked(child: impl UiNode, checked: impl IntoVar<Option<bool>>) -> impl UiNode {
    with_access_state(child, checked, |&b| Some(AccessState::Checked(b)))
}

// !!: TODO, all other states

fn with_access_state<T: VarValue>(
    child: impl UiNode,
    value: impl IntoVar<T>,
    to_state: impl Fn(&T) -> Option<AccessState> + Send + 'static,
) -> impl UiNode {
    let value = value.into_var();
    let mut handle = VarHandle::dummy();
    match_node(child, move |_, op| match op {
        UiNodeOp::Deinit => {
            handle = VarHandle::dummy();
        }
        UiNodeOp::Info { info } => {
            if info.access_enabled() {
                if handle.is_dummy() {
                    handle = value.subscribe(UpdateOp::Info, WIDGET.id());
                }

                if let Some(state) = value.with(&to_state) {
                    info.push_access_state(state);
                }
            }
        }
        _ => {}
    })
}
