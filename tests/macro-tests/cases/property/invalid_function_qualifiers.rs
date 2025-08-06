// FunctionQualifiers: https://doc.rust-lang.org/reference/items/functions.html
use zng::prelude_wgt::{property, IntoVar, UiNode};

#[property(CONTEXT)]
pub async fn invalid_async(child: impl IntoUiNode, _input: impl IntoVar<u32>) -> UiNode {
    child
}

#[property(CONTEXT)]
pub unsafe fn invalid_unsafe(child: impl IntoUiNode, _input: impl IntoVar<u32>) -> UiNode {
    child
}

#[property(CONTEXT)]
pub extern "C" fn invalid_extern(child: impl IntoUiNode, _input: impl IntoVar<u32>) -> UiNode {
    child
}

fn main() {}
