use zero_ui::{gesture::is_pressed, layout::margin, widget::Wgt, APP};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        margin = 0;
        when *#is_pressed.unknown { // only .0 or .state allowed.
            margin = 1;
        }
    };
}
