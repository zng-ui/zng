use zng::prelude_wgt::{hot_node, UiNode};

zng::hot_reload::zng_hot_entry!();

pub struct Foo {}

#[hot_node]
pub fn invalid(_foo: Foo) -> UiNode {
    UiNode::nil()
}

fn main() {}
