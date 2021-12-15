use crate::prelude::new_widget::*;

/// Base single content container.
#[widget($crate::widgets::container)]
pub mod container {
    use super::*;

    properties! {
        child {
            /// Content UI.
            ///
            /// Can be any type that implements [`UiNode`](zero_ui::core::UiNode), any widget.
            #[allowed_in_when = false]
            #[required]
            content(impl UiNode);
            /// Content margin.
            margin as padding;
            /// Content alignment.
            align as content_align = Alignment::CENTER;
            /// Content overflow clipping.
            clip_to_bounds;
        }
    }

    #[inline]
    fn new_child(content: impl UiNode) -> impl UiNode {
        content
    }
}
