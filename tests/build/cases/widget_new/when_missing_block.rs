use zero_ui::properties::states::is_pressed;
use zero_ui::widgets::wgt;

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = wgt! {
        when *#is_pressed
    };
}
