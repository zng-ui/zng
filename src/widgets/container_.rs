use crate::widget;

#[doc(hidden)]
pub use crate::properties::{align, margin, Alignment};

widget! {
    /// Base single content container.
    pub container;

    default_child {
        /// Content margin.
        padding -> margin;
        /// Content alignment.
        content_align -> align: Alignment::CENTER;
    }
}
