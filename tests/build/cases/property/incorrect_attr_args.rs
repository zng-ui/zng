use zero_ui::wgt_prelude::{property, UiNode};

#[property(CONTEXT, unknown = true)]
fn unknown_arg(child: impl UiNode, input: bool) -> impl UiNode {
    let _ = input;
    child
}
