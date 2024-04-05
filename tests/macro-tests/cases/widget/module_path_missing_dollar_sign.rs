use zng::prelude_wgt::{widget, WidgetBase};

// doesn't start with $
#[widget(crate::TestWidget)]
pub struct TestWidget(WidgetBase);

fn main() {}
