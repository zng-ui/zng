use crate::core::types::{rgb, ColorF};
#[doc(hidden)]
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
