use zero_ui::core::{property, widget_instance::UiNode};
use zero_ui::properties::margin;
use zero_ui::widgets::wgt;

#[property(CONTEXT)]
pub fn foo(child: impl UiNode, value: impl UiNode) -> impl UiNode {
    let _ = value;
    child
}

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = wgt! {
        margin = 0;
        when {
            let node = #foo;
            true
        } {
            margin = 1;
        }
    };
}
