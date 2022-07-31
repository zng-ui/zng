use zero_ui::core::widget;

#[widget($crate::foo)]
pub mod foo {}

#[widget($crate::foo)]
pub mod foo {}

// the hash for the widget path is the same, so unfortunately all all generated macros end-up with the same name, at least the
// just the second widget is highlighted?

fn main() {}
