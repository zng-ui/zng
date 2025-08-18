use zng::{
    APP,
    layout::AngleUnits,
    widget::{Wgt, background_gradient},
};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        // only background_gradient gets highlighted here because generics..
        background_gradient = {
            axis: 0.deg(),
            stops: true,
        };
    };
}
