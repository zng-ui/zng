use zng::prelude_wgt::{property, IntoUiNode, UiNode};

#[property(CONTEXT)]
pub fn no_inputs(child: impl IntoUiNode) -> UiNode {
    child.into_node()
}

fn main() {}
