use zero_ui::wgt_prelude::{widget, WidgetBase};

// doesn't start with $crate::
#[widget(TestWidget)]
pub struct TestWidget(WidgetBase);

fn main() {}
