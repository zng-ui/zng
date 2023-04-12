use zero_ui::prelude::*;

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _err = toggle! {
        child = Text!("");
        value::<bool> = 0;
    };
}
