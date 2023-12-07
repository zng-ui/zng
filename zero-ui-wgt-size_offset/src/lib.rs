//! Exact size constraints and exact positioning properties.

use zero_ui_wgt::prelude::*;

/// Widget layout offset.
///
/// Relative values are computed of the parent fill size or the widget's size, whichever is greater.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
///
/// Button! {
///     offset = (100, 20.pct());
///     child = Text!("Click Me!")
/// }
/// # ;
/// ```
///
/// In the example the button is offset 100 layout pixels to the right and 20% of the fill height down.
///
/// # `x` and `y`
///
/// You can use the [`x`](fn@x) and [`y`](fn@y) properties to only set the position in one dimension.
#[property(LAYOUT, default((0, 0)))]
pub fn offset(child: impl UiNode, offset: impl IntoVar<Vector>) -> impl UiNode {
    let offset = offset.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&offset);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let size = child.layout(wl);
            let offset = LAYOUT.with_constraints(PxConstraints2d::new_exact_size(LAYOUT.constraints().fill_size().max(size)), || {
                offset.layout()
            });
            wl.translate(offset);
            *final_size = size;
        }
        _ => {}
    })
}

/// Offset on the ***x*** axis.
///
/// Relative values are computed of the parent fill width or the widget's width, whichever is greater.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
///
/// Button! {
///     x = 20.pct();
///     child = Text!("Click Me!")
/// };
/// # ;
/// ```
///
/// In the example the button is moved 20 percent of the fill width to the right.
///
/// # `offset`
///
/// You can set both `x` and `y` at the same time using the [`offset`](fn@offset) property.
#[property(LAYOUT, default(0))]
pub fn x(child: impl UiNode, x: impl IntoVar<Length>) -> impl UiNode {
    let x = x.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&x);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let size = child.layout(wl);
            let x = LAYOUT.with_constraints(PxConstraints2d::new_exact_size(LAYOUT.constraints().fill_size().max(size)), || {
                x.layout_x()
            });
            wl.translate(PxVector::new(x, Px(0)));
            *final_size = size;
        }
        _ => {}
    })
}

/// Offset on the ***y*** axis.
///
/// Relative values are computed of the parent fill height or the widget's height, whichever is greater.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
///
/// Button! {
///     y = 20.pct();
///     child = Text!("Click Me!")
/// }
/// # ;
/// ```
///
/// In the example the button is moved down 20 percent of the fill height.
///
/// # `offset`
///
/// You can set both `x` and `y` at the same time using the [`offset`](fn@offset) property.
#[property(LAYOUT, default(0))]
pub fn y(child: impl UiNode, y: impl IntoVar<Length>) -> impl UiNode {
    let y = y.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&y);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let size = child.layout(wl);
            let y = LAYOUT.with_constraints(PxConstraints2d::new_exact_size(LAYOUT.constraints().fill_size().max(size)), || {
                y.layout_y()
            });
            wl.translate(PxVector::new(Px(0), y));
            *final_size = size;
        }
        _ => {}
    })
}

/// Minimum size of the widget.
///
/// The widget size can be larger then this but not smaller.
/// Relative values are computed from the constraints maximum bounded size.
///
/// This property does not force the minimum constrained size, the `min_size` is only used
/// in a dimension if it is greater then the constrained minimum.
///
/// This property disables inline layout for the widget.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # let label = formatx!("");
///
/// Button! {
///     child = Text!(label);
///     min_size = (100, 50);
/// }
/// # ;
/// ```
///
/// In the example the button will change size depending on the `label` value but it will
/// always have a minimum width of `100` and a minimum height of `50`.
///
/// # `min_width` and `min_height`
///
/// You can use the [`min_width`](fn@min_width) and [`min_height`](fn@min_height) properties to only
/// set the minimum size of one dimension.
#[property(SIZE-2, default((0, 0)))]
pub fn min_size(child: impl UiNode, min_size: impl IntoVar<Size>) -> impl UiNode {
    let min_size = min_size.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&min_size);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let c = LAYOUT.constraints();
            let min = LAYOUT.with_constraints(c.with_fill_vector(c.is_bounded()), || min_size.layout());
            let size = LAYOUT.with_constraints(c.with_min_size(min), || wm.measure_block(child));
            *desired_size = size.max(min);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let c = LAYOUT.constraints();
            let min = LAYOUT.with_constraints(c.with_fill_vector(c.is_bounded()), || min_size.layout());
            let size = LAYOUT.with_constraints(c.with_min_size(min), || child.layout(wl));
            *final_size = size.max(min);
        }
        _ => {}
    })
}

