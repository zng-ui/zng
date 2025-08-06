use zng::prelude_wgt::{hot_node, IntoVar, UiNode};

zng::hot_reload::zng_hot_entry!();

pub struct NotUiNode;

#[hot_node]
pub fn invalid_output1(_child: impl IntoUiNode, _input: impl IntoVar<bool>) -> NotUiNode {
    NotUiNode
}

#[hot_node]
pub fn invalid_output2(_child: impl IntoUiNode, _input: impl IntoVar<bool>) {}

fn main() {}
