use zng::{
    gesture::is_pressed,
    layout::margin,
    prelude_wgt::{property, IntoValue, UiNode},
    widget::Wgt,
    APP,
};

struct NotVarValue;

#[property(CONTEXT)]
pub fn foo(child: impl IntoUiNode, value: impl IntoValue<bool>) -> UiNode {
    let _ = value;
    child
}

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        foo = false;
        margin = 0;

        when *#is_pressed {
            foo = true;
            margin = 1;
        }
    };
}
