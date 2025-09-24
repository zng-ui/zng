use zng_wgt::prelude::*;

/// Getter property, gets the latest layout widget inner size.
#[property(WIDGET_INNER - 1)]
pub fn actual_size(child: impl IntoUiNode, size: impl IntoVar<DipSize>) -> UiNode {
    let size = size.into_var();
    match_node(child, move |c, op| {
        if let UiNodeOp::Layout { wl, final_size } = op {
            *final_size = c.layout(wl);
            let f = LAYOUT.scale_factor();
            // inner size updated in `widget_inner` -> `WidgetLayout::with_inner`
            let s = WIDGET.bounds().inner_size().to_dip(f);
            if size.get() != s {
                // manually check equality here to avoid scheduling a var modify for each layout
                size.set(s);
            }
        }
    })
}

/// Getter property, gets the latest layout widget inner width.
#[property(LAYOUT)]
pub fn actual_width(child: impl IntoUiNode, width: impl IntoVar<Dip>) -> UiNode {
    let width = width.into_var();
    match_node(child, move |c, op| {
        if let UiNodeOp::Layout { wl, final_size } = op {
            *final_size = c.layout(wl);
            let f = LAYOUT.scale_factor();
            let w = WIDGET.bounds().inner_size().width.to_dip(f);
            if width.get() != w {
                width.set(w);
            }
        }
    })
}

/// Getter property, gets the latest layout widget inner height.
#[property(LAYOUT)]
pub fn actual_height(child: impl IntoUiNode, height: impl IntoVar<Dip>) -> UiNode {
    let height = height.into_var();
    match_node(child, move |c, op| {
        if let UiNodeOp::Layout { wl, final_size } = op {
            *final_size = c.layout(wl);
            let f = LAYOUT.scale_factor();
            let h = WIDGET.bounds().inner_size().height.to_dip(f);
            if height.get() != h {
                height.set(h);
            }
        }
    })
}

/// Getter property, gets the latest layout widget inner size, in device pixels.
#[property(LAYOUT)]
pub fn actual_size_px(child: impl IntoUiNode, size: impl IntoVar<PxSize>) -> UiNode {
    let size = size.into_var();
    match_node(child, move |c, op| {
        if let UiNodeOp::Layout { wl, final_size } = op {
            *final_size = c.layout(wl);
            let s = WIDGET.bounds().inner_size();
            if size.get() != s {
                size.set(s);
            }
        }
    })
}

/// Getter property, gets the latest layout widget inner width, in device pixels.
#[property(LAYOUT)]
pub fn actual_width_px(child: impl IntoUiNode, width: impl IntoVar<Px>) -> UiNode {
    let width = width.into_var();
    match_node(child, move |c, op| {
        if let UiNodeOp::Layout { wl, final_size } = op {
            *final_size = c.layout(wl);
            let w = WIDGET.bounds().inner_size().width;
            if width.get() != w {
                width.set(w);
            }
        }
    })
}

/// Getter property, gets the latest layout widget inner height, in device pixels.
#[property(LAYOUT)]
pub fn actual_height_px(child: impl IntoUiNode, height: impl IntoVar<Px>) -> UiNode {
    let height = height.into_var();
    match_node(child, move |c, op| {
        if let UiNodeOp::Layout { wl, final_size } = op {
            *final_size = c.layout(wl);
            let h = WIDGET.bounds().inner_size().height;
            if height.get() != h {
                height.set(h);
            }
        }
    })
}

/// Getter property, gets the latest rendered widget inner transform.
#[property(LAYOUT)]
pub fn actual_transform(child: impl IntoUiNode, transform: impl IntoVar<PxTransform>) -> UiNode {
    let transform = transform.into_var();
    match_node(child, move |c, op| match &op {
        UiNodeOp::Render { .. } | UiNodeOp::RenderUpdate { .. } => {
            c.op(op);
            let t = WIDGET.info().bounds_info().inner_transform();
            if transform.get() != t {
                transform.set(t);
            }
        }
        _ => {}
    })
}

/// Getter property, gets the latest rendered widget inner bounds in the window space.
#[property(LAYOUT)]
pub fn actual_bounds(child: impl IntoUiNode, bounds: impl IntoVar<PxRect>) -> UiNode {
    let bounds = bounds.into_var();
    match_node(child, move |c, op| match &op {
        UiNodeOp::Render { .. } | UiNodeOp::RenderUpdate { .. } => {
            c.op(op);
            let t = WIDGET.info().bounds_info().inner_bounds();
            if bounds.get() != t {
                bounds.set(t);
            }
        }
        _ => {}
    })
}
