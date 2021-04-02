use crate::prelude::new_widget::*;

/// A clickable container.
#[widget($crate::widgets::button)]
pub mod button {
    use super::*;
    use crate::properties::button_theme::*;
    use crate::properties::capture_mouse;
    use crate::properties::text_theme::{TextColorDisabledVar, TextColorVar};

    inherit!(focusable_mixin);
    inherit!(container);

    properties! {
        /// Button click event.
        on_click;

        /// Set to [`ButtonBackgroundVar`].
        background_color = ButtonBackgroundVar;

        /// Set to [`ButtonBorderWidthsVar`] and [`ButtonBorderDetailsVar`].
        border = {
            widths: ButtonBorderWidthsVar,
            details: ButtonBorderDetailsVar,
        };

        text_color = TextColorVar;

        /// Enabled by default.
        ///
        /// Blocks pointer interaction with other widgets while the button is pressed.
        capture_mouse = true;

        child {
            /// Set to [`ButtonPaddingVar`].
            padding = ButtonPaddingVar;
        }

        /// When the pointer device is over this button.
        when self.is_cap_hovered {
            background_color = ButtonBackgroundHoveredVar;
            border = {
                widths: ButtonBorderWidthsHoveredVar,
                details: ButtonBorderDetailsHoveredVar,
            };
        }

        /// When the button is pressed in a way that press release will cause a button click.
        when self.is_pressed  {
            background_color = ButtonBackgroundPressedVar;
            border = {
                widths: ButtonBorderWidthsPressedVar,
                details: ButtonBorderDetailsPressedVar,
            };
        }

        /// When the button is disabled.
        when self.is_disabled {
            background_color = ButtonBackgroundDisabledVar;
            border = {
                widths: ButtonBorderWidthsDisabledVar,
                details: ButtonBorderDetailsDisabledVar,
            };
            text_color = TextColorDisabledVar;
        }
    }
}
