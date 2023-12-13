use zero_ui::wgt_prelude::{property, UiNode};

#[property(CONTEXT)]
pub fn no_inputs(child: impl UiNode) -> impl UiNode {
    child
}

fn main() {}
