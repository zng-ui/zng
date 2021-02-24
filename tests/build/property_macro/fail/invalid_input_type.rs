use zero_ui::core::{property, UiNode};

// This type is invalid because it does not implement `Clone`
// and does not implement any of these traits: `IntoVar`, `Var` or `Debug`.
//
// These traits are needed in a debug build to show the value in the inspector.
pub struct NotDebuggable;

#[property(context)]
pub fn invalid_input(child: impl UiNode, input: NotDebuggable) -> impl UiNode {
    child
}

fn main() {}
