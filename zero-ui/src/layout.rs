//! Layout service, units and other types.
//!
//! # Measure & Layout
//!
//! TODO !!:
//!
//! # Exact Size & Units
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
//!                 width!(50.pct()), // 50% of the available width
//!                 width!(1.fct()), // 1 times the available width
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
//!                 width!(1.lft()), //1 parcel of the leftover space.
//!             ];
//!             widget::border = 1, colors::RED.desaturate(50.pct());
//!         };
//!     };
//! }
//! # ;
//! ```
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
