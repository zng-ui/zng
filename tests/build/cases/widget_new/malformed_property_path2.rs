use zero_ui::{properties::margin, widgets::wgt};

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = wgt! {
        margin! = 0;
    };
}
