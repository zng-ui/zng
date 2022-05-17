//! Border and line types.

use std::{fmt, mem};

use crate::context::RenderContext;
use crate::render::{webrender_api as w_api, FrameBuilder, FrameUpdate};
use crate::{nodes, UiNodeList, WidgetId};

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
    pub fn layout(&self, ctx: &LayoutMetrics, mut default_value: impl FnMut(&LayoutMetrics) -> PxCornerRadius) -> PxCornerRadius {
        PxCornerRadius {
            top_left: self.top_left.layout(ctx, |ctx| default_value(ctx).top_left),
            top_right: self.top_right.layout(ctx, |ctx| default_value(ctx).top_right),
            bottom_left: self.bottom_left.layout(ctx, |ctx| default_value(ctx).bottom_left),
            bottom_right: self.bottom_right.layout(ctx, |ctx| default_value(ctx).bottom_right),
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

/// Defines how the [`corner_radius`] is computed for each usage.
///
/// Nesting borders with round corners need slightly different radius values to perfectly fit, the [`ContextBorders`]
/// coordinator can adjusts the radius inside each border to match the inside curve of the border, this behavior is
/// controlled by [`corner_radius_fit`].
#[derive(Clone, Copy)]
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
    Tree,
}
impl Default for CornerRadiusFit {
    fn default() -> Self {
        CornerRadiusFit::Tree
    }
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
#[property(context, default(CornerRadiusVar))]
pub fn corner_radius(child: impl UiNode, radius: impl IntoVar<CornerRadius>) -> impl UiNode {
    struct CornerRadiusNode<C> {
        child: C,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for CornerRadiusNode<C> {
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            ContextBorders::with_corner_radius(ctx.vars, || self.child.layout(ctx, wl))
        }
    }
    with_context_var(CornerRadiusNode { child }, CornerRadiusVar, radius)
}

/// Defines how the [`corner_radius`] is computed for each usage.
///
/// Nesting borders with round corners need slightly different radius values to perfectly fit, the [`ContextBorders`]
/// coordinator can adjusts the radius inside each border to match the inside curve of the border.
#[property(context, default(CornerRadiusFitVar))]
pub fn corner_radius_fit(child: impl UiNode, fit: impl IntoVar<CornerRadiusFit>) -> impl UiNode {
    with_context_var(child, CornerRadiusFitVar, fit)
}

/// Position of an widget borders in relation to the widget fill.
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

    /// Corner radius.
    ///
    /// See [`corner_radius`] for more details.
    pub struct CornerRadiusVar: CornerRadius = CornerRadius::zero();

    /// Corner radius fit.
    ///
    /// See [`corner_radius_fit`] for more details.
    pub struct CornerRadiusFitVar: CornerRadiusFit = CornerRadiusFit::default();
}

/// Transforms and clips the `content` node according with the default widget border behavior.
///
/// Properties that *fill* the widget can wrap their fill content in this node to automatically implement
/// the expected behavior of interaction with the widget borders, the content will positioned, sized and clipped according to the
/// widget borders, [`corner_radius`] and [`border_align`].
///
/// Note that this node should **not** be used for the property child node (first argument), only other
/// content that fills the widget, for examples, a *background* property would wrap its background node with this
/// but just pass thought layout and render for its child node.
///
/// [`corner_radius`]: fn@corner_radius
/// [`border_align`]: fn@border_align
pub fn fill_node(content: impl UiNode) -> impl UiNode {
    struct FillNodeNode<C> {
        content: C,

        clip_bounds: PxSize,
        clip_corners: PxCornerRadius,
    }
    #[impl_ui_node(
        delegate = &self.content,
        delegate_mut = &mut self.content,
    )]
    impl<C: UiNode> UiNode for FillNodeNode<C> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &BorderAlignVar::new());
            self.content.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if BorderAlignVar::is_new(ctx) {
                ctx.updates.layout();
            }
            self.content.update(ctx);
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            // We are inside the *inner* bounds AND inside border_nodes:
            //
            // .. ( layout ( new_border/inner ( border_nodes ( FILL_NODES ( new_child_context ( new_child_layout ( ..
            //
            // `wl` is targeting the `content` transform, potentially a new *child* transform, because we wrap this node with `child_layout`,
            // so we don't need to render the transform, only the clip, but we do need to compute the inverse offset and size.

            let offsets = ContextBorders::border_offsets(ctx.path.widget_id(), ctx.vars);
            let align = BorderAlignVar::get_clone(ctx.vars);

            let our_offsets = offsets * align;
            wl.translate(PxVector::new(our_offsets.left, our_offsets.top));

            let size_offset = offsets - our_offsets;
            let size_increase = PxSize::new(size_offset.horizontal(), size_offset.vertical());
            let fill_bounds = ctx.constrains().fill_size() + size_increase;
            let corners = ContextBorders::corner_radius(ctx).deflate(our_offsets);

            if self.clip_bounds != fill_bounds || self.clip_corners != corners {
                self.clip_bounds = fill_bounds;
                self.clip_corners = corners;
                ctx.updates.render();
            }

            ctx.with_constrains(|_| PxSizeConstrains::exact(fill_bounds), |ctx| self.content.layout(ctx, wl));

            fill_bounds
        }
        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let bounds = PxRect::from_size(self.clip_bounds);

            if self.clip_corners != PxCornerRadius::zero() {
                frame.push_clip_rounded_rect(bounds, self.clip_corners, false, |f| self.content.render(ctx, f))
            } else {
                frame.push_clip_rect(bounds, |f| self.content.render(ctx, f))
            }
        }
    }

    let node = FillNodeNode {
        content: content.cfg_boxed(),
        clip_bounds: PxSize::zero(),
        clip_corners: PxCornerRadius::zero(),
    }
    .cfg_boxed();

    // if content is a `Widget` we use its outer-transform, otherwise a new reference-frame is introduced here.
    crate::widget_base::implicit_base::nodes::child_layout(node)
}

