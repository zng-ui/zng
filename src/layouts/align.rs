use crate::core2::UiNode;
use crate::properties::{align, Alignment};

pub fn center(child: impl UiNode) -> impl UiNode {
    align::set(child, Alignment::CENTER)
}
