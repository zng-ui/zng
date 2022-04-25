use zero_ui::core::{property, var::IntoVar, UiNode};

#[derive(Clone)]
pub struct MyType;
impl IntoVar<bool> for MyType {
    type Var = zero_ui::core::var::LocalVar<bool>;

    fn into_var(self) -> Self::Var {
        zero_ui::core::var::LocalVar(true)
    }
}

#[property(context)]
pub fn invalid(child: impl UiNode, value: MyType) -> impl UiNode {
    let _ = value;
    // for allowed_in_when, the arg type needs to be IntoVar AND accept the resulting var back.
    child
}

fn main() {}
