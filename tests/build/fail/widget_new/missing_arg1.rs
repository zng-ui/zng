use zero_ui::core::units::*;
use zero_ui::properties::{background_gradient, margin};
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        background_gradient = 0.deg(), ;
        margin = 0;
    };
}
