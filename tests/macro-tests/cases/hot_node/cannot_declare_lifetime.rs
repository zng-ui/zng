use zng::prelude_wgt::{hot_node, UiNode};

zng::hot_reload::zng_hot_entry!();

#[hot_node]
pub fn invalid<'a>(child: impl UiNode, input: &'a str) -> impl UiNode {
    let _ = input;
    child
}

fn main() {}
