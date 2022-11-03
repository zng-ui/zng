use zero_ui::core::{property, widget, UiNode};
use zero_ui::properties::{margin, states::is_pressed};
struct NotVarValue;

#[property(context)]
pub fn foo(child: impl UiNode, value: NotVarValue) -> impl UiNode {
    let _ = value;
    child
}

#[widget($crate::bar)]
pub mod bar {
    use super::*;

    properties! {
        foo = NotVarValue;
        margin = 0;

        when self.is_pressed {
            foo = NotVarValue;
            margin = 1;
        }
    }
}

fn main() {}
