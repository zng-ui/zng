use crate::core::types::{rgba, LayoutSideOffsets};
use crate::core::var::context_var;
use crate::core::widget_mixin;
use crate::properties::{focusable, foreground_highlight, is_focused_hgl, BorderDetails};

context_var! {
    pub struct FocusedBorderWidths: LayoutSideOffsets = once LayoutSideOffsets::new_all_same(1.0);
    pub struct FocusedBorderOffsets: LayoutSideOffsets = once LayoutSideOffsets::new_all_same(2.0);
    pub struct FocusedBorderDetails: BorderDetails = once BorderDetails::dashed(rgba(0, 255, 255, 0.7));
}

widget_mixin! {
    /// Focusable widget mix-in. Enables keyboard focusing on the widget and adds a focused
    /// highlight border.
    pub focusable_mixin;

    default {

        /// Enables keyboard focusing in this widget.
        focusable: true;

        /// A border overlay that is visible when the widget is focused.
        focus_highlight -> foreground_highlight: {
            widths: LayoutSideOffsets::new_all_same(0.0),
            offsets: LayoutSideOffsets::new_all_same(0.0),
            details: FocusedBorderDetails
        };
    }

    when self.is_focused_hgl {
        focus_highlight: {
            widths: FocusedBorderWidths,
            offsets: FocusedBorderOffsets,
            details: FocusedBorderDetails
        };
    }
}
