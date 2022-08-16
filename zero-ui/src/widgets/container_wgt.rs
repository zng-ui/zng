use crate::prelude::new_widget::*;

/// Base single content container.
/// 
/// See also [`element`], it behaves like container, but supports theming.
/// 
/// [`element`]: mod@element
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
        child_align as content_align;

        /// Content overflow clipping.
        clip_to_bounds;
    }

    fn new_child(content: impl UiNode) -> impl UiNode {
        content
    }
}

/// Base themable single element content container.
/// 
/// This base widget has the same function as [`container`], but is also [`themable`].
/// 
/// [`container`]: mod@container
/// [`themable`]: mod@themable
#[widget($crate::widgets::element)]
pub mod element {
    use super::*;

    inherit!(themable);

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
        child_align as content_align;

        /// Content overflow clipping.
        clip_to_bounds;
    }

    fn new_child(content: impl UiNode) -> impl UiNode {
        content
    }
}