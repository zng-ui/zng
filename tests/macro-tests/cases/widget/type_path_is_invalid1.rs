use zng::prelude_wgt::{WidgetBase, widget};

#[widget($crate::not::a::valid::path)]
pub struct TextWidget(WidgetBase);

fn main() {}
