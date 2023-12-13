use zero_ui::wgt_prelude::{property, UiNode};

#[property(CONTEXT)]
pub fn no_args() -> impl UiNode {
    zero_ui::wgt_prelude::NilUiNode
}

fn main() {}
