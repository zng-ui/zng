use zng_wgt::prelude::*;

/// Maximum size of the widget.
///
/// The widget size can be smaller then this but not larger. Relative values are computed from the
/// constraints maximum bounded size.
///
/// This property does not force the maximum constrained size, the `max_size` is only used
/// in a dimension if it is less then the constrained maximum, or the maximum was not constrained.
///
/// This property disables inline layout for the widget.
///
/// # `max_width` and `max_height`
///
/// You can use the [`max_width`](fn@crate::max_width) and [`max_height`](fn@crate::max_height) properties to only
/// set the maximum size of one dimension.
#[property(SIZE-1,  default(PxSize::default()))]
pub fn max_size(child: impl IntoUiNode, max_size: impl IntoVar<Size>) -> UiNode {
    let max_size = max_size.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&max_size);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            child.delegated();
            *desired_size = MaxSizeLayout::new(&max_size).measure(&max_size, child.node(), wm);
        }
        UiNodeOp::Layout { wl, final_size } => {
            child.delegated();
            *final_size = MaxSizeLayout::new(&max_size).layout(&max_size, child.node(), wl);
        }
        _ => {}
    })
}
struct MaxSizeLayout {
    parent_constraints: PxConstraints2d,
    constraints: PxConstraints2d,
    max: PxSize,
    is_default: bool,
}
impl MaxSizeLayout {
    // compute constraints for measure & layout
    pub fn new(max_size: &Var<Size>) -> Self {
        let parent_constraints = LAYOUT.constraints();
        let mut constraints = parent_constraints;
        let mut max = PxSize::splat(Px::MAX);
        let mut is_default = true;
        max_size.with(|s| {
            if !s.width.is_default() {
                is_default = false;
                let dft = parent_constraints.x.max_bounded();
                max.width = LAYOUT.with_constraints(parent_constraints.with_fill_x(parent_constraints.x.is_bounded()), || {
                    s.width.layout_dft_x(dft)
                });
                constraints.x = constraints.x.with_max(max.width);
            }
            if !s.height.is_default() {
                is_default = false;
                let dft = parent_constraints.y.max_bounded();
                max.height = LAYOUT.with_constraints(parent_constraints.with_fill_y(parent_constraints.y.is_bounded()), || {
                    s.height.layout_dft_y(dft)
                });
                constraints.y = constraints.y.with_max(max.height);
            }
        });
        Self {
            parent_constraints,
            constraints,
            max,
            is_default,
        }
    }

    pub fn measure(&self, max_size: &Var<Size>, child: &mut UiNode, wm: &mut WidgetMeasure) -> PxSize {
        if self.is_default {
            // default is noop (widget API requirement)
            return child.measure(wm);
        }
        let size = LAYOUT.with_constraints(self.constraints, || wm.measure_block(child));
        self.clamp_outer_bounds(max_size, size)
    }

    pub fn layout(&self, max_size: &Var<Size>, child: &mut UiNode, wl: &mut WidgetLayout) -> PxSize {
        if self.is_default {
            return child.layout(wl);
        }
        let size = LAYOUT.with_constraints(self.constraints, || child.layout(wl));
        self.clamp_outer_bounds(max_size, size)
    }

    // clamp/expand outer-bounds to fulfill parent constraints
    fn clamp_outer_bounds(&self, max_size: &Var<Size>, mut size: PxSize) -> PxSize {
        size = size.min(self.max);
        max_size.with(|s| {
            if !s.width.is_default() {
                size.width = Align::TOP_LEFT.measure_x(size.width, self.parent_constraints.x);
            }
            if !s.height.is_default() {
                size.height = Align::TOP_LEFT.measure_y(size.height, self.parent_constraints.y);
            }
        });
        size
    }
}

