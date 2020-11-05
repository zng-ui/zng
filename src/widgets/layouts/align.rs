use crate::prelude::new_widget::*;

widget! {
    center;

    default_child {
        content -> widget_child: required!;
    }

    #[inline]
    fn new_child(content) -> impl UiNode {
        align::set(content.unwrap(), Alignment::CENTER)
    }
}

/// Centralizes the content in the available space.
///
/// This is the equivalent of setting [`align`] to [`Alignment::CENTER`], but as a widget.
#[inline]
pub fn center(content: impl UiNode) -> impl Widget {
    center! { content; }
}
