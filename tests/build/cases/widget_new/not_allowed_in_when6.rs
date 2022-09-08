use zero_ui::core::{
    property,
    var::{IntoVar, VarValue},
    UiNode,
};

use zero_ui::widgets::blank;

// invalid signature, but disabled so property is ok.
#[property(context, allowed_in_when = false)]
pub fn foo<T: VarValue>(child: impl UiNode, value: impl IntoVar<T>) -> impl UiNode {
    let _ = value;
    child
}

fn main() {
    let _ = blank! {
        foo = false;
        when true {
            // expect only the allowed_in_when error not the generics type error
            foo = true;
        }
    };
}
