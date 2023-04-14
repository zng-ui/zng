use zero_ui::prelude::*;

fn main() {
    let _scope = App::minimal();
    let _ = Wgt! {
        margin = 0 // missing ; here
        when *#is_pressed {
            margin = 20;
        }
    };
}
