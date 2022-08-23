use zero_ui::prelude::*;

fn main() {
    let _err = toggle! {
        content = text("");
        value<bool> = 0;
    };
}
