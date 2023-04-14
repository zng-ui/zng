use zero_ui::prelude::{new_property::*, *};

#[property(CONTEXT)]
pub fn foo(child: impl UiNode, value: impl UiNode) -> impl UiNode {
    let _ = value;
    child
}

fn main() {
    let _scope = App::minimal();
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
