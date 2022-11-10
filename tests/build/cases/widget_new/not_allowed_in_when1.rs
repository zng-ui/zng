use zero_ui::core::{property, widget_instance::UiNode};
use zero_ui::properties::margin;
use zero_ui::widgets::blank;

#[property(CONTEXT)]
pub fn foo(child: impl UiNode, value: impl UiNode) -> impl UiNode {
    let _ = value;
    child
}

fn main() {
    let _ = blank! {
        margin = 0;
        when {
            let node = #foo;
            true
        } {
            margin = 1;
        }
    };
}
