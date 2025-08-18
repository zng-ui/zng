use zng::{
    APP,
    color::colors,
    layout::margin,
    widget::{Wgt, background_color},
};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        margin = 0;
        background_color = colors::BLACK;

        when *#margin {
            background_color = colors::WHITE;
        }
    };
}
