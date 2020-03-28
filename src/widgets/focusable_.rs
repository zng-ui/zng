use crate::core::types::{rgba, LayoutSideOffsets};
#[doc(hidden)]
pub use crate::properties::{border, focusable, id, BorderDetails};
use crate::widget_mixin;

context_var! {
    pub struct FocusedBorderWidths: LayoutSideOffsets = LayoutSideOffsets::new_all_same(1.0);
    pub struct FocusedBorderDetails: BorderDetails = BorderDetails::new_solid_color(rgba(0, 0, 255, 0.7));
}

widget_mixin! {
    /// Focusable widget mix-in. Enables keyboard focusing on the widget and adds a focused
    /// high-light border.
    pub focusable_mixin;

    default(self) {

        /// Enables keyboard focusing in this widget.
        focusable: true;

        /// A Border that is visible when the widget is focused.
        focused_border -> border: {
            widths: LayoutSideOffsets::new_all_same(0.0),
            details: FocusedBorderDetails
        };
    }

    //when self.is_focused {
    //    focused_border: {
    //        widths: FocusedBorderWidths,
    //        details: FocusedBorderDetails
    //    };
    //}
}
