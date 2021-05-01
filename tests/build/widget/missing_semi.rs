use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::properties::{cursor, margin};

    properties! {
        cursor //;
        margin = 10
    }
}

fn main() {
    let _ = test_widget! {
        cursor = zero_ui::core::window::CursorIcon::Hand;
        margin = 5;
    };
}
