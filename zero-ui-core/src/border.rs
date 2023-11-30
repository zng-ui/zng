//! Border and line types.

use std::{fmt, mem};

use crate::{
    color::*,
    context::{context_local, LAYOUT, WIDGET},
    property,
    render::{webrender_api as w_api, FrameValueKey},
    ui_vec,
    units::*,
    var::{
        animation::{easing::EasingStep, Transitionable},
        helpers::with_context_var,
        impl_from_and_into_var, *,
    },
    widget_info::WidgetBorderInfo,
    widget_instance::{match_node, match_node_list, UiNode, UiNodeList, UiNodeOp, WidgetId},
};

/// Orientation of a straight line.
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LineOrientation {
    /// Top-bottom line.
    Vertical,
    /// Left-right line.
    Horizontal,
}
impl fmt::Debug for LineOrientation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "LineOrientation::")?;
        }
        match self {
            LineOrientation::Vertical => {
                write!(f, "Vertical")
            }
            LineOrientation::Horizontal => {
                write!(f, "Horizontal")
            }
        }
    }
}
impl From<LineOrientation> for w_api::LineOrientation {
    fn from(o: LineOrientation) -> Self {
        match o {
            LineOrientation::Vertical => w_api::LineOrientation::Vertical,
            LineOrientation::Horizontal => w_api::LineOrientation::Horizontal,
        }
    }
}

/// Represents a line style.
#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum LineStyle {
    /// A solid line.
    Solid,
    /// Two solid lines in parallel.
    Double,

    /// Dotted line.
    Dotted,
    /// Dashed line.
    Dashed,

    /// Faux shadow with carved appearance.
    Groove,
    /// Faux shadow with extruded appearance.
    Ridge,

    /// A wavy line, like an error underline.
    ///
    /// The wave magnitude is defined by the overall line thickness, the associated value
    /// here defines the thickness of the wavy line.
    Wavy(f32),

    /// Fully transparent line.
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

/// The line style for the sides of a widget's border.
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Hash, Eq, serde::Serialize, serde::Deserialize)]
pub enum BorderStyle {
    /// A single straight solid line.
    Solid = 1,
    /// Two straight solid lines that add up to the pixel size defined by the side width.
    Double = 2,

    /// Displays a series of rounded dots.
    Dotted = 3,
    /// Displays a series of short square-ended dashes or line segments.
    Dashed = 4,

    /// Fully transparent line.
    Hidden = 5,

    /// Displays a border with a carved appearance.
    Groove = 6,
    /// Displays a border with an extruded appearance.
    Ridge = 7,

