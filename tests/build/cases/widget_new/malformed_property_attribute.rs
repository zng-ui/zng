use zero_ui::prelude::*;

fn main() {
    let _app = APP.minimal();
    let _w = Button! {
        #![allow(inner_attribute)]
        #[!foo]
        margin = 10;
    };
}
