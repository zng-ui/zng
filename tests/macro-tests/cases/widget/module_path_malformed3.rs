use zng::prelude_wgt::{widget, WidgetBase};

// doesn't start with $crate
#[widget($self::TestWidget)]
pub struct TestWidget(WidgetBase);

fn main() {}
