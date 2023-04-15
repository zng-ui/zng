use zero_ui::prelude::*;

fn main() {
    let _scope = App::minimal();
    let _ = Wgt! {
        margin = 0;
        background_color = colors::BLACK;

        when *#margin {
            background_color = colors::WHITE;
        }
    };
}
