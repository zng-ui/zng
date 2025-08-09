use zng::prelude_wgt::{hot_node, IntoUiNode, UiNode};

zng::hot_reload::zng_hot_entry!();

#[hot_node]
pub fn invalid_destruct(child: impl IntoUiNode, (a, b): (bool, u8)) -> UiNode {
    let _ = (a, b);
    child.into_node()
}

fn main() {}
