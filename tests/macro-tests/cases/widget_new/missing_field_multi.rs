use zng::{
    APP,
    layout::AngleUnits,
    widget::{Wgt, background_gradient},
};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        // background_gradient has two fields
        // the error highlights the property
        // in a struct initializer the struct name is highlighted
        background_gradient = { axis: 0.deg() }
    };
}
