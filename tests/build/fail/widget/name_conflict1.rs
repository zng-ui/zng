use zero_ui::core::widget;

#[allow(unused_macros)]
macro_rules! foo {
    () => {};
}
pub use crate::foo;

#[widget($crate::foo)]
pub mod foo {}

// #[widget] expands to another `macro_rules! foo` and `pub use foo;`
// The full call_site (line 8) gets highlighted here, that is usually
// bad, but in this case it is the least confusing span we can use.

fn main() {}
