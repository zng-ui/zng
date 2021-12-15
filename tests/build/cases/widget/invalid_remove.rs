use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::properties::margin;

    properties! {
        remove { margin }
        remove { zero_ui::properties::cursor }
    }
}

fn main() {}
