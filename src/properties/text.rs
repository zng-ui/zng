use crate::core::property;
use crate::core::types::*;
use crate::core::var::IntoVar;
use crate::core::UiNode;
use crate::properties::with_context_var;
use crate::widgets::{
    FontFamilyVar, FontSizeVar, FontStretchVar, FontStyleVar, FontWeightVar, TextColorVar, TextTransformFn, TextTransformVar,
};

/// Sets the [`FontFamilyVar`](FontFamilyVar) context var.
#[property(context)]
pub fn font_family(child: impl UiNode, names: impl IntoVar<Box<[FontName]>>) -> impl UiNode {
    with_context_var(child, FontFamilyVar, names)
}

/// Sets the [`FontStyleVar`](FontStyleVar) context var.
#[property(context)]
pub fn font_style(child: impl UiNode, style: impl IntoVar<FontStyle>) -> impl UiNode {
    with_context_var(child, FontStyleVar, style)
}

/// Sets the [`FontWeightVar`](FontWeightVar) context var.
#[property(context)]
pub fn font_weight(child: impl UiNode, weight: impl IntoVar<FontWeight>) -> impl UiNode {
    with_context_var(child, FontWeightVar, weight)
}

/// Sets the [`FontStretchVar`](FontStretchVar) context var.
#[property(context)]
pub fn font_stretch(child: impl UiNode, stretch: impl IntoVar<FontStretch>) -> impl UiNode {
    with_context_var(child, FontStretchVar, stretch)
}

/// Sets the [`FontSizeVar`](FontSizeVar) context var.
#[property(context)]
pub fn font_size(child: impl UiNode, size: impl IntoVar<FontSize>) -> impl UiNode {
    with_context_var(child, FontSizeVar, size)
}

/// Sets the [`TextColorVar`](TextColorVar) context var.
#[property(context)]
pub fn text_color(child: impl UiNode, color: impl IntoVar<ColorF>) -> impl UiNode {
    with_context_var(child, TextColorVar, color)
}

/// Sets the [`TextTransformVar`](TextTransformVar) context var.
#[property(context)]
pub fn text_transform(child: impl UiNode, transform: impl IntoVar<TextTransformFn>) -> impl UiNode {
    with_context_var(child, TextTransformVar, transform)
}