/// Creates a border node that delegates rendering to a `border_visual`, but manages the `border_offsets` coordinating
/// with the other borders of the widget.
pub fn border_node(child: impl UiNode, border_offsets: impl IntoVar<SideOffsets>, border_visual: impl UiNode) -> impl UiNode {
    struct BorderNode<C, O> {
        children: C,
        offsets: O,
        render_offsets: PxSideOffsets,

        border_rect: PxRect,
    }
    #[impl_ui_node(children)]
    impl<C: UiNodeList, O: Var<SideOffsets>> UiNode for BorderNode<C, O> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.offsets.is_new(ctx) {
                ctx.updates.layout();
            }
            self.children.update_all(ctx, &mut ());
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            // We are inside the *inner* bounds or inside a parent border_node:
            //
            // .. ( layout ( new_border/inner ( BORDER_NODES ( fill_nodes ( new_child_context ( new_child_layout ( ..
            //
            // `wl` is targeting the child transform, child nodes are naturally inside borders, so we
            // need to add to the offset and take the size, fill_nodes optionally cancel this transform.

            let offsets = self.offsets.get(ctx.vars).layout(ctx.metrics, |_| PxSideOffsets::zero());
            if self.render_offsets != offsets {
                self.render_offsets = offsets;
                ctx.updates.render();
            }

            let parent_offsets = ContextBorders::border_offsets(ctx.path.widget_id(), ctx.vars);
            let origin = PxPoint::new(parent_offsets.left, parent_offsets.top);
            if self.border_rect.origin != origin {
                self.border_rect.origin = origin;
                ctx.updates.render();
            }

            // layout child and border size
            ContextBorders::with_border(ctx, offsets, |ctx| {
                wl.translate(PxVector::new(offsets.left, offsets.top));

                let taken_size = PxSize::new(offsets.horizontal(), offsets.vertical());
                self.border_rect.size = ctx.with_sub_size(taken_size, |ctx| self.children.widget_layout(0, ctx, wl));

                // layout border visual
                ctx.with_constrains(
                    |_| PxSizeConstrains::exact(self.border_rect.size),
                    |ctx| {
                        ContextBorders::with_border_layout(ctx.vars, self.border_rect, offsets, || {
                            self.children.widget_layout(1, ctx, wl);
                        });
                    },
                );
            });

            self.border_rect.size
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.children.widget_render(0, ctx, frame);
            ContextBorders::with_border_layout(ctx.vars, self.border_rect, self.render_offsets, || {
                self.children.widget_render(1, ctx, frame);
            });
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            self.children.widget_render_update(0, ctx, update);

            ContextBorders::with_border_layout(ctx.vars, self.border_rect, self.render_offsets, || {
                self.children.widget_render_update(1, ctx, update);
            })
        }
    }
    BorderNode {
        children: nodes![child, border_visual],
        offsets: border_offsets.into_var(),
        render_offsets: PxSideOffsets::zero(),
        border_rect: PxRect::zero(),
    }
    .cfg_boxed()
}

/// Coordinates nested borders and corner-radius.
pub struct ContextBorders {}
impl ContextBorders {
    /// Gets the accumulated border offsets set on the current widget.
    ///
    /// This is only valid to call during layout.
    pub fn border_offsets<Vr: WithVarsRead>(widget_id: WidgetId, vars: &Vr) -> PxSideOffsets {
        vars.with_vars_read(|vars| {
            let data = BorderDataVar::get(vars);
            if data.widget_id == Some(widget_id) {
                data.wgt_offsets
            } else {
                PxSideOffsets::zero()
            }
        })
    }

