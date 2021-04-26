use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::{NilUiNode, UiNode, WidgetId};

    fn new_child(id: WidgetId) -> NilUiNode {
        let _ = id;
        NilUiNode
    }

    fn new(child: impl UiNode, id: WidgetId) {
        let _ = id;
    }
}

fn main() {}
