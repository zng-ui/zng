use zero_ui::core::widget_mixin;

#[widget_mixin($crate::test_mixin)]
pub mod test_mixin {
    use zero_ui::core::{NilUiNode, UiNode};

    fn new(child: impl UiNode) -> impl UiNode {
        child
    }
    fn new_child() -> NilUiNode {
        NilUiNode
    }
}

fn main() {}
