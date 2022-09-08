use zero_ui::core::{property, var::IntoVar, widget, UiNode};

#[property(context, allowed_in_when = false)] // valid signature, but disabled
pub fn foo(child: impl UiNode, value: impl IntoVar<bool>) -> impl UiNode {
    let _ = value;
    child
}

#[widget($crate::bar)]
pub mod bar {
    use super::*;

    properties! {
        foo = false;
        when true {
            foo = true;
        }
    }
}

fn main() {}
