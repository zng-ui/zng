use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    pub use zero_ui::properties::{cursor, margin};

    inherit!(zero_ui::core::widget_base::base);

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
