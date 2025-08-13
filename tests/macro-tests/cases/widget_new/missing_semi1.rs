use zng::{
    APP,
    layout::margin,
    mouse::{CursorIcon, cursor},
    widget::{Wgt, enabled},
};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        margin = 0
        // we expect this properties to be used.
        enabled = true;
        cursor = CursorIcon::Pointer;
    };
}
