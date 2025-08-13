use zng::prelude_wgt::{WidgetBase, widget};

#[allow(unused_macros)]
macro_rules! TestWidget {
    () => {};
}
#[allow(unused_imports)]
pub use crate::TestWidget;

#[widget($crate::TestWidget)]
pub struct TestWidget(WidgetBase);

// #[widget] expands to another `macro_rules! foo` and `pub use foo;`
// The full call_site (line 8) gets highlighted here, that is usually
// bad, but in this case it is the least confusing span we can use.

fn main() {}
