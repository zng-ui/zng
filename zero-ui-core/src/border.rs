//! Border and line types.

use std::fmt;

use crate::context::RenderContext;
use crate::render::{webrender_api as w_api, FrameBinding, FrameBuilder, SpatialFrameId};

use crate::{
    color::*,
    context::LayoutMetrics,
    context::{InfoContext, LayoutContext, WidgetContext},
    impl_ui_node, property,
    units::*,
    var::impl_from_and_into_var,
    var::*,
    widget_info::{WidgetLayout, WidgetSubscriptions},
    UiNode,
};

/// Orientation of a straight line.
#[derive(Clone, Copy, PartialEq, Eq)]
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
#[derive(Clone, Copy, PartialEq)]
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

/// The line style for the sides of a widget's border.
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Hash, Eq)]
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

/// The line style and color for the sides of a widget's border.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
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
#[derive(Clone, Default, PartialEq)]
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

    /// Compute the radii in a layout context.
    pub fn layout(&self, ctx: &LayoutMetrics, default_value: PxCornerRadius) -> PxCornerRadius {
        PxCornerRadius {
            top_left: self.top_left.layout(ctx, default_value.top_left),
            top_right: self.top_right.layout(ctx, default_value.top_right),
            bottom_left: self.bottom_left.layout(ctx, default_value.bottom_left),
            bottom_right: self.bottom_right.layout(ctx, default_value.bottom_right),
        }
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
    fn from<TL: Into<Size> + Clone, TR: Into<Size> + Clone, BR: Into<Size> + Clone, BL: Into<Size> + Clone>(
        (top_left, top_right, bottom_right, bottom_left): (TL, TR, BR, BL)
    ) -> CornerRadius {
        CornerRadius::new(top_left, top_right, bottom_right, bottom_left)
    }
}

/// The line style and color for each side of a widget's border.
#[derive(Clone, Copy, PartialEq)]
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
    pub fn new_dimension<TB: Into<BorderSide>, LR: Into<BorderSide>>(top_bottom: TB, left_right: LR) -> Self {
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
    fn from<C: Into<Rgba> + Clone, S: Into<BorderStyle> + Clone>((color, style): (C, S)) -> BorderSide {
        BorderSide::new(color, style)
    }

    /// (color, style) sides.
    fn from<C: Into<Rgba> + Clone, S: Into<BorderStyle> + Clone>((color, style): (C, S)) -> BorderSides {
        BorderSides::new_all(BorderSide::new(color, style))
    }

    /// (top, right, bottom, left) sides.
    fn from<T: Into<BorderSide> + Clone, R: Into<BorderSide> + Clone, B: Into<BorderSide> + Clone, L: Into<BorderSide> + Clone>(
        (top, right, bottom, left): (T, R, B, L)
    ) -> BorderSides {
        BorderSides::new(top, right, bottom, left)
    }

    /// (top-bottom-color, left-right-color, style) sides.
    fn from<TB: Into<Rgba> + Clone, LR: Into<Rgba> + Clone, S: Into<BorderStyle> + Clone>((top_bottom, left_right, style): (TB, LR, S)) -> BorderSides {
        let style = style.into();
        BorderSides::new_dimension((top_bottom, style), (left_right, style))
    }

    /// (top-color, right-color, bottom-color, left-color, style) sides.
    fn from<T: Into<Rgba> + Clone, R: Into<Rgba> + Clone, B: Into<Rgba> + Clone, L: Into<Rgba> + Clone, S: Into<BorderStyle> + Clone>(
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

/// Corner radius of widget and inner widgets.
///
/// The [`Default`] value is calculated to fit inside the parent widget corner curve.
///
/// [`Default`]: crate::units::Length::Default
#[property(context, default(CornerRadius::default()))]
pub fn corner_radius(child: impl UiNode, radius: impl IntoVar<CornerRadius>) -> impl UiNode {
    struct CornerRadiusNode<C, R> {
        child: C,
        radius: R,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, R: Var<CornerRadius>> UiNode for CornerRadiusNode<C, R> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.radius);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.radius.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx);
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            wl.with_corner_radius(self.radius.get(ctx.vars), |wl| self.child.layout(ctx, wl))
        }
    }
    CornerRadiusNode {
        child,
        radius: radius.into_var(),
    }
}

