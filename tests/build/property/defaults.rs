use zero_ui::core::{property, var::IntoVar, UiNode};

#[property(context, default)]
pub fn missing_default_parethesis(child: impl UiNode, a: impl IntoVar<bool>, b: impl IntoVar<u32>) -> impl UiNode {
    let _ = (a, b);
    child
}

#[property(context, default())]
pub fn missing_default_values(child: impl UiNode, a: impl IntoVar<bool>, b: impl IntoVar<u32>) -> impl UiNode {
    let _ = (a, b);
    child
}

#[property(context, default(true))]
pub fn incorrect_default_args_count_u_1(child: impl UiNode, a: impl IntoVar<bool>, b: impl IntoVar<u32>) -> impl UiNode {
    let _ = (a, b);
    child
}

#[property(context, default(a: true))]
pub fn incorrect_default_args_count_n_1(child: impl UiNode, a: impl IntoVar<bool>, b: impl IntoVar<u32>) -> impl UiNode {
    let _ = (a, b);
    child
}

#[property(context, default(true, 2555, "ABC"))]
pub fn incorrect_default_args_count_u_2(child: impl UiNode, a: impl IntoVar<bool>, b: impl IntoVar<u32>) -> impl UiNode {
    let _ = (a, b);
    child
}

#[property(context, default(a: true, b: 2555, c: "ABC"))]
pub fn incorrect_default_args_count_n_2(child: impl UiNode, a: impl IntoVar<bool>, b: impl IntoVar<u32>) -> impl UiNode {
    let _ = (a, b);
    child
}

#[property(context, default(2555, true))]
pub fn invalid_default_args_types_u_2(child: impl UiNode, a: impl IntoVar<bool>, b: impl IntoVar<u32>) -> impl UiNode {
    let _ = (a, b);
    child
}

#[property(context, default(a: 2555, b: true))]
pub fn invalid_default_args_types_n_2(child: impl UiNode, a: impl IntoVar<bool>, b: impl IntoVar<u32>) -> impl UiNode {
    let _ = (a, b);
    child
}

fn main() {}
