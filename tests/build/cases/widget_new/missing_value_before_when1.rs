use zero_ui::prelude::*;

fn main() {
    let _scope = App::minimal();
    let _ = Wgt! {
        margin = // missing 0; here
        when *#is_pressed {
            margin = 20;
        }
    };
}
