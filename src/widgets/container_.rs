use crate::prelude::WidgetId;
use crate::widget;

#[doc(hidden)]
pub use crate::properties::{align, id, margin, Alignment};

widget! {
    /// Base single content container.
    pub container;

    default(child) {
        /// Content margin.
        padding -> margin;
        /// Content alignment.
        content_align -> align: Alignment::CENTER;
    }

    default(self) {
        /// Unique identifier of the widget.
        id: WidgetId::new_unique();
    }
}
