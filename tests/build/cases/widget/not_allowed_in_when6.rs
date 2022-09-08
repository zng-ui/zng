use zero_ui::core::{
    property,
    var::{IntoVar, VarValue},
    widget, UiNode,
};

// invalid signature, but disabled so property is ok.
#[property(context, allowed_in_when = false)]
pub fn foo<T: VarValue>(child: impl UiNode, value: impl IntoVar<T>) -> impl UiNode {
    let _ = value;
    child
}

#[widget($crate::bar)]
pub mod bar {
    use super::*;

    properties! {
        foo<bool> = false;
        when true {
            // expect only the allowed_in_when error not the generics type error
            foo<bool> = true;
        }
    }
}

fn main() {}
