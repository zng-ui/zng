use zng::{
    prelude_wgt::{property, IntoUiNode, IntoVar, UiNode},
    widget::Wgt,
    APP,
};

#[property(CONTEXT)]
pub fn simple_type(child: impl IntoUiNode, simple_a: impl IntoVar<u32>, simple_b: impl IntoVar<u32>) -> UiNode {
    let _ = (simple_a, simple_b);
    child
}

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        simple_type = 42, true
    };
}
