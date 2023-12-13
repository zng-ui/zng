use zero_ui::wgt_prelude::{property, IntoVar, NilUiNode, UiNode};

#[property(CONTEXT)]
pub fn invalid_child(child: NilUiNode, input: impl IntoVar<bool>) -> impl UiNode {
    let _ = input;
    child
}

fn main() {}
