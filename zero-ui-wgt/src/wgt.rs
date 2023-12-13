use crate::prelude::*;

/// Minimal widget.
///
/// You can use this to create a quick new custom widget that is only used in one code place and can be created entirely
/// by properties and `when` conditions.
#[widget($crate::Wgt)]
pub struct Wgt(WidgetBase);
