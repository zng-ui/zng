use zng::prelude_wgt::{hot_node, UiNode};

zng::hot_reload::zng_hot_entry!();

#[hot_node]
pub fn invalid_destruct(child: impl UiNode, (a, b): (bool, u8)) -> impl UiNode {
    let _ = (a, b);
    child
}

fn main() {}
