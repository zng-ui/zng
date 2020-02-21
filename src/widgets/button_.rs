use crate::core::types::{rgb, ColorF};
pub use crate::properties::{align, background_color, on_click};
use crate::widget;
use crate::widgets::container;

context_var! {
    /// Default background of [`button!`](crate::widgets::button) widgets.
    pub struct ButtonBackground: ColorF = rgb(0, 0, 0);
    pub struct ButtonBackgroundHovered: ColorF = rgb(0, 0, 0);
    pub struct ButtonBackgroundPressed: ColorF = rgb(0, 0, 0);
    pub struct ButtonBackgroundDisabled: ColorF = rgb(0, 0, 0);
}

widget! {
    /// A clickable container.
    pub button: container;

    use crate::properties::{on_click, background_color};
    use crate::widgets::ButtonBackground;

    default(self) {
        /// Button click event.
        on_click: required!;

        /// Set to [`ButtonBackground`](crate::widgets::ButtonBackground).
        background_color: ButtonBackground;
    }

    /// When the button has keyboard focus.
    when self.is_focused {

    }

    /// When the pointer device is over this button.
    when self.is_hovered  {
        background_color: ButtonBackgroundHovered;
    }

    /// When the pointer device is over this button.
    when self.is_hovered && self.is_focused {
        background_color: ButtonBackgroundHovered;
    }

    /// When the mouse or touch pressed on this button and has not yet released.
    when self.is_pressed  {
        background_color: ButtonBackgroundPressed;
    }

    /// When the button is not enabled.
    when !self.is_enabled {
        background_color: ButtonBackgroundDisabled;
    }
}

/// Docs reference [rgb](rgb) works here.
pub mod button_w {
    #[doc(hidden)]
    pub use super::*;

    /// New child docs.
    #[inline]
    pub fn new_child<C: zero_ui::core::UiNode>(child: C) -> C {
        zero_ui::core::default_new_widget_child(child)
    }

    /// New widget docs.
    #[inline]
    pub fn new(child: impl zero_ui::core::UiNode, id: impl zero_ui::properties::id::Args) -> impl zero_ui::core::UiNode {
        zero_ui::core::default_new_widget(child, id)
    }

    // Properties used in widget.
    #[doc(hidden)]
    pub mod ps {
        // validate re-export.
        pub use super::on_click;

        // Alias and validate.
        pub use super::align as content_align;

        pub use super::background_color;
    }

    // Default values from the widget.
    #[doc(hidden)]
    pub mod df {
        use super::*;

        pub fn background_color() -> impl ps::background_color::Args {
            ps::background_color::args(ButtonBackground)
        }
    }
}
