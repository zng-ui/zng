use zng::prelude_wgt::{hot_node, NilUiNode, UiNode};

zng::hot_reload::zng_hot_entry!();

pub struct Foo {}

#[hot_node]
pub fn invalid(_foo: Foo) -> UiNode {
    NilUiNode
}

fn main() {}
