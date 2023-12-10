use zero_ui::prelude::new_widget::*;

#[widget_mixin]
pub struct TextMix<P>(P);

fn main() {
    let _scope = zero_ui::core::app::APP.minimal();
    let _ = TextMix!();
}
