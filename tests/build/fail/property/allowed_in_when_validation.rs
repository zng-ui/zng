use zero_ui::core::{property, var::IntoVar, UiNode};
use zero_ui::properties::margin;

#[property(context)]
pub fn invalid(child: impl UiNode, value: bool) -> impl UiNode {
    let _ = value;
    // value does not take Var but property is allowed_in_when by default.
    child
}

#[property(context)]
pub fn valid(child: impl UiNode, value: impl IntoVar<bool>) -> impl UiNode {
    let _ = value;
    // value takes Var and is allowed_in_when.
    child
}

fn main() {
    let _ = zero_ui::widgets::blank! {
        margin = 0;
        valid = true;
        when self.valid {
            margin = 1;
        }
    };
}
