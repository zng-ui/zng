use zero_ui::prelude::*;

fn main() {
    let _err = toggle! {
        content = text("");
        value = 0;
    };

    let _ok = toggle! {
        content = text("");
        value::<i32> = 0;
    };
    let _ok = toggle! {
        content = text("");
        value<i32> = 0;
    };
}