    /// Gets the corner radius at the current context.
    ///
    /// This value is influenced by [`CornerRadiusVar`], [`CornerRadiusFitVar`] and all contextual borders.
    pub fn corner_radius(ctx: &mut LayoutContext) -> PxCornerRadius {
        match CornerRadiusFitVar::get_clone(ctx) {
            CornerRadiusFit::Tree => BorderDataVar::get(ctx.vars).corner_radius(),
            CornerRadiusFit::Widget => {
                let data = BorderDataVar::get(ctx.vars);
                if data.widget_id == Some(ctx.path.widget_id()) {
                    data.corner_radius()
                } else {
                    CornerRadiusVar::get(ctx.vars).layout(ctx.metrics, |_| PxCornerRadius::zero())
                }
            }
            _ => CornerRadiusVar::get(ctx.vars).layout(ctx.metrics, |_| PxCornerRadius::zero()),
        }
    }

    pub(super) fn with_inner(ctx: &mut LayoutContext, f: impl FnOnce(&mut LayoutContext) -> PxSize) -> PxSize {
        let corner_radius = ContextBorders::corner_radius(ctx);
        ctx.widget_info.border.set_corner_radius(corner_radius);
        ctx.widget_info.border.set_offsets(PxSideOffsets::zero());
        f(ctx)
    }

    fn with_border(ctx: &mut LayoutContext, offsets: PxSideOffsets, f: impl FnOnce(&mut LayoutContext)) {
        let mut data = BorderDataVar::get_clone(ctx.vars);
        data.add_offset(ctx, offsets);
        ctx.vars.with_context_var(BorderDataVar, ContextVarData::fixed(&data), || f(ctx));
    }

    fn with_corner_radius<R>(vars: &VarsRead, f: impl FnOnce() -> R) -> R {
        let mut data = BorderDataVar::get_clone(vars);
        data.set_corner_radius(vars);

        vars.with_context_var(BorderDataVar, ContextVarData::fixed(&data), f)
    }

    /// Gets the computed border rect and side offsets for the border visual.
    ///
    /// This is only valid to call in the border visual node (in [`border_node`]) during layout and render.
    ///
    /// [`border_node`]: Self::border_node
    pub fn border_layout<Vr: WithVarsRead>(vars: &Vr) -> (PxRect, PxSideOffsets) {
        BorderLayoutVar::get_clone(vars).unwrap_or_else(|| {
            #[cfg(debug_assertions)]
            tracing::error!("the `border_layout` is only available inside the layout and render methods of the border visual node");
            (PxRect::zero(), PxSideOffsets::zero())
        })
    }
    fn with_border_layout(vars: &VarsRead, rect: PxRect, offsets: PxSideOffsets, f: impl FnOnce()) {
        vars.with_context_var(BorderLayoutVar, ContextVarData::fixed(&Some((rect, offsets))), f);
    }
}

context_var! {
    struct BorderDataVar: BorderOffsetsData = BorderOffsetsData::default();
    struct BorderLayoutVar: Option<(PxRect, PxSideOffsets)> = None;
}

#[derive(Debug, Clone, Default)]
struct BorderOffsetsData {
    widget_id: Option<WidgetId>,
    wgt_offsets: PxSideOffsets,

    eval_cr: bool,
    corner_radius: PxCornerRadius,
    cr_offsets: PxSideOffsets,
}
impl BorderOffsetsData {
    /// Adds to the widget offsets, or start a new one.
    ///
    /// Computes a new `corner_radius` if fit is Widget and is in a new one.
    fn add_offset(&mut self, ctx: &mut LayoutContext, offset: PxSideOffsets) {
        let widget_id = Some(ctx.path.widget_id());
        let is_wgt_start = self.widget_id != widget_id;
        if is_wgt_start {
            // changed widget, reset offsets, and maybe corner-radius too.
            self.widget_id = widget_id;
            self.wgt_offsets = offset;
            self.eval_cr |= matches!(CornerRadiusFitVar::get(ctx.vars), CornerRadiusFit::Widget);
        } else {
            self.wgt_offsets += offset;
            self.cr_offsets += offset;
        }

        if mem::take(&mut self.eval_cr) {
            self.corner_radius = CornerRadiusVar::get(ctx.vars).layout(ctx.metrics, |_| PxCornerRadius::zero());
            self.cr_offsets = PxSideOffsets::zero();
        } else {
            self.cr_offsets += offset;
        }

        if is_wgt_start {
            ctx.widget_info.border.set_corner_radius(self.corner_radius);
        }
        ctx.widget_info.border.set_offsets(self.wgt_offsets);
    }

    fn set_corner_radius(&mut self, vars: &VarsRead) {
        self.eval_cr = matches!(CornerRadiusFitVar::get(vars), CornerRadiusFit::Tree);
    }

    fn corner_radius(&self) -> PxCornerRadius {
        self.corner_radius.deflate(self.cr_offsets)
    }
}