/// Minimum width of the widget.
///
/// The widget width can be larger then this but not smaller.
/// Relative values are computed from the constraints maximum bounded width.
///
/// This property does not force the minimum constrained width, the `min_width` is only used
/// if it is greater then the constrained minimum.
///
/// This property disables inline layout for the widget.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # let label = formatx!("");
///
/// Button! {
///     child = Text!(label);
///     min_width = 100;
/// }
/// # ;
/// ```
///
/// In the example the button will change size depending on the `label` value but it will
/// always have a minimum width of `100`.
///
/// # `min_size`
///
/// You can set both `min_width` and `min_height` at the same time using the [`min_size`](fn@min_size) property.
#[property(SIZE-2, default(0))]
pub fn min_width(child: impl UiNode, min_width: impl IntoVar<Length>) -> impl UiNode {
    let min_width = min_width.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&min_width);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let c = LAYOUT.constraints();
            let min = LAYOUT.with_constraints(c.with_fill_vector(c.is_bounded()), || min_width.layout_x());
            let mut size = LAYOUT.with_constraints(c.with_min_x(min), || wm.measure_block(child));
            size.width = size.width.max(min);
            *desired_size = size;
        }
        UiNodeOp::Layout { wl, final_size } => {
            let c = LAYOUT.constraints();
            let min = LAYOUT.with_constraints(c.with_fill_vector(c.is_bounded()), || min_width.layout_x());
            let mut size = LAYOUT.with_constraints(c.with_min_x(min), || child.layout(wl));
            size.width = size.width.max(min);
            *final_size = size;
        }
        _ => {}
    })
}

