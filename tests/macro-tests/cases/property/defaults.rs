use zng::prelude_wgt::{property, IntoVar, UiNode};

#[property(CONTEXT, default)]
pub fn missing_default_parentheses(child: impl IntoUiNode, a: impl IntoVar<bool>, b: impl IntoVar<u32>) -> UiNode {
    let _ = (a, b);
    child
}

#[property(CONTEXT, default())]
pub fn missing_default_values(child: impl IntoUiNode, a: impl IntoVar<bool>, b: impl IntoVar<u32>) -> UiNode {
    let _ = (a, b);
    child
}

#[property(CONTEXT, default(true))]
pub fn incorrect_default_args_count_u_1(child: impl IntoUiNode, a: impl IntoVar<bool>, b: impl IntoVar<u32>) -> UiNode {
    let _ = (a, b);
    child
}

#[property(CONTEXT, default(a: true))]
pub fn incorrect_default_args_count_n_1(child: impl IntoUiNode, a: impl IntoVar<bool>, b: impl IntoVar<u32>) -> UiNode {
    let _ = (a, b);
    child
}

#[property(CONTEXT, default(true, 2555, "ABC"))]
pub fn incorrect_default_args_count_u_2(child: impl IntoUiNode, a: impl IntoVar<bool>, b: impl IntoVar<u32>) -> UiNode {
    let _ = (a, b);
    child
}

#[property(CONTEXT, default(a: true, b: 2555, c: "ABC"))]
pub fn incorrect_default_args_count_n_2(child: impl IntoUiNode, a: impl IntoVar<bool>, b: impl IntoVar<u32>) -> UiNode {
    let _ = (a, b);
    child
}

#[property(CONTEXT, default(2555, true))]
pub fn invalid_default_args_types_u_2(child: impl IntoUiNode, a: impl IntoVar<bool>, b: impl IntoVar<u32>) -> UiNode {
    let _ = (a, b);
    child
}

#[property(CONTEXT, default(a: 2555, b: true))]
pub fn invalid_default_args_types_n_2(child: impl IntoUiNode, a: impl IntoVar<bool>, b: impl IntoVar<u32>) -> UiNode {
    let _ = (a, b);
    child
}

fn main() {}
