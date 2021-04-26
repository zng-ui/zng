use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::properties::{cursor, margin};

    properties! {
        cursor //;
        margin = 10
    }
}

fn main() {}
