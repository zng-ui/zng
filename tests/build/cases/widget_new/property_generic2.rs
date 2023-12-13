use zero_ui::{text::Text, toggle::Toggle};

fn main() {
    let _scope = zero_ui::APP.minimal();
    let _err = Toggle! {
        child = Text!("");
        value::<bool> = 0;
    };
}
