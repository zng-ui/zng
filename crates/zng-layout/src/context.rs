//! Layout context.

use std::{fmt, sync::Arc};

use bitflags::bitflags;
use unicode_bidi::BidiDataSource as _;
use zng_app_context::context_local;
use zng_unit::{Factor, Px, PxRect, PxSize, about_eq, about_eq_hash, euclid};
use zng_var::context_var;

use atomic::{Atomic, Ordering::Relaxed};

use crate::unit::{LayoutAxis, Ppi, PxConstraints, PxConstraints2d};

/// Current layout context.
///
/// Only available in measure and layout methods.
pub struct LAYOUT;
impl LAYOUT {
    /// Gets the current window layout pass.
    ///
    /// Widgets can be layout more then once per window layout pass, you can use this ID to identify such cases.
    pub fn pass_id(&self) -> LayoutPassId {
        LAYOUT_PASS_CTX.get_clone()
    }

    /// Calls `f` in a new layout pass.
    pub fn with_root_context<R>(&self, pass_id: LayoutPassId, metrics: LayoutMetrics, f: impl FnOnce() -> R) -> R {
        let mut pass = Some(Arc::new(pass_id));
        LAYOUT_PASS_CTX.with_context(&mut pass, || self.with_context(metrics, f))
    }

    /// Calls `f` in a new layout context.
    pub fn with_context<R>(&self, metrics: LayoutMetrics, f: impl FnOnce() -> R) -> R {
        let mut ctx = Some(Arc::new(LayoutCtx { metrics }));
        LAYOUT_CTX.with_context(&mut ctx, f)
    }

    /// Calls `f` without a layout context.
    pub fn with_no_context<R>(&self, f: impl FnOnce() -> R) -> R {
        LAYOUT_CTX.with_default(f)
    }

    /// Gets the context metrics.
    pub fn metrics(&self) -> LayoutMetrics {
        LAYOUT_CTX.get().metrics.clone()
    }

    /// Capture all layout metrics used in `f`.
    ///
    /// Note that the captured mask is not propagated to the current context, you can use [`register_metrics_use`] to propagate
    /// the returned mask.
    ///
    /// [`register_metrics_use`]: Self::register_metrics_use
    pub fn capture_metrics_use<R>(&self, f: impl FnOnce() -> R) -> (LayoutMask, R) {
        METRICS_USED_CTX.with_context(&mut Some(Arc::new(Atomic::new(LayoutMask::empty()))), || {
            let r = f();
            let uses = METRICS_USED_CTX.get().load(Relaxed);
            (uses, r)
        })
    }

    /// Register that the node layout depends on these contextual values.
    ///
    /// Note that the value methods already register by the [`LayoutMetrics`] getter methods.
    pub fn register_metrics_use(&self, uses: LayoutMask) {
        let ctx = METRICS_USED_CTX.get();
        let m = ctx.load(Relaxed);
        ctx.store(m | uses, Relaxed);
    }

    /// Current size constraints.
    pub fn constraints(&self) -> PxConstraints2d {
        LAYOUT_CTX.get().metrics.constraints()
    }

    /// Current perspective constraints.
    pub fn z_constraints(&self) -> PxConstraints {
        LAYOUT_CTX.get().metrics.z_constraints()
    }

    /// Current length constraints for the given axis.
    pub fn constraints_for(&self, axis: LayoutAxis) -> PxConstraints {
        match axis {
            LayoutAxis::X => self.constraints().x,
            LayoutAxis::Y => self.constraints().y,
            LayoutAxis::Z => self.z_constraints(),
        }
    }

    /// Calls `f` with the `constraints` in context.
    pub fn with_constraints<R>(&self, constraints: PxConstraints2d, f: impl FnOnce() -> R) -> R {
        self.with_context(self.metrics().with_constraints(constraints), f)
    }

    /// Calls `f` with the `constraints` for perspective in context.
    pub fn with_z_constraints<R>(&self, constraints: PxConstraints, f: impl FnOnce() -> R) -> R {
        self.with_context(self.metrics().with_z_constraints(constraints), f)
    }

