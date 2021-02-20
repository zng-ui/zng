use zero_ui::core::{property, UiNode};

#[property(event)]
pub fn on_event_wrong_input_count(child: impl UiNode) -> impl UiNode { 
    child
}

fn main() { }