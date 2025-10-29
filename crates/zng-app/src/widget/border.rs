//! Border and line types.

use std::{fmt, mem, sync::Arc};

use zng_app_context::context_local;
use zng_color::{Hsla, Hsva, Rgba, colors};
use zng_layout::{
    context::{LAYOUT, LayoutMask},
    unit::{
        Factor, FactorPercent, FactorSideOffsets, FactorUnits, Layout2d, Length, PxCornerRadius, PxPoint, PxRect, PxSideOffsets, PxSize,
        Size,
    },
};
use zng_var::{
    animation::{Transitionable, easing::EasingStep},
    context_var, impl_from_and_into_var,
};

pub use zng_view_api::LineOrientation;

use crate::widget::VarLayout;

use super::{WIDGET, WidgetId, info::WidgetBorderInfo};

/// Represents a line style.
#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum LineStyle {
    /// A solid line.
    ///
    /// The shorthand unit `Solid!` converts into this.
    Solid,
    /// Two solid lines in parallel.
    ///
    /// The shorthand unit `Double!` converts into this.
    Double,

    /// Dotted line.
    ///
    /// The shorthand unit `Dotted!` converts into this.
    Dotted,
    /// Dashed line.
    ///
    /// The shorthand unit `Dashed!` converts into this.
    Dashed,

    /// Faux shadow with carved appearance.
    ///
    /// The shorthand unit `Groove!` converts into this.
    Groove,
    /// Faux shadow with extruded appearance.
    ///
    /// The shorthand unit `Ridge!` converts into this.
    Ridge,

    /// A wavy line, like an error underline.
    ///
    /// The wave magnitude is defined by the overall line thickness, the associated value
    /// here defines the thickness of the wavy line.
    ///
    /// The shorthand `Wavy!` converts into this with `1.0` wavy line thickness.
    Wavy(f32),

    /// Fully transparent line.
    ///
    /// Note that the line space is still reserved, this is will have the same effect as `Solid` with a fully
    /// transparent color.
    ///
    /// The shorthand unit `Hidden!` converts into this.
    Hidden,
}
impl fmt::Debug for LineStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "LineStyle::")?;
        }
        match self {
            LineStyle::Solid => write!(f, "Solid"),
            LineStyle::Double => write!(f, "Double"),
            LineStyle::Dotted => write!(f, "Dotted"),
            LineStyle::Dashed => write!(f, "Dashed"),
            LineStyle::Groove => write!(f, "Groove"),
            LineStyle::Ridge => write!(f, "Ridge"),
            LineStyle::Wavy(t) => write!(f, "Wavy({t})"),
            LineStyle::Hidden => write!(f, "Hidden"),
        }
    }
}
impl Transitionable for LineStyle {
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        match (self, *to) {
            (Self::Wavy(a), Self::Wavy(b)) => Self::Wavy(a.lerp(&b, step)),
            (a, b) => {
                if step < 1.fct() {
                    a
                } else {
                    b
                }
            }
        }
    }
}
impl_from_and_into_var! {
    fn from(_: ShorthandUnit![Solid]) -> LineStyle {
        LineStyle::Solid
    }
    fn from(_: ShorthandUnit![Double]) -> LineStyle {
        LineStyle::Double
    }
    fn from(_: ShorthandUnit![Dotted]) -> LineStyle {
        LineStyle::Dotted
    }
    fn from(_: ShorthandUnit![Dashed]) -> LineStyle {
        LineStyle::Dashed
    }
    fn from(_: ShorthandUnit![Groove]) -> LineStyle {
        LineStyle::Groove
    }
    fn from(_: ShorthandUnit![Ridge]) -> LineStyle {
        LineStyle::Ridge
    }
    fn from(_: ShorthandUnit![Wavy]) -> LineStyle {
        LineStyle::Wavy(1.0)
    }
    fn from(_: ShorthandUnit![Hidden]) -> LineStyle {
        LineStyle::Hidden
    }
}

/// The line style for the sides of a widget's border.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Hash, Eq, serde::Serialize, serde::Deserialize)]
pub enum BorderStyle {
    /// Displays a single, straight, solid line.
    ///
    /// The shorthand `Solid!` converts into this.
    Solid = 1,
    /// Displays two straight lines that add up to the pixel size defined by the side width.
    ///
    /// The shorthand `Double!` converts into this.
    Double = 2,

