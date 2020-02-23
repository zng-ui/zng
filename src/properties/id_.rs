use crate::core::{types::WidgetId, UiNode};
use crate::property;

/// Widget id.
///
/// # Placeholder
///
/// This property is a placeholder that does not do anything directly, widgets can
/// capture this value for their own initialization.
#[property(context)]
pub fn id(child: impl UiNode, id: WidgetId) -> impl UiNode {
    let _id = id;
    eprintln!("id property cannot be set directly, must be captured in widget!'s new()");
    child
}
