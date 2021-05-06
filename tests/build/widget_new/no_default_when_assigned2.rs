use zero_ui::properties::margin;
use zero_ui::widgets::blank;
use zero_ui::core::{property, UiNode, var::IntoVar};

#[property(context)]
pub fn my_property(child: impl UiNode, a: impl IntoVar<u32>) -> impl UiNode {
    let _ = a;
    child
}

fn main() {
    let _ = blank! {
        margin = 10;
        when self.my_property == 1 {
            margin = 20;
        }
    };
}
