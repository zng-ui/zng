use zero_ui::core::{property, UiNode};

#[property(context)]
fn trailing_comma_1(child: impl UiNode, input: bool) -> impl UiNode {
    let _ = input;
    child
}

#[property(context, allowed_in_when = true)]
fn allowed_in_when(child: impl UiNode, input: bool) -> impl UiNode {
    let _ = input;
    child
}

#[property(context, allowed_in_when = false)]
fn not_allowed_in_when(child: impl UiNode, input: bool) -> impl UiNode {
    let _ = input;
    child
}

#[property(context, allowed_in_when = false,)]
fn trailing_comma_2(child: impl UiNode, input: bool) -> impl UiNode {
    let _ = input;
    child
}

fn main() {}
