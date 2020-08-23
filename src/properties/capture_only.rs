//! Properties that are only used by widgets directly.

use crate::{
    core::{gesture::Shortcut, property, types::*, var::IntoVar, UiNode, UiVec},
    widgets::LineStyle,
};

/// Widget id.
///
/// # Implicit
///
/// All widgets automatically inherit from [`implicit_mixin`] that defines an `id`
/// property that maps to this property and sets a default value of `WidgetId::new_unique()`.
///
/// The default widget `new` function captures this `id` property and uses in the default
/// [`Widget`](crate::core::Widget) implementation.
#[property(capture_only)]
pub fn widget_id(id: WidgetId) -> ! {}

/// Widget child node.
///
/// # Container
///
/// Widgets that contain a single other widget can capture this property in their implementation.
#[property(capture_only, allowed_in_when: false)]
pub fn widget_child(child: impl UiNode) -> ! {}

/// Widget children nodes.
///
/// # Layout
///
/// Layout widgets can capture this property in their implementation.
#[property(capture_only, allowed_in_when: false)]
pub fn widget_children(children: UiVec) -> ! {}

/// A `f32` spacing.
#[property(capture_only)]
pub fn spacing(spacing: impl IntoVar<f32>) -> ! {}

/// A [`text!`](crate::widgets::text) value.
#[property(capture_only)]
pub fn text_value(text: impl IntoVar<Text>) -> ! {}

/// A [`KeyShortcut`] variable.
#[property(capture_only)]
pub fn key_shortcut(shortcut: impl IntoVar<Shortcut>) -> ! {}

/// A [`line!`](crate::widgets::line) orientation.
#[property(capture_only)]
pub fn line_orientation(orientation: impl IntoVar<LineOrientation>) -> ! {}

/// `ColoF` value.
#[property(capture_only)]
pub fn color(color: impl IntoVar<ColorF>) -> ! {}

/// A 'f32' width.
#[property(capture_only)]
pub fn width(width: impl IntoVar<f32>) -> ! {}

/// A [`line!`](crate::widgets::line) length.
#[property(capture_only)]
pub fn length(length: impl IntoVar<f32>) -> ! {}

/// A [`line!`](crate::widgets::line) style.
#[property(capture_only)]
pub fn line_style(style: impl IntoVar<LineStyle>) -> ! {}
