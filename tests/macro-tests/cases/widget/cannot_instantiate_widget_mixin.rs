use zng::prelude_wgt::widget_mixin;

#[widget_mixin]
pub struct TextMix<P>(P);

fn main() {
    let _scope = zng::APP.minimal();
    let _ = TextMix!();
}
