use zng::{
    APP,
    prelude_wgt::{IntoUiNode, IntoVar, UiNode, property},
    widget::Wgt,
};

#[property(CONTEXT)]
pub fn simple_type(child: impl IntoUiNode, simple: impl IntoVar<u32>) -> UiNode {
    let _ = simple;
    child
}

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        simple_type = true
    };
}
