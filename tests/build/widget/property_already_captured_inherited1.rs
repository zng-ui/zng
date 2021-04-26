use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::{NilUiNode, WidgetId};

    fn new_child(id: WidgetId) -> NilUiNode {
        let _ = id;
        NilUiNode
    }
}

fn main() {}
