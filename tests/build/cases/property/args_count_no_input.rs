use zero_ui::prelude_wgt::{property, UiNode};

#[property(CONTEXT)]
pub fn no_inputs(child: impl UiNode) -> impl UiNode {
    child
}

fn main() {}
