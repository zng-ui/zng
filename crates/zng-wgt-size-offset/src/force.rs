use euclid::BoolVector2D;
use zng_wgt::prelude::*;

use crate::WIDGET_SIZE;

/// Exact size of the widget ignoring the contextual max.
///
/// When set the widget is layout with exact size constraints, ignoring the contextual max.
/// Relative values are computed from the constraints maximum bounded size.
///
/// Note that this property deliberately breaks layout and causes out-of-bounds rendering. You
/// can use [`size`](fn@crate::size) instead to set an exact size that is coerced by the contextual max.
///
/// # `force_width` and `force_height`
///
/// You can use the [`force_width`] and [`force_height`] properties to only set the size of one dimension.
///
/// [`force_width`]: fn@crate::force_width
/// [`force_height`]: fn@crate::force_height
#[property(SIZE, default(Size::default()))]
pub fn force_size(child: impl IntoUiNode, size: impl IntoVar<Size>) -> UiNode {
    let size = size.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&size);
            child.init();
            size.with(|l| WIDGET_SIZE.set(l));
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);
            size.with_new(|s| {
                WIDGET_SIZE.set(s);
                WIDGET.layout();
            });
        }
        UiNodeOp::Measure { wm, desired_size } => {
            child.delegated();
            *desired_size = ForceSizeLayout::new(&size, || child.node().measure(wm)).measure(child.node(), wm);
        }
        UiNodeOp::Layout { wl, final_size } => {
            child.delegated();
            *final_size = ForceSizeLayout::new(&size, || child.node().measure(&mut wl.to_measure(None))).layout(child.node(), wl);
        }
        _ => {}
    })
}
struct ForceSizeLayout {
    parent_constraints: PxConstraints2d,
    constraints: PxConstraints2d,
    size: PxSize,
    is_default: BoolVector2D,
}
impl ForceSizeLayout {
    pub fn new(size: &Var<Size>, measure: impl FnOnce() -> PxSize) -> Self {
        let parent_constraints = LAYOUT.constraints();
        let mut constraints = parent_constraints;
        let mut is_default = BoolVector2D { x: true, y: true };
        let mut size_px = PxSize::zero();
        size.with(|s| {
            let mut dft = PxSize::zero();
            if !s.width.is_default() || s.height.is_default() && s.width.has_default() || s.height.has_default() {
                // has Length::Expr with Default components, needs measure
                // this is usually an animation from Default size to a fixed size
                dft = measure();
            }

            if !s.width.is_default() {
                is_default.x = false;
                size_px.width = LAYOUT.with_constraints(parent_constraints.with_fill_x(parent_constraints.x.is_bounded()), || {
                    s.width.layout_dft_x(dft.width)
                });
                constraints.x = PxConstraints::new_exact(size_px.width);
            }
            if !s.height.is_default() {
                is_default.y = false;
                size_px.height = LAYOUT.with_constraints(parent_constraints.with_fill_y(parent_constraints.y.is_bounded()), || {
                    s.width.layout_dft_y(dft.height)
                });
                constraints.y = PxConstraints::new_exact(size_px.height);
            }
        });
        Self {
            parent_constraints,
            constraints,
            size: size_px,
            is_default,
        }
    }

    pub fn measure(&self, child: &mut UiNode, wm: &mut WidgetMeasure) -> PxSize {
        if self.is_default.all() {
            return child.measure(wm);
        }

        let size = if self.is_default.any() {
            LAYOUT.with_constraints(self.constraints, || wm.measure_block(child))
        } else {
            wm.measure_block(&mut UiNode::nil());
            self.size
        };

        self.clamp_outer_bounds(size)
    }

    pub fn layout(&self, child: &mut UiNode, wl: &mut WidgetLayout) -> PxSize {
        if self.is_default.all() {
            return child.layout(wl);
        }

        let size = LAYOUT.with_constraints(self.constraints, || child.layout(wl));

        self.clamp_outer_bounds(size)
    }

    fn clamp_outer_bounds(&self, mut size: PxSize) -> PxSize {
        if !self.is_default.x {
            size.width = Align::TOP_LEFT.measure_x(self.size.width, self.parent_constraints.x);
        }
        if !self.is_default.y {
            size.height = Align::TOP_LEFT.measure_y(self.size.height, self.parent_constraints.y);
        }
        size
    }
}

