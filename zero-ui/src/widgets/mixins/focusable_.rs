use crate::core::widget_mixin;

/// Focusable widget mix-in. Enables keyboard focusing on the widget and adds a focused
/// highlight border.
#[widget_mixin($crate::widgets::mixins::focusable_mixin)]
pub mod focusable_mixin {
    use crate::core::border::{BorderSides, BorderStyle};
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
        };

        /// When widget has keyboard focus and highlight is requested.
        when self.is_focused_hgl {
            focus_highlight = {
                offsets: theme::FocusHighlightOffsetsVar,
                widths: theme::FocusHighlightWidthsVar,
                sides: theme::FocusHighlightSidesVar,
            };
        }
    }

    /// Theme variables.
    pub mod theme {
        use super::*;
        use crate::prelude::new_property::*;

        context_var! {
            /// Padding offsets of the `focus_highlight` when the widget is focused.
            pub struct FocusHighlightOffsetsVar: SideOffsets = SideOffsets::new_all(1);
            /// Border widths of the `focus_highlight` when the widget is focused.
            pub struct FocusHighlightWidthsVar: SideOffsets = SideOffsets::new_all(0.5);
            /// Border sides of the `focus_highlight` when the widget is focused.
            pub struct FocusHighlightSidesVar: BorderSides = BorderSides::dashed(rgba(200, 200, 200, 1.0));
        }

        /// Sets the `focus_highlight` values used when the widget is focused and highlighted.
        #[property(context, default(FocusHighlightOffsetsVar, FocusHighlightWidthsVar, FocusHighlightSidesVar))]
        pub fn focus_highlight(
            child: impl UiNode,
            offsets: impl IntoVar<SideOffsets>,
            widths: impl IntoVar<SideOffsets>,
            sides: impl IntoVar<BorderSides>,
        ) -> impl UiNode {
            let child = with_context_var(child, FocusHighlightWidthsVar, offsets);
            let child = with_context_var(child, FocusHighlightOffsetsVar, widths);
            with_context_var(child, FocusHighlightSidesVar, sides)
        }
    }
}
