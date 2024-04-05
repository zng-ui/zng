use zng::prelude_wgt::{widget, WidgetBase};

// doesn't start with $crate::
#[widget(TestWidget)]
pub struct TestWidget(WidgetBase);

fn main() {}
