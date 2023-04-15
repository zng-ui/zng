use zero_ui::prelude::new_widget::*;

#[widget_mixin]
pub struct TextMix<P>(P);

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = TextMix!();
}
