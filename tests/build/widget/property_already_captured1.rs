use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::{NilUiNode, UiNode, WidgetId, var::IntoValue};

    fn new_child(id: impl IntoValue<WidgetId>) -> NilUiNode {
        let _ = id;
        NilUiNode
    }

    fn new(child: impl UiNode, id: impl IntoValue<WidgetId>) {
        let _ = id;
    }
}

fn main() {}
