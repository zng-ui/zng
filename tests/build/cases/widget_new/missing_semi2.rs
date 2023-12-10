use zero_ui::prelude::*;

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
