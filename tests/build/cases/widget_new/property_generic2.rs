use zero_ui::prelude::*;

fn main() {
    let _scope = zero_ui::core::app::APP.minimal();
    let _err = Toggle! {
        child = Text!("");
        value::<bool> = 0;
    };
}
