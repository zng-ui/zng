use zero_ui::prelude::*;

fn main() {
    let _err = toggle! {
        child = text("");
        value = 0;
    };

    let _ok = toggle! {
        child = text("");
        value::<i32> = 0;
    };
}
