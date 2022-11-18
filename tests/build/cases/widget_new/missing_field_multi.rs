use zero_ui::core::units::*;
use zero_ui::properties::background_gradient;
use zero_ui::widgets::blank;

fn main() {
    let _scope = zero_ui::core::app::App::blank();
    let _ = blank! {
        // background_gradient has two fields
        // the error highlights the property
        // in a struct initializer the struct name is highlighted
        background_gradient = {
            axis: 0.deg(),
        }
    };
}
