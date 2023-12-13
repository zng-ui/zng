use zero_ui::{
    layout::margin,
    mouse::{cursor, CursorIcon},
    widget::{enabled, Wgt},
    APP,
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
