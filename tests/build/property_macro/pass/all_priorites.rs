use zero_ui::core::{property, UiNode};

#[property(context)]
pub fn context_property(child: impl UiNode, input: bool) -> impl UiNode {
    let _ = input;
    child
}

#[property(event)]
pub fn on_event_property(child: impl UiNode, input: bool) -> impl UiNode {
    let _ = input;
    child
}

#[property(outer)]
pub fn outer_property(child: impl UiNode, input: bool) -> impl UiNode {
    let _ = input;
    child
}

#[property(size)]
pub fn size_property(child: impl UiNode, input: bool) -> impl UiNode {
    let _ = input;
    child
}

#[property(inner)]
pub fn inner_property(child: impl UiNode, input: bool) -> impl UiNode {
    let _ = input;
    child
}

#[property(capture_only)]
pub fn capture_only_property(input: bool) -> ! {}

fn main() { }