    /// Displays a series of rounded dots.
    ///
    /// The shorthand `Dotted` converts into this.
    Dotted = 3,
    /// Displays a series of short square-ended dashes or line segments.
    ///
    /// The shorthand `Dashed` converts into this.
    Dashed = 4,

    /// Fully transparent line.
    ///
    /// The shorthand `Hidden` converts into this.
    Hidden = 5,

    /// Displays a border with a carved appearance.
    ///
    /// The shorthand `Groove` converts into this.
    Groove = 6,
    /// Displays a border with an extruded appearance.
    ///
    /// The shorthand `Ridge` converts into this.
    Ridge = 7,

    /// Displays a border that makes the widget appear embedded.
    ///
    /// The shorthand `Inset` converts into this.
    Inset = 8,
    /// Displays a border that makes the widget appear embossed.
    ///
    /// The shorthand `Outset` converts into this.
    Outset = 9,
}
impl fmt::Debug for BorderStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "BorderStyle::")?;
        }
        match self {
            BorderStyle::Solid => write!(f, "Solid"),
            BorderStyle::Double => write!(f, "Double"),
            BorderStyle::Dotted => write!(f, "Dotted"),
            BorderStyle::Dashed => write!(f, "Dashed"),
            BorderStyle::Groove => write!(f, "Groove"),
            BorderStyle::Ridge => write!(f, "Ridge"),
            BorderStyle::Hidden => write!(f, "Hidden"),
            BorderStyle::Inset => write!(f, "Inset"),
            BorderStyle::Outset => write!(f, "Outset"),
        }
    }
}
impl From<BorderStyle> for zng_view_api::BorderStyle {
    fn from(s: BorderStyle) -> Self {
        match s {
            BorderStyle::Solid => zng_view_api::BorderStyle::Solid,
            BorderStyle::Double => zng_view_api::BorderStyle::Double,
            BorderStyle::Dotted => zng_view_api::BorderStyle::Dotted,
            BorderStyle::Dashed => zng_view_api::BorderStyle::Dashed,
            BorderStyle::Hidden => zng_view_api::BorderStyle::Hidden,
            BorderStyle::Groove => zng_view_api::BorderStyle::Groove,
            BorderStyle::Ridge => zng_view_api::BorderStyle::Ridge,
            BorderStyle::Inset => zng_view_api::BorderStyle::Inset,
            BorderStyle::Outset => zng_view_api::BorderStyle::Outset,
        }
    }
}
impl Transitionable for BorderStyle {
    /// Returns `self` for `step < 1.fct()` or `to` for `step >= 1.fct()`.
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        if step < 1.fct() { self } else { *to }
    }
}
impl_from_and_into_var! {
    fn from(_: ShorthandUnit![Solid]) -> BorderStyle {
        BorderStyle::Solid
    }
    fn from(_: ShorthandUnit![Double]) -> BorderStyle {
        BorderStyle::Double
    }
    fn from(_: ShorthandUnit![Dotted]) -> BorderStyle {
        BorderStyle::Dotted
    }
    fn from(_: ShorthandUnit![Dashed]) -> BorderStyle {
        BorderStyle::Dashed
    }
    fn from(_: ShorthandUnit![Groove]) -> BorderStyle {
        BorderStyle::Groove
    }
    fn from(_: ShorthandUnit![Ridge]) -> BorderStyle {
        BorderStyle::Ridge
    }
    fn from(_: ShorthandUnit![Inset]) -> BorderStyle {
        BorderStyle::Inset
    }
    fn from(_: ShorthandUnit![Outset]) -> BorderStyle {
        BorderStyle::Outset
    }
    fn from(_: ShorthandUnit![Hidden]) -> BorderStyle {
        BorderStyle::Hidden
    }
}

/// The line style and color for the sides of a widget's border.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, Transitionable)]
pub struct BorderSide {
    /// Line color.
    pub color: Rgba,
    /// Line style.
    pub style: BorderStyle,
}
impl fmt::Debug for BorderSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("BorderSide")
                .field("color", &self.color)
                .field("style", &self.style)
                .finish()
        } else {
            if let BorderStyle::Hidden = self.style
                && self.color.alpha.abs() < 0.0001
            {
                return write!(f, "Hidden");
            }
            write!(f, "({:?}, {:?})", self.color, self.style)
        }
    }
}
impl BorderSide {
    /// New border side from color and style value.
    pub fn new<C: Into<Rgba>, S: Into<BorderStyle>>(color: C, style: S) -> Self {
        BorderSide {
            color: color.into(),
            style: style.into(),
        }
    }

