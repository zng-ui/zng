use zng::prelude_wgt::{property, IntoVar, NilUiNode, UiNode};

#[property(CONTEXT)]
pub fn invalid_child(child: NilUiNode, input: impl IntoVar<bool>) -> UiNode {
    let _ = input;
    child
}

fn main() {}
