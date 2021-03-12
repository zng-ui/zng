use zero_ui::core::widget;

#[widget($crate::foo)]
pub mod foo {}

#[widget($crate::foo)]
pub mod foo {}

// Rust thinks the foo at line 4 is the second one for some reason, also line 3 and 6 are highlighted
// as expected because of the macro name conflict, but line 4 and 7 are not highlighted, good enough.

fn main() {}
