use zero_ui::prelude_wgt::{property, UiNode};

#[property(CONTEXT)]
pub fn no_args() -> impl UiNode {
    zero_ui::prelude_wgt::NilUiNode
}

fn main() {}
