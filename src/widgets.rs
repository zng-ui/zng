//! Common widgets.

#[macro_use]
mod container_;
// depends on container_
#[macro_use]
mod button_;
#[macro_use]
mod window_;

mod fill;
mod text_;
mod ui_n;
mod view_;

pub use button_::{button, ButtonBackground, ButtonBackgroundDisabled, ButtonBackgroundHovered, ButtonBackgroundPressed};
pub use container_::*;
pub use fill::*;
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
            => child
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

            => child
        }
    }

    fn _id(child: impl UiNode) -> impl UiNode {
        button! {
            on_click: |_|{};
            id: WidgetId::new_unique();
            => child
        }
    }

    fn _id_args(child: impl UiNode) -> impl UiNode {
        button! {
            on_click: |_|{};
            id: {
                id: WidgetId::new_unique()
            };
            => child
        }
    }
}
