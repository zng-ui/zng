use zero_ui::core::{property, UiNode};
use zero_ui::properties::{margin, states::is_pressed};
use zero_ui::widgets::blank;
struct NotVarValue;

#[property(context, allowed_in_when = false)]
pub fn foo(child: impl UiNode, value: NotVarValue) -> impl UiNode {
    let _ = value;
    child
}

fn main() {
    let _ = blank! {
        foo = NotVarValue;
        margin = 0;

        when self.is_pressed {
            foo = NotVarValue;
            margin = 1;
        }
    };
}
