use crate::core::property;
use crate::core::types::*;
use crate::core::var::IntoVar;
use crate::core::UiNode;
use crate::properties::with_context_var;
use crate::widgets::{ButtonBackgroundVar, ButtonBackgroundHoveredVar, ButtonBackgroundPressedVar, ButtonPaddingVar};

/// Sets the [`ButtonBackground`](ButtonBackground) context var.
#[property(context)]
pub fn button_background(child: impl UiNode, color: impl IntoVar<ColorF>) -> impl UiNode {
    with_context_var(child, ButtonBackgroundVar, color)
}

/// Sets the [`ButtonBackgroundHovered`](ButtonBackgroundHovered) context var.
#[property(context)]
pub fn button_background_hovered(child: impl UiNode, color: impl IntoVar<ColorF>) -> impl UiNode {
    with_context_var(child, ButtonBackgroundHoveredVar, color)
}

/// Sets the [`ButtonBackgroundPressed`](ButtonBackgroundPressed) context var.
#[property(context)]
pub fn button_background_pressed(child: impl UiNode, color: impl IntoVar<ColorF>) -> impl UiNode {
    with_context_var(child, ButtonBackgroundPressedVar, color)
}

/// Sets the [`ButtonPadding`](ButtonPadding) context var.
#[property(context)]
pub fn button_padding(child: impl UiNode, padding: impl IntoVar<LayoutSideOffsets>) -> impl UiNode {
    with_context_var(child, ButtonPaddingVar, padding)
}
