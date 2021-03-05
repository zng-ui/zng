use zero_ui::core::units::*;
use zero_ui::properties::background::background_gradient;
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        // `true` and background_gradient gets highlighted here because generics? Good enough for now.
        background_gradient = 0.deg(), true
    };
}
