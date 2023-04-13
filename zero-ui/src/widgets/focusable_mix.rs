//! Focusable widget mix-in, properties and nodes..

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
            focusable = true;
            when *#focus::is_focused_hgl {
                crate::properties::foreground_highlight = {
                    offsets: FOCUS_HIGHLIGHT_OFFSETS_VAR,
                    widths: FOCUS_HIGHLIGHT_WIDTHS_VAR,
                    sides: FOCUS_HIGHLIGHT_SIDES_VAR,
                };
            }
        }
    }

    impl_properties! {
        /// If the widget can receive keyboard focus.
        ///
        /// Is enabled by default in this widget.
        pub fn crate::properties::focus::focusable(focusable: impl IntoVar<bool>);
    }
}

context_var! {
    /// Padding offsets of the foreground highlight when the widget is focused.
    pub static FOCUS_HIGHLIGHT_OFFSETS_VAR: SideOffsets = 1;
    /// Border widths of the foreground highlight when the widget is focused.
    pub static FOCUS_HIGHLIGHT_WIDTHS_VAR: SideOffsets = 0.5;
    /// Border sides of the foreground highlight when the widget is focused.
    pub static FOCUS_HIGHLIGHT_SIDES_VAR: BorderSides = BorderSides::dashed(rgba(200, 200, 200, 1.0));
}

/// Sets the foreground highlight values used when the widget is focused and highlighted.
#[property(
    CONTEXT,
    default(FOCUS_HIGHLIGHT_OFFSETS_VAR, FOCUS_HIGHLIGHT_WIDTHS_VAR, FOCUS_HIGHLIGHT_SIDES_VAR),
    impl(FocusableMix<P>)
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
