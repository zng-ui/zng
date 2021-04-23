use crate::prelude::new_widget::*;

#[widget($crate::widgets::layouts::align::center)]
mod center {
    use super::*;

    properties! {
        child {
            #[allowed_in_when = false]
            content { impl Widget };
        }
    }

    #[inline]
    fn new_child(content: impl Widget) -> impl UiNode {
        align(content, Alignment::CENTER)
    }
}

/// Centralizes the content in the available space.
///
/// This is the equivalent of setting [`align`](fn@align) to [`Alignment::CENTER`], but as a widget.
#[inline]
pub fn center(content: impl Widget) -> impl Widget {
    center! { content; }
}
