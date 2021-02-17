use zero_ui::core::{property, UiNode, var::Var};

#[property(context)]
fn not_into_var_input(child: impl UiNode, input: impl Var<&'static str>) -> impl UiNode {
    let _ = input;
    child
}

#[property(context)]
fn not_var_input(child: impl UiNode, input: &'static str) -> impl UiNode {
    let _ = input;
    child
}

fn main() { }