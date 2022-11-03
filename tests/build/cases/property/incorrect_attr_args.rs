use zero_ui::core::{property, widget_instance::UiNode};

#[property(context, unknown = true)]
fn unknown_arg(child: impl UiNode, input: bool) -> impl UiNode {
    child
}
