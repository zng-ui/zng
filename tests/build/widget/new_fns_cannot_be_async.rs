use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::{NilUiNode, UiNode, WidgetId};

    async fn new(child: impl UiNode, id: impl Into<WidgetId>) -> impl UiNode {
        child
    }
    async fn new_child() -> NilUiNode {
        NilUiNode
    }
}

fn main() {}
