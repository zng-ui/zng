use zng::prelude_wgt::{property, IntoUiNode, IntoVar, UiNode};

#[property(INVALID_PRI)]
pub fn invalid_priority(child: impl IntoUiNode, input: impl IntoVar<bool>) -> UiNode {
    let _ = input;
    child.into_node()
}

fn main() {}
