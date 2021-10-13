use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::{UiNode, WidgetId};

    fn new(child: impl UiNode, id: impl Into<WidgetId>, id: impl Into<WidgetId>) {
        let _ = id;
    }
}

fn main() {}
