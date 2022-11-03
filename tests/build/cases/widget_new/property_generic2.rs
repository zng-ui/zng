use zero_ui::prelude::*;

fn main() {
    let _err = toggle! {
        child = text("");
        value::<bool> = 0;
    };
}
