use zng::{APP, button::Button, widget::background_color};

fn main() {
    let _app = APP.minimal();
    #[rustfmt::skip]
    let _w = Button! {
        background_color = invalid!;
    };
}
