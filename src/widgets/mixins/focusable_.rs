use crate::core::widget_mixin;

/// Focusable widget mix-in. Enables keyboard focusing on the widget and adds a focused
/// highlight border.
#[widget_mixin($crate::widgets::mixins::focusable_mixin)]
pub mod focusable_mixin {
    use crate::core::color::rgba;
    use crate::core::line::BorderDetails;
    use crate::core::units::SideOffsets;
    use crate::core::var::context_var;
    use crate::properties::{
        focus::{focusable, is_focused_hgl},
        foreground_highlight,
    };

    context_var! {
        pub struct FocusHighlightWidthsVar: SideOffsets = once SideOffsets::new_all(0.5);
        pub struct FocusHighlightOffsetsVar: SideOffsets = once SideOffsets::new_all(1.0);
        pub struct FocusHighlightDetailsVar: BorderDetails = once BorderDetails::dashed(rgba(200, 200, 200, 1.0));
    }

    properties! {
        /// Enables keyboard focusing in the widget.
        focusable = true;

        /// A border overlay that is visible when the widget is focused.
        foreground_highlight as focus_highlight = {
            widths: SideOffsets::new_all(0.0),
            offsets: SideOffsets::new_all(0.0),
            details: FocusHighlightDetailsVar
        };

        when self.is_focused_hgl {
            focus_highlight = {
                widths: FocusHighlightWidthsVar,
                offsets: FocusHighlightOffsetsVar,
                details: FocusHighlightDetailsVar
            };
        }
    }
}
