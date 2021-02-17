use zero_ui::core::{property, UiNode};

#[property(event)]
pub fn event_property(child: impl UiNode, input: bool) -> impl UiNode { // expected `on_*` prefix
    let _ = input;
    child
}

fn main() { }
