use zng::prelude_wgt::{WidgetBase, widget};

// doesn't start with $
#[widget(crate::TestWidget)]
pub struct TestWidget(WidgetBase);

fn main() {}