/// Maximum width of the widget.
///
/// The widget width can be smaller then this but not larger.
/// Relative values are computed from the constraints maximum bounded width.
///
/// This property does not force the maximum constrained width, the `max_width` is only used
/// if it is less then the constrained maximum, or the maximum was not constrained.
///
/// This property disables inline layout for the widget.
///
/// # `max_size`
///
/// You can set both `max_width` and `max_height` at the same time using the [`max_size`](fn@crate::max_size) property.
#[property(SIZE-1, default(Length::Default))]
pub fn max_width(child: impl IntoUiNode, max_width: impl IntoVar<Length>) -> UiNode {
    let max_width = max_width.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&max_width);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            child.delegated();
            *desired_size = MaxWidthLayout::new(&max_width).measure(child.node(), wm);
        }
        UiNodeOp::Layout { wl, final_size } => {
            child.delegated();
            *final_size = MaxWidthLayout::new(&max_width).layout(child.node(), wl);
        }
        _ => {}
    })
}
struct MaxWidthLayout {
    parent_constraints: PxConstraints2d,
    constraints: PxConstraints2d,
    max: Px,
    is_default: bool,
}
impl MaxWidthLayout {
    pub fn new(max_width: &Var<Length>) -> Self {
        let parent_constraints = LAYOUT.constraints();
        let mut constraints = parent_constraints;
        let mut max = Px::MAX;
        let mut is_default = true;

        max_width.with(|w| {
            if !w.is_default() {
                is_default = false;
                let dft = parent_constraints.x.max_bounded();
                max = LAYOUT.with_constraints(parent_constraints.with_fill_x(parent_constraints.x.is_bounded()), || {
                    w.layout_dft_x(dft)
                });
                constraints.x = constraints.x.with_max(max);
            }
        });

        Self {
            parent_constraints,
            constraints,
            max,
            is_default,
        }
    }

    pub fn measure(&self, child: &mut UiNode, wm: &mut WidgetMeasure) -> PxSize {
        if self.is_default {
            // default is noop (widget API requirement)
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

    // clamp/expand outer-bounds to fulfill parent constraints
    fn clamp_outer_bounds(&self, mut size: PxSize) -> PxSize {
        size.width = size.width.min(self.max);
        size.width = Align::TOP_LEFT.measure_x(size.width, self.parent_constraints.x);
        size
    }
}

/// Maximum height of the widget.
///
/// The widget height can be smaller then this but not larger.
/// Relative values are computed from the constraints maximum bounded height.
///
/// This property does not force the maximum constrained height, the `max_height` is only used
/// if it is less then the constrained maximum, or the maximum was not constrained.
///
/// This property disables inline layout for the widget.
///
/// # `max_size`
///
/// You can set both `max_width` and `max_height` at the same time using the [`max_size`](fn@crate::max_size) property.
#[property(SIZE-1, default(Length::Default))]
pub fn max_height(child: impl IntoUiNode, max_height: impl IntoVar<Length>) -> UiNode {
    let max_height = max_height.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&max_height);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            child.delegated();
            *desired_size = MaxHeightLayout::new(&max_height).measure(child.node(), wm);
        }
        UiNodeOp::Layout { wl, final_size } => {
            child.delegated();
            *final_size = MaxHeightLayout::new(&max_height).layout(child.node(), wl);
        }
        _ => {}
    })
}
struct MaxHeightLayout {
    parent_constraints: PxConstraints2d,
    constraints: PxConstraints2d,
    max: Px,
    is_default: bool,
}
impl MaxHeightLayout {
    pub fn new(max_height: &Var<Length>) -> Self {
        let parent_constraints = LAYOUT.constraints();
        let mut constraints = parent_constraints;
        let mut max = Px::MAX;
        let mut is_default = true;

        max_height.with(|h| {
            if !h.is_default() {
                is_default = false;
                let dft = parent_constraints.y.max_bounded();
                max = LAYOUT.with_constraints(parent_constraints.with_fill_y(parent_constraints.y.is_bounded()), || {
                    h.layout_dft_y(dft)
                });
                constraints.y = constraints.y.with_max(max);
            }
        });

        Self {
            parent_constraints,
            constraints,
            max,
            is_default,
        }
    }

    pub fn measure(&self, child: &mut UiNode, wm: &mut WidgetMeasure) -> PxSize {
        if self.is_default {
            // default is noop (widget API requirement)
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

    // clamp/expand outer-bounds to fulfill parent constraints
    fn clamp_outer_bounds(&self, mut size: PxSize) -> PxSize {
        size.height = size.height.min(self.max);
        size.height = Align::TOP_LEFT.measure_y(size.height, self.parent_constraints.y);
        size
    }
}
