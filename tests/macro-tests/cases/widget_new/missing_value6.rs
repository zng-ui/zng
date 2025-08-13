use zng::{
    APP,
    layout::margin,
    widget::{Wgt, background_gradient},
};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        background_gradient = 0.deg(),
        margin = 0;
    };
}
