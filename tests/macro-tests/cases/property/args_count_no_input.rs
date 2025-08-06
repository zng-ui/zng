use zng::prelude_wgt::{property, UiNode};

#[property(CONTEXT)]
pub fn no_inputs(child: impl IntoUiNode) -> UiNode {
    child
}

fn main() {}
