use zero_ui::properties::{cursor, margin};
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        cursor =
        #[allow(unused_imports)]
        margin = {
            use zero_ui::core::units::PxPoint;
            0
        }
    };
}
