use zng::prelude_wgt::{WidgetBase, widget};

// doesn't start with $crate
#[widget($self::TestWidget)]
pub struct TestWidget(WidgetBase);

fn main() {}