    /// New border side with [`Solid`](BorderStyle::Solid) style.
    pub fn solid<C: Into<Rgba>>(color: C) -> Self {
        Self::new(color, BorderStyle::Solid)
    }
    /// New border side with [`Double`](BorderStyle::Double) style.
    pub fn double<C: Into<Rgba>>(color: C) -> Self {
        Self::new(color, BorderStyle::Double)
    }

    /// New border side with [`Dotted`](BorderStyle::Dotted) style.
    pub fn dotted<C: Into<Rgba>>(color: C) -> Self {
        Self::new(color, BorderStyle::Dotted)
    }
    /// New border side with [`Dashed`](BorderStyle::Dashed) style.
    pub fn dashed<C: Into<Rgba>>(color: C) -> Self {
        Self::new(color, BorderStyle::Dashed)
    }

    /// New border side with [`Groove`](BorderStyle::Groove) style.
    pub fn groove<C: Into<Rgba>>(color: C) -> Self {
        Self::new(color, BorderStyle::Groove)
    }
    /// New border side with [`Ridge`](BorderStyle::Ridge) style.
    pub fn ridge<C: Into<Rgba>>(color: C) -> Self {
        Self::new(color, BorderStyle::Ridge)
    }

    /// New border side with [`Inset`](BorderStyle::Inset) style.
    pub fn inset<C: Into<Rgba>>(color: C) -> Self {
        Self::new(color, BorderStyle::Inset)
    }

    /// New border side with [`Outset`](BorderStyle::Outset) style.
    pub fn outset<C: Into<Rgba>>(color: C) -> Self {
        Self::new(color, BorderStyle::Outset)
    }

    /// New border side with [`Hidden`](BorderStyle::Hidden) style and transparent color.
    ///
    /// The hidden
    pub fn hidden() -> Self {
        Self::new(colors::BLACK.transparent(), BorderStyle::Hidden)
    }
}
impl From<BorderSide> for zng_view_api::BorderSide {
    fn from(s: BorderSide) -> Self {
        zng_view_api::BorderSide {
            color: s.color,
            style: s.style.into(),
        }
    }
}
impl Default for BorderSide {
    /// Returns [`hidden`](BorderSide::hidden).
    fn default() -> Self {
        Self::hidden()
    }
}

/// Radius of each corner of a border defined from [`Size`] values.
///
/// [`Size`]: zng_layout::unit::Size
#[derive(Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, Transitionable)]
pub struct CornerRadius {
    /// Top-left corner.
    pub top_left: Size,
    /// Top-right corner.
    pub top_right: Size,
    /// Bottom-right corner.
    pub bottom_right: Size,
    /// Bottom-left corner.
    pub bottom_left: Size,
}
impl fmt::Debug for CornerRadius {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("BorderRadius")
                .field("top_left", &self.top_left)
                .field("top_right", &self.top_right)
                .field("bottom_right", &self.bottom_right)
                .field("bottom_left", &self.bottom_left)
                .finish()
        } else if self.all_corners_eq() {
            write!(f, "{:?}", self.top_left)
        } else {
            write!(
                f,
                "({:?}, {:?}, {:?}, {:?})",
                self.top_left, self.top_right, self.bottom_right, self.bottom_left
            )
        }
    }
}
impl CornerRadius {
    /// New every corner unique.
    pub fn new<TL: Into<Size>, TR: Into<Size>, BR: Into<Size>, BL: Into<Size>>(
        top_left: TL,
        top_right: TR,
        bottom_right: BR,
        bottom_left: BL,
    ) -> Self {
        CornerRadius {
            top_left: top_left.into(),
            top_right: top_right.into(),
            bottom_right: bottom_right.into(),
            bottom_left: bottom_left.into(),
        }
    }

    /// New all corners the same.
    pub fn new_all<E: Into<Size>>(ellipse: E) -> Self {
        let e = ellipse.into();
        CornerRadius {
            top_left: e.clone(),
            top_right: e.clone(),
            bottom_left: e.clone(),
            bottom_right: e,
        }
    }

    /// No corner radius.
    pub fn zero() -> Self {
        Self::new_all(Size::zero())
    }

