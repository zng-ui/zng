use crate::widget;
use crate::widgets::container;

widget! {
    /// A clickable container.
    pub button: container;

    use crate::properties::on_click;

    default(self) {
        /// Button click event.
        on_click: required!;
    }
}