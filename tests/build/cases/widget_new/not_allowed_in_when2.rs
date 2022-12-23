use zero_ui::core::{property, var::IntoValue, widget_instance::UiNode};
use zero_ui::properties::{margin, states::is_pressed};
use zero_ui::widgets::wgt;
struct NotVarValue;

#[property(CONTEXT)]
pub fn foo(child: impl UiNode, value: impl IntoValue<bool>) -> impl UiNode {
    let _ = value;
    child
}

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = wgt! {
        foo = false;
        margin = 0;

        when *#is_pressed {
            foo = true;
            margin = 1;
        }
    };
}