/// Exact width of the widget ignoring the contextual max.
///
/// When set the widget is layout with exact size constraints, ignoring the contextual max.
/// Relative values are computed from the constraints maximum bounded width.
///
/// Note that this property deliberately breaks layout and causes out-of-bounds rendering. You
/// can use [`width`](fn@crate::width) instead to set an exact width that is coerced by the contextual max.
///
/// # `force_size`
///
/// You can set both `force_width` and `force_height` at the same time using the [`force_size`](fn@crate::force_size) property.
#[property(SIZE, default(Length::Default))]
pub fn force_width(child: impl IntoUiNode, width: impl IntoVar<Length>) -> UiNode {
    let width = width.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&width);
            child.init();
            width.with(|s| WIDGET_SIZE.set_width(s));
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);
            width.with_new(|w| {
                WIDGET_SIZE.set_width(w);
                WIDGET.layout();
            });
        }
        UiNodeOp::Measure { wm, desired_size } => {
            child.delegated();
            *desired_size = ForceWidthLayout::new(&width, || child.node().measure(wm)).measure(child.node(), wm);
        }
        UiNodeOp::Layout { wl, final_size } => {
            child.delegated();
            *final_size = ForceWidthLayout::new(&width, || child.node().measure(&mut wl.to_measure(None))).layout(child.node(), wl);
        }
        _ => {}
    })
}
struct ForceWidthLayout {
    parent_constraints: PxConstraints2d,
    constraints: PxConstraints2d,
    width: Px,
    is_default: bool,
}
impl ForceWidthLayout {
    pub fn new(width: &Var<Length>, measure: impl FnOnce() -> PxSize) -> Self {
        let parent_constraints = LAYOUT.constraints();
        let mut constraints = parent_constraints;
        let mut is_default = true;
        let mut width_px = Px(0);
        width.with(|w| {
            if !w.is_default() {
                is_default = false;
                let mut dft = Px(0);
                if w.has_default() {
                    // Length::Expr with default components, needs measure
                    dft = measure().width;
                }
                width_px = LAYOUT.with_constraints(parent_constraints.with_fill_x(parent_constraints.x.is_bounded()), || {
                    w.layout_dft_x(dft)
                });
                constraints.x = PxConstraints::new_exact(width_px);
            }
        });
        Self {
            parent_constraints,
            constraints,
            width: width_px,
            is_default,
        }
    }

    pub fn measure(&self, child: &mut UiNode, wm: &mut WidgetMeasure) -> PxSize {
        if self.is_default {
            return child.measure(wm);
        }

        let size = LAYOUT.with_constraints(self.constraints, || wm.measure_block(child));

        self.clamp_outer_bounds(size)
    }

    pub fn layout(&self, child: &mut UiNode, wl: &mut WidgetLayout) -> PxSize {
        if self.is_default {
            return child.layout(wl);
        }

        let size = LAYOUT.with_constraints(self.constraints, || child.layout(wl));

        self.clamp_outer_bounds(size)
    }

    fn clamp_outer_bounds(&self, mut size: PxSize) -> PxSize {
        if !self.is_default {
            size.width = Align::TOP_LEFT.measure_x(self.width, self.parent_constraints.x);
        }
        size
    }
}

/// Exact height of the widget ignoring the contextual max.
///
/// When set the widget is layout with exact size constraints, ignoring the contextual max.
/// Relative values are computed from the constraints maximum bounded height.
///
/// Note that this property deliberately breaks layout and causes out-of-bounds rendering. You
/// can use [`height`](fn@crate::height) instead to set an exact height that is coerced by the contextual max.
///
/// # `force_size`
///
/// You can set both `force_width` and `force_height` at the same time using the [`force_size`](fn@crate::force_size) property.
#[property(SIZE, default(Length::Default))]
pub fn force_height(child: impl IntoUiNode, height: impl IntoVar<Length>) -> UiNode {
    let height = height.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&height);
            child.init();
            height.with(|s| WIDGET_SIZE.set_height(s));
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);

            height.with_new(|h| {
                WIDGET_SIZE.set_height(h);
                WIDGET.layout();
            });
        }
        UiNodeOp::Measure { wm, desired_size } => {
            child.delegated();
            *desired_size = ForceHeightLayout::new(&height, || child.node().measure(wm)).measure(child.node(), wm);
        }
        UiNodeOp::Layout { wl, final_size } => {
            child.delegated();
            *final_size = ForceHeightLayout::new(&height, || child.node().measure(&mut wl.to_measure(None))).layout(child.node(), wl);
        }
        _ => {}
    })
}
struct ForceHeightLayout {
    parent_constraints: PxConstraints2d,
    constraints: PxConstraints2d,
    height: Px,
    is_default: bool,
}
impl ForceHeightLayout {
    pub fn new(height: &Var<Length>, measure: impl FnOnce() -> PxSize) -> Self {
        let parent_constraints = LAYOUT.constraints();
        let mut constraints = parent_constraints;
        let mut is_default = true;
        let mut height_px = Px(0);
        height.with(|h| {
            if !h.is_default() {
                is_default = false;
                let mut dft = Px(0);
                if h.has_default() {
                    // Length::Expr with default components, needs measure
                    dft = measure().height;
                }
                height_px = LAYOUT.with_constraints(parent_constraints.with_fill_y(parent_constraints.y.is_bounded()), || {
                    h.layout_dft_y(dft)
                });
                constraints.y = PxConstraints::new_exact(height_px);
            }
        });
        Self {
            parent_constraints,
            constraints,
            height: height_px,
            is_default,
        }
    }

    pub fn measure(&self, child: &mut UiNode, wm: &mut WidgetMeasure) -> PxSize {
        if self.is_default {
            return child.measure(wm);
        }

        let size = LAYOUT.with_constraints(self.constraints, || wm.measure_block(child));

        self.clamp_outer_bounds(size)
    }

    pub fn layout(&self, child: &mut UiNode, wl: &mut WidgetLayout) -> PxSize {
        if self.is_default {
            return child.layout(wl);
        }

        let size = LAYOUT.with_constraints(self.constraints, || child.layout(wl));

        self.clamp_outer_bounds(size)
    }

    fn clamp_outer_bounds(&self, mut size: PxSize) -> PxSize {
        if !self.is_default {
            size.height = Align::TOP_LEFT.measure_y(self.height, self.parent_constraints.y);
        }
        size
    }
}
