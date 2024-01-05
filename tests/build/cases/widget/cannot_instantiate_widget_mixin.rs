use zero_ui::prelude_wgt::widget_mixin;

#[widget_mixin]
pub struct TextMix<P>(P);

fn main() {
    let _scope = zero_ui::APP.minimal();
    let _ = TextMix!();
}