/// Minimum height of the widget.
///
/// The widget height can be larger then this but not smaller.
/// Relative values are computed from the constraints maximum bounded height.
///
/// This property does not force the minimum constrained height, the `min_height` is only used
/// if it is greater then the constrained minimum.
///
/// This property disables inline layout for the widget.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # let btn_content = Text!("");
/// #
/// Button! {
///     child = btn_content;
///     min_height = 50;
/// }
/// # ;
/// ```
///
/// In the example the button will change size depending on the `btn_content` value but it will
/// always have a minimum height of `50`.
///
/// # `min_size`
///
/// You can set both `min_width` and `min_height` at the same time using the [`min_size`](fn@min_size) property.
#[property(SIZE-2, default(0))]
pub fn min_height(child: impl UiNode, min_height: impl IntoVar<Length>) -> impl UiNode {
    let min_height = min_height.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&min_height);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let c = LAYOUT.constraints();
            let min = LAYOUT.with_constraints(c.with_fill_vector(c.is_bounded()), || min_height.layout_y());
            let mut size = LAYOUT.with_constraints(c.with_min_y(min), || wm.measure_block(child));
            size.height = size.height.max(min);
            *desired_size = size;
        }
        UiNodeOp::Layout { wl, final_size } => {
            let c = LAYOUT.constraints();
            let min = LAYOUT.with_constraints(c.with_fill_vector(c.is_bounded()), || min_height.layout_y());
            let mut size = LAYOUT.with_constraints(c.with_min_y(min), || child.layout(wl));
            size.height = size.height.max(min);
            *final_size = size;
        }
        _ => {}
    })
}

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
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # let btn_content = Text!("");
/// #
/// Button! {
///     child = btn_content;
///     max_size = (200, 100);
/// }
/// # ;
/// ```
///
/// In the example the button will change size depending on the `btn_content` value but it will
/// always have a maximum width of `200` and a maximum height of `100`.
///
/// # `max_width` and `max_height`
///
/// You can use the [`max_width`](fn@max_width) and [`max_height`](fn@max_height) properties to only
/// set the maximum size of one dimension.
#[property(SIZE-1)]
pub fn max_size(child: impl UiNode, max_size: impl IntoVar<Size>) -> impl UiNode {
    let max_size = max_size.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&max_size);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let max = with_fill_metrics(|d| max_size.layout_dft(d));
            let size = LAYOUT.with_constraints(LAYOUT.constraints().with_max_size(max), || wm.measure_block(child));
            *desired_size = size.min(max);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let max = with_fill_metrics(|d| max_size.layout_dft(d));
            let size = LAYOUT.with_constraints(LAYOUT.constraints().with_max_size(max), || child.layout(wl));
            *final_size = size.min(max);
        }
        _ => {}
    })
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
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # let btn_content = Text!("");
///
/// Button! {
///     child = btn_content;
///     max_width = 200;
/// }
/// # ;
/// ```
///
/// In the example the button will change size depending on the `btn_content` value but it will
/// always have a maximum width of `200`.
///
/// # `max_size`
///
/// You can set both `max_width` and `max_height` at the same time using the [`max_size`](fn@max_size) property.
#[property(SIZE-1)]
pub fn max_width(child: impl UiNode, max_width: impl IntoVar<Length>) -> impl UiNode {
    let max_width = max_width.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&max_width);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let max = with_fill_metrics(|d| max_width.layout_dft_x(d.width));

            let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_max_x(max), || wm.measure_block(child));
            size.width = size.width.min(max);
            *desired_size = size;
        }
        UiNodeOp::Layout { wl, final_size } => {
            let max = with_fill_metrics(|d| max_width.layout_dft_x(d.width));

            let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_max_x(max), || child.layout(wl));
            size.width = size.width.min(max);
            *final_size = size;
        }
        _ => {}
    })
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
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # let btn_content = Text!("");
///
/// Button! {
///     child = btn_content;
///     max_height = 100;
/// }
/// # ;
/// ```
///
/// In the example the button will change size depending on the `btn_content` value but it will
/// always have a maximum height of `100`.
///
/// # `max_size`
///
/// You can set both `max_width` and `max_height` at the same time using the [`max_size`](fn@max_size) property.
#[property(SIZE-1)]
pub fn max_height(child: impl UiNode, max_height: impl IntoVar<Length>) -> impl UiNode {
    let max_height = max_height.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&max_height);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let max = with_fill_metrics(|d| max_height.layout_dft_y(d.height));

            let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_max_y(max), || wm.measure_block(child));
            size.height = size.height.min(max);
            *desired_size = size;
        }
        UiNodeOp::Layout { wl, final_size } => {
            let max = with_fill_metrics(|d| max_height.layout_dft_y(d.height));

            let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_max_y(max), || child.layout(wl));
            size.height = size.height.min(max);
            *final_size = size;
        }
        _ => {}
    })
}

/// Exact size of the widget.
///
/// When set the widget is layout with exact size constraints, clamped by the contextual min/max.
/// Note that this means nested exact sized widgets will have the size of the parent, the exact size constraints
/// set by the parent clamp the requested size on the child, you can use the [`align`] property on the child to
/// loosen the minimum size constrain, the parent's size will still be enforced as the maximum size.
///
/// Relative size values are computed from the constraints maximum bounded size.
///
/// This property disables inline layout for the widget. This property sets the [`WIDGET_SIZE`].
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// Button! {
///     background_color = rgb(255, 0, 0);
///     size = (200, 300);
///     child = Text!("200x300 red");
/// }
/// # ;
/// ```
///
/// In the example the red button is set to a fixed size of `200` width and `300` height.
///
/// # `width` and `height`
///
/// You can use the [`width`] and [`height`] properties to only set the size of one dimension.
///
/// [`min_size`]: fn@min_size
/// [`max_size`]: fn@max_size
/// [`width`]: fn@width
/// [`height`]: fn@height
/// [`align`]: fn@align
#[property(SIZE)]
pub fn size(child: impl UiNode, size: impl IntoVar<Size>) -> impl UiNode {
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
            let size = with_fill_metrics(|d| size.layout_dft(d));
            let size = LAYOUT.constraints().clamp_size(size);
            wm.measure_block(&mut NilUiNode); // no need to actually measure child
            *desired_size = size;
        }
        UiNodeOp::Layout { wl, final_size } => {
            let size = with_fill_metrics(|d| size.layout_dft(d));
            let size = LAYOUT.constraints().clamp_size(size);
            LAYOUT.with_constraints(PxConstraints2d::new_exact_size(size), || child.layout(wl));
            *final_size = size;
        }
        _ => {}
    })
}

