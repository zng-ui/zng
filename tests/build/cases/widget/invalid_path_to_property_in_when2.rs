use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::properties::states::is_pressed;

    properties! {
        zero_ui::properties::margin = 0;

        when *#is_pressed {
            zero_ui::properties::margin = 1;
        }
    }
}

fn main() {}
