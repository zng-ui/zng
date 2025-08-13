use zng::prelude_wgt::{IntoUiNode, UiNode, property};

#[property(CONTEXT, unknown = true)]
fn unknown_arg(child: impl IntoUiNode, input: bool) -> UiNode {
    let _ = input;
    child.into_node()
}
