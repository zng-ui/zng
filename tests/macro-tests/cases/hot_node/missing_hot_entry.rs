use zng::prelude_wgt::{IntoUiNode, UiNode, hot_node};

// zng::hot_reload::zng_hot_entry!();

#[hot_node]
pub fn valid(child: impl IntoUiNode) -> UiNode {
    child.into_node()
}

fn main() {}
