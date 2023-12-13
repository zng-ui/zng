use zero_ui::wgt_prelude::widget_mixin;

#[widget_mixin]
pub struct TextMix<P>(P);

fn main() {
    let _scope = zero_ui::APP.minimal();
    let _ = TextMix!();
}
