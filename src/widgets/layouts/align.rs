use crate::core::{units::Alignment, UiNode};
use crate::properties::align;

/// Centralizes the node.
#[inline]
pub fn center(child: impl UiNode) -> impl UiNode {
    align::set(child, Alignment::CENTER)
}
