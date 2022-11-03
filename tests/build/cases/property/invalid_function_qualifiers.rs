// FunctionQualifiers: https://doc.rust-lang.org/reference/items/functions.html
use zero_ui::core::{property, var::IntoVar, widget_instance::UiNode};

#[property(context)]
pub async fn invalid_async(child: impl UiNode, _input: impl IntoVar<u32>) -> impl UiNode {
    child
}

#[property(context)]
pub unsafe fn invalid_unsafe(child: impl UiNode, _input: impl IntoVar<u32>) -> impl UiNode {
    child
}

#[property(context)]
pub extern "C" fn invalid_extern(child: impl UiNode, _input: impl IntoVar<u32>) -> impl UiNode {
    child
}

fn main() {}
