use zero_ui::core::{property, var::Var, UiNode};

#[property(context)]
fn not_into_var_input(child: impl UiNode, input: impl Var<&'static str>) -> impl UiNode {
    let _ = input;
    child
}

#[property(context, allowed_in_when = false)]
fn not_var_input(child: impl UiNode, input: &'static str) -> impl UiNode {
    let _ = input;
    child
}

fn main() {}
