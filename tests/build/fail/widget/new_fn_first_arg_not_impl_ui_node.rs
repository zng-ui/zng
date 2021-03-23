use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::WidgetId;

    fn new(id: WidgetId) {}
}

fn main() {}