    /// Displays a border that makes the widget appear embedded.
    Inset = 8,
    /// Displays a border that makes the widget appear embossed.
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
impl From<BorderStyle> for w_api::BorderStyle {
    fn from(s: BorderStyle) -> Self {
        match s {
            BorderStyle::Solid => w_api::BorderStyle::Solid,
            BorderStyle::Double => w_api::BorderStyle::Double,
            BorderStyle::Dotted => w_api::BorderStyle::Dotted,
            BorderStyle::Dashed => w_api::BorderStyle::Dashed,
            BorderStyle::Hidden => w_api::BorderStyle::Hidden,
            BorderStyle::Groove => w_api::BorderStyle::Groove,
            BorderStyle::Ridge => w_api::BorderStyle::Ridge,
            BorderStyle::Inset => w_api::BorderStyle::Inset,
            BorderStyle::Outset => w_api::BorderStyle::Outset,
        }
    }
}
impl Transitionable for BorderStyle {
    /// Returns `self` for `step < 1.fct()` or `to` for `step >= 1.fct()`.
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        if step < 1.fct() {
            self
        } else {
            *to
        }
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
            if let BorderStyle::Hidden = self.style {
                if self.color.alpha.abs() < 0.0001 {
                    return write!(f, "Hidden");
                }
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

    /// New border side with [`Solid`](BorderStyle::Dotted) style.
    pub fn dotted<C: Into<Rgba>>(color: C) -> Self {
        Self::new(color, BorderStyle::Dotted)
    }
    /// New border side with [`Solid`](BorderStyle::Dashed) style.
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
    pub fn hidden() -> Self {
        Self::new(colors::BLACK.transparent(), BorderStyle::Hidden)
    }
}
impl From<BorderSide> for w_api::BorderSide {
    fn from(s: BorderSide) -> Self {
        w_api::BorderSide {
            color: s.color.into(),
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
        (top_left, top_right, bottom_right, bottom_left): (TL, TR, BR, BL)
    ) -> CornerRadius {
        CornerRadius::new(top_left, top_right, bottom_right, bottom_left)
    }

    /// From layout corner-radius.
    fn from(corner_radius: PxCornerRadius) -> CornerRadius {
        CornerRadius::new(corner_radius.top_left, corner_radius.top_right, corner_radius.bottom_right, corner_radius.bottom_left)
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
        (top, right, bottom, left): (T, R, B, L)
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
        (top, right, bottom, left, style): (T, R, B, L, S)
    ) -> BorderSides {
        let style = style.into();
        BorderSides::new(
            (top, style),
            (right, style),
            (bottom, style),
            (left, style),
        )
    }
}

/// Defines how the [`corner_radius`] is computed for each usage.
///
/// Nesting borders with round corners need slightly different radius values to perfectly fit, the [`BORDER`]
/// coordinator can adjusts the radius inside each border to match the inside curve of the border, this behavior is
/// controlled by [`corner_radius_fit`].
///
/// [`corner_radius`]: fn@corner_radius
/// [`corner_radius_fit`]: fn@corner_radius_fit
#[derive(Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CornerRadiusFit {
    /// Corner radius is computed for each usage.
    None,
    /// Corner radius is computed for the first usage in a widget, other usages are [deflated] by the widget border offsets.
    ///
    /// [deflated]: PxCornerRadius::deflate
    Widget,
    /// Corner radius is computed on the first usage inside the [`corner_radius`], other usages are [deflated] by the widget border offsets.
    ///
    /// This is the default value.
    ///
    /// [deflated]: PxCornerRadius::deflate
    /// [`corner_radius`]: fn@corner_radius
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

/// Corner radius of widget and inner widgets.
///
/// The [`Default`] value is calculated to fit inside the parent widget corner curve, see [`corner_radius_fit`].
///
/// [`Default`]: crate::units::Length::Default
/// [`corner_radius_fit`]: fn@corner_radius_fit
#[property(CONTEXT, default(CORNER_RADIUS_VAR))]
pub fn corner_radius(child: impl UiNode, radius: impl IntoVar<CornerRadius>) -> impl UiNode {
    let child = match_node(child, move |child, op| {
        if let UiNodeOp::Layout { wl, final_size } = op {
            *final_size = BORDER.with_corner_radius(|| child.layout(wl));
        }
    });
    with_context_var(child, CORNER_RADIUS_VAR, radius)
}

/// Defines how the [`corner_radius`] is computed for each usage.
///
/// Nesting borders with round corners need slightly different radius values to perfectly fit, the [`BORDER`]
/// coordinator can adjusts the radius inside each border to match the inside curve of the border.
///
/// Sets the [`CORNER_RADIUS_FIT_VAR`].
///
/// [`corner_radius`]: fn@corner_radius
#[property(CONTEXT, default(CORNER_RADIUS_FIT_VAR))]
pub fn corner_radius_fit(child: impl UiNode, fit: impl IntoVar<CornerRadiusFit>) -> impl UiNode {
    with_context_var(child, CORNER_RADIUS_FIT_VAR, fit)
}

/// Position of a widget borders in relation to the widget fill.
///
/// This property defines how much the widget's border offsets affect the layout of the fill content, by default
/// (0%) the fill content stretchers *under* the borders and is clipped by the [`corner_radius`], in the other end
/// of the scale (100%), the fill content is positioned *inside* the borders and clipped by the adjusted [`corner_radius`]
/// that fits the insider of the inner most border.
///
/// Note that widget's content is always *inside* the borders, this property only affects the *fill* properties content, such as a
/// the image in a background image.
///
/// Fill property implementers, see [`fill_node`], a helper function for quickly implementing support for `border_align`.
///
/// Sets the [`BORDER_ALIGN_VAR`].
///
/// [`corner_radius`]: fn@corner_radius
#[property(CONTEXT, default(BORDER_ALIGN_VAR))]
pub fn border_align(child: impl UiNode, align: impl IntoVar<FactorSideOffsets>) -> impl UiNode {
    with_context_var(child, BORDER_ALIGN_VAR, align)
}

/// If the border is rendered over the fill and child visuals.
///
/// Is `true` by default, if set to `false` the borders will render under the fill. Note that
/// this means the border will be occluded by the *background* if [`border_align`] is not set to `1.fct()`.
///
/// Sets the [`BORDER_OVER_VAR`].
///
/// [`border_align`]: fn@border_align
#[property(CONTEXT, default(BORDER_OVER_VAR))]
pub fn border_over(child: impl UiNode, over: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, BORDER_OVER_VAR, over)
}

context_var! {
    /// How much a widget's border offsets affects the widget's fill content.
    ///
    /// See [`border_align`](fn@border_align) for more details.
    pub static BORDER_ALIGN_VAR: FactorSideOffsets = FactorSideOffsets::zero();

    /// If the border is rendered over the child nodes.
    ///
    /// See [`border_over`](fn@border_over) for more details.
    pub static BORDER_OVER_VAR: bool = true;

    /// Corner radius.
    ///
    /// See [`corner_radius`](fn@corner_radius) for more details.
    pub static CORNER_RADIUS_VAR: CornerRadius = CornerRadius::zero();

    /// Corner radius fit.
    ///
    /// See [`corner_radius_fit`](fn@corner_radius_fit) for more details.
    pub static CORNER_RADIUS_FIT_VAR: CornerRadiusFit = CornerRadiusFit::default();
}

/// Transforms and clips the `content` node according with the default widget border behavior.
///
/// Properties that *fill* the widget can wrap their fill content in this node to automatically implement
/// the expected behavior of interaction with the widget borders, the content will positioned, sized and clipped according to the
/// widget borders, [`corner_radius`] and [`border_align`]. If the widget is inlined
///
/// Note that this node should **not** be used for the property child node (first argument), only other
/// content that fills the widget, for examples, a *background* property would wrap its background node with this
/// but just pass thought layout and render for its child node.
///
/// [`corner_radius`]: fn@corner_radius
/// [`border_align`]: fn@border_align
pub fn fill_node(content: impl UiNode) -> impl UiNode {
    let mut clip_bounds = PxSize::zero();
    let mut clip_corners = PxCornerRadius::zero();

    let mut offset = PxVector::zero();
    let offset_key = FrameValueKey::new_unique();
    let mut define_frame = false;

    match_node(content, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&BORDER_ALIGN_VAR);
            define_frame = false;
            offset = PxVector::zero();
        }
        UiNodeOp::Measure { desired_size, .. } => {
            let offsets = BORDER.inner_offsets();
            let align = BORDER_ALIGN_VAR.get();

            let our_offsets = offsets * align;
            let size_offset = offsets - our_offsets;

            let size_increase = PxSize::new(size_offset.horizontal(), size_offset.vertical());

            *desired_size = LAYOUT.constraints().fill_size() + size_increase;
        }
        UiNodeOp::Layout { wl, final_size } => {
            // We are inside the *inner* bounds AND inside border_nodes:
            //
            // .. ( layout ( new_border/inner ( border_nodes ( FILL_NODES ( new_child_context ( new_child_layout ( ..

            let (bounds, corners) = BORDER.fill_bounds();

            let mut new_offset = bounds.origin.to_vector();

            if clip_bounds != bounds.size || clip_corners != corners {
                clip_bounds = bounds.size;
                clip_corners = corners;
                WIDGET.render();
            }

            let (_, branch_offset) = LAYOUT.with_constraints(PxConstraints2d::new_exact_size(bounds.size), || {
                wl.with_branch_child(|wl| child.layout(wl))
            });
            new_offset += branch_offset;

            if offset != new_offset {
                offset = new_offset;

                if define_frame {
                    WIDGET.render_update();
                } else {
                    define_frame = true;
                    WIDGET.render();
                }
            }

            *final_size = bounds.size;
        }
        UiNodeOp::Render { frame } => {
            let mut render = |frame: &mut crate::render::FrameBuilder| {
                let bounds = PxRect::from_size(clip_bounds);
                frame.push_clips(
                    |c| {
                        if clip_corners != PxCornerRadius::zero() {
                            c.push_clip_rounded_rect(bounds, clip_corners, false, false);
                        } else {
                            c.push_clip_rect(bounds, false, false);
                        }

                        if let Some(inline) = WIDGET.bounds().inline() {
                            for r in inline.negative_space().iter() {
                                c.push_clip_rect(*r, true, false);
                            }
                        }
                    },
                    |f| child.render(f),
                );
            };

            if define_frame {
                frame.push_reference_frame(offset_key.into(), offset_key.bind(offset.into(), false), true, false, |frame| {
                    render(frame);
                });
            } else {
                render(frame);
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            if define_frame {
                update.with_transform(offset_key.update(offset.into(), false), false, |update| {
                    child.render_update(update);
                });
            } else {
                child.render_update(update);
            }
        }
        _ => {}
    })
}

