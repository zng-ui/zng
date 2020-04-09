use crate::core::types::*;
use crate::core::var::IntoVar;
use crate::core::UiNode;
use crate::properties::with_context_var;
use crate::property;
use crate::widgets::{FontFamily, FontSize, TextColor, TextTransform, TextTransformFn};

/// Sets the [`FontFamily`](FontFamily) context var.
#[property(context)]
pub fn font_family(child: impl UiNode, font: impl IntoVar<Text>) -> impl UiNode {
    with_context_var(child, FontFamily, font)
}

/// Sets the [`FontSize`](FontSize) context var.
#[property(context)]
pub fn font_size(child: impl UiNode, size: impl IntoVar<u32>) -> impl UiNode {
    with_context_var(child, FontSize, size)
}

/// Sets the [`TextColor`](TextColor) context var.
#[property(context)]
pub fn text_color(child: impl UiNode, color: impl IntoVar<ColorF>) -> impl UiNode {
    with_context_var(child, TextColor, color)
}

/// Sets the [`TextTransform`](TextTransform) context var.
#[property(context)]
pub fn text_transform(child: impl UiNode, transform: impl IntoVar<TextTransformFn>) -> impl UiNode {
    with_context_var(child, TextTransform, transform)
}
