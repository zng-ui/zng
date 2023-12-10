use zero_ui::prelude::{new_property::*, *};

#[property(CONTEXT)]
pub fn simple_type(child: impl UiNode, simple_a: impl IntoVar<u32>, simple_b: impl IntoVar<u32>) -> impl UiNode {
    let _ = (simple_a, simple_b);
    child
}

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        simple_type = {
            simple_a: 42,
            simple_b: true,
        }
    };
}
