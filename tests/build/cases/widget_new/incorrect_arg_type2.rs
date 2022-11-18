use zero_ui::core::{property, var::IntoVar, widget_instance::UiNode};
use zero_ui::widgets::blank;

#[property(CONTEXT)]
pub fn simple_type(child: impl UiNode, simple: impl IntoVar<u32>) -> impl UiNode {
    child
}

fn main() {
    let _scope = zero_ui::core::app::App::blank();
    let _ = blank! {
        simple_type = true
    };
}
