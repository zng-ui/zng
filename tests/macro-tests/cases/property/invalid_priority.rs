use zng::prelude_wgt::{property, IntoVar, UiNode};

#[property(INVALID_PRI)]
pub fn invalid_priority(child: impl UiNode, input: impl IntoVar<bool>) -> impl UiNode {
    let _ = input;
    child
}

fn main() {}
