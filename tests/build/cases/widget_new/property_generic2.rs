use zng::{text::Text, toggle::Toggle};

fn main() {
    let _scope = zng::APP.minimal();
    let _err = Toggle! {
        child = Text!("");
        value::<bool> = 0;
    };
}