/// Exact width of the widget.
///
/// When set the widget is layout with exact size constraints, clamped by the contextual min/max.
/// Relative values are computed from the constraints maximum bounded width.
///
/// This property disables inline layout for the widget. This property sets the [`WIDGET_SIZE`] width.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// Button! {
///     background_color = rgb(255, 0, 0);
///     width = 200;
///     child = Text!("200x? red");
/// }
/// # ;
/// ```
///
/// In the example the red button is set to a fixed width of `200`.
///
/// # `size`
///
/// You can set both `width` and `height` at the same time using the [`size`](fn@size) property.
///
/// [`min_width`]: fn@min_width
/// [`max_width`]: fn@max_width
#[property(SIZE)]
pub fn width(child: impl UiNode, width: impl IntoVar<Length>) -> impl UiNode {
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
            let width = with_fill_metrics(|d| width.layout_dft_x(d.width));
            let c = LAYOUT.constraints();
            let width = c.x.clamp(width);
            let mut size = LAYOUT.with_constraints(c.with_exact_x(width), || wm.measure_block(child));
            size.width = width;
            *desired_size = size;
        }
        UiNodeOp::Layout { wl, final_size } => {
            let width = with_fill_metrics(|d| width.layout_dft_x(d.width));
            let c = LAYOUT.constraints();
            let width = c.x.clamp(width);
            let mut size = LAYOUT.with_constraints(c.with_exact_x(width), || child.layout(wl));
            size.width = width;
            *final_size = size;
        }
        _ => {}
    })
}

/// Exact height of the widget.
///
/// When set the widget is layout with exact size constraints, clamped by the contextual min/max.
/// Relative values are computed from the constraints maximum bounded height.
///
/// This property disables inline layout for the widget. This property sets the [`WIDGET_SIZE`] height.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// Button! {
///     background_color = rgb(255, 0, 0);
///     height = 300;
///     child = Text!("?x300 red");
/// }
/// # ;
/// ```
///
/// In the example the red button is set to a fixed size of `300` height.
///
/// # `size`
///
/// You can set both `width` and `height` at the same time using the [`size`](fn@size) property.
///
/// [`min_height`]: fn@min_height
/// [`max_height`]: fn@max_height
#[property(SIZE)]
pub fn height(child: impl UiNode, height: impl IntoVar<Length>) -> impl UiNode {
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
            let height = with_fill_metrics(|dft| height.layout_dft_y(dft.height));
            let c = LAYOUT.constraints();
            let height = c.y.clamp(height);
            let mut size = LAYOUT.with_constraints(c.with_new_exact_y(height), || wm.measure_block(child));
            size.height = height;
            *desired_size = size;
        }
        UiNodeOp::Layout { wl, final_size } => {
            let height = with_fill_metrics(|dft| height.layout_dft_y(dft.height));
            let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_new_exact_y(height), || child.layout(wl));
            size.height = height;
            *final_size = size;
        }
        _ => {}
    })
}

fn with_fill_metrics<R>(f: impl FnOnce(PxSize) -> R) -> R {
    let c = LAYOUT.constraints();
    let dft = c.fill_size();
    LAYOUT.with_constraints(c.with_fill_vector(c.is_bounded()), || f(dft))
}

/// Set or overwrite the baseline of the widget.
///
/// The `baseline` is a vertical offset from the bottom edge of the widget's inner bounds up, it defines the
/// line where the widget naturally *sits*, some widgets like [`Text!`] have a non-zero default baseline, most others leave it at zero.
///
/// Relative values are computed from the widget's height.
///
/// [`Text!`]: struct@crate::widgets::Text
#[property(BORDER, default(Length::Default))]
pub fn baseline(child: impl UiNode, baseline: impl IntoVar<Length>) -> impl UiNode {
    let baseline = baseline.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&baseline);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let size = child.layout(wl);

            let bounds = WIDGET.bounds();
            let inner_size = bounds.inner_size();
            let default = bounds.baseline();

            let baseline = LAYOUT.with_constraints(LAYOUT.constraints().with_max_size(inner_size).with_fill(true, true), || {
                baseline.layout_dft_y(default)
            });
            wl.set_baseline(baseline);

            *final_size = size;
        }
        _ => {}
    })
}

/// Retain the widget's previous width if the new layout width is smaller.
/// The widget is layout using its previous width as the minimum width constrain.
///
/// This property disables inline layout for the widget.
#[property(SIZE, default(false))]
pub fn sticky_width(child: impl UiNode, sticky: impl IntoVar<bool>) -> impl UiNode {
    let sticky = sticky.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&sticky);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            if !sticky.get() {
                return;
            }

            let min = WIDGET.bounds().inner_size().width;
            let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_min_x(min), || wm.measure_block(child));
            size.width = size.width.max(min);
            *desired_size = size;
        }
        UiNodeOp::Layout { wl, final_size } => {
            if !sticky.get() {
                return;
            }

            let min = WIDGET.bounds().inner_size().width;
            let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_min_x(min), || child.layout(wl));
            size.width = size.width.max(min);
            *final_size = size;
        }
        _ => {}
    })
}

/// Retain the widget's previous height if the new layout height is smaller.
/// The widget is layout using its previous height as the minimum height constrain.
///
/// This property disables inline layout for the widget.
#[property(SIZE, default(false))]
pub fn sticky_height(child: impl UiNode, sticky: impl IntoVar<bool>) -> impl UiNode {
    let sticky = sticky.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&sticky);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            if !sticky.get() {
                return;
            }

            let min = WIDGET.bounds().inner_size().height;
            let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_min_x(min), || wm.measure_block(child));
            size.height = size.height.max(min);
            *desired_size = size;
        }
        UiNodeOp::Layout { wl, final_size } => {
            if !sticky.get() {
                return;
            }

            let min = WIDGET.bounds().inner_size().height;
            let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_min_x(min), || child.layout(wl));
            size.height = size.height.max(min);
            *final_size = size;
        }
        _ => {}
    })
}

/// Retain the widget's previous size if the new layout size is smaller.
/// The widget is layout using its previous size as the minimum size constrain.
///
/// This property disables inline layout for the widget.
#[property(SIZE, default(false))]
pub fn sticky_size(child: impl UiNode, sticky: impl IntoVar<bool>) -> impl UiNode {
    let sticky = sticky.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&sticky);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            if !sticky.get() {
                return;
            }

            let min = WIDGET.bounds().inner_size();
            let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_min_size(min), || wm.measure_block(child));
            size = size.max(min);
            *desired_size = size;
        }
        UiNodeOp::Layout { wl, final_size } => {
            if !sticky.get() {
                return;
            }

            let min = WIDGET.bounds().inner_size();
            let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_min_size(min), || child.layout(wl));
            size = size.max(min);
            *final_size = size;
        }
        _ => {}
    })
}

/// Exact size property info.
///
/// Properties like [`size`], [`width`] and [`height`] set this metadata in the widget state.
/// Panels can use this info to implement [`Length::Leftover`] support.
///
/// [`size`]: fn@size
/// [`width`]: fn@width
/// [`height`]: fn@height
#[allow(non_camel_case_types)]
pub struct WIDGET_SIZE;
impl WIDGET_SIZE {
    /// Set the width state.
    pub fn set_width(&self, width: &Length) {
        WIDGET.with_state_mut(|mut state| {
            let width = width.into();
            match state.entry(&WIDGET_SIZE_ID) {
                state_map::StateMapEntry::Occupied(mut e) => e.get_mut().width = width,
                state_map::StateMapEntry::Vacant(e) => {
                    e.insert(euclid::size2(width, WidgetLength::Default));
                }
            }
        });
    }

