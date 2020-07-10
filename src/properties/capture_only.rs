//! Properties that are only used by widgets directly by capturing them in the `new` or `new_child` function.

use crate::{
    core::{property, types::WidgetId, var::IntoVar, UiVec},
    prelude::{Text, UiNode},
};

/// Widget id.
///
/// # Implicit
///
/// All widgets automatically inherit from [`implicit_mixin`](implicit_mixin) that defines an `id`
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
#[property(capture_only)]
pub fn widget_child(child: impl UiNode) -> ! {}

/// Widget children nodes.
///
/// # Layout
///
/// Layout widgets can capture this property in their implementation.
#[property(capture_only)]
pub fn widget_children(children: UiVec) -> ! {}

/// Stack in-between spacing.
#[property(capture_only)]
pub fn stack_spacing(spacing: impl IntoVar<f32>) -> ! {}

/// A [`text!`](crate::widgets::text) value.
#[property(capture_only)]
pub fn text_value(text: impl IntoVar<Text>) -> ! {}
