use zero_ui::{text::Text, toggle::Toggle};

fn main() {
    let _scope = zero_ui::APP.minimal();
    let _err = Toggle! {
        child = Text!("");
        value = 0;
    };

    let _ok = Toggle! {
        child = Text!("");
        value::<i32> = 0;
    };
}
