use crate::prelude::new_widget::*;

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

    /// `content.0` is the new child.
    #[inline]
    fn new_child(content) -> impl UiNode {
        content.unwrap()
    }
}
