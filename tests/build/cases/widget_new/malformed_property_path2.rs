use zero_ui::{properties::margin, widgets::blank};

fn main() {
    let _scope = zero_ui::core::app::App::blank();
    let _ = blank! {
        margin! = 0;
    };
}
