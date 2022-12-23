use zero_ui::core::units::*;
use zero_ui::properties::background_gradient;
use zero_ui::widgets::wgt;

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = wgt! {
        background_gradient = 0.deg(), true
    };
}
