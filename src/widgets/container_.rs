use crate::widget;

widget! {
    /// Base single content container.
    pub container;

    use crate::properties::{margin, align, Alignment};

    default(child) {
        /// Content margin.
        padding -> margin;
        /// Content alignment.
        content_align -> align: Alignment::CENTER;
    }
}
