use zng::{
    APP,
    gesture::is_pressed,
    layout::margin,
    mouse::{CursorIcon, cursor},
    widget::Wgt,
};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        margin = 0;
        cursor = CursorIcon::Default;
        when *#is_pressed {
            margin = cursor = CursorIcon::Pointer;
        }
    };
}
