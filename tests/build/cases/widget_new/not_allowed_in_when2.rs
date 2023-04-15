use zero_ui::prelude::{new_property::*, *};

struct NotVarValue;

#[property(CONTEXT)]
pub fn foo(child: impl UiNode, value: impl IntoValue<bool>) -> impl UiNode {
    let _ = value;
    child
}

fn main() {
    let _scope = App::minimal();
    let _ = Wgt! {
        foo = false;
        margin = 0;

        when *#is_pressed {
            foo = true;
            margin = 1;
        }
    };
}
