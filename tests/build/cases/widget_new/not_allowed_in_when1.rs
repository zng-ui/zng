use zero_ui::{
    layout::margin,
    wgt_prelude::{property, UiNode},
    widget::Wgt,
    APP,
};

#[property(CONTEXT)]
pub fn foo(child: impl UiNode, value: impl UiNode) -> impl UiNode {
    let _ = value;
    child
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
