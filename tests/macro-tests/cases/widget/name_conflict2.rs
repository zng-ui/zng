use zng::prelude_wgt::{WidgetBase, widget};

#[widget($crate::TestWidget)]
pub struct TestWidget(WidgetBase);

#[widget($crate::TestWidget)]
pub struct TestWidget(WidgetBase);

// the hash for the widget path is the same, so unfortunately all generated macros end-up with the same name, at least the
// just the second widget is highlighted?

fn main() {}
