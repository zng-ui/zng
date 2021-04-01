use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::properties::margin;

    properties! {
        margin = 0;

        when self.zero_ui::properties::states::is_pressed {
            margin = 1;
        }
    }
}

fn main() {}