    /// Calls `f` with the `constraints` in context.
    pub fn with_constraints_for<R>(&self, axis: LayoutAxis, constraints: PxConstraints, f: impl FnOnce() -> R) -> R {
        match axis {
            LayoutAxis::X => {
                let mut c = self.constraints();
                c.x = constraints;
                self.with_constraints(c, f)
            }
            LayoutAxis::Y => {
                let mut c = self.constraints();
                c.y = constraints;
                self.with_constraints(c, f)
            }
            LayoutAxis::Z => self.with_z_constraints(constraints, f),
        }
    }

    /// Runs a function `f` in a context that has its max size subtracted by `removed` and its final size added by `removed`.
    pub fn with_sub_size(&self, removed: PxSize, f: impl FnOnce() -> PxSize) -> PxSize {
        self.with_constraints(self.constraints().with_less_size(removed), f) + removed
    }

    /// Runs a function `f` in a layout context that has its max size added by `added` and its final size subtracted by `added`.
    pub fn with_add_size(&self, added: PxSize, f: impl FnOnce() -> PxSize) -> PxSize {
        self.with_constraints(self.constraints().with_more_size(added), f) - added
    }

    /// Current inline constraints.
    pub fn inline_constraints(&self) -> Option<InlineConstraints> {
        LAYOUT_CTX.get().metrics.inline_constraints()
    }

    /// Calls `f` with no inline constraints.
    pub fn with_no_inline<R>(&self, f: impl FnOnce() -> R) -> R {
        let metrics = self.metrics();
        if metrics.inline_constraints().is_none() {
            f()
        } else {
            self.with_context(metrics.with_inline_constraints(None), f)
        }
    }

    /// Root font size.
    pub fn root_font_size(&self) -> Px {
        LAYOUT_CTX.get().metrics.root_font_size()
    }

    /// Current font size.
    pub fn font_size(&self) -> Px {
        LAYOUT_CTX.get().metrics.font_size()
    }

    /// Calls `f` with `font_size` in the context.
    pub fn with_font_size<R>(&self, font_size: Px, f: impl FnOnce() -> R) -> R {
        self.with_context(self.metrics().with_font_size(font_size), f)
    }

    /// Current viewport size.
    pub fn viewport(&self) -> PxSize {
        LAYOUT_CTX.get().metrics.viewport()
    }

    /// Current smallest dimension of the viewport.
    pub fn viewport_min(&self) -> Px {
        LAYOUT_CTX.get().metrics.viewport_min()
    }

    /// Current largest dimension of the viewport.
    pub fn viewport_max(&self) -> Px {
        LAYOUT_CTX.get().metrics.viewport_max()
    }

    /// Current viewport length for the given axis.
    pub fn viewport_for(&self, axis: LayoutAxis) -> Px {
        let vp = self.viewport();
        match axis {
            LayoutAxis::X => vp.width,
            LayoutAxis::Y => vp.height,
            LayoutAxis::Z => Px::MAX,
        }
    }

    /// Calls `f` with `viewport` in the context.
    pub fn with_viewport<R>(&self, viewport: PxSize, f: impl FnOnce() -> R) -> R {
        self.with_context(self.metrics().with_viewport(viewport), f)
    }

    /// Current scale factor.
    pub fn scale_factor(&self) -> Factor {
        LAYOUT_CTX.get().metrics.scale_factor()
    }

    /// Calls `f` with `scale_factor` in the context.
    pub fn with_scale_factor<R>(&self, scale_factor: Factor, f: impl FnOnce() -> R) -> R {
        self.with_context(self.metrics().with_scale_factor(scale_factor), f)
    }

    /// Current screen PPI.
    pub fn screen_ppi(&self) -> Ppi {
        LAYOUT_CTX.get().metrics.screen_ppi()
    }

    /// Calls `f` with `screen_ppi` in the context.
    pub fn with_screen_ppi<R>(&self, screen_ppi: Ppi, f: impl FnOnce() -> R) -> R {
        self.with_context(self.metrics().with_screen_ppi(screen_ppi), f)
    }

    /// Current layout direction.
    pub fn direction(&self) -> LayoutDirection {
        LAYOUT_CTX.get().metrics.direction()
    }

    /// Calls `f` with `direction` in the context.
    pub fn with_direction<R>(&self, direction: LayoutDirection, f: impl FnOnce() -> R) -> R {
        self.with_context(self.metrics().with_direction(direction), f)
    }

