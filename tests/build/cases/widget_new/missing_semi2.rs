use zero_ui::{
    layout::margin,
    mouse::{cursor, CursorIcon},
    widget::{enabled, Wgt},
    APP,
};

fn main() {
    let _scope = APP.minimal();
    let margin = 0;
    let _ = Wgt! {
        margin
        // we expect this properties to be used.
        enabled = true;
        cursor = CursorIcon::Pointer;
    };
}