    /// Set the height state.
    pub fn set_height(&self, height: &Length) {
        WIDGET.with_state_mut(|mut state| {
            let height = height.into();
            match state.entry(&WIDGET_SIZE_ID) {
                state_map::StateMapEntry::Occupied(mut e) => e.get_mut().height = height,
                state_map::StateMapEntry::Vacant(e) => {
                    e.insert(euclid::size2(WidgetLength::Default, height));
                }
            }
        })
    }

    /// Set the size state.
    pub fn set(&self, size: &Size) {
        WIDGET.set_state(&WIDGET_SIZE_ID, euclid::size2((&size.width).into(), (&size.height).into()));
    }

    /// Get the size set in the state.
    pub fn get(&self) -> euclid::Size2D<WidgetLength, ()> {
        WIDGET.get_state(&WIDGET_SIZE_ID).unwrap_or_default()
    }

    /// Get the size set in the widget state.
    pub fn get_wgt(&self, wgt: &mut impl UiNode) -> euclid::Size2D<WidgetLength, ()> {
        wgt.with_context(WidgetUpdateMode::Ignore, || self.get()).unwrap_or_default()
    }
}

static WIDGET_SIZE_ID: StaticStateId<euclid::Size2D<WidgetLength, ()>> = StaticStateId::new_unique();

