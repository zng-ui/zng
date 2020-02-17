use crate::core::types::{rgb, ColorF};
use crate::widget;
use crate::widgets::container;

context_var! {
    /// Default background of [`button!`](crate::widgets::button) widgets.
    pub struct ButtonBackground: ColorF = rgb(0, 0, 0);
}

widget! {
    /// A clickable container.
    pub button: container;

    use crate::properties::{on_click, background_color};
    use crate::widgets::ButtonBackground;

    default(self) {
        /// Button click event.
        on_click: required!;
    }

    default(self) {
        /// Set to [`ButtonBackground`](crate::widgets::ButtonBackground).
        background_color: ButtonBackground;
    }
}
