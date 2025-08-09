use zng::prelude_wgt::{property, UiNode};

#[property(CONTEXT)]
pub fn no_args() -> UiNode {
    UiNode::nil()
}

fn main() {}
