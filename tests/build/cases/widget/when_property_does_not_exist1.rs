use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::properties::margin;
    properties! {
        margin = 0;
        when *#is_pressed {
            margin = 1;
        }
    }
}

fn main() {}
