use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    properties! {
        zero_ui::properties::margin = unset!;
    }
}

fn main() {}
