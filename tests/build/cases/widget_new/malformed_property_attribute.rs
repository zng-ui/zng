use zero_ui::{button::Button, layout::margin, APP};

fn main() {
    let _app = APP.minimal();
    let _w = Button! {
        #![allow(inner_attribute)]
        #[!foo]
        margin = 10;
    };
}
