use zero_ui::prelude::*;

fn main() {
    let _scope = App::minimal();
    let _ = Wgt! {
        cursor =
        #[allow(unused_imports)]
        margin = {
            use zero_ui::core::units::PxPoint;
            0
        }
    };
}
