use zero_ui::prelude_wgt::{property, IntoVar, UiNode};

#[property(invalid)]
pub fn invalid_priority(child: impl UiNode, input: impl IntoVar<bool>) -> impl UiNode {
    let _ = input;
    child
}

fn main() {}
