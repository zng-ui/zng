use zero_ui::{gesture::is_pressed, widget::Wgt, APP};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        when *#is_pressed
    };
}
