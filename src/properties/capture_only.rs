//! Properties that are only used by widgets directly.
//!
//! Setting these properties in a widget that does not reexport them is an error.
use crate::core::gesture::Shortcut;
use crate::prelude::new_property::*;
use crate::widgets::{LineOrientation, LineStyle};

/// [`Widget`] child node.
///
/// # Container
///
/// Widgets that contain a single other widget can capture this property in their implementation.
#[property(capture_only, allowed_in_when = false)]
pub fn widget_child(child: impl Widget) -> ! {}

/// [`UiNode`] child node.
#[property(capture_only, allowed_in_when = false)]
pub fn node_child(child: impl UiNode) -> ! {}

/// [`Widget`] children nodes.
///
/// # Layout
///
/// Layout widgets can capture this property in their implementation.
#[property(capture_only, allowed_in_when = false)]
pub fn widget_children(children: impl WidgetList) -> ! {}

/// [`UiNode`] children nodes.
#[property(capture_only, allowed_in_when = false)]
pub fn node_children(children: impl UiNodeList) -> ! {}

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
