use zero_ui::prelude::*;

fn main() {
    let _scope = App::minimal();
    let _ = Wgt! {
        margin = 0;
        when *#is_pressed.1 { // only .0 or .state allowed.
            margin = 1;
        }
    };
}
