use zng::{
    APP,
    layout::margin,
    prelude_wgt::{IntoUiNode, UiNode, property},
    widget::Wgt,
};

#[property(CONTEXT)]
pub fn foo(child: impl IntoUiNode, value: impl IntoUiNode) -> UiNode {
    let _ = value;
    child.into_node()
}

fn main() {
    let _scope = APP.minimal();
    #[rustfmt::skip]
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
