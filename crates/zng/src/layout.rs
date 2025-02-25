//! Layout service, units and other types.
//!
//! A widget final size and position is influenced by the widget and all ancestor widgets, the properties
//! and nodes that influence the size and position can be grouped into [widget intrinsics](#widget-intrinsics),
//! [widget properties](#widget-properties), [layout properties](#layout-properties) and [transform properties](#transform-properties).
//!
//! Internally this is split into two passes [`UiNode::layout`] and [`UiNode::render`], transform properties are only applied
//! during render and only influence the size and position of the widget and descendants, the other properties are true layout
//! and influence the size and position of the parent widget and siblings too.
//!
//! ## Widget Intrinsics
//!
//! Each widget defines a size preference, the default widget has no minimum nor maximum size, it fills available space and collapses
//! to zero when aligned, most widgets override this and have a minimum size preference.
//! The `Text!` prefers a size that fits the entire text without introducing wrap line breaks,
//! the `Stack!` widget prefers a size that fits all its children positioned in a given direction.
//!
//! ### Widget Properties
//!
//! Widget size can be influenced by properties widget specific properties, the `Text!` widget is affected by the font properties
//! for example, as different fonts have different sizes. The `Stack!` widget is affected by the `direction` property that changes
//! position of children widgets and so changes the size that best fits the children.
//!
//! ## Layout Properties
//!
//! Widget size and position can be more directly configured using the standalone layout properties defined in this module,
//! as an example the [`min_size`](fn@min_size) property influences the widget size and the [`align`](fn@align) property
//! influences the widget position, the [`margin`](fn@margin) property potentially influences both size and position.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! # let _ =
//! Window! {
//!     child = Wgt! {
//!         layout::min_size = 40;
//!         layout::align = layout::Align::CENTER;
//!         widget::background_color = colors::AZURE;
//!     };
//! }
//! # ;
//! ```
//!
//! ## Transform Properties
//!
//! Widget size and position can be affected during render only, the standalone [`transform`](fn@transform) property
//! and derived properties like [`scale`](fn@scale) change the final size and position of the widget by transforming
//! the final layout size and position, this affects only the widget and descendants, widget interactions like clicks
//! will *see* the widget at its final transformed bounds, but the parent widget will size itself and position other
//! children using the layout size and position.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! # let _ =
//! Stack! {
//!     layout::align = layout::Align::CENTER;
//!     direction = StackDirection::left_to_right();
//!     children = ui_vec![
//!         Wgt! {
//!             layout::size = (100, 200);
//!             widget::background_color = colors::RED;
//!         },
//!         Wgt! {
//!             layout::scale = 120.pct();
//!             layout::size = (100, 200);
//!             widget::z_index = widget::ZIndex::FRONT;
//!             widget::background_color = colors::GREEN;
//!         },
//!         Wgt! {
//!             layout::size = (100, 200);
//!             widget::background_color = colors::BLUE;
//!         },
//!     ];
//! }
//! # ;
//! ```
//!
//! The example above declares a horizontal stack with 3 rectangles, the green rectangle is rendered
//! slightly over the other rectangles because it is [`scale`](fn@scale) to 120% of the size, scale
//! is a render transform only so the stack widget still positions the other rectangles as if the middle
//! one was not scaled. Also note the [`widget::z_index`](fn@crate::widget::z_index) usage, the stack widget
//! render each children in declaration order by default, this is overridden for the green rectangle so
//! it is rendered last, over the blue rectangle too.
//!
//! # Layout Units
//!
//! Most layout properties receive inputs in [`Length`] or length composite types like [`Size`]. These
//! types are layout in the widget context to compute their actual length, the example below demonstrates
//! every [`LengthUnits`], [`FactorUnits`] and length expressions.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! macro_rules! width {
//!     ($width:expr) => {
//!         Text! {
//!             layout::force_width = $width;
//!             txt = stringify!($width);
//!             widget::background_color = colors::BLUE.desaturate(50.pct());
//!         }
//!     };
//! }
//! # let _ =
//! Window! {
//!     child_align = layout::Align::START;
//!     child = Scroll! {
//!         mode = zng::scroll::ScrollMode::VERTICAL;
//!         padding = 10;
//!         child = Stack! {
//!             direction = StackDirection::top_to_bottom();
//!             spacing = 2;
//!             children = ui_vec![
//!                 width!(100), // 100 device independent pixels
//!                 width!(100.dip()), // 100 device independent pixels
//!                 width!(100.px()), // 100 device pixels
//!                 width!(100.pct()), // 100% of the available width
//!                 width!(100.pct_l()), // 100% of the available width
//!                 width!(50.pct()), // 50% of the available width
//!                 width!(1.fct()), // 1 times the available width
//!                 width!(1.fct_l()), // 1 times the available width
//!                 width!(0.5.fct()), // 0.5 times the available width
//!                 width!(100.pt()), // 100 font points
//!                 width!(8.em()), // 8 times the font size
//!                 width!(800.em_pct()), // 800% of the font size
//!                 width!(8.rem()), // 8 times the root font size
//!                 width!(800.rem_pct()), // 800% of the root font size
//!                 width!(1.vw()), // 1 times the viewport width
//!                 width!(100.vw_pct()), // 100% of the viewport width
//!                 width!(0.5.vw()), // 0.5 times the viewport width
//!                 width!(1.vh()), // 1 times the viewport height
//!                 width!(100.vh_pct()), // 100% of the viewport height
//!                 width!(0.5.vh()), // 0.5 times the viewport height
//!                 width!(0.5.vmin()), // 0.5 times the viewport min(width, height)
//!                 width!(50.vmin_pct()), // 50% of the viewport min(width, height)
//!                 width!(0.5.vmax()), // 0.5 times the viewport max(width, height)
//!                 width!(50.vmax_pct()), // 50% of the viewport max(width, height)
//!                 width!(100.dip() + 50.pct()), // expression, 100dip + 50%.
//!                 width!(1.lft()), //1 parcel of the leftover space.
//!                 width!(Length::Default), // default value
//!             ];
//!             widget::border = 1, colors::RED.desaturate(50.pct());
//!         };
//!     };
//! }
//! # ;
//! ```
//!
//! ## Length & Factor Units
//!
//! Length units are defined by [`LengthUnits`] that provides extension methods for `f32` and `i32` values.
//!
//! The most common unit is the *device independent pixel*, or DIP, this is a value that is multiplied by the system scale
//! factor to compute the an exact pixel length, widgets sized in DIPs have a similar apparent size indented of the
//! screen pixel density. This is the default unit, `f32` and `i32` convert to it so `width = 100;` is the same as `width = 100.dip();`.
//!
//! Length can be relative to the available space provided by the parent widget, `100.pct()` and `1.fct()` declare [`FactorPercent`]
//! and [`Factor`] values that convert to [`Length::Factor`]. The [`FactorUnits`] provide the extension methods and
//! is implemented for `f32` and `i32`. You can also use `100.pct_l()` and `1.fct_l()` to get a [`Length`] value directly in places
//! that don't convert the factor types to length.
//!
//! There are multiple units related to font size, `24.pt()` defines a size in *font points*, one font point is `96/72 * scale_factor`
//! device pixels. Size can be relative to the contextual font size, `2.em()` and `200.em_pct()` declare a length twice the computed
//! contextual font size, `2.rem()` and `2.rem_pct()` declare a length twice the computed root font size (the `Window!` font size).
//!
//! Lengths can also be relative to the *viewport*. The viewport is the window content area size, or the parent `Scroll!` visible area size.
//! Lengths `0.2.vw()` and `20.vw_pct()` are 20% of the viewport width, `0.2.vh()` and `20.vh_pct()` are 20% of the viewport height,
//! `1.vmin()` is the minimum viewport length (`min(w, h)`), `1.vmax()` is the maximum viewport length.
//!
//! ### Length Expressions
//!
//! Different length units can be mixed into a length expression, `1.em() + 5.dip()` will create a [`Length::Expr`] value that on layout
//! will compute the pixel length of both terms and then sum. Length expressions support the four basic arithmetic operations, negation,
//! maximum and minimum and absolute.
//!
//! Some basic length expressions are pre-computed on the spot, `10.dip() + 10.dip()` declares a `Length::Dip(20)` value, but most
//! expression declare an object that dynamically executes the expression after all terms are layout.
//!
//! ### Leftover Length
//!
//! The leftover length is a special value that represents the space leftover after non-leftover sibling widgets are layout. This
//! must be implemented by a parent widget to fully work, the `Grid!` widget implements it, in widgets that don't implement it
//! the unit behaves like a factor.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! # let _ =
//! Window! {
//!     child = Grid! {
//!         columns = ui_vec![
//!             grid::Column!(1.lft()),
//!             grid::Column!(100.dip()),
//!             grid::Column!(2.lft()),
//!         ];
//!         rows = ui_vec![grid::Row!(100.pct())];
//!         cells = ui_vec![
//!             Wgt! {
//!                 grid::cell::column = 0;
//!                 widget::background_color = colors::RED;
//!             },
//!             Wgt! {
//!                 grid::cell::column = 1;
//!                 widget::background_color = colors::GREEN;
//!             },
//!             Wgt! {
//!                 grid::cell::column = 2;
//!                 widget::background_color = colors::BLUE;
//!             },
//!         ];
//!     }
//! }
//! # ;
//! ```
//!
//! The example above declares a grid with 3 columns, on layout the grid computes the width of the middle column first (`100.dip()`),
//! the leftover available width is divided between the other 2 columns proportional to the leftover value. Note that value range
//! of leftover is normalized across all leftover siblings, in the example above changing the values to `2.lft()` and `4.lft()`
//! will produce the column sizes.
//!
//! ### Default Length
//!
//! The [`Length::Default`] value represents the length that is used when no other length is set. It is a placeholder value
//! that is filled in by the widget or property that is resolving the layout. The `grid::Column!()` has `Default` width, in
//! grids this means *auto-size*, the column is sized to fit all cells. In the standalone [`width`](fn@width) property
//! the default width means the fill width.
//!
//! # Measure & Layout
//!
//! Nodes that implement custom layout must handle [`UiNode::measure`] and [`UiNode::layout`].
//! Measure and layout provide a desired size and final size respectively, given the same context both methods return the
//! same size, the different is that the measure call must not actually affect the widget, it exists to allow a parent widget
//! to query what the layout result would be for a given context.
//!
//! Consider a `Stack!` that is aligned `CENTER` and has children aligned `FILL`, to fulfill these constraints
//! the stack does the layout in two passes, first it measures each child to find the width, then it layouts
//! each child constrained to this width. If this same stack is given an exact size it will skip the measure
//! pass and just do the layout directly.
//!
//! The coordination between layout properties on a widget and between widgets is centered on the [`LAYOUT`], [`WidgetMeasure`],
//! [`WidgetLayout`] and the return [`PxSize`]. Parent nodes set context metrics and constraints using the [`LAYOUT`] service,
//! child nodes returns the size and optionally set more return metadata in the [`WidgetMeasure`] and [`WidgetLayout`] args.
//! The parent node then sets the child position using [`WidgetLayout`] or by manually transforming the child during render.
//!
//! Other contextual services and variables may complement the layout computation, the [`WIDGET_SIZE`] is used to implement
//! [`Length::Leftover`] layouts, the [`widget::BORDER`] is used to implement the alignment between borders and the background.
//! Widgets can use context vars to define layout preferences that only apply to their special layout, the `Text!` and `Image!`
//! widgets are examples of this.
//!
//! UI components are very modular, during layout is when they are the closest coupled, implementers must careful consider
//! the full [`LAYOUT`], [`WidgetMeasure`] [`WidgetLayout`] APIs, understand what properties placed in the [`NestGroup::LAYOUT`] can do
//! and what the widget outer and inner bounds are. Implementers also must consider if their layout will support inlining or
//! if it will only be a block. After reading the APIs a good way to learn is by studying the source code of properties in this
//! module, followed by the `Image!`, `Stack!`, `Grid!` and `Wrap!` implementations.
//!
//! ## Outer & Inner Bounds
//!
//! Each laidout widget has two computed rectangles, the inner bounds define the rendered area, the outer bounds define
//! the extra space taken by the widget layout, properties like [`align`](fn@align) and [`margin`](fn@margin) are still
//! a part of the widget, the blank space they add *around* the widget is inside the widget outer bounds.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! # let _ =
//! Window! {
//!     padding = 20;
//!     child = Wgt! {
//!         layout::size = 80;
//!         layout::align = layout::Align::CENTER;
//!         window::inspector::show_bounds = true;
//!     };
//! }
//! # ;
//! ```
//!
//! The example above uses the [`window::inspector::show_bounds`] property to inspect the bounds of a widget, it shows the
//! outer bounds of the widget extend to almost cover the entire window, that happens because the window default `child_align` is
//! `FILL` and it only reserved `20` of padding space, leaving the rest of the space for the child widget to handle. The widget
//! wants to have an exact size of `80` centered on the available space, so it ends up with the outer bounds taking the available space
//! and the inner bounds taking the exact size.
//!
//! ## Inline
//!
//! Layout has two modes, block and inline, in block layout the shape of the laidout widgets is not changed, they are always
//! rectangular, inline layout expands layout to alter the shape of laidout widgets to potentially split into multiple rectangles that
//! define the first line, the middle block of lines and the last line.
//!
//! The example below declares a `Wrap!` with three `Text!` children, both the wrap and text widgets support inline layout so the end-result
//! is that the green text will be reshaped as two rectangles, one after the red text and one before the blue text.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! # let _ =
//! Wrap! {
//!     children = ui_vec![
//!         Text! {
//!             widget::background_color = colors::RED.with_alpha(40.pct());
//!             txt = "RED";
//!         },
//!         Text! {
//!             widget::background_color = colors::GREEN.with_alpha(40.pct());
//!             txt = "GREEN\nGREEN";
//!         },
//!         Text! {
//!             widget::background_color = colors::BLUE.with_alpha(40.pct());
//!             txt = "BLUE";
//!         },
//!     ]
//! }
//! # ;
//! ```
//!
//! Inline layout is modeled to support complex text layout interactions, like bidirectional text reordering, inlined widgets don't need
//! to be text however, the `Wrap!` widget itself can be nested.
//!
//! If a widget does not support inline it calls [`WidgetMeasure::disable_inline`], in an inline context these widgets
//! are *inline-blocks*. If a panel widget does not support inline and it needs to measure children it calls [`WidgetMeasure::measure_block`].
//!
//! If a widget or property supports inline it can detect it is in an inline context by [`WidgetMeasure::inline`] where the preferred
//! segments of the widget can be set for the parent inline panel to analyze, if inline is set during measure it will also be inline
//! during layout and [`LAYOUT`] will have inline constraints. During layout the [`WidgetLayout::inline`] value can
//! be set to the final inline info.
//!
//! After inline layout the children are positioned so that the last line of the previous sibling connects with the first line of the next, all
//! of the widget visual properties must support this however, the [`WIDGET.bounds().inline()`] is available during render with cached
//! negative space clips that can quickly be used. If a visual property is not aware of inline it can potentially render over the
//! previous sibling, inline should be disabled for the widget if the property cannot support inline.
//!
//! [`WIDGET.bounds().inline()`]: crate::widget::info::WidgetBoundsInfo::inline
//! [`UiNode::measure`]: crate::widget::node::UiNode::measure
//! [`UiNode::Layout`]: crate::widget::node::UiNode::layout
//! [`UiNode::render`]: crate::widget::node::UiNode::render
//! [`widget::BORDER`]: crate::widget::BORDER
//! [`NestGroup::LAYOUT`]: crate::widget::builder::NestGroup::LAYOUT
//! [`window::inspector::show_bounds`]: fn@crate::window::inspector::show_bounds
//!
//! # Full API
//!
//! See [`zng_layout`], [`zng_wgt_transform`] and [`zng_wgt_size_offset`] for the full API.

