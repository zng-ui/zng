use crate::prelude::new_widget::*;

#[widget($crate::widgets::layouts::align::center)]
mod center {
    use super::*;

    properties! {
        #[allowed_in_when = false]
        content(impl Widget);
    }

    fn new_child(content: impl Widget) -> impl UiNode {
        align(content, Align::CENTER)
    }
}

/// Centralizes the content in the available space.
///
/// This is the equivalent of setting [`align`](fn@align) to [`Align::CENTER`], but as a widget.

pub fn center(content: impl Widget) -> impl Widget {
    center! { content; }
}
