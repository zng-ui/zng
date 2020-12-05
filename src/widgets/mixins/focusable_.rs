use crate::core::color::rgba;
use crate::core::units::SideOffsets;
use crate::core::var::context_var;
use crate::core::widget_mixin;
use crate::properties::{border::BorderDetails, focus::focusable, foreground::foreground_highlight, states::is_focused_hgl};

context_var! {
    pub struct FocusHighlightWidthsVar: SideOffsets = once SideOffsets::new_all(0.5);
    pub struct FocusHighlightOffsetsVar: SideOffsets = once SideOffsets::new_all(1.0);
    pub struct FocusHighlightDetailsVar: BorderDetails = once BorderDetails::dashed(rgba(0, 255, 255, 1.0));
}

widget_mixin! {
    /// Focusable widget mix-in. Enables keyboard focusing on the widget and adds a focused
    /// highlight border.
    pub focusable_mixin;

    default {

        /// Enables keyboard focusing in the widget.
        focusable: true;

        /// A border overlay that is visible when the widget is focused.
        focus_highlight -> foreground_highlight: {
            widths: SideOffsets::new_all(0.0),
            offsets: SideOffsets::new_all(0.0),
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
