//! Properties that are only used by widgets directly.
//!
//! Setting this properties in a widget that does not reexports then is an error.
use crate::core::gesture::Shortcut;
use crate::prelude::new_property::*;
use crate::widgets::LineStyle;

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

/// A [`Length`] spacing.
#[property(capture_only)]
pub fn spacing(spacing: impl IntoVar<Length>) -> ! {}

/// A [`GridSpacing`] spacing.
#[property(capture_only)]
pub fn grid_spacing(spacing: impl IntoVar<GridSpacing>) -> ! {}

/// A [`text!`](crate::widgets::text) value.
#[property(capture_only)]
pub fn text_value(text: impl IntoVar<Text>) -> ! {}

/// A [`Shortcut`] variable.
#[property(capture_only)]
pub fn key_shortcut(shortcut: impl IntoVar<Shortcut>) -> ! {}

/// A [`line!`](crate::widgets::line) orientation.
#[property(capture_only)]
pub fn line_orientation(orientation: impl IntoVar<LineOrientation>) -> ! {}

/// `Rgba` value.
#[property(capture_only)]
pub fn color(color: impl IntoVar<Rgba>) -> ! {}

/// A 'f32' width.
#[property(capture_only)]
pub fn width(width: impl IntoVar<f32>) -> ! {}

/// A [`line!`](crate::widgets::line) length.
#[property(capture_only)]
pub fn length(length: impl IntoVar<f32>) -> ! {}

/// A [`line!`](crate::widgets::line) style.
#[property(capture_only)]
pub fn line_style(style: impl IntoVar<LineStyle>) -> ! {}

/// An [`usize`] that represents a zero-based index.
#[property(capture_only)]
pub fn index(index: impl IntoVar<usize>) -> ! {}

/// An [`usize`] that represents a list length.
#[property(capture_only)]
pub fn len(len: impl IntoVar<usize>) -> ! {}

/// A [`bool`] that enables a feature.
#[property(capture_only)]
pub fn enabled(enabled: impl IntoVar<bool>) -> ! {}