    /// Context leftover length for the widget, given the [`Length::Leftover`] value it communicated to the parent.
    ///
    /// [`Length::Leftover`]: crate::unit::Length::Leftover
    pub fn leftover(&self) -> euclid::Size2D<Option<Px>, ()> {
        LAYOUT_CTX.get().metrics.leftover()
    }

    /// Context leftover length for the given axis.
    pub fn leftover_for(&self, axis: LayoutAxis) -> Option<Px> {
        let l = self.leftover();

        match axis {
            LayoutAxis::X => l.width,
            LayoutAxis::Y => l.height,
            LayoutAxis::Z => None,
        }
    }

    /// Calls `f` with [`leftover`] set to `with` and `height`.
    ///
    /// [`leftover`]: Self::leftover
    pub fn with_leftover<R>(&self, width: Option<Px>, height: Option<Px>, f: impl FnOnce() -> R) -> R {
        self.with_context(self.metrics().with_leftover(width, height), f)
    }
}

context_local! {
    static LAYOUT_CTX: LayoutCtx = LayoutCtx::no_context();
    static LAYOUT_PASS_CTX: LayoutPassId = LayoutPassId::new();
    static METRICS_USED_CTX: Atomic<LayoutMask> = Atomic::new(LayoutMask::empty());
}

struct LayoutCtx {
    metrics: LayoutMetrics,
}
impl LayoutCtx {
    fn no_context() -> Self {
        panic!("no layout context")
    }
}

/// Identifies the layout pass of a window.
///
/// This value is different for each window layout, but the same for children of panels that do more then one layout pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LayoutPassId(u32);
impl LayoutPassId {
    /// New default.
    pub const fn new() -> Self {
        LayoutPassId(0)
    }

    /// Gets the next layout pass ID.
    pub const fn next(self) -> LayoutPassId {
        LayoutPassId(self.0.wrapping_add(1))
    }
}

/// Constraints for inline measure.
///
/// See [`InlineConstraints`] for more details.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct InlineConstraintsMeasure {
    /// Available space on the first row.
    pub first_max: Px,
    /// Current height of the row in the parent. If the widget wraps and defines the first
    /// row in *this* parent's row, the `mid_clear` value will be the extra space needed to clear
    /// this minimum or zero if the first row is taller. The widget must use this value to estimate the `mid_clear`
    /// value and include it in the overall measured height of the widget.
    pub mid_clear_min: Px,
}
impl InlineConstraintsMeasure {
    /// New constraint.
    pub fn new(first_max: Px, mid_clear_min: Px) -> Self {
        Self { first_max, mid_clear_min }
    }
}

/// Position of an inline segment set by the inlining parent.
///
/// See [`InlineConstraintsLayout::first_segs`] for more details.
///
/// [`InlineConstraintsLayout::first_segs`]: crate::context::InlineConstraintsLayout::first_segs
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct InlineSegmentPos {
    /// Seg offset to the right from the row origin, in pixels.
    pub x: f32,
}
impl InlineSegmentPos {
    /// New pos.
    pub fn new(x: f32) -> Self {
        Self { x }
    }
}
impl PartialEq for InlineSegmentPos {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.x, other.x, 0.001)
    }
}
impl Eq for InlineSegmentPos {}
impl std::hash::Hash for InlineSegmentPos {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.x, 0.001, state);
    }
}

/// Constraints for inline layout.
///
/// See [`InlineConstraints`] for more details.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct InlineConstraintsLayout {
    /// First row rect, defined by the parent.
    pub first: PxRect,
    /// Extra space in-between the first row and the mid-rows that must be offset to clear the other segments in the row.
    pub mid_clear: Px,
    /// Last row rect, defined by the parent.
    pub last: PxRect,

    /// Position of inline segments of the first row.
    pub first_segs: Arc<Vec<InlineSegmentPos>>,
    /// Position of inline segments of the last row.
    pub last_segs: Arc<Vec<InlineSegmentPos>>,
}

impl InlineConstraintsLayout {
    /// New constraint.
    pub fn new(
        first: PxRect,
        mid_clear: Px,
        last: PxRect,
        first_segs: Arc<Vec<InlineSegmentPos>>,
        last_segs: Arc<Vec<InlineSegmentPos>>,
    ) -> Self {
        Self {
            first,
            mid_clear,
            last,
            first_segs,
            last_segs,
        }
    }
}

