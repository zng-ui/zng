use zero_ui::core::{property, var::IntoVar, widget_instance::UiNode};
use zero_ui::widgets::wgt;

#[property(CONTEXT)]
pub fn simple_type(child: impl UiNode, simple_a: impl IntoVar<u32>, simple_b: impl IntoVar<u32>) -> impl UiNode {
    child
}

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = wgt! {
        simple_type = 42, true
    };
}
