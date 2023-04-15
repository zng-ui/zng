use zero_ui_core::*;

pub use zero_ui_core::app::App;

/// Widget macro references `zero_ui_core::widget_new!`.
#[widget($crate::Foo)]
pub struct Foo(widget_base::WidgetBase);