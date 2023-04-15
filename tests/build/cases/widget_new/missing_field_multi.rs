use zero_ui::prelude::*;

fn main() {
    let _scope = App::minimal();
    let _ = Wgt! {
        // background_gradient has two fields
        // the error highlights the property
        // in a struct initializer the struct name is highlighted
        background_gradient = {
            axis: 0.deg(),
        }
    };
}
