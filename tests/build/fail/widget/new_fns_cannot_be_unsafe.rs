use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::{NilUiNode, UiNode, WidgetId};

    unsafe fn new(child: impl UiNode, id: WidgetId) -> impl UiNode {
        child
    }
    unsafe fn new_child() -> NilUiNode {
        NilUiNode
    }
}

fn main() {}
