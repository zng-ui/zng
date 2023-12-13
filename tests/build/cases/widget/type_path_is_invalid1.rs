use zero_ui::wgt_prelude::{widget, WidgetBase};

#[widget($crate::not::a::valid::path)]
pub struct TextWidget(WidgetBase);

fn main() {}
