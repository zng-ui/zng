use zng::prelude_wgt::{property, UiNode};

#[property(CONTEXT)]
pub fn invalid<'a>(child: impl UiNode, input: &'a str) -> impl UiNode {
    let _ = input;
    child
}

fn main() {}
