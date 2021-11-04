use crate::core::widget_mixin;

/// Focusable widget mix-in. Enables keyboard focusing on the widget and adds a focused
/// highlight border.
#[widget_mixin($crate::widgets::mixins::focusable_mixin)]
pub mod focusable_mixin {
    use crate::core::border::{BorderRadius, BorderSides, BorderStyle};
    use crate::core::color::rgba;
    use crate::core::units::SideOffsets;
    use crate::core::var::context_var;
    use crate::properties::{
        focus::{focusable, is_focused_hgl},
        foreground_highlight,
    };

    properties! {
        /// Enables keyboard focusing in the widget.
        focusable = true;

        /// A border overlay that is visible when the widget is focused.
        foreground_highlight as focus_highlight = {
            offsets: 0,
            widths: 0,
            sides: BorderStyle::Hidden,
            radius: 0
        };

        /// When widget has keyboard focus and highlight is requested.
        when self.is_focused_hgl {
            focus_highlight = {
                offsets: theme::FocusHighlightOffsetsVar,
                widths: theme::FocusHighlightWidthsVar,
                sides: theme::FocusHighlightSidesVar,
                radius: theme::FocusHighlightRadiusVar
            };
        }
    }

    /// Theme variables.
    pub mod theme {
        use super::*;

        context_var! {
            pub struct FocusHighlightWidthsVar: SideOffsets = SideOffsets::new_all(0.5);
            pub struct FocusHighlightOffsetsVar: SideOffsets = SideOffsets::new_all(1.0);
            pub struct FocusHighlightSidesVar: BorderSides = BorderSides::dashed(rgba(200, 200, 200, 1.0));
            pub struct FocusHighlightRadiusVar: BorderRadius = BorderRadius::new_all(2.0);
        }
    }
}