/// Creates a border node that delegates rendering to a `border_visual`, but manages the `border_offsets` coordinating
/// with the other borders of the widget.
///
/// This node disables inline layout for the widget.
pub fn border_node(child: impl UiNode, border_offsets: impl IntoVar<SideOffsets>, border_visual: impl UiNode) -> impl UiNode {
    let offsets = border_offsets.into_var();
    let mut render_offsets = PxSideOffsets::zero();
    let mut border_rect = PxRect::zero();

    match_node_list(ui_vec![child, border_visual], move |children, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&offsets).sub_var_render(&BORDER_OVER_VAR);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let offsets = offsets.layout();
            *desired_size = BORDER.measure_with_border(offsets, || {
                LAYOUT.with_sub_size(PxSize::new(offsets.horizontal(), offsets.vertical()), || {
                    children.with_node(0, |n| LAYOUT.disable_inline(wm, n))
                })
            });
        }
        UiNodeOp::Layout { wl, final_size } => {
            // We are inside the *inner* bounds or inside a parent border_node:
            //
            // .. ( layout ( new_border/inner ( BORDER_NODES ( fill_nodes ( new_child_context ( new_child_layout ( ..
            //
            // `wl` is targeting the child transform, child nodes are naturally inside borders, so we
            // need to add to the offset and take the size, fill_nodes optionally cancel this transform.

            let offsets = offsets.layout();
            if render_offsets != offsets {
                render_offsets = offsets;
                WIDGET.render();
            }

            let parent_offsets = BORDER.inner_offsets();
            let origin = PxPoint::new(parent_offsets.left, parent_offsets.top);
            if border_rect.origin != origin {
                border_rect.origin = origin;
                WIDGET.render();
            }

            // layout child and border visual
            BORDER.with_border(offsets, || {
                wl.translate(PxVector::new(offsets.left, offsets.top));

                let taken_size = PxSize::new(offsets.horizontal(), offsets.vertical());
                border_rect.size = LAYOUT.with_sub_size(taken_size, || children.with_node(0, |n| n.layout(wl)));

                // layout border visual
                LAYOUT.with_constraints(PxConstraints2d::new_exact_size(border_rect.size), || {
                    BORDER.with_border_layout(border_rect, offsets, || {
                        children.with_node(1, |n| n.layout(wl));
                    });
                });
            });

            *final_size = border_rect.size;
        }
        UiNodeOp::Render { frame } => {
            if BORDER_OVER_VAR.get() {
                children.with_node(0, |c| c.render(frame));
                BORDER.with_border_layout(border_rect, render_offsets, || {
                    children.with_node(1, |c| c.render(frame));
                });
            } else {
                BORDER.with_border_layout(border_rect, render_offsets, || {
                    children.with_node(1, |c| c.render(frame));
                });
                children.with_node(0, |c| c.render(frame));
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            children.with_node(0, |c| c.render_update(update));
            BORDER.with_border_layout(border_rect, render_offsets, || {
                children.with_node(1, |c| c.render_update(update));
            })
        }
        _ => {}
    })
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

        BORDER_DATA.with_context_value(data, || {
            let corner_radius = BORDER.border_radius();
            border.set_corner_radius(corner_radius);
            border.set_offsets(PxSideOffsets::zero());
            f()
        })
    }

    fn with_border(&self, offsets: PxSideOffsets, f: impl FnOnce()) {
        let mut data = BORDER_DATA.get_clone();
        data.add_offset(Some(&WIDGET.border()), offsets);
        BORDER_DATA.with_context_value(data, f);
    }

    fn measure_with_border(&self, offsets: PxSideOffsets, f: impl FnOnce() -> PxSize) -> PxSize {
        let mut data = BORDER_DATA.get_clone();
        data.add_offset(None, offsets);
        BORDER_DATA.with_context_value(data, f)
    }

    /// Indicates a boundary point where the [`CORNER_RADIUS_VAR`] backing context changes during layout.
    ///
    /// The variable must have been just rebound before this call, the [`corner_radius`] property implements this method.
    ///
    /// Note that the corner radius is not set during [`measure`].
    ///
    /// [`corner_radius`]: fn@corner_radius
    /// [`measure`]: UiNode::measure
    pub fn with_corner_radius<R>(&self, f: impl FnOnce() -> R) -> R {
        let mut data = BORDER_DATA.get_clone();
        data.set_corner_radius();
        BORDER_DATA.with_context_value(data, f)
    }

    /// Gets the computed border rect and side offsets for the border visual.
    ///
    /// This is only valid to call in the border visual node (in [`border_node`]) during layout and render.
    pub fn border_layout(&self) -> (PxRect, PxSideOffsets) {
        BORDER_LAYOUT.get().unwrap_or_else(|| {
            #[cfg(debug_assertions)]
            tracing::error!("the `border_layout` is only available inside the layout and render methods of the border visual node");
            (PxRect::zero(), PxSideOffsets::zero())
        })
    }
    fn with_border_layout(&self, rect: PxRect, offsets: PxSideOffsets, f: impl FnOnce()) {
        BORDER_LAYOUT.with_context_value(Some((rect, offsets)), f)
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
            border.set_offsets(self.wgt_offsets);
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