/// Constraints for inline measure or layout.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum InlineConstraints {
    /// Constraints for the measure pass.
    Measure(InlineConstraintsMeasure),
    /// Constraints the layout pass.
    Layout(InlineConstraintsLayout),
}
impl InlineConstraints {
    /// Get the `Measure` data or default.
    pub fn measure(self) -> InlineConstraintsMeasure {
        match self {
            InlineConstraints::Measure(m) => m,
            InlineConstraints::Layout(l) => InlineConstraintsMeasure {
                first_max: l.first.width(),
                mid_clear_min: l.mid_clear,
            },
        }
    }

    /// Get the `Layout` data or default.
    pub fn layout(self) -> InlineConstraintsLayout {
        match self {
            InlineConstraints::Layout(m) => m,
            InlineConstraints::Measure(_) => Default::default(),
        }
    }
}

/// Layout metrics snapshot.
///
/// A snapshot can be taken using the [`LayoutMetrics::snapshot`].
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct LayoutMetricsSnapshot {
    /// The [`constraints`].
    ///
    /// [`constraints`]: LayoutMetrics::constraints
    pub constraints: PxConstraints2d,

    /// The [`inline_constraints`].
    ///
    /// [`inline_constraints`]: LayoutMetrics::inline_constraints
    pub inline_constraints: Option<InlineConstraints>,

    /// The [`z_constraints`].
    ///
    /// [`z_constraints`]: LayoutMetrics::z_constraints
    pub z_constraints: PxConstraints,

    /// The [`font_size`].
    ///
    /// [`font_size`]: LayoutMetrics::font_size
    pub font_size: Px,
    /// The [`root_font_size`].
    ///
    /// [`root_font_size`]: LayoutMetrics::root_font_size
    pub root_font_size: Px,
    /// The [`scale_factor`].
    ///
    /// [`scale_factor`]: LayoutMetrics::scale_factor
    pub scale_factor: Factor,
    /// The [`viewport`].
    ///
    /// [`viewport`]: LayoutMetrics::viewport
    pub viewport: PxSize,
    /// The [`screen_ppi`].
    ///
    /// [`screen_ppi`]: LayoutMetrics::screen_ppi
    pub screen_ppi: Ppi,

    /// The [`direction`].
    ///
    /// [`direction`]: LayoutMetrics::direction
    pub direction: LayoutDirection,

    /// The [`leftover`].
    ///
    /// [`leftover`]: LayoutMetrics::leftover
    pub leftover: euclid::Size2D<Option<Px>, ()>,
}
impl LayoutMetricsSnapshot {
    /// Gets if all of the fields in `mask` are equal between `self` and `other`.
    pub fn masked_eq(&self, other: &Self, mask: LayoutMask) -> bool {
        (!mask.contains(LayoutMask::CONSTRAINTS)
            || (self.constraints == other.constraints
                && self.z_constraints == other.z_constraints
                && self.inline_constraints == other.inline_constraints))
            && (!mask.contains(LayoutMask::FONT_SIZE) || self.font_size == other.font_size)
            && (!mask.contains(LayoutMask::ROOT_FONT_SIZE) || self.root_font_size == other.root_font_size)
            && (!mask.contains(LayoutMask::SCALE_FACTOR) || self.scale_factor == other.scale_factor)
            && (!mask.contains(LayoutMask::VIEWPORT) || self.viewport == other.viewport)
            && (!mask.contains(LayoutMask::SCREEN_PPI) || self.screen_ppi == other.screen_ppi)
            && (!mask.contains(LayoutMask::DIRECTION) || self.direction == other.direction)
            && (!mask.contains(LayoutMask::LEFTOVER) || self.leftover == other.leftover)
    }
}
impl PartialEq for LayoutMetricsSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.constraints == other.constraints
            && self.z_constraints == other.z_constraints
            && self.inline_constraints == other.inline_constraints
            && self.font_size == other.font_size
            && self.root_font_size == other.root_font_size
            && self.scale_factor == other.scale_factor
            && self.viewport == other.viewport
            && self.screen_ppi == other.screen_ppi
    }
}
impl std::hash::Hash for LayoutMetricsSnapshot {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.constraints.hash(state);
        self.inline_constraints.hash(state);
        self.font_size.hash(state);
        self.root_font_size.hash(state);
        self.scale_factor.hash(state);
        self.viewport.hash(state);
        self.screen_ppi.hash(state);
    }
}

