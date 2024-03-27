use zng::{button::Button, widget::background_color, APP};

fn main() {
    let _app = APP.minimal();
    let _w = Button! {
        background_color = invalid!;
    };
}
