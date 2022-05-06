use crate::prelude::new_widget::*;

/// Base single content container.
#[widget($crate::widgets::container)]
pub mod container {
    use super::*;

    properties! {
        /// Content UI.
        ///
        /// Can be any type that implements [`UiNode`], any widget.
        ///
        /// [`UiNode`]: zero_ui::core::UiNode
        #[allowed_in_when = false]
        #[required]
        content(impl UiNode);

        /// Spacing around content, inside the border.
        padding;

        /// Content alignment.
        child_align as content_align = Align::CENTER;

        /// Content overflow clipping.
        clip_to_bounds;
    }

    fn new_child(content: impl UiNode) -> impl UiNode {
        implicit_base::nodes::leaf_transform(content)
    }
}