/// Layout metrics in a [`LAYOUT`] context.
#[derive(Debug, Clone)]
pub struct LayoutMetrics {
    s: LayoutMetricsSnapshot,
}
impl LayoutMetrics {
    /// New root [`LayoutMetrics`].
    ///
    /// The `font_size` sets both font sizes, the initial PPI is `96.0`, you can use the builder style method and
    /// [`with_screen_ppi`] to set a different value.
    ///
    /// [`with_screen_ppi`]: LayoutMetrics::with_screen_ppi
    pub fn new(scale_factor: Factor, viewport: PxSize, font_size: Px) -> Self {
        LayoutMetrics {
            s: LayoutMetricsSnapshot {
                constraints: PxConstraints2d::new_fill_size(viewport),
                z_constraints: PxConstraints::new_unbounded().with_min(Px(1)),
                inline_constraints: None,
                font_size,
                root_font_size: font_size,
                scale_factor,
                viewport,
                screen_ppi: Ppi::default(),
                direction: LayoutDirection::default(),
                leftover: euclid::size2(None, None),
            },
        }
    }

    /// Current size constraints.
    pub fn constraints(&self) -> PxConstraints2d {
        LAYOUT.register_metrics_use(LayoutMask::CONSTRAINTS);
        self.s.constraints
    }

    /// Current perspective constraints.
    pub fn z_constraints(&self) -> PxConstraints {
        LAYOUT.register_metrics_use(LayoutMask::CONSTRAINTS);
        self.s.z_constraints
    }

    /// Current inline constraints.
    ///
    /// Only present if the parent widget supports inline.
    pub fn inline_constraints(&self) -> Option<InlineConstraints> {
        LAYOUT.register_metrics_use(LayoutMask::CONSTRAINTS);
        self.s.inline_constraints.clone()
    }

    /// Gets the inline or text flow direction.
    pub fn direction(&self) -> LayoutDirection {
        LAYOUT.register_metrics_use(LayoutMask::DIRECTION);
        self.s.direction
    }

    /// Current computed font size.
    pub fn font_size(&self) -> Px {
        LAYOUT.register_metrics_use(LayoutMask::FONT_SIZE);
        self.s.font_size
    }

    /// Computed font size at the root widget.
    pub fn root_font_size(&self) -> Px {
        LAYOUT.register_metrics_use(LayoutMask::ROOT_FONT_SIZE);
        self.s.root_font_size
    }

    /// Pixel scale factor.
    pub fn scale_factor(&self) -> Factor {
        LAYOUT.register_metrics_use(LayoutMask::SCALE_FACTOR);
        self.s.scale_factor
    }

    /// Computed size of the nearest viewport ancestor.
    ///
    /// This is usually the window content area size, but can be the scroll viewport size or any other
    /// value depending on the implementation of the context widgets.
    pub fn viewport(&self) -> PxSize {
        LAYOUT.register_metrics_use(LayoutMask::VIEWPORT);
        self.s.viewport
    }

    /// Smallest dimension of the [`viewport`].
    ///
    /// [`viewport`]: Self::viewport
    pub fn viewport_min(&self) -> Px {
        self.s.viewport.width.min(self.s.viewport.height)
    }

    /// Largest dimension of the [`viewport`].
    ///
    /// [`viewport`]: Self::viewport
    pub fn viewport_max(&self) -> Px {
        self.s.viewport.width.max(self.s.viewport.height)
    }

    /// The current screen "pixels-per-inch" resolution.
    ///
    /// This value is dependent in the actual physical size of the screen.
    ///
    /// Default is `96.0`.
    pub fn screen_ppi(&self) -> Ppi {
        self.s.screen_ppi
    }

    /// Computed leftover length for the widget, given the [`Length::Leftover`] value it communicated to the parent.
    ///
    /// [`Length::Leftover`]: crate::unit::Length::Leftover
    pub fn leftover(&self) -> euclid::Size2D<Option<Px>, ()> {
        LAYOUT.register_metrics_use(LayoutMask::LEFTOVER);
        self.s.leftover
    }

