use zng::{
    APP,
    layout::margin,
    mouse::{CursorIcon, cursor},
    widget::{Wgt, enabled},
};

fn main() {
    let _scope = APP.minimal();
    let margin = 0;
    #[rustfmt::skip]
    let _ = Wgt! {
        margin
        // we expect this properties to be used.
        enabled = true;
        cursor = CursorIcon::Pointer;
    };
}
