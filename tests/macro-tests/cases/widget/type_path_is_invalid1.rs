use zng::prelude_wgt::{widget, WidgetBase};

#[widget($crate::not::a::valid::path)]
pub struct TextWidget(WidgetBase);

fn main() {}
