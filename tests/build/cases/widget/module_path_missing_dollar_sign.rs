use zero_ui::wgt_prelude::{widget, WidgetBase};

// doesn't start with $
#[widget(crate::TestWidget)]
pub struct TestWidget(WidgetBase);

fn main() {}
