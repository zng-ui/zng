use crate::core::widget_mixin;

/// Focusable widget mix-in. Enables keyboard focusing on the widget and adds a focused highlight visual.
#[widget_mixin($crate::widgets::mixins::focusable_mixin)]
pub mod focusable_mixin {
    use crate::core::border::BorderStyle;
    use crate::properties::{
        focus::{focusable, is_focused_hgl},
        foreground_highlight,
    };

    #[doc(inline)]
    pub use super::vis;

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
                offsets: vis::FOCUS_HIGHLIGHT_OFFSETS_VAR,
                widths: vis::FOCUS_HIGHLIGHT_WIDTHS_VAR,
                sides: vis::FOCUS_HIGHLIGHT_SIDES_VAR,
            };
        }
    }
}

/// Context variables and properties that affect the focusable visual from parent widgets.
pub mod vis {
    use crate::prelude::new_property::*;

    use crate::core::border::BorderSides;
    use crate::core::color::rgba;
    use crate::core::units::SideOffsets;
    use crate::core::var::context_var;

    context_var! {
        /// Padding offsets of the `focus_highlight` when the widget is focused.
        pub static FOCUS_HIGHLIGHT_OFFSETS_VAR: SideOffsets = 1;
        /// Border widths of the `focus_highlight` when the widget is focused.
        pub static FOCUS_HIGHLIGHT_WIDTHS_VAR: SideOffsets = 0.5;
        /// Border sides of the `focus_highlight` when the widget is focused.
        pub static FOCUS_HIGHLIGHT_SIDES_VAR: BorderSides = BorderSides::dashed(rgba(200, 200, 200, 1.0));
    }

    /// Sets the `focus_highlight` values used when the widget is focused and highlighted.
    #[property(
        context,
        default(FOCUS_HIGHLIGHT_OFFSETS_VAR, FOCUS_HIGHLIGHT_WIDTHS_VAR, FOCUS_HIGHLIGHT_SIDES_VAR)
    )]
    pub fn focus_highlight(
        child: impl UiNode,
        offsets: impl IntoVar<SideOffsets>,
        widths: impl IntoVar<SideOffsets>,
        sides: impl IntoVar<BorderSides>,
    ) -> impl UiNode {
        let child = with_context_var(child, FOCUS_HIGHLIGHT_WIDTHS_VAR, offsets);
        let child = with_context_var(child, FOCUS_HIGHLIGHT_OFFSETS_VAR, widths);
        with_context_var(child, FOCUS_HIGHLIGHT_SIDES_VAR, sides)
    }
}
