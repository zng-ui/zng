use zero_ui::prelude::*;

fn main() {
    let _scope = App::minimal();
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
