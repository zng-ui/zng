use crate::core::{widget, UiNode};
use crate::properties::{align, capture_only::widget_child, clip_to_bounds, margin, Alignment};

widget! {
    /// Base single content container.
    pub container;

    default_child {
        /// Content UI.
        content -> widget_child: required!;
        /// Content margin.
        padding -> margin;
        /// Content alignment.
        content_align -> align: Alignment::CENTER;
        /// Content overflow clipping.
        clip_to_bounds;
    }

    fn new_child(content) -> impl UiNode {
        content.unwrap()
    }
}
