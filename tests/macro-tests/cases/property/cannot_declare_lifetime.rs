use zng::prelude_wgt::{IntoUiNode, UiNode, property};

#[property(CONTEXT)]
pub fn invalid<'a>(child: impl IntoUiNode, input: &'a str) -> UiNode {
    let _ = input;
    child.into_node()
}

fn main() {}
