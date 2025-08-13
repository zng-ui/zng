use zng::prelude_wgt::{UiNode, property};

#[property(CONTEXT)]
pub fn no_args() -> UiNode {
    UiNode::nil()
}

fn main() {}
