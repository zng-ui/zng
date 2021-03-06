use zero_ui::core::widget2;

#[widget2($crate::widget)]
pub mod widget {
    use zero_ui::properties::{cursor, margin};

    properties! {
        cursor //;
        margin = 10
    }
}

fn main() {}