/// Getter property, gets the latest rendered widget inner size.
#[property(LAYOUT)]
pub fn actual_size(child: impl UiNode, size: impl IntoVar<DipSize>) -> impl UiNode {
    let size = size.into_var();
    match_node(child, move |c, op| match op {
        UiNodeOp::Render { frame } => {
            c.render(frame);

            let f = frame.scale_factor();
            let s = WIDGET.info().bounds_info().inner_size().to_dip(f);
            if size.get() != s {
                let _ = size.set(s);
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            c.render_update(update);

            let info = WIDGET.info();
            let f = info.tree().scale_factor();
            let s = info.bounds_info().inner_size().to_dip(f);
            if size.get() != s {
                let _ = size.set(s);
            }
        }
        _ => {}
    })
}

/// Getter property, gets the latest rendered widget inner width in device independent pixels.
#[property(LAYOUT)]
pub fn actual_width(child: impl UiNode, width: impl IntoVar<Dip>) -> impl UiNode {
    let width = width.into_var();
    match_node(child, move |c, op| match op {
        UiNodeOp::Render { frame } => {
            c.render(frame);

            let f = frame.scale_factor();
            let w = WIDGET.info().bounds_info().inner_size().width.to_dip(f);
            if width.get() != w {
                let _ = width.set(w);
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            c.render_update(update);

            let info = WIDGET.info();
            let f = info.tree().scale_factor();
            let w = info.bounds_info().inner_size().width.to_dip(f);
            if width.get() != w {
                let _ = width.set(w);
            }
        }
        _ => {}
    })
}

/// Getter property, gets the latest rendered widget inner height.
#[property(LAYOUT)]
pub fn actual_height(child: impl UiNode, height: impl IntoVar<Dip>) -> impl UiNode {
    let height = height.into_var();
    match_node(child, move |c, op| match op {
        UiNodeOp::Render { frame } => {
            c.render(frame);

            let f = frame.scale_factor();
            let h = WIDGET.info().bounds_info().inner_size().height.to_dip(f);
            if height.get() != h {
                let _ = height.set(h);
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            c.render_update(update);

            let info = WIDGET.info();
            let f = info.tree().scale_factor();
            let h = info.bounds_info().inner_size().height.to_dip(f);
            if height.get() != h {
                let _ = height.set(h);
            }
        }
        _ => {}
    })
}

/// Getter property, gets the latest rendered widget inner size.
#[property(LAYOUT)]
pub fn actual_size_px(child: impl UiNode, size: impl IntoVar<PxSize>) -> impl UiNode {
    let size = size.into_var();
    match_node(child, move |c, op| match &op {
        UiNodeOp::Render { .. } | UiNodeOp::RenderUpdate { .. } => {
            c.op(op);
            let s = WIDGET.info().bounds_info().inner_size();
            if size.get() != s {
                // avoid pushing var changes every frame.
                let _ = size.set(s);
            }
        }
        _ => {}
    })
}

/// Getter property, gets the latest rendered widget inner width.
#[property(LAYOUT)]
pub fn actual_width_px(child: impl UiNode, width: impl IntoVar<Px>) -> impl UiNode {
    let width = width.into_var();
    match_node(child, move |c, op| match &op {
        UiNodeOp::Render { .. } | UiNodeOp::RenderUpdate { .. } => {
            c.op(op);
            let w = WIDGET.info().bounds_info().inner_size().width;
            if width.get() != w {
                let _ = width.set(w);
            }
        }
        _ => {}
    })
}

/// Getter property, gets the latest rendered widget inner height.
#[property(LAYOUT)]
pub fn actual_height_px(child: impl UiNode, height: impl IntoVar<Px>) -> impl UiNode {
    let height = height.into_var();
    match_node(child, move |c, op| match &op {
        UiNodeOp::Render { .. } | UiNodeOp::RenderUpdate { .. } => {
            c.op(op);
            let h = WIDGET.info().bounds_info().inner_size().height;
            if height.get() != h {
                let _ = height.set(h);
            }
        }
        _ => {}
    })
}

/// Getter property, gets the latest rendered widget inner transform.
#[property(LAYOUT)]
pub fn actual_transform(child: impl UiNode, transform: impl IntoVar<PxTransform>) -> impl UiNode {
    let transform = transform.into_var();
    match_node(child, move |c, op| match &op {
        UiNodeOp::Render { .. } | UiNodeOp::RenderUpdate { .. } => {
            c.op(op);
            let t = WIDGET.info().bounds_info().inner_transform();
            if transform.get() != t {
                let _ = transform.set(t);
            }
        }
        _ => {}
    })
}

/// Getter property, gets the latest rendered widget inner bounds in the window space.
#[property(LAYOUT)]
pub fn actual_bounds(child: impl UiNode, bounds: impl IntoVar<PxRect>) -> impl UiNode {
    let bounds = bounds.into_var();
    match_node(child, move |c, op| match &op {
        UiNodeOp::Render { .. } | UiNodeOp::RenderUpdate { .. } => {
            c.op(op);
            let t = WIDGET.info().bounds_info().inner_bounds();
            if bounds.get() != t {
                let _ = bounds.set(t);
            }
        }
        _ => {}
    })
}

/// Represents the width or height property value set on a widget.
///
/// Properties like [`size`], [`width`] and [`height`] set the [`WIDGET_SIZE`]
/// metadata in the widget state. Panels can use this info to implement [`Length::Leftover`] support.
///  
/// [`size`]: fn@size
/// [`width`]: fn@width
/// [`height`]: fn@height
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub enum WidgetLength {
    /// Evaluates to [`PxConstraints2d::fill_size`] when measured, can serve as a request for *size-to-fit*.
    ///
    /// The `Grid!` widget uses this to fit the column and row widgets to *their* cells, as they don't
    /// logically own the cells, this fit needs to be computed by the parent panel.
    #[default]
    Default,
    /// The [`Length::Leftover`] value. Evaluates to the [`LayoutMetrics::leftover`] value when measured, if
    /// a leftover value is not provided evaluates like a [`Length::Relative`].
    ///
    /// The *leftover* length needs to be computed by the parent panel, as it depends on the length of the sibling widgets,
    /// not just the panel constraints. Panels that support this, compute the value for each widget and measure/layout each using
    /// [`LAYOUT.with_leftover`] to inject the computed value.
    ///
    /// [`LAYOUT.with_leftover`]: crate::core::context::LAYOUT::with_leftover
    Leftover(Factor),
    /// Any of the other [`Length`] kinds. All contextual metrics needed to compute these values is already available
    /// in the [`LayoutMetrics`], panels that support [`Length::Leftover`] can layout this widget first to compute the
    /// leftover length.
    Exact,
}

impl From<&Length> for WidgetLength {
    fn from(value: &Length) -> Self {
        match value {
            Length::Default => WidgetLength::Default,
            Length::Leftover(f) => WidgetLength::Leftover(*f),
            _ => WidgetLength::Exact,
        }
    }
}
