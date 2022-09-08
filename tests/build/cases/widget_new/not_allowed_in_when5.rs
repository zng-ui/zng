use zero_ui::core::{property, var::IntoVar, UiNode};
use zero_ui::widgets::blank;

#[property(context, allowed_in_when = false)] // valid signature, but disabled
pub fn foo(child: impl UiNode, value: impl IntoVar<bool>) -> impl UiNode {
    let _ = value;
    child
}

fn main() {
    let _ = blank! {
        foo = false;
        when true {
            foo = true;
        }
    };
}