/// Position of an widget borders in relation to the widget fill.
///
/// This property defines how much the widget's border offsets affect the layout of the fill content, by default
/// (0%) the fill content stretchers *under* the borders and is clipped by the [`corner_radius`], in the other end
/// of the scale (100%), the fill content is positioned *inside* the borders and clipped by the adjusted corner_radius
/// that fits the insider of the inner most border.
///
/// Note that widget's content is always *inside* the borders, this property only affects the *fill* properties content, such as a
/// the image in a background image.
///
/// Fill property implementers, see [`fill_node`], a helper function for quickly implementing support for `border_align`.
///
/// [`corner_radius`]: fn@corner_radius
#[property(context, default(BorderAlignVar))]
pub fn border_align(child: impl UiNode, align: impl IntoVar<FactorSideOffsets>) -> impl UiNode {
    with_context_var(child, BorderAlignVar, align)
}

context_var! {
    /// How much an widget's border offsets affects the widget's fill content.
    ///
    /// See [`border_align`](fn@border_align) for more details.
    pub struct BorderAlignVar: FactorSideOffsets = FactorSideOffsets::zero();
}

/// Transforms and clips the `content` node according with the default widget border behavior.
///
/// Properties that *fill* the widget can wrap their fill content in this node to automatically implement
/// the expected behavior of interaction with the widget borders, the content will positioned, sized and clipped according to the
/// widget borders, [`corner_radius`] and [`border_align`].
///
/// Note that this node should **not** be used for the a properties child node (first argument), only other
/// content that fills the widget, for examples, a *background* property would wrap its background node with this
/// but just pass thought layout and render for its child node.
///
/// [`corner_radius`]: fn@corner_radius
/// [`border_align`]: fn@border_align
pub fn fill_node(content: impl UiNode) -> impl UiNode {
    struct FillNodeNode<C> {
        child: C,
        offset: PxVector,
        clip: (PxSize, PxCornerRadius),
        spatial_id: SpatialFrameId,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for FillNodeNode<C> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &BorderAlignVar::new());
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if BorderAlignVar::is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx);
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let border_offsets = wl.border_offsets();

            let border_align = *BorderAlignVar::get(ctx);
            let used_offsets = border_offsets * border_align;

            let offset = PxVector::new(border_offsets.left - used_offsets.left, border_offsets.top - used_offsets.top);

            ctx.with_less_available_size(PxSize::new(used_offsets.horizontal(), used_offsets.vertical()), |ctx| {
                let final_size = wl.with_custom_transform(&RenderTransform::translation_px(self.offset), |wl| {
                    self.child.layout(ctx, wl)
                });

                let clip = (final_size, wl.corner_radius().inflate(used_offsets));

                if offset != self.offset || clip != self.clip {
                    self.offset = offset;
                    self.clip = clip;
                    ctx.updates.render();
                }

                final_size
            })
        }
        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let mut clip_render = |frame: &mut FrameBuilder| {
                let (bounds, corners) = self.clip;
                let bounds = PxRect::from_size(bounds);

                if corners != PxCornerRadius::zero() {
                    frame.push_clip_rounded_rect(bounds, corners, false, |f| self.child.render(ctx, f))
                } else {
                    frame.push_clip_rect(bounds, |f| self.child.render(ctx, f))
                }
            };
            if self.offset != PxVector::zero() {
                frame.push_reference_frame(
                    self.spatial_id,
                    FrameBinding::Value(RenderTransform::translation_px(self.offset)),
                    true,
                    clip_render,
                );
            } else {
                clip_render(frame);
            }
        }
    }
    FillNodeNode {
        child: content.cfg_boxed(),
        offset: PxVector::zero(),
        clip: (PxSize::zero(), PxCornerRadius::zero()),
        spatial_id: SpatialFrameId::new_unique(),
    }
    .cfg_boxed()
}
