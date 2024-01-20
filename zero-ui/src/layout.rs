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
//! [`UiNode::Layout`]: crate::widget::node::UiNode::layout
//! [`UiNode::render`]: crate::widget::node::UiNode::render
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
//! use zero_ui::prelude::*;
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
//! use zero_ui::prelude::*;
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
//! use zero_ui::prelude::*;
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
//!         mode = zero_ui::scroll::ScrollMode::VERTICAL;
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
//! use zero_ui::prelude::*;
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
//! # Layout Pass
//!
//! !!:
//!
//! ## Service
//!
//! ## Constraints & Align
//!
//! ## Outer & Inner Bounds
//!
//! ## Inline
//!
//! ## Border?
//!
//! # Full API
//!
//! See [`zero_ui_layout`], [`zero_ui_wgt_transform`] and [`zero_ui_wgt_size_offset`] for the full API.

pub use zero_ui_layout::unit::{
    slerp_enabled, slerp_sampler, Align, AngleDegree, AngleGradian, AngleRadian, AngleTurn, AngleUnits, BoolVector2D, ByteLength,
    ByteUnits, CornerRadius2D, Deadline, Dip, DipBox, DipCornerRadius, DipPoint, DipRect, DipSideOffsets, DipSize, DipToPx, DipVector,
    DistanceKey, Factor, Factor2d, FactorPercent, FactorSideOffsets, FactorUnits, GridSpacing, Layout1d, Layout2d, LayoutAxis, LayoutMask,
    Length, LengthExpr, LengthUnits, Line, LineFromTuplesBuilder, Orientation2D, Point, Ppi, Ppm, Px, PxBox, PxConstraints,
    PxConstraints2d, PxCornerRadius, PxGridSpacing, PxLine, PxPoint, PxRect, PxSideOffsets, PxSize, PxToDip, PxTransform, PxVector, Rect,
    RectFromTuplesBuilder, RenderAngle, ResolutionUnits, SideOffsets, SideOffsets2D, Size, TimeUnits, Transform, Vector,
};

pub use zero_ui_layout::context::{
    InlineConstraints, InlineConstraintsLayout, InlineConstraintsMeasure, InlineSegment, InlineSegmentPos, LayoutDirection, LayoutMetrics,
    LayoutMetricsSnapshot, LayoutPassId, TextSegmentKind, DIRECTION_VAR, LAYOUT,
};

pub use zero_ui_app::widget::info::{WidgetLayout, WidgetMeasure};

pub use zero_ui_wgt_transform::{
    backface_visibility, perspective, perspective_origin, rotate, rotate_x, rotate_y, rotate_z, scale, scale_x, scale_xy, scale_y, skew,
    skew_x, skew_y, transform, transform_origin, transform_style, translate, translate_x, translate_y, translate_z,
};

pub use zero_ui_wgt_size_offset::{
    actual_bounds, actual_height, actual_height_px, actual_size, actual_size_px, actual_transform, actual_width, actual_width_px, baseline,
    force_height, force_size, force_width, height, max_height, max_size, max_width, min_height, min_size, min_width, offset, size,
    sticky_height, sticky_size, sticky_width, width, x, y, WidgetLength, WIDGET_SIZE,
};

pub use zero_ui_wgt::{align, inline, is_ltr, is_rtl, margin, InlineMode};

pub use zero_ui_wgt_container::{child_align, padding};

pub use zero_ui_app::render::TransformStyle;