pub use zng_layout::unit::{
    Align, AngleDegree, AngleGradian, AngleRadian, AngleTurn, AngleUnits, BoolVector2D, ByteLength, ByteUnits, CornerRadius2D, Dip, DipBox,
    DipCornerRadius, DipPoint, DipRect, DipSideOffsets, DipSize, DipToPx, DipVector, DistanceKey, Factor, Factor2d, FactorPercent,
    FactorSideOffsets, FactorUnits, GridSpacing, Layout1d, Layout2d, LayoutAxis, Length, LengthExpr, LengthUnits, Line,
    LineFromTuplesBuilder, Orientation2D, Point, Ppi, Ppm, Px, PxBox, PxConstraints, PxConstraints2d, PxCornerRadius, PxGridSpacing,
    PxLine, PxPoint, PxRect, PxSideOffsets, PxSize, PxToDip, PxTransform, PxVector, Rect, RectFromTuplesBuilder, ResolutionUnits,
    SideOffsets, SideOffsets2D, Size, TimeUnits, Transform, Vector,
};

pub use zng_var::types::{slerp_enabled, slerp_sampler};

pub use zng_layout::context::{
    DIRECTION_VAR, InlineConstraints, InlineConstraintsLayout, InlineConstraintsMeasure, InlineSegment, InlineSegmentPos, LAYOUT,
    LayoutDirection, LayoutMask, LayoutMetrics, LayoutMetricsSnapshot, LayoutPassId, TextSegmentKind,
};

pub use zng_app::widget::info::{WidgetLayout, WidgetMeasure};

pub use zng_wgt_transform::{
    backface_visibility, perspective, perspective_origin, rotate, rotate_x, rotate_y, rotate_z, scale, scale_x, scale_xy, scale_y, skew,
    skew_x, skew_y, transform, transform_origin, transform_style, translate, translate_x, translate_y, translate_z,
};

pub use zng_wgt_size_offset::{
    WIDGET_SIZE, WidgetLength, actual_bounds, actual_height, actual_height_px, actual_size, actual_size_px, actual_transform, actual_width,
    actual_width_px, baseline, force_height, force_size, force_width, height, max_height, max_size, max_width, min_height, min_size,
    min_width, offset, size, sticky_height, sticky_size, sticky_width, width, x, y,
};

pub use zng_wgt::{InlineMode, align, inline, is_ltr, is_rtl, margin};

pub use zng_wgt_container::{child_align, padding};

pub use zng_app::render::TransformStyle;
