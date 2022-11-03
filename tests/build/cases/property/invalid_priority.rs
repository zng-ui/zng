use zero_ui::core::{property, var::IntoVar, widget_instance::UiNode};

#[property(invalid)]
pub fn invalid_priority(child: impl UiNode, input: impl IntoVar<bool>) -> impl UiNode {
    let _ = input;
    child
}

fn main() {}
