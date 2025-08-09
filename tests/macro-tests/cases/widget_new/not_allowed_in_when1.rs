use zng::{
    layout::margin,
    prelude_wgt::{property, IntoUiNode, UiNode},
    widget::Wgt,
    APP,
};

#[property(CONTEXT)]
pub fn foo(child: impl IntoUiNode, value: impl IntoUiNode) -> UiNode {
    let _ = value;
    child.into_node()
}

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        margin = 0;
        when {
            let node = #foo;
            true
        } {
            margin = 1;
        }
    };
}
