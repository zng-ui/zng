use zero_ui::core::units::*;
use zero_ui::properties::background::background_gradient;
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        // only background_gradient gets highlighted here because generics..
        background_gradient = {
            axis: 0.deg(),
            stops: true
        }
    };
}
