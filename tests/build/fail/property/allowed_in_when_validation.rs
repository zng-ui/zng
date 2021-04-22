use zero_ui::core::{property, var::IntoVar, UiNode};
use zero_ui::properties::margin;

#[property(context)] // error because no allowed_in_when = false
pub fn invalid1(child: impl UiNode, value: bool) -> impl UiNode {
    let _ = value;
    // value does not take Var but property is allowed_in_when by default.
    child
}

#[property(context)] // error because no allowed_in_when = false
pub fn invalid2(child: impl UiNode, value: impl UiNode) -> impl UiNode {
    let _ = value;
    // value is generic, this complicates things, some confusing errors
    // appear in the call_site, because generics errors highlight the full context of the call
    // not just the invalid input...
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
