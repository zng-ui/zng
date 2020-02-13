use crate::core::types::*;
use crate::core::var::IntoVar;
use crate::core::UiNode;
use crate::properties::set_context_var;
use crate::property;
use crate::widgets::{FontFamily, FontSize, TextColor};
use std::borrow::Cow;

/// Sets the [FontFamily] context var.
#[property(context_var)]
pub fn font_family(child: impl UiNode, font: impl IntoVar<Cow<'static, str>>) -> impl UiNode {
    set_context_var::set(child, FontFamily, font)
}

/// Sets the [FontSize] context var.
#[property(context_var)]
pub fn font_size(child: impl UiNode, size: impl IntoVar<u32>) -> impl UiNode {
    set_context_var::set(child, FontSize, size)
}

/// Sets the [TextColor] context var.
#[property(context_var)]
pub fn text_color(child: impl UiNode, color: impl IntoVar<ColorF>) -> impl UiNode {
    set_context_var::set(child, TextColor, color)
}