    /// Sets the [`constraints`] to `constraints`.
    ///
    /// [`constraints`]: Self::constraints
    pub fn with_constraints(mut self, constraints: PxConstraints2d) -> Self {
        self.s.constraints = constraints;
        self
    }

    /// Sets the [`z_constraints`] to `constraints`.
    ///
    /// [`z_constraints`]: Self::z_constraints
    pub fn with_z_constraints(mut self, constraints: PxConstraints) -> Self {
        self.s.z_constraints = constraints;
        self
    }

    /// Set the [`inline_constraints`].
    ///
    /// [`inline_constraints`]: Self::inline_constraints
    pub fn with_inline_constraints(mut self, inline_constraints: Option<InlineConstraints>) -> Self {
        self.s.inline_constraints = inline_constraints;
        self
    }

    /// Sets the [`font_size`].
    ///
    /// [`font_size`]: Self::font_size
    pub fn with_font_size(mut self, font_size: Px) -> Self {
        self.s.font_size = font_size;
        self
    }

    /// Sets the [`viewport`].
    ///
    /// [`viewport`]: Self::viewport
    pub fn with_viewport(mut self, viewport: PxSize) -> Self {
        self.s.viewport = viewport;
        self
    }

    /// Sets the [`scale_factor`].
    ///
    /// [`scale_factor`]: Self::scale_factor
    pub fn with_scale_factor(mut self, scale_factor: Factor) -> Self {
        self.s.scale_factor = scale_factor;
        self
    }

    /// Sets the [`screen_ppi`].
    ///
    /// [`screen_ppi`]: Self::screen_ppi
    pub fn with_screen_ppi(mut self, screen_ppi: Ppi) -> Self {
        self.s.screen_ppi = screen_ppi;
        self
    }

    /// Sets the [`direction`].
    ///
    /// [`direction`]: Self::direction
    pub fn with_direction(mut self, direction: LayoutDirection) -> Self {
        self.s.direction = direction;
        self
    }

    /// Sets the [`leftover`].
    ///
    /// [`leftover`]: Self::leftover
    pub fn with_leftover(mut self, width: Option<Px>, height: Option<Px>) -> Self {
        self.s.leftover = euclid::size2(width, height);
        self
    }

    /// Clones all current metrics into a [snapshot].
    ///
    /// [snapshot]: LayoutMetricsSnapshot
    pub fn snapshot(&self) -> LayoutMetricsSnapshot {
        self.s.clone()
    }
}

context_var! {
    /// Wrap direction of text in a widget context.
    pub static DIRECTION_VAR: LayoutDirection = LayoutDirection::LTR;
}

/// Defines the layout flow direction.
///
/// This affects inline layout, some [`Align`] options and the base text shaping direction.
///
/// The contextual value can be read during layout in [`LayoutMetrics::direction`], and it can be set using [`LayoutMetrics::with_direction`].
/// Properties that define a more specific *direction* value also set this value, for example, a *TextDirection* property will also set the
/// layout direction.
///
/// Note that this does not affect the layout origin, all points are offsets from the top-left corner independent of this value.
///
/// [`Align`]: crate::unit::Align
#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LayoutDirection {
    /// left-to-right.
    LTR,
    /// Right-to-left.
    RTL,
}
impl LayoutDirection {
    /// Matches `LTR`.
    pub fn is_ltr(self) -> bool {
        matches!(self, Self::LTR)
    }

    /// Matches `RTL`.
    pub fn is_rtl(self) -> bool {
        matches!(self, Self::RTL)
    }
}
impl Default for LayoutDirection {
    /// Default is `LTR`.
    fn default() -> Self {
        Self::LTR
    }
}
impl fmt::Debug for LayoutDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "LayoutDirection::")?;
        }
        match self {
            Self::LTR => write!(f, "LTR"),
            Self::RTL => write!(f, "RTL"),
        }
    }
}

/// Represents a segment in an inlined widget first or last row.
///
/// This info is used by inlining parent to sort the joiner row in a way that preserves bidirectional text flow.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct InlineSegment {
    /// Width of the segment, in pixels.
    pub width: f32,
    /// Info for bidirectional reorder.
    pub kind: TextSegmentKind,
}

