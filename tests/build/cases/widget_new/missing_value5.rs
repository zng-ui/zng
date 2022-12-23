use zero_ui::properties::{cursor, margin};
use zero_ui::widgets::wgt;

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = wgt! {
        cursor =
        #[allow(unused_imports)]
        margin = {
            use zero_ui::core::units::PxPoint;
            0
        }
    };
}
