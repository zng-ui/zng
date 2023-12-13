use zero_ui::wgt_prelude::{property, IntoVar, UiNode};

#[property(invalid)]
pub fn invalid_priority(child: impl UiNode, input: impl IntoVar<bool>) -> impl UiNode {
    let _ = input;
    child
}

fn main() {}
