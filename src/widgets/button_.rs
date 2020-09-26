use crate::core::widget;
use crate::properties::{background_color, border, button_theme::*, is_hovered, is_pressed, on_click};
use crate::widgets::{container, mixins::focusable_mixin};

widget! {
    /// A clickable container.
    pub button: container + focusable_mixin;

    default {
        /// Button click event.
        on_click;

        /// Set to [`ButtonBackground`](super::ButtonBackground).
        background_color: ButtonBackgroundVar;

        /// Set to [`ButtonBorderWidthsVar`](super::ButtonBorderWidthsVar) and
        /// [`ButtonBorderDetailsVar`](super::ButtonBorderDetailsVar).
        border: {
            widths: ButtonBorderWidthsVar,
            details: ButtonBorderDetailsVar,
        };
    }

    default_child {
        /// Set to [`ButtonPadding`](super::ButtonPadding).
        padding: ButtonPaddingVar;
    }

    /// When the pointer device is over this button.
    when self.is_hovered {
        background_color: ButtonBackgroundHoveredVar;
        border: {
            widths: ButtonBorderWidthsHoveredVar,
            details: ButtonBorderDetailsHoveredVar,
        };
    }

    /// When the mouse or touch pressed on this button and has not yet released.
    when self.is_pressed  {
        background_color: ButtonBackgroundPressedVar;
        border: {
            widths: ButtonBorderWidthsPressedVar,
            details: ButtonBorderDetailsPressedVar,
        };
    }
}
