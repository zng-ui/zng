use zero_ui::core::{property, UiNode};

#[property(context)]
pub fn is_state_wrong_input_count(child: impl UiNode) -> impl UiNode { 
    child
}

fn main() { }