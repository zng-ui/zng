use zero_ui::core::{
    property,
    var::{IntoVar, VarValue},
    UiNode,
};

#[property(context)]
pub fn invalid<T: VarValue>(child: impl UiNode, value: impl IntoVar<T>) -> impl UiNode {
    let _ = value;
    child
}

fn main() {}
