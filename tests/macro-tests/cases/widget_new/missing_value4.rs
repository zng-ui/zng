use zng::{
    APP,
    layout::margin,
    mouse::cursor,
    widget::{Wgt, enabled},
};

fn main() {
    let _scope = APP.minimal();
    #[rustfmt::skip]
    let _ = Wgt! {
        cursor =
        // we expect these properties to be used.
        margin = 0;
        enabled = true;
    };
}
