use zng::{
    widget::{background_gradient, Wgt},
    APP,
};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        // background_gradient has two fields
        // this is interpreted as an unnamed assign `{ }` is the value
        // and the second value is missing
        background_gradient = {}
    };
}
