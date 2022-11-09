use zero_ui::core::{
    property,
    var::*,
    widget_instance::{NilUiNode, UiNode},
};

struct NotUiNode;

#[property(CONTEXT)]
pub fn invalid_child(child: NotUiNode, input: impl IntoVar<bool>) -> impl UiNode {
    let _ = input;
    NilUiNode
}

fn main() {}
