use crate::core::color::rgba;
use crate::core::types::LayoutSideOffsets;
use crate::core::var::context_var;
use crate::core::widget_mixin;
use crate::properties::{focusable, foreground_highlight, is_focused_hgl, BorderDetails};

context_var! {
    pub struct FocusHighlightWidthsVar: LayoutSideOffsets = once LayoutSideOffsets::new_all_same(0.5);
    pub struct FocusHighlightOffsetsVar: LayoutSideOffsets = once LayoutSideOffsets::new_all_same(1.0);
    pub struct FocusHighlightDetailsVar: BorderDetails = once BorderDetails::dashed(rgba(0, 255, 255, 1.0));
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
            details: FocusHighlightDetailsVar
        };
    }

    when self.is_focused_hgl {
        focus_highlight: {
            widths: FocusHighlightWidthsVar,
            offsets: FocusHighlightOffsetsVar,
            details: FocusHighlightDetailsVar
        };
    }
}
