use crate::core::types::{rgb, ColorF, LayoutSideOffsets};
use crate::core::var::context_var;
use crate::core::widget;
use crate::properties::{background_color, is_hovered, is_pressed, on_click};
use crate::widgets::{container, mixins::focusable_mixin};

context_var! {
    /// Default background of [`button!`](crate::widgets::button) widgets.
    pub struct ButtonBackground: ColorF = once rgb(0.2, 0.2, 0.2);
    pub struct ButtonBackgroundHovered: ColorF = once rgb(0.25, 0.25, 0.25);
    pub struct ButtonBackgroundPressed: ColorF = once rgb(0.3, 0.3, 0.3);
    pub struct ButtonPadding: LayoutSideOffsets = once LayoutSideOffsets::new(8.0, 16.0, 8.0, 16.0);
}

widget! {
    /// A clickable container.
    pub button: container + focusable_mixin;

    default {
        /// Button click event.
        on_click;

        /// Set to [`ButtonBackground`](crate::widgets::ButtonBackground).
        background_color: ButtonBackground;
    }

    default_child {
        /// Set to [`ButtonPadding`](crate::widgets::ButtonPadding).
        padding: ButtonPadding;
    }

    /// When the pointer device is over this button.
    when self.is_hovered {
        background_color: ButtonBackgroundHovered;
    }

    /// When the mouse or touch pressed on this button and has not yet released.
    when self.is_pressed  {
        background_color: ButtonBackgroundPressed;
    }
}
