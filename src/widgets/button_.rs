use crate::core::types::{rgb, ColorF, LayoutSideOffsets};
use crate::core::var::context_var;
use crate::core::widget;
use crate::properties::{background_color, border, is_hovered, is_pressed, on_click, BorderDetails};
use crate::widgets::{container, mixins::focusable_mixin};

context_var! {
    /// Default background of [`button!`](crate::widgets::button) widgets.
    pub struct ButtonBackgroundVar: ColorF = once rgb(0.2, 0.2, 0.2);
    pub struct ButtonBackgroundHoveredVar: ColorF = once rgb(0.25, 0.25, 0.25);
    pub struct ButtonBackgroundPressedVar: ColorF = once rgb(0.3, 0.3, 0.3);

    pub struct ButtonBorderWidthsVar: LayoutSideOffsets = once LayoutSideOffsets::new_all_same(1.0);
    pub struct ButtonBorderWidthsHoveredVar: LayoutSideOffsets = once LayoutSideOffsets::new_all_same(1.0);
    pub struct ButtonBorderWidthsPressedVar: LayoutSideOffsets = once LayoutSideOffsets::new_all_same(1.0);

    pub struct ButtonBorderDetailsVar: BorderDetails = once BorderDetails::solid(rgb(0.2, 0.2, 0.2));
    pub struct ButtonBorderDetailsHoveredVar: BorderDetails = once BorderDetails::solid(rgb(0.4, 0.4, 0.4));
    pub struct ButtonBorderDetailsPressedVar: BorderDetails = once BorderDetails::solid(rgb(0.6, 0.6, 0.6));

    pub struct ButtonPaddingVar: LayoutSideOffsets = once LayoutSideOffsets::new(7.0, 15.0, 7.0, 15.0);
}

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
