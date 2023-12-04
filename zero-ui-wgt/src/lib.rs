//! Basic widget properties and helpers for declaring widgets.

use zero_ui_app::widget::*;

pub mod nodes;

/// Empty widget.
#[widget($crate::Wgt)]
pub struct Wgt(base::WidgetBase);
