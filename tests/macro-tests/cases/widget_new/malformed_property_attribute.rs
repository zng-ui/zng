use zng::{APP, button::Button, layout::margin};

fn main() {
    let _app = APP.minimal();
    #[rustfmt::skip]
    let _w = Button! {
        #![allow(inner_attribute)]
        #[!foo]
        margin = 10;
    };
}
