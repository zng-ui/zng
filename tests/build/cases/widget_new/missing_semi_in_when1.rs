use zng::{
    gesture::is_pressed,
    layout::margin,
    mouse::{cursor, CursorIcon},
    widget::Wgt,
    APP,
};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        margin = 0;
        cursor = CursorIcon::Default;
        when *#is_pressed {
            margin = 0
            cursor = CursorIcon::Pointer;
        }
    };
}
