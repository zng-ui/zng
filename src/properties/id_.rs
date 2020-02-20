use crate::core::{types::WidgetId, UiNode};
use crate::property;

#[property(context)]
pub fn id(child: impl UiNode, id: WidgetId) -> impl UiNode {
    eprintln!("id property cannot be set directly, must be captured in widget!'s new()");
    child
}
