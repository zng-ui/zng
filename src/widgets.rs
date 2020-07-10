//! Common widgets.

mod container_;
mod focusable_;
mod implicit_;
// depends on container_ and focusable_
mod button_;
mod window_;

mod fill;
mod text_;
mod ui_n;
mod view_;

pub mod layouts;

pub use button_::*;
pub use container_::*;
pub use fill::*;
pub use focusable_::*;
pub use implicit_::*;
pub use text_::*;
pub use ui_n::*;
pub use view_::*;
pub use window_::*;

/// Tests on the widget! code generator.
#[cfg(test)]
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