impl InlineSegment {
    /// New from width and text kind.
    pub fn new(width: f32, kind: TextSegmentKind) -> Self {
        Self { width, kind }
    }
}
impl PartialEq for InlineSegment {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.width, other.width, 0.001) && self.kind == other.kind
    }
}
impl Eq for InlineSegment {}
impl std::hash::Hash for InlineSegment {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.width, 0.001, state);
        self.kind.hash(state);
    }
}

/// The type of an inline/text segment.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TextSegmentKind {
    /// Any strong left-to-right character.
    LeftToRight,
    /// Any strong right-to-left (non-Arabic-type) character.
    RightToLeft,
    /// Any strong right-to-left (Arabic-type) character.
    ArabicLetter,

    /// Any ASCII digit or Eastern Arabic-Indic digit.
    EuropeanNumber,
    /// Plus and minus signs.
    EuropeanSeparator,
    /// A terminator in a numeric format context, includes currency signs.
    EuropeanTerminator,
    /// Any Arabic-Indic digit.
    ArabicNumber,
    /// Commas, colons, and slashes.
    CommonSeparator,
    /// Any non-spacing mark.
    NonSpacingMark,
    /// Most format characters, control codes, or non-characters.
    BoundaryNeutral,

    /// Emoji chars, components and zero-width-joiner between emoji.
    Emoji,

    /// Various newline characters.
    LineBreak,
    /// A sequence of `'\t', '\v'` or `'\u{1F}'`.
    Tab,
    /// Spaces.
    Space,
    /// Most other symbols and punctuation marks.
    OtherNeutral,
    /// Open or close bidi bracket.
    ///
    /// Can be any chars in <https://unicode.org/Public/UNIDATA/BidiBrackets.txt>.
    Bracket(char),

    /// Bidi control character.
    ///
    /// Chars can be:
    ///
    /// * `\u{202A}`: The LR embedding control.
    /// * `\u{202D}`: The LR override control.
    /// * `\u{202B}`: The RL embedding control.
    /// * `\u{202E}`: The RL override control.
    /// * `\u{202C}`: Terminates an embedding or override control.
    ///
    /// * `\u{2066}`: The LR isolate control.
    /// * `\u{2067}`: The RL isolate control.
    /// * `\u{2068}`: The first strong isolate control.
    /// * `\u{2069}`: Terminates an isolate control.
    BidiCtrl(char),
}
impl TextSegmentKind {
    /// Returns `true` if the segment can be considered part of a word for the purpose of inserting letter spacing.
    pub fn is_word(self) -> bool {
        use TextSegmentKind::*;
        matches!(
            self,
            LeftToRight
                | RightToLeft
                | ArabicLetter
                | EuropeanNumber
                | EuropeanSeparator
                | EuropeanTerminator
                | ArabicNumber
                | CommonSeparator
                | NonSpacingMark
                | BoundaryNeutral
                | OtherNeutral
                | Bracket(_)
                | Emoji
        )
    }

    /// Returns `true` if the segment can be considered part of space between words for the purpose of inserting word spacing.
    pub fn is_space(self) -> bool {
        matches!(self, Self::Space | Self::Tab)
    }

    /// Returns `true` if the segment terminates the current line.
    ///
    /// Line break segments are the last segment of their line and explicitly start a new line.
    pub fn is_line_break(self) -> bool {
        matches!(self, Self::LineBreak)
    }

    /// If multiple segments of this same kind can be represented by a single segment in the Unicode bidi algorithm.
    pub fn can_merge(self) -> bool {
        use TextSegmentKind::*;
        !matches!(self, Bracket(_) | BidiCtrl(_))
    }

    /// Get more info about the bracket char if `self` is `Bracket(_)` with a valid char.
    pub fn bracket_info(self) -> Option<unicode_bidi::data_source::BidiMatchedOpeningBracket> {
        if let TextSegmentKind::Bracket(c) = self {
            unicode_bidi::HardcodedBidiData.bidi_matched_opening_bracket(c)
        } else {
            None
        }
    }

