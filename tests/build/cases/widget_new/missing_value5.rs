use zero_ui::properties::{cursor, margin};
use zero_ui::widgets::blank;

fn main() {
    let _scope = zero_ui::core::app::App::blank();
    let _ = blank! {
        cursor =
        #[allow(unused_imports)]
        margin = {
            use zero_ui::core::units::PxPoint;
            0
        }
    };
}
