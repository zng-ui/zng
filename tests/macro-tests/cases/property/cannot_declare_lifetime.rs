use zng::prelude_wgt::{property, IntoUiNode, UiNode};

#[property(CONTEXT)]
pub fn invalid<'a>(child: impl IntoUiNode, input: &'a str) -> UiNode {
    let _ = input;
    child.into_node()
}

fn main() {}
