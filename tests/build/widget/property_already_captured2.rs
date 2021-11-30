use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::{var::IntoValue, UiNode, WidgetId};

    fn new(child: impl UiNode, id: impl IntoValue<WidgetId>, id: impl IntoValue<WidgetId>) {
        let _ = id;
    }
}

fn main() {}
