// FunctionQualifiers: https://doc.rust-lang.org/reference/items/functions.html
use zng::prelude_wgt::{IntoUiNode, IntoVar, UiNode, property};

#[property(CONTEXT)]
pub async fn invalid_async(child: impl IntoUiNode, _input: impl IntoVar<u32>) -> UiNode {
    child.into_node()
}

#[property(CONTEXT)]
pub unsafe fn invalid_unsafe(child: impl IntoUiNode, _input: impl IntoVar<u32>) -> UiNode {
    child.into_node()
}

#[property(CONTEXT)]
pub extern "C" fn invalid_extern(child: impl IntoUiNode, _input: impl IntoVar<u32>) -> UiNode {
    child.into_node()
}

fn main() {}
