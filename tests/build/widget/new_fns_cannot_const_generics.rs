use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::{UiNode, WidgetId};

    fn new<const N: usize>(child: impl UiNode, id: impl Into<WidgetId>) -> [bool; N] {
        [true; N]
    }
    fn new_child<const N: usize>() -> [bool; N] {
        [true; N]
    }
}

fn main() {}
