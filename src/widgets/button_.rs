use crate::prelude::new_widget::*;
use crate::properties::button_theme::*;
use crate::properties::text_theme::{TextColorDisabledVar, TextColorVar};

widget! {
    /// A clickable container.
    pub button: container + focusable_mixin;

    default {
        /// Button click event.
        on_click;

        /// Set to [`ButtonBackgroundVar`].
        background_color: ButtonBackgroundVar;

        /// Set to [`ButtonBorderWidthsVar`] and [`ButtonBorderDetailsVar`].
        border: {
            widths: ButtonBorderWidthsVar,
            details: ButtonBorderDetailsVar,
        };

        text_color: TextColorVar;
    }

    default_child {
        /// Set to [`ButtonPaddingVar`].
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

    when !self.is_enabled {
        background_color: ButtonBackgroundDisabledVar;
        border: {
            widths: ButtonBorderWidthsDisabledVar,
            details: ButtonBorderDetailsDisabledVar,
        };
        text_color: TextColorDisabledVar;
    }
}
