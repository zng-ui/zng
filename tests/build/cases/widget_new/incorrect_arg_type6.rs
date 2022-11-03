use zero_ui::core::{property, UiNode};
use zero_ui::widgets::blank;

#[property(context)]
pub fn simple_type(child: impl UiNode, simple_a: u32, simple_b: u32) -> impl UiNode {
    child
}

fn main() {
    let _ = blank! {
        simple_type = {
            simple_a: 42,
            simple_b: true,
        }
    };
}