    /// If all corners are the same value.
    pub fn all_corners_eq(&self) -> bool {
        self.top_left == self.top_right && self.top_left == self.bottom_right && self.top_left == self.bottom_left
    }
}
impl Layout2d for CornerRadius {
    type Px = PxCornerRadius;

    fn layout_dft(&self, default: Self::Px) -> Self::Px {
        PxCornerRadius {
            top_left: self.top_left.layout_dft(default.top_left),
            top_right: self.top_right.layout_dft(default.top_right),
            bottom_left: self.bottom_left.layout_dft(default.bottom_left),
            bottom_right: self.bottom_right.layout_dft(default.bottom_right),
        }
    }

    fn affect_mask(&self) -> LayoutMask {
        self.top_left.affect_mask() | self.top_right.affect_mask() | self.bottom_left.affect_mask() | self.bottom_right.affect_mask()
    }
}
impl_from_and_into_var! {
    /// All corners same.
    fn from(all: Size) -> CornerRadius {
        CornerRadius::new_all(all)
    }
    /// All corners same length.
    fn from(all: Length) -> CornerRadius {
        CornerRadius::new_all(all)
    }

    /// All corners same relative length.
    fn from(percent: FactorPercent) -> CornerRadius {
        CornerRadius::new_all(percent)
    }
    /// All corners same relative length.
    fn from(norm: Factor) -> CornerRadius {
        CornerRadius::new_all(norm)
    }

    /// All corners same exact length.
    fn from(f: f32) -> CornerRadius {
        CornerRadius::new_all(f)
    }
    /// All corners same exact length.
    fn from(i: i32) -> CornerRadius {
        CornerRadius::new_all(i)
    }

    /// (top-left, top-right, bottom-left, bottom-right) corners.
    fn from<TL: Into<Size>, TR: Into<Size>, BR: Into<Size>, BL: Into<Size>>(
        (top_left, top_right, bottom_right, bottom_left): (TL, TR, BR, BL),
    ) -> CornerRadius {
        CornerRadius::new(top_left, top_right, bottom_right, bottom_left)
    }

    /// From layout corner-radius.
    fn from(corner_radius: PxCornerRadius) -> CornerRadius {
        CornerRadius::new(
            corner_radius.top_left,
            corner_radius.top_right,
            corner_radius.bottom_right,
            corner_radius.bottom_left,
        )
    }
}

/// The line style and color for each side of a widget's border.
#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, Transitionable)]
pub struct BorderSides {
    /// Color and style of the left border.
    pub left: BorderSide,
    /// Color and style of the right border.
    pub right: BorderSide,

    /// Color and style of the top border.
    pub top: BorderSide,
    /// Color and style of the bottom border.
    pub bottom: BorderSide,
}
impl fmt::Debug for BorderSides {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("BorderSides")
                .field("left", &self.left)
                .field("right", &self.right)
                .field("top", &self.top)
                .field("bottom", &self.bottom)
                .finish()
        } else if self.all_eq() {
            write!(f, "{:?}", self.top)
        } else if self.dimensions_eq() {
            write!(f, "({:?}, {:?})", self.top, self.left)
        } else {
            write!(f, "({:?}, {:?}, {:?}, {:?})", self.top, self.right, self.bottom, self.left)
        }
    }
}
impl BorderSides {
    /// All sides equal.
    pub fn new_all<S: Into<BorderSide>>(side: S) -> Self {
        let side = side.into();
        BorderSides {
            left: side,
            right: side,
            top: side,
            bottom: side,
        }
    }

    /// Top-bottom and left-right equal.
    pub fn new_vh<TB: Into<BorderSide>, LR: Into<BorderSide>>(top_bottom: TB, left_right: LR) -> Self {
        let top_bottom = top_bottom.into();
        let left_right = left_right.into();
        BorderSides {
            left: left_right,
            right: left_right,
            top: top_bottom,
            bottom: top_bottom,
        }
    }

    /// New top, right, bottom left.
    pub fn new<T: Into<BorderSide>, R: Into<BorderSide>, B: Into<BorderSide>, L: Into<BorderSide>>(
        top: T,
        right: R,
        bottom: B,
        left: L,
    ) -> Self {
        BorderSides {
            left: left.into(),
            right: right.into(),
            top: top.into(),
            bottom: bottom.into(),
        }
    }

    /// New top only, other sides hidden.
    pub fn new_top<T: Into<BorderSide>>(top: T) -> Self {
        Self::new(top, BorderSide::hidden(), BorderSide::hidden(), BorderSide::hidden())
    }

    /// New right only, other sides hidden.
    pub fn new_right<R: Into<BorderSide>>(right: R) -> Self {
        Self::new(BorderSide::hidden(), right, BorderSide::hidden(), BorderSide::hidden())
    }

    /// New bottom only, other sides hidden.
    pub fn new_bottom<B: Into<BorderSide>>(bottom: B) -> Self {
        Self::new(BorderSide::hidden(), BorderSide::hidden(), bottom, BorderSide::hidden())
    }

    /// New left only, other sides hidden.
    pub fn new_left<L: Into<BorderSide>>(left: L) -> Self {
        Self::new(BorderSide::hidden(), BorderSide::hidden(), BorderSide::hidden(), left)
    }

    /// All sides a solid color.
    pub fn solid<C: Into<Rgba>>(color: C) -> Self {
        Self::new_all(BorderSide::solid(color))
    }
    /// All sides a double line solid color.
    pub fn double<C: Into<Rgba>>(color: C) -> Self {
        Self::new_all(BorderSide::double(color))
    }

    /// All sides a dotted color.
    pub fn dotted<C: Into<Rgba>>(color: C) -> Self {
        Self::new_all(BorderSide::dotted(color))
    }
    /// All sides a dashed color.
    pub fn dashed<C: Into<Rgba>>(color: C) -> Self {
        Self::new_all(BorderSide::dashed(color))
    }

    /// All sides a grooved color.
    pub fn groove<C: Into<Rgba>>(color: C) -> Self {
        Self::new_all(BorderSide::groove(color))
    }
    /// All sides a ridged color.
    pub fn ridge<C: Into<Rgba>>(color: C) -> Self {
        Self::new_all(BorderSide::ridge(color))
    }

    /// All sides a inset color.
    pub fn inset<C: Into<Rgba>>(color: C) -> Self {
        Self::new_all(BorderSide::inset(color))
    }
    /// All sides a outset color.
    pub fn outset<C: Into<Rgba>>(color: C) -> Self {
        Self::new_all(BorderSide::outset(color))
    }

    /// All sides hidden.
    ///
    /// The shorthand unit `hidden!` converts to this.
    pub fn hidden() -> Self {
        Self::new_all(BorderSide::hidden())
    }

    /// If all sides are equal.
    pub fn all_eq(&self) -> bool {
        self.top == self.bottom && self.top == self.left && self.top == self.right
    }

    /// If top and bottom are equal; and left and right are equal.
    pub fn dimensions_eq(&self) -> bool {
        self.top == self.bottom && self.left == self.right
    }
}
impl Default for BorderSides {
    /// Returns [`hidden`](BorderSides::hidden).
    fn default() -> Self {
        Self::hidden()
    }
}

impl_from_and_into_var! {
    /// Solid color.
    fn from(color: Rgba) -> BorderSide {
        BorderSide::solid(color)
    }
    /// Solid color.
    fn from(color: Hsva) -> BorderSide {
        BorderSide::solid(color)
    }
    /// Solid color.
    fn from(color: Hsla) -> BorderSide {
        BorderSide::solid(color)
    }
    /// All sides solid color.
    fn from(color: Rgba) -> BorderSides {
        BorderSides::new_all(color)
    }
    /// All sides solid color.
    fn from(color: Hsva) -> BorderSides {
        BorderSides::new_all(color)
    }
    /// All sides solid color.
    fn from(color: Hsla) -> BorderSides {
        BorderSides::new_all(color)
    }

    /// Side transparent black with the style.
    ///
    /// This is only useful with [`BorderStyle::Hidden`] variant.
    fn from(style: BorderStyle) -> BorderSide {
        BorderSide::new(colors::BLACK.transparent(), style)
    }
    /// All sides transparent black with the style.
    ///
    /// This is only useful with [`BorderStyle::Hidden`] variant.
    fn from(style: BorderStyle) -> BorderSides {
        BorderSides::new_all(style)
    }

    /// (color, style) side.
    fn from<C: Into<Rgba>, S: Into<BorderStyle>>((color, style): (C, S)) -> BorderSide {
        BorderSide::new(color, style)
    }

    /// (color, style) sides.
    fn from<C: Into<Rgba>, S: Into<BorderStyle>>((color, style): (C, S)) -> BorderSides {
        BorderSides::new_all(BorderSide::new(color, style))
    }

    /// (top, right, bottom, left) sides.
    fn from<T: Into<BorderSide>, R: Into<BorderSide>, B: Into<BorderSide>, L: Into<BorderSide>>(
        (top, right, bottom, left): (T, R, B, L),
    ) -> BorderSides {
        BorderSides::new(top, right, bottom, left)
    }

    /// (top-bottom-color, left-right-color, style) sides.
    fn from<TB: Into<Rgba>, LR: Into<Rgba>, S: Into<BorderStyle>>((top_bottom, left_right, style): (TB, LR, S)) -> BorderSides {
        let style = style.into();
        BorderSides::new_vh((top_bottom, style), (left_right, style))
    }

    /// (top-color, right-color, bottom-color, left-color, style) sides.
    fn from<T: Into<Rgba>, R: Into<Rgba>, B: Into<Rgba>, L: Into<Rgba>, S: Into<BorderStyle>>(
        (top, right, bottom, left, style): (T, R, B, L, S),
    ) -> BorderSides {
        let style = style.into();
        BorderSides::new((top, style), (right, style), (bottom, style), (left, style))
    }

    fn from(_: ShorthandUnit![hidden]) -> BorderSides {
        BorderSides::hidden()
    }
}

/// Defines how the corner radius is computed for each usage.
///
/// Nesting borders with round corners need slightly different radius values to perfectly fit, the [`BORDER`]
/// coordinator can adjusts the radius inside each border to match the inside curve of the border, this behavior is
/// controlled by corner radius fit.
#[derive(Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CornerRadiusFit {
    /// Corner radius is computed for each usage.
    None,
    /// Corner radius is computed for the first usage in the widget, other usages are [deflated] by the widget border offsets.
    ///
    /// [deflated]: PxCornerRadius::deflate
    Widget,
    /// Corner radius is computed on the first usage in the window, other usages are [deflated] by the widget border offsets.
    ///
    /// This is the default value.
    ///
    /// [deflated]: PxCornerRadius::deflate
    #[default]
    Tree,
}
impl fmt::Debug for CornerRadiusFit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "CornerRadiusFit::")?;
        }
        match self {
            Self::None => write!(f, "None"),
            Self::Widget => write!(f, "Widget"),
            Self::Tree => write!(f, "Tree"),
        }
    }
}

context_var! {
    /// How much a widget's border offsets affects the widget's fill content.
    pub static BORDER_ALIGN_VAR: FactorSideOffsets = FactorSideOffsets::zero();

    /// If the border is rendered over the child nodes.
    pub static BORDER_OVER_VAR: bool = true;

    /// Corner radius.
    pub static CORNER_RADIUS_VAR: CornerRadius = CornerRadius::zero();

    /// Corner radius fit.
    pub static CORNER_RADIUS_FIT_VAR: CornerRadiusFit = CornerRadiusFit::default();
}

/// Coordinates nested borders and corner-radius.
pub struct BORDER;
impl BORDER {
    /// Gets the accumulated border offsets on the outside of the current border set on the current widget.
    ///
    /// This is only valid to call during layout.
    pub fn border_offsets(&self) -> PxSideOffsets {
        let data = BORDER_DATA.get();
        if data.widget_id == WIDGET.try_id() {
            data.wgt_offsets
        } else {
            PxSideOffsets::zero()
        }
    }

    /// Gets the accumulated border offsets including the current border.
    pub fn inner_offsets(&self) -> PxSideOffsets {
        let data = BORDER_DATA.get();
        if data.widget_id == WIDGET.try_id() {
            data.wgt_inner_offsets
        } else {
            PxSideOffsets::zero()
        }
    }

    /// Gets the corner radius for the border at the current context.
    ///
    /// This value is influenced by [`CORNER_RADIUS_VAR`], [`CORNER_RADIUS_FIT_VAR`] and all contextual borders.
    pub fn border_radius(&self) -> PxCornerRadius {
        match CORNER_RADIUS_FIT_VAR.get() {
            CornerRadiusFit::Tree => BORDER_DATA.get().border_radius(),
            CornerRadiusFit::Widget => {
                let data = BORDER_DATA.get();
                if data.widget_id == Some(WIDGET.id()) {
                    data.border_radius()
                } else {
                    CORNER_RADIUS_VAR.layout()
                }
            }
            _ => CORNER_RADIUS_VAR.layout(),
        }
    }

    /// Gets the corner radius for the inside of the current border at the current context.
    pub fn inner_radius(&self) -> PxCornerRadius {
        match CORNER_RADIUS_FIT_VAR.get() {
            CornerRadiusFit::Tree => BORDER_DATA.get().inner_radius(),
            CornerRadiusFit::Widget => {
                let data = BORDER_DATA.get();
                if data.widget_id == WIDGET.try_id() {
                    data.inner_radius()
                } else {
                    CORNER_RADIUS_VAR.layout()
                }
            }
            _ => CORNER_RADIUS_VAR.layout(),
        }
    }

    /// Gets the corner radius for the outside of the outer border of the current widget.
    pub fn outer_radius(&self) -> PxCornerRadius {
        BORDER_DATA.get().corner_radius
    }

    /// Gets the bounds and corner radius for the widget fill content.
    ///
    /// Must be called during layout in FILL nesting group.
    ///
    /// This value is influenced by [`CORNER_RADIUS_VAR`], [`CORNER_RADIUS_FIT_VAR`] and [`BORDER_ALIGN_VAR`].
    pub fn fill_bounds(&self) -> (PxRect, PxCornerRadius) {
        let align = BORDER_ALIGN_VAR.get();

        let fill_size = LAYOUT.constraints().fill_size();
        let inner_offsets = self.inner_offsets();

        if align == FactorSideOffsets::zero() {
            let fill_size = PxSize::new(
                fill_size.width + inner_offsets.horizontal(),
                fill_size.height + inner_offsets.vertical(),
            );
            return (PxRect::from_size(fill_size), self.outer_radius());
        } else if align == FactorSideOffsets::new_all(1.0.fct()) {
            return (
                PxRect::new(PxPoint::new(inner_offsets.left, inner_offsets.top), fill_size),
                self.inner_radius(),
            );
        }

        let outer = self.outer_radius();
        let inner = self.inner_radius();

        let b_align = FactorSideOffsets {
            top: 1.0.fct() - align.top,
            right: 1.0.fct() - align.right,
            bottom: 1.0.fct() - align.bottom,
            left: 1.0.fct() - align.left,
        };
        let bounds = PxRect {
            origin: PxPoint::new(inner_offsets.left * (align.left), inner_offsets.top * align.top),
            size: PxSize::new(
                fill_size.width + inner_offsets.left * b_align.left + inner_offsets.right * b_align.right,
                fill_size.height + inner_offsets.top * b_align.top + inner_offsets.bottom * b_align.bottom,
            ),
        };

        let radius = PxCornerRadius {
            top_left: PxSize::new(
                outer.top_left.width.lerp(&inner.top_left.width, align.left),
                outer.top_left.height.lerp(&inner.top_left.height, align.top),
            ),
            top_right: PxSize::new(
                outer.top_right.width.lerp(&inner.top_right.width, align.right),
                outer.top_right.height.lerp(&inner.top_right.height, align.top),
            ),
            bottom_left: PxSize::new(
                outer.bottom_left.width.lerp(&inner.bottom_left.width, align.left),
                outer.bottom_left.height.lerp(&inner.bottom_left.height, align.bottom),
            ),
            bottom_right: PxSize::new(
                outer.bottom_right.width.lerp(&inner.bottom_right.width, align.right),
                outer.bottom_right.height.lerp(&inner.bottom_right.height, align.bottom),
            ),
        };

        (bounds, radius)
    }

    pub(super) fn with_inner(&self, f: impl FnOnce() -> PxSize) -> PxSize {
        let mut data = BORDER_DATA.get_clone();
        let border = WIDGET.border();
        data.add_inner(&border);

        BORDER_DATA.with_context(&mut Some(Arc::new(data)), || {
            let corner_radius = BORDER.border_radius();
            border.set_corner_radius(corner_radius);
            border.set_offsets(PxSideOffsets::zero());
            f()
        })
    }

    /// Measure a border node, adding the `offsets` to the context for the `f` call.
    pub fn measure_border(&self, offsets: PxSideOffsets, f: impl FnOnce() -> PxSize) -> PxSize {
        let mut data = BORDER_DATA.get_clone();
        data.add_offset(None, offsets);
        BORDER_DATA.with_context(&mut Some(Arc::new(data)), f)
    }

    /// Measure a border node, adding the `offsets` to the context for the `f` call.
    pub fn layout_border(&self, offsets: PxSideOffsets, f: impl FnOnce()) {
        let mut data = BORDER_DATA.get_clone();
        data.add_offset(Some(&WIDGET.border()), offsets);
        BORDER_DATA.with_context(&mut Some(Arc::new(data)), f);
    }

    /// Indicates a boundary point where the [`CORNER_RADIUS_VAR`] backing context changes during layout.
    ///
    /// The variable must have been just rebound before this call, the `corner_radius` property implements this method.
    ///
    /// Note that the corner radius is not set during [`measure`].
    ///
    /// [`measure`]: crate::widget::node::UiNode::measure
    pub fn with_corner_radius<R>(&self, f: impl FnOnce() -> R) -> R {
        let mut data = BORDER_DATA.get_clone();
        data.set_corner_radius();
        BORDER_DATA.with_context(&mut Some(Arc::new(data)), f)
    }

    /// Gets the computed border rect and side offsets for the border visual.
    ///
    /// This is only valid to call in the border visual node during layout and render.
    pub fn border_layout(&self) -> (PxRect, PxSideOffsets) {
        BORDER_LAYOUT.get().unwrap_or_else(|| {
            #[cfg(debug_assertions)]
            tracing::error!("the `border_layout` is only available inside the layout and render methods of the border visual node");
            (PxRect::zero(), PxSideOffsets::zero())
        })
    }

    /// Sets the border layout for the context of `f`.
    pub fn with_border_layout(&self, rect: PxRect, offsets: PxSideOffsets, f: impl FnOnce()) {
        BORDER_LAYOUT.with_context(&mut Some(Arc::new(Some((rect, offsets)))), f)
    }
}

context_local! {
    static BORDER_DATA: BorderOffsetsData = BorderOffsetsData::default();
    static BORDER_LAYOUT: Option<(PxRect, PxSideOffsets)> = None;
}

#[derive(Debug, Clone, Default)]
struct BorderOffsetsData {
    widget_id: Option<WidgetId>,
    wgt_offsets: PxSideOffsets,
    wgt_inner_offsets: PxSideOffsets,

    eval_cr: bool,
    corner_radius: PxCornerRadius,
    cr_offsets: PxSideOffsets,
    cr_inner_offsets: PxSideOffsets,
}
impl BorderOffsetsData {
    /// Adds to the widget offsets, or start a new one.
    ///
    /// Computes a new `corner_radius` if fit is Widget and is in a new one.
    fn add_offset(&mut self, layout_info: Option<&WidgetBorderInfo>, offset: PxSideOffsets) {
        let widget_id = Some(WIDGET.id());
        let is_wgt_start = self.widget_id != widget_id;
        if is_wgt_start {
            // changed widget, reset offsets, and maybe corner-radius too.
            self.widget_id = widget_id;
            self.wgt_offsets = PxSideOffsets::zero();
            self.wgt_inner_offsets = PxSideOffsets::zero();
            self.eval_cr |= layout_info.is_some() && matches!(CORNER_RADIUS_FIT_VAR.get(), CornerRadiusFit::Widget);
        }
        self.wgt_offsets = self.wgt_inner_offsets;
        self.wgt_inner_offsets += offset;

        if mem::take(&mut self.eval_cr) {
            self.corner_radius = CORNER_RADIUS_VAR.layout();
            self.cr_offsets = PxSideOffsets::zero();
            self.cr_inner_offsets = PxSideOffsets::zero();
        }
        self.cr_offsets = self.cr_inner_offsets;
        self.cr_inner_offsets += offset;

        if let Some(border) = layout_info {
            if is_wgt_start {
                border.set_corner_radius(self.corner_radius);
            }
            border.set_offsets(self.wgt_inner_offsets);
        }
    }

    fn add_inner(&mut self, layout_info: &WidgetBorderInfo) {
        // ensure at least one "border" so that we have an up-to-date corner radius.
        self.add_offset(Some(layout_info), PxSideOffsets::zero());
    }

    fn set_corner_radius(&mut self) {
        self.eval_cr = matches!(CORNER_RADIUS_FIT_VAR.get(), CornerRadiusFit::Tree);
    }

    fn border_radius(&self) -> PxCornerRadius {
        self.corner_radius.deflate(self.cr_offsets)
    }

    fn inner_radius(&self) -> PxCornerRadius {
        self.corner_radius.deflate(self.cr_inner_offsets)
    }
}
