use zero_ui::{
    layout::margin,
    mouse::cursor,
    widget::{enabled, Wgt},
    APP,
};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        cursor =
        // we expect these properties to be used.
        margin = 0;
        enabled = true;
    };
}
