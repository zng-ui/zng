use zng::{
    APP,
    gesture::is_pressed,
    layout::margin,
    prelude_wgt::{IntoUiNode, IntoValue, UiNode, property},
    widget::Wgt,
};

struct NotVarValue;

#[property(CONTEXT)]
pub fn foo(child: impl IntoUiNode, value: impl IntoValue<bool>) -> UiNode {
    let _ = value;
    child.into_node()
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
