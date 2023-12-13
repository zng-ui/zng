use zero_ui::{gesture::is_pressed, layout::margin, widget::Wgt, APP};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        margin = 0 // missing ; here
        when *#is_pressed {
            margin = 20;
        }
    };
}
