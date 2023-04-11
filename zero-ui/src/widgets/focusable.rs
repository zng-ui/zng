//! Focusable widget mix-in and helpers.

use crate::prelude::new_widget::*;

use crate::core::widget_mixin;

/// Focusable widget mix-in. Enables keyboard focusing on the widget and adds a focused highlight visual.
#[widget_mixin]
pub struct FocusableMix<P>(P);
impl<P: WidgetImpl> FocusableMix<P> {
    #[widget(on_start)]
    fn on_start(&mut self) {
        defaults! {
            self;

            /// Enables keyboard focusing in the widget.
            focusable = true;
        
            /// When widget has keyboard focus and highlight is requested.
            when *#focus::is_focused_hgl {
                focus_highlight = {
                    offsets: vis::FOCUS_HIGHLIGHT_OFFSETS_VAR,
                    widths: vis::FOCUS_HIGHLIGHT_WIDTHS_VAR,
                    sides: vis::FOCUS_HIGHLIGHT_SIDES_VAR,
                };
            }
        }
    }

    properties! {
        /// If the widget can receive keyboard focus.
        /// 
        /// Is enabled by default in this widget.
        pub fn crate::properties::focusable(enabled: impl IntoVar<bool>);
    }
}


/// A border overlay that is visible when the widget is focused.
#[property(FILL, default(0, 0, BorderStyle::Hidden))]
pub fn focus_highlight(
    child: impl UiNode,
    offsets: impl IntoVar<SideOffsets>,
    widths: impl IntoVar<SideOffsets>,
    sides: impl IntoVar<BorderSides>,
) -> impl UiNode {
    crate::properties::foreground_highlight(child, offsets, widths, sides)
}

mod temp {
    use crate::properties::focus;

    #[doc(inline)]
    pub use super::vis;

    properties! {
        /// Enables keyboard focusing in the widget.
        pub focus::focusable = true;

        /// A border overlay that is visible when the widget is focused.
        pub crate::properties::foreground_highlight as focus_highlight;

        /// When widget has keyboard focus and highlight is requested.
        when *#focus::is_focused_hgl {
            focus_highlight = {
                offsets: vis::FOCUS_HIGHLIGHT_OFFSETS_VAR,
                widths: vis::FOCUS_HIGHLIGHT_WIDTHS_VAR,
                sides: vis::FOCUS_HIGHLIGHT_SIDES_VAR,
            };
        }
    }
}

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
    CONTEXT,
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