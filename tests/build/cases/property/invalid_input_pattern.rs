use zero_ui::wgt_prelude::{property, UiNode};

#[property(CONTEXT)]
pub fn invalid_destruct(child: impl UiNode, (a, b): (bool, u8)) -> impl UiNode {
    let _ = (a, b);
    child
}

fn main() {}