    /// Gets the layout direction this segment will always be in, independent of the base direction.
    ///
    /// Returns `None` if the segment direction depends on the line context.
    pub fn strong_direction(self) -> Option<LayoutDirection> {
        use TextSegmentKind::*;

        match self {
            LeftToRight => Some(LayoutDirection::LTR),
            RightToLeft | ArabicLetter => Some(LayoutDirection::RTL),
            BidiCtrl(_) => {
                use unicode_bidi::BidiClass::*;
                match unicode_bidi::BidiClass::from(self) {
                    LRE | LRO | LRI => Some(LayoutDirection::LTR),
                    RLE | RLO | RLI => Some(LayoutDirection::RTL),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
impl From<char> for TextSegmentKind {
    fn from(c: char) -> Self {
        use unicode_bidi::*;

        unicode_bidi::HardcodedBidiData.bidi_class(c).into()
    }
}

impl From<unicode_bidi::BidiClass> for TextSegmentKind {
    fn from(value: unicode_bidi::BidiClass) -> Self {
        use TextSegmentKind::*;
        use unicode_bidi::BidiClass::*;

        match value {
            WS => Space,
            L => LeftToRight,
            R => RightToLeft,
            AL => ArabicLetter,
            AN => ArabicNumber,
            CS => CommonSeparator,
            B => LineBreak,
            EN => EuropeanNumber,
            ES => EuropeanSeparator,
            ET => EuropeanTerminator,
            S => Tab,
            ON => OtherNeutral,
            BN => BoundaryNeutral,
            NSM => NonSpacingMark,
            RLE => BidiCtrl('\u{202B}'),
            LRI => BidiCtrl('\u{2066}'),
            RLI => BidiCtrl('\u{2067}'),
            LRO => BidiCtrl('\u{202D}'),
            FSI => BidiCtrl('\u{2068}'),
            PDF => BidiCtrl('\u{202C}'),
            LRE => BidiCtrl('\u{202A}'),
            PDI => BidiCtrl('\u{2069}'),
            RLO => BidiCtrl('\u{202E}'),
        }
    }
}
impl From<TextSegmentKind> for unicode_bidi::BidiClass {
    fn from(value: TextSegmentKind) -> Self {
        use TextSegmentKind::*;
        use unicode_bidi::BidiClass::*;

        match value {
            Space => WS,
            LeftToRight => L,
            RightToLeft => R,
            ArabicLetter => AL,
            ArabicNumber => AN,
            CommonSeparator => CS,
            LineBreak => B,
            EuropeanNumber => EN,
            EuropeanSeparator => ES,
            EuropeanTerminator => ET,
            Tab => S,
            OtherNeutral | Emoji | Bracket(_) => ON,
            BoundaryNeutral => BN,
            NonSpacingMark => NSM,
            BidiCtrl(c) => match c {
                '\u{202A}' => LRE,
                '\u{202D}' => LRO,
                '\u{202B}' => RLE,
                '\u{202E}' => RLO,
                '\u{202C}' => PDF,
                '\u{2066}' => LRI,
                '\u{2067}' => RLI,
                '\u{2068}' => FSI,
                '\u{2069}' => PDI,
                _c => {
                    #[cfg(debug_assertions)]
                    {
                        tracing::error!("invalid bidi ctrl char '{_c}'");
                    }
                    ON
                }
            },
        }
    }
}

bitflags! {
    /// Mask of values that can affect the layout operation of a value.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bytemuck::NoUninit)]
    #[repr(transparent)]
    pub struct LayoutMask: u32 {
        /// The `default_value`.
        const DEFAULT_VALUE = 1 << 31;
        /// The [`LayoutMetrics::constraints`], [`LayoutMetrics::z_constraints`] and [`LayoutMetrics::inline_constraints`].
        const CONSTRAINTS = 1 << 30;

        /// The [`LayoutMetrics::font_size`].
        const FONT_SIZE = 1;
        /// The [`LayoutMetrics::root_font_size`].
        const ROOT_FONT_SIZE = 1 << 1;
        /// The [`LayoutMetrics::scale_factor`].
        const SCALE_FACTOR = 1 << 2;
        /// The [`LayoutMetrics::viewport`].
        const VIEWPORT = 1 << 3;
        /// The [`LayoutMetrics::screen_ppi`].
        const SCREEN_PPI = 1 << 4;
        /// The [`LayoutMetrics::direction`].
        const DIRECTION = 1 << 5;
        /// The [`LayoutMetrics::leftover`].
        const LEFTOVER = 1 << 6;
    }
}
impl Default for LayoutMask {
    /// Empty.
    fn default() -> Self {
        LayoutMask::empty()
    }
}
