use euclid::BoolVector2D;
use zng_wgt::prelude::*;

use crate::WIDGET_SIZE;

/// Exact size of the widget.
///
/// When set the widget is layout with exact size constraints, clamped by the contextual max.
/// Relative size values are computed from the constraints maximum bounded size.
///
/// This property disables inline layout for the widget. This property sets the [`WIDGET_SIZE`].
///
/// See also [`force_size`] to deliberately break layout and cause out-of-bounds rendering when
/// the exact size cannot fit in the contextual min/max.
///
/// # `width` and `height`
///
/// You can use the [`width`] and [`height`] properties to only set the size of one dimension.
///
/// [`min_size`]: fn@crate::min_size
/// [`max_size`]: fn@crate::max_size
/// [`width`]: fn@crate::width
/// [`height`]: fn@crate::height
/// [`force_size`]: fn@crate::force_size
/// [`align`]: fn@zng_wgt::align
#[property(SIZE, default(Size::default()))]
pub fn size(child: impl IntoUiNode, size: impl IntoVar<Size>) -> UiNode {
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
            *desired_size = SizeLayout::new(&size, || child.node().measure(wm)).measure(child.node(), wm);
        }
        UiNodeOp::Layout { wl, final_size } => {
            child.delegated();
            *final_size = SizeLayout::new(&size, || child.node().measure(&mut wl.to_measure(None))).layout(child.node(), wl);
        }
        _ => {}
    })
}
struct SizeLayout {
    parent_constraints: PxConstraints2d,
    constraints: PxConstraints2d,
    size: PxSize,
    is_default: BoolVector2D,
}
impl SizeLayout {
    // compute constraints for measure & layout
    pub fn new(size: &Var<Size>, measure: impl FnOnce() -> PxSize) -> Self {
        let parent_constraints = LAYOUT.constraints();
        let mut constraints = parent_constraints;
        let mut is_default = BoolVector2D { x: true, y: true };
        let mut size_px = PxSize::zero();
        size.with(|s| {
            let unit_constraints = parent_constraints.with_new_min(Px(0), Px(0));
            let mut dft = PxSize::zero();
            if !s.width.is_default() || s.height.is_default() && s.width.has_default() || s.height.has_default() {
                // has Length::Expr with Default components, needs measure
                // this is usually an animation from Default size to a fixed size
                dft = measure();
            }
            if !s.width.is_default() {
                is_default.x = false;
                size_px.width = LAYOUT.with_constraints(unit_constraints, || s.width.layout_dft_x(dft.width));
                size_px.width = unit_constraints.x.clamp(size_px.width);
                constraints.x = PxConstraints::new_exact(size_px.width);
            }
            if !s.height.is_default() {
                is_default.y = false;
                size_px.height = LAYOUT.with_constraints(unit_constraints, || s.height.layout_dft_y(dft.height));
                size_px.height = unit_constraints.y.clamp(size_px.height);
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
            // default is noop (widget API requirement)
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

    // clamp/expand outer-bounds to fulfill parent constraints
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

/// Exact width of the widget.
///
/// When set the widget is layout with exact size constraints, clamped by the contextual max.
/// Relative values are computed from the constraints maximum bounded width.
///
/// This property disables inline layout for the widget. This property sets the [`WIDGET_SIZE`] width.
///
/// See also [`force_width`] to deliberately break layout and cause out-of-bounds rendering when
/// the exact width cannot fit in the contextual min/max.
///
/// # `size`
///
/// You can set both `width` and `height` at the same time using the [`size`](fn@size) property.
///
/// [`min_width`]: fn@crate::min_width
/// [`max_width`]: fn@crate::max_width
/// [`force_width`]: fn@crate::force_width
#[property(SIZE, default(Length::Default))]
pub fn width(child: impl IntoUiNode, width: impl IntoVar<Length>) -> UiNode {
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
            *desired_size = WidthLayout::new(&width, || child.node().measure(wm)).measure(child.node(), wm);
        }
        UiNodeOp::Layout { wl, final_size } => {
            child.delegated();
            *final_size = WidthLayout::new(&width, || child.node().measure(&mut wl.to_measure(None))).layout(child.node(), wl);
        }
        _ => {}
    })
}
struct WidthLayout {
    parent_constraints: PxConstraints2d,
    constraints: PxConstraints2d,
    width: Px,
    is_default: bool,
}
impl WidthLayout {
    pub fn new(width: &Var<Length>, measure: impl FnOnce() -> PxSize) -> Self {
        let parent_constraints = LAYOUT.constraints();
        let mut constraints = parent_constraints;
        let mut is_default = true;
        let mut width_px = Px(0);
        width.with(|w| {
            if !w.is_default() {
                let mut dft = Px(0);
                if w.has_default() {
                    // Length::Expr with default components, needs measure
                    dft = measure().width;
                }

                let unit_constraints = parent_constraints.with_new_min_x(Px(0));
                is_default = false;

                width_px = LAYOUT.with_constraints(unit_constraints, || w.layout_dft_x(dft));
                width_px = unit_constraints.x.clamp(width_px);
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

/// Exact height of the widget.
///
/// When set the widget is layout with exact size constraints, clamped by the contextual min/max.
/// Relative values are computed from the constraints maximum bounded height.
///
/// This property disables inline layout for the widget. This property sets the [`WIDGET_SIZE`] height.
///
/// See also [`force_height`] to deliberately break layout and cause out-of-bounds rendering when
/// the exact height cannot fit in the contextual min/max.
///
/// # `size`
///
/// You can set both `width` and `height` at the same time using the [`size`](fn@size) property.
///
/// [`min_height`]: fn@crate::min_height
/// [`max_height`]: fn@crate::max_height
/// [`force_height`]: fn@crate::force_height
#[property(SIZE, default(Length::Default))]
pub fn height(child: impl IntoUiNode, height: impl IntoVar<Length>) -> UiNode {
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
            *desired_size = HeightLayout::new(&height, || child.node().measure(wm)).measure(child.node(), wm);
        }
        UiNodeOp::Layout { wl, final_size } => {
            child.delegated();
            *final_size = HeightLayout::new(&height, || child.node().measure(&mut wl.to_measure(None))).layout(child.node(), wl);
        }
        _ => {}
    })
}
struct HeightLayout {
    parent_constraints: PxConstraints2d,
    constraints: PxConstraints2d,
    height: Px,
    is_default: bool,
}
impl HeightLayout {
    pub fn new(height: &Var<Length>, measure: impl FnOnce() -> PxSize) -> Self {
        let parent_constraints = LAYOUT.constraints();
        let mut constraints = parent_constraints;
        let mut is_default = true;
        let mut height_px = Px(0);
        height.with(|h| {
            if !h.is_default() {
                let mut dft = Px(0);
                if h.has_default() {
                    // Length::Expr with default components, needs measure
                    dft = measure().height;
                }
                let unit_constraints = parent_constraints.with_new_min_y(Px(0));
                is_default = false;
                height_px = LAYOUT.with_constraints(unit_constraints, || h.layout_dft_y(dft));
                height_px = unit_constraints.y.clamp(height_px);
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
