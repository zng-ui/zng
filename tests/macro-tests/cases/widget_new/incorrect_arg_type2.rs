use zng::{
    prelude_wgt::{property, IntoVar, UiNode},
    widget::Wgt,
    APP,
};

#[property(CONTEXT)]
pub fn simple_type(child: impl UiNode, simple: impl IntoVar<u32>) -> impl UiNode {
    let _ = simple;
    child
}

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        simple_type = true
    };
}
