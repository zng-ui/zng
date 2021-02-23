use zero_ui::core::{property, UiNode};

// This will be how we support destructuring in the input while getting
// a name for the property named assign.
//
// For now only @ _ is stable.
#[property(context)]
fn sub_pattern_all(child: impl UiNode, input @ _: bool) -> impl UiNode {
    let _ = input;
    child
}

fn main() {}
