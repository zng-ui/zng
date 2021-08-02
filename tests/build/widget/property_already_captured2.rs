use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::{UiNode, WidgetId, var::IntoValue};

    fn new(child: impl UiNode, id: impl IntoValue<WidgetId>, id: impl IntoValue<WidgetId>) {
        let _ = id;
    }
}

fn main() {}
