use crate::core::types::{rgb, ColorF};
use crate::widget;
use crate::widgets::container;

context_var! {
    /// [button] default background.
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
        background_color: ButtonBackground;
    }
}

mod build_tests {
    use super::*;
    use crate::core::UiNode;
    use crate::properties::*;

    fn _basic(child: impl UiNode) -> impl UiNode {
        button! {
            on_click: |_|{};
            background_gradient: (0.0, 0.0), (1.0, 1.0), vec![rgb(0.0, 0.0, 0.0), rgb(1.0, 1.0, 1.0)];
            => child
        }
    }

    fn _args(child: impl UiNode) -> impl UiNode {
        button! {
            on_click: |_|{};

            background_gradient: {
                start: (0.0, 0.0),
                end: (1.0, 1.0),
                stops: vec![rgb(0.0, 0.0, 0.0), rgb(1.0, 1.0, 1.0)]
            };

            => child
        }
    }
}
