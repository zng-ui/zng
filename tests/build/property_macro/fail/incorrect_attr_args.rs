use zero_ui::core::{property, UiNode};

#[property(context, unknown = true)]
fn unknown_arg(child: impl UiNode, input: bool) -> impl UiNode {
    child
}

#[property(context, allowed_in_when = "false")]
fn invalid_allowed_in_when_value(child: impl UiNode, input: bool) -> impl UiNode {
    child
}

#[property(context, allowed_in_when)]
fn missing_allowed_in_when_value_1(child: impl UiNode, input: bool) -> impl UiNode {
    child
}

#[property(context, allowed_in_when = )]
fn missing_allowed_in_when_value_2(child: impl UiNode, input: bool) -> impl UiNode {
    child
}

fn main() {}
