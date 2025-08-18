use zng::prelude_wgt::{IntoUiNode, UiNode, property};

#[property(CONTEXT)]
pub fn no_inputs(child: impl IntoUiNode) -> UiNode {
    child.into_node()
}

fn main() {}
