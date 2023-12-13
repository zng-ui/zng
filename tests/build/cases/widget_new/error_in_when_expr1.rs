use zero_ui::{gesture::is_pressed, layout::margin, widget::Wgt, APP};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        margin = 0;
        when {
            let a: u32 = true;
            *#is_pressed
        } {
            margin = 10;
        }
    };
}
