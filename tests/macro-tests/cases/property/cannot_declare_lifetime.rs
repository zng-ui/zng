use zng::prelude_wgt::{property, UiNode};

#[property(CONTEXT)]
pub fn invalid<'a>(child: impl IntoUiNode, input: &'a str) -> UiNode {
    let _ = input;
    child
}

fn main() {}
