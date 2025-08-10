use zng::prelude_wgt::{property, IntoUiNode, UiNode};

#[property(CONTEXT)]
pub fn invalid_destruct(child: impl IntoUiNode, (a, b): (bool, u8)) -> UiNode {
    let _ = (a, b);
    child.into_node()
}

fn main() {}
