use crate::prelude::*;

/// Minimal widget.
///
/// You can use this to create a quick new custom widget defined entirely
/// by standalone properties and `when` conditions.
#[widget($crate::Wgt)]
pub struct Wgt(WidgetBase);
