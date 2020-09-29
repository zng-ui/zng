use crate::core::{units::Alignment, widget, UiNode, Widget};
use crate::properties::{align, capture_only::widget_child};

widget! {
    center;

    default_child {
        child -> widget_child: required!;
    }

    #[inline]
    fn new_child(child) -> impl UiNode {
        align::set(child.unwrap(), Alignment::CENTER)
    }
}

/// Centralizes the node.
#[inline]
pub fn center(child: impl UiNode) -> impl Widget {
    center! { child; }
}
