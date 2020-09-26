//! Theme context vars and properties for the [`button!`](module@crate::widgets::button) widget.

use crate::core::color::rgb;
use crate::core::color::Rgba;
use crate::core::property;
use crate::core::units::*;
use crate::core::var::{context_var, IntoVar};
use crate::core::UiNode;
use crate::properties::{with_context_var, BorderDetails};

context_var! {
    /// Default background of [`button!`](module@crate::widgets::button) widgets.
    pub struct ButtonBackgroundVar: Rgba = once rgb(0.2, 0.2, 0.2);
    pub struct ButtonBackgroundHoveredVar: Rgba = once rgb(0.25, 0.25, 0.25);
    pub struct ButtonBackgroundPressedVar: Rgba = once rgb(0.3, 0.3, 0.3);

    pub struct ButtonBorderWidthsVar: SideOffsets = once SideOffsets::new_all(1.0);
    pub struct ButtonBorderWidthsHoveredVar: SideOffsets = once SideOffsets::new_all(1.0);
    pub struct ButtonBorderWidthsPressedVar: SideOffsets = once SideOffsets::new_all(1.0);

    pub struct ButtonBorderDetailsVar: BorderDetails = once BorderDetails::solid(rgb(0.2, 0.2, 0.2));
    pub struct ButtonBorderDetailsHoveredVar: BorderDetails = once BorderDetails::solid(rgb(0.4, 0.4, 0.4));
    pub struct ButtonBorderDetailsPressedVar: BorderDetails = once BorderDetails::solid(rgb(0.6, 0.6, 0.6));

    pub struct ButtonPaddingVar: SideOffsets = once SideOffsets::new(7.0, 15.0, 7.0, 15.0);
}

/// Sets the [`ButtonBackgroundVar`] context var.
#[property(context)]
pub fn button_background(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    with_context_var(child, ButtonBackgroundVar, color)
}

/// Sets the [`ButtonBackgroundHoveredVar`] context var.
#[property(context)]
pub fn button_background_hovered(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    with_context_var(child, ButtonBackgroundHoveredVar, color)
}

/// Sets the [`ButtonBackgroundPressedVar`] context var.
#[property(context)]
pub fn button_background_pressed(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    with_context_var(child, ButtonBackgroundPressedVar, color)
}

/// Sets the [`ButtonPaddingVar`] context var.
#[property(context)]
pub fn button_padding(child: impl UiNode, padding: impl IntoVar<SideOffsets>) -> impl UiNode {
    with_context_var(child, ButtonPaddingVar, padding)
}

/// Sets the [`ButtonBorderWidthsVar`] and [`ButtonBorderDetailsVar`] context var.
#[property(context)]
pub fn button_border(child: impl UiNode, widths: impl IntoVar<SideOffsets>, details: impl IntoVar<BorderDetails>) -> impl UiNode {
    let child = with_context_var(child, ButtonBorderWidthsVar, widths);
    with_context_var(child, ButtonBorderDetailsVar, details)
}

/// Sets the [`ButtonBorderWidthsVar`] and [`ButtonBorderDetailsVar`] context var.
#[property(context)]
pub fn button_border_hovered(child: impl UiNode, widths: impl IntoVar<SideOffsets>, details: impl IntoVar<BorderDetails>) -> impl UiNode {
    let child = with_context_var(child, ButtonBorderWidthsHoveredVar, widths);
    with_context_var(child, ButtonBorderDetailsHoveredVar, details)
}

/// Sets the [`ButtonBorderWidthsPressedVar`] and [`ButtonBorderDetailsPressedVar`] context var.
#[property(context)]
pub fn button_border_pressed(child: impl UiNode, widths: impl IntoVar<SideOffsets>, details: impl IntoVar<BorderDetails>) -> impl UiNode {
    let child = with_context_var(child, ButtonBorderWidthsPressedVar, widths);
    with_context_var(child, ButtonBorderDetailsPressedVar, details)
}
