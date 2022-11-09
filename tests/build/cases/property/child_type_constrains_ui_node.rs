use zero_ui::core::{
    property,
    var::*,
    widget_instance::{NilUiNode, UiNode},
};

#[property(CONTEXT)]
pub fn invalid_child(child: NilUiNode, input: impl IntoVar<bool>) -> impl UiNode {
    let _ = input;
    child
}

fn main() {}
