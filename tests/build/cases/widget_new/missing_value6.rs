use zng::{
    layout::margin,
    widget::{background_gradient, Wgt},
    APP,
};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        background_gradient = 0.deg(),
        margin = 0;
    };
}
