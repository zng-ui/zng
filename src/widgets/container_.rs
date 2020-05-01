use crate::core::widget;

#[doc(hidden)]
pub use crate::properties::{align, clip_to_bounds, margin, Alignment};

widget! {
    /// Base single content container.
    pub container;

    default_child {
        /// Content margin.
        padding -> margin;
        /// Content alignment.
        content_align -> align: Alignment::CENTER;
        /// Content overflow clipping.
        clip_to_bounds -> clip_to_bounds;
    }
}
