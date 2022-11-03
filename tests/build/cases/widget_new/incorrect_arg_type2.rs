use zero_ui::core::{property, var::IntoVar, widget_instance::UiNode};
use zero_ui::widgets::blank;

#[property(context)]
pub fn simple_type(child: impl UiNode, simple: impl IntoVar<u32>) -> impl UiNode {
    child
}

fn main() {
    let _ = blank! {
        simple_type = true
    };
}
