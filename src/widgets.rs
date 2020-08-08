//! Common widgets.
pub mod layouts;
pub mod mixins;

mod button_;
mod container_;
mod window_;

mod fill;
mod text_;
mod ui_n;
mod view_;

pub use button_::*;
pub use container_::*;
pub use fill::*;
pub use text_::*;
pub use ui_n::*;
pub use view_::*;
pub use window_::*;

/// Tests on the widget! code generator.

mod build_tests {
    use super::*;
    use crate::prelude::*;

    fn _basic(child: impl UiNode) -> impl UiNode {
        button! {
            on_click: |_|{};
            background_gradient: (0.0, 0.0), (1.0, 1.0), vec![rgb(0.0, 0.0, 0.0), rgb(1.0, 1.0, 1.0)];
            content: child;
        }
    }

    fn _args(child: impl UiNode) -> impl UiNode {
        button! {
            on_click: {
                handler: |_|{},
            };

            background_gradient: {
                start: (0.0, 0.0),
                end: (1.0, 1.0),
                stops: vec![rgb(0.0, 0.0, 0.0), rgb(1.0, 1.0, 1.0)]
            };

            content: child;
        }
    }

    fn _id(child: impl UiNode) -> impl UiNode {
        button! {
            on_click: |_|{};
            id: WidgetId::new_unique();
            content: child;
        }
    }

    fn _id_args(child: impl UiNode) -> impl UiNode {
        button! {
            on_click: |_|{};
            id: {
                id: WidgetId::new_unique()
            };
            content: child;
        }
    }
}
