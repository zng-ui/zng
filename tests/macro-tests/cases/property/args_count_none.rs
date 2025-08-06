use zng::prelude_wgt::{property, UiNode};

#[property(CONTEXT)]
pub fn no_args() -> UiNode {
    zng::prelude_wgt::NilUiNode
}

fn main() {}
