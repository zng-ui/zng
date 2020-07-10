use crate::core::UiNode;
use crate::properties::{align, Alignment};

/// Centralizes the node.
#[inline]
pub fn center(child: impl UiNode) -> impl UiNode {
    align::set(child, Alignment::CENTER)
}
