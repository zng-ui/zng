use zero_ui::core::units::*;
use zero_ui::properties::background_gradient;
use zero_ui::widgets::blank;

fn main() {
    let _scope = zero_ui::core::app::App::blank();
    let _ = blank! {
        // only background_gradient gets highlighted here because generics..
        background_gradient = {
            axis: 0.deg(),
            stops: true
        }
    };
}
