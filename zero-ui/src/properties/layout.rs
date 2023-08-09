//! Properties that affect the widget layout only.

use std::fmt;

use zero_ui::prelude::new_property::*;

/// Margin space around the widget.
///
/// This property adds side offsets to the widget inner visual, it will be combined with the other
/// layout properties of the widget to define the inner visual position and widget size.
///
/// This property disables inline layout for the widget.
///
/// Note that the margin is collapsed in a dimension if it is zero, that is margin top-bottom is zero if the widget
/// height is zero and margin left-right is zero if the widget width is zero.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// Button! {
///     margin = 10;
///     child = Text!("Click Me!")
/// }
/// # ;
/// ```
///
/// In the example the button has `10` layout pixels of space in all directions around it. You can
/// also control each side in specific:
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// Container! {
///     child = Button! {
///         margin = (10, 5.pct());
///         child = Text!("Click Me!")
///     };
///     margin = (1, 2, 3, 4);
/// }
/// # ;
/// ```
///
/// In the example the button has `10` pixels of space above and bellow and `5%` of the container width to the left and right.
/// The container itself has margin of `1` to the top, `2` to the right, `3` to the bottom and `4` to the left.
///
#[property(LAYOUT, default(0))]
pub fn margin(child: impl UiNode, margin: impl IntoVar<SideOffsets>) -> impl UiNode {
    let margin = margin.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&margin);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let margin = margin.layout();
            let size_increment = PxSize::new(margin.horizontal(), margin.vertical());
            *desired_size = LAYOUT.with_constraints(LAYOUT.constraints().with_less_size(size_increment), || {
                LAYOUT.disable_inline(wm, child)
            });
            if desired_size.width > Px(0) {
                desired_size.width += size_increment.width;
            }
            if desired_size.height > Px(0) {
                desired_size.height += size_increment.height;
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            let margin = margin.layout();
            let size_increment = PxSize::new(margin.horizontal(), margin.vertical());

            *final_size = LAYOUT.with_constraints(LAYOUT.constraints().with_less_size(size_increment), || child.layout(wl));
            let mut translate = PxVector::zero();
            if final_size.width > Px(0) {
                final_size.width += size_increment.width;
                translate.x = margin.left;
            }
            if final_size.height > Px(0) {
                final_size.height += size_increment.height;
                translate.y = margin.top;
            }
            wl.translate(translate);
        }
        _ => {}
    })
}

/// Margin space around the *content* of a widget.
///
/// This property is [`margin`](fn@margin) with nest group `CHILD_LAYOUT`.
#[property(CHILD_LAYOUT, default(0))]
pub fn padding(child: impl UiNode, padding: impl IntoVar<SideOffsets>) -> impl UiNode {
    margin(child, padding)
}

/// Aligns the widget within the available space.
///
/// This property disables inline layout for the widget.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// #
/// Container! {
///     child = Button! {
///         align = Align::TOP;
///         child = Text!("Click Me!")
///     };
/// }
/// # ;
/// ```
///
/// In the example the button is positioned at the top-center of the container. See [`Align`] for
/// more details.
#[property(LAYOUT, default(Align::FILL))]
pub fn align(child: impl UiNode, alignment: impl IntoVar<Align>) -> impl UiNode {
    let alignment = alignment.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&alignment);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let align = alignment.get();
            let child_size = LAYOUT.with_constraints(align.child_constraints(LAYOUT.constraints()), || LAYOUT.disable_inline(wm, child));
            *desired_size = align.measure(child_size, LAYOUT.constraints());
        }
        UiNodeOp::Layout { wl, final_size } => {
            let align = alignment.get();
            let child_size = LAYOUT.with_constraints(align.child_constraints(LAYOUT.constraints()), || child.layout(wl));
            *final_size = align.layout(child_size, LAYOUT.constraints(), LAYOUT.direction(), wl);
        }
        _ => {}
    })
}

/// Aligns the widget *content* within the available space.
///
/// This property is [`align`](fn@align) with nest group `CHILD_LAYOUT`.
#[property(CHILD_LAYOUT, default(Align::FILL))]
pub fn child_align(child: impl UiNode, alignment: impl IntoVar<Align>) -> impl UiNode {
    align(child, alignment)
}

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
            let size = LAYOUT.with_constraints(c.with_min_size(min), || LAYOUT.disable_inline(wm, child));
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
            let mut size = LAYOUT.with_constraints(c.with_min_x(min), || LAYOUT.disable_inline(wm, child));
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
            let mut size = LAYOUT.with_constraints(c.with_min_y(min), || LAYOUT.disable_inline(wm, child));
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
            let size = LAYOUT.with_constraints(LAYOUT.constraints().with_max_size(max), || LAYOUT.disable_inline(wm, child));
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

            let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_max_x(max), || LAYOUT.disable_inline(wm, child));
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

            let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_max_y(max), || LAYOUT.disable_inline(wm, child));
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
            LAYOUT.disable_inline(wm, &mut NilUiNode); // no need to actually measure child
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
            let mut size = LAYOUT.with_constraints(c.with_exact_x(width), || LAYOUT.disable_inline(wm, child));
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
            let mut size = LAYOUT.with_constraints(c.with_new_exact_y(height), || LAYOUT.disable_inline(wm, child));
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
            let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_min_x(min), || LAYOUT.disable_inline(wm, child));
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
            let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_min_x(min), || LAYOUT.disable_inline(wm, child));
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
            let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_min_size(min), || LAYOUT.disable_inline(wm, child));
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

/// Placement of a node inserted by the [`child_insert`] property.
///
/// [`child_insert`]: fn@child_insert
#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ChildInsertPlace {
    /// Insert node above the child.
    Above,
    /// Insert node to the right of child.
    Right,
    /// Insert node below the child.
    Below,
    /// Insert node to the left of child.
    Left,

    /// Insert node to the left of child in [`LayoutDirection::LTR`] contexts and to the right of child
    /// in [`LayoutDirection::RTL`] contexts.
    Start,
    /// Insert node to the right of child in [`LayoutDirection::LTR`] contexts and to the left of child
    /// in [`LayoutDirection::RTL`] contexts.
    End,
}
impl fmt::Debug for ChildInsertPlace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "ChildInsertPlace::")?;
        }
        match self {
            Self::Above => write!(f, "Above"),
            Self::Right => write!(f, "Right"),
            Self::Below => write!(f, "Below"),
            Self::Left => write!(f, "Left"),
            Self::Start => write!(f, "Start"),
            Self::End => write!(f, "End"),
        }
    }
}
impl ChildInsertPlace {
    /// Convert [`ChildInsertPlace::Start`] and [`ChildInsertPlace::End`] to the fixed place they represent in the `direction` context.
    pub fn resolve_direction(self, direction: LayoutDirection) -> Self {
        match self {
            Self::Start => match direction {
                LayoutDirection::LTR => Self::Left,
                LayoutDirection::RTL => Self::Right,
            },
            Self::End => match direction {
                LayoutDirection::LTR => Self::Right,
                LayoutDirection::RTL => Self::Left,
            },
            p => p,
        }
    }

    /// Inserted node is to the left or right of child.
    pub fn is_x_axis(self) -> bool {
        !matches!(self, Self::Above | Self::Below)
    }

    /// Inserted node is above or bellow the child node.
    pub fn is_y_axis(self) -> bool {
        matches!(self, Self::Above | Self::Below)
    }
}

/// Insert the `insert` node in the `place` relative to the widget's child.
///
/// This property disables inline layout for the widget.
#[property(CHILD, default(ChildInsertPlace::Start, NilUiNode, 0))]
pub fn child_insert(
    child: impl UiNode,
    place: impl IntoVar<ChildInsertPlace>,
    insert: impl UiNode,
    spacing: impl IntoVar<Length>,
) -> impl UiNode {
    let place = place.into_var();
    let spacing = spacing.into_var();
    let offset_key = FrameValueKey::new_unique();
    let mut offset_child = 0;
    let mut offset = PxVector::zero();

    match_node_list(ui_vec![child, insert], move |children, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&place).sub_var_layout(&spacing);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let c = LAYOUT.constraints();
            *desired_size = if place.get().is_x_axis() {
                let mut spacing = spacing.layout_x();
                let insert_size = children.with_node(1, |n| {
                    LAYOUT.with_constraints(c.with_new_min(Px(0), Px(0)).with_fill_x(false), || LAYOUT.disable_inline(wm, n))
                });
                if insert_size.width == Px(0) {
                    spacing = Px(0);
                }
                let child_size = children.with_node(0, |n| {
                    LAYOUT.with_constraints(c.with_less_x(insert_size.width + spacing), || LAYOUT.disable_inline(wm, n))
                });

                PxSize::new(
                    insert_size.width + spacing + child_size.width,
                    insert_size.height.max(child_size.height),
                )
            } else {
                let mut spacing = spacing.layout_y();
                let insert_size = children.with_node(1, |n| {
                    LAYOUT.with_constraints(c.with_new_min(Px(0), Px(0)).with_fill_y(false), || LAYOUT.disable_inline(wm, n))
                });
                if insert_size.height == Px(0) {
                    spacing = Px(0);
                }
                let child_size = children.with_node(0, |n| {
                    LAYOUT.with_constraints(c.with_less_y(insert_size.height + spacing), || LAYOUT.disable_inline(wm, n))
                });
                if child_size.height == Px(0) {
                    spacing = Px(0);
                }
                PxSize::new(
                    insert_size.width.max(child_size.width),
                    insert_size.height + spacing + child_size.height,
                )
            };
        }
        UiNodeOp::Layout { wl, final_size } => {
            let place = place.get().resolve_direction(LAYOUT.direction());
            let c = LAYOUT.constraints();

            *final_size = match place {
                ChildInsertPlace::Left | ChildInsertPlace::Right => {
                    let spacing = spacing.layout_x();

                    let mut constraints_y = LAYOUT.constraints().y;
                    if constraints_y.fill_or_exact().is_none() {
                        // measure to find fill height
                        let mut wm = wl.to_measure(None);
                        let wm = &mut wm;
                        let mut spacing = spacing;

                        let insert_size = children.with_node(1, |n| {
                            LAYOUT.with_constraints(c.with_new_min(Px(0), Px(0)).with_fill_x(false), || n.measure(wm))
                        });
                        if insert_size.width == Px(0) {
                            spacing = Px(0);
                        }
                        let child_size = children.with_node(0, |n| {
                            LAYOUT.with_constraints(c.with_less_x(insert_size.width + spacing), || n.measure(wm))
                        });

                        constraints_y = constraints_y.with_fill(true).with_max(child_size.height.max(insert_size.height));
                    }

                    let mut spacing = spacing;
                    let insert_size = children.with_node(1, |n| {
                        LAYOUT.with_constraints(
                            {
                                let mut c = c;
                                c.y = constraints_y;
                                c.with_new_min(Px(0), Px(0)).with_fill_x(false)
                            },
                            || n.layout(wl),
                        )
                    });
                    if insert_size.width == Px(0) {
                        spacing = Px(0);
                    }
                    let child_size = children.with_node(0, |n| {
                        LAYOUT.with_constraints(
                            {
                                let mut c = c;
                                c.y = constraints_y;
                                c.with_less_x(insert_size.width + spacing)
                            },
                            || n.layout(wl),
                        )
                    });
                    if child_size.width == Px(0) {
                        spacing = Px(0);
                    }

                    // position
                    let (child, o) = match place {
                        ChildInsertPlace::Left => (0, insert_size.width + spacing),
                        ChildInsertPlace::Right => (1, child_size.width + spacing),
                        _ => unreachable!(),
                    };
                    let o = PxVector::new(o, Px(0));
                    if offset != o || offset_child != child {
                        offset_child = child;
                        offset = o;
                        WIDGET.render_update();
                    }

                    PxSize::new(
                        insert_size.width + spacing + child_size.width,
                        insert_size.height.max(child_size.height),
                    )
                }
                ChildInsertPlace::Above | ChildInsertPlace::Below => {
                    let spacing = spacing.layout_y();

                    let mut constraints_x = c.x;
                    if constraints_x.fill_or_exact().is_none() {
                        // measure fill width

                        let mut wm = wl.to_measure(None);
                        let wm = &mut wm;
                        let mut spacing = spacing;

                        let insert_size = children.with_node(1, |n| {
                            LAYOUT.with_constraints(c.with_new_min(Px(0), Px(0)).with_fill_y(false), || n.measure(wm))
                        });
                        if insert_size.height == Px(0) {
                            spacing = Px(0);
                        }
                        let child_size = children.with_node(0, |n| {
                            LAYOUT.with_constraints(c.with_less_y(insert_size.height + spacing), || n.measure(wm))
                        });

                        constraints_x = constraints_x.with_fill(true).with_max(child_size.width.max(insert_size.width));
                    }

                    let mut spacing = spacing;
                    let insert_size = children.with_node(1, |n| {
                        LAYOUT.with_constraints(
                            {
                                let mut c = c;
                                c.x = constraints_x;
                                c.with_new_min(Px(0), Px(0)).with_fill_y(false)
                            },
                            || n.layout(wl),
                        )
                    });
                    if insert_size.height == Px(0) {
                        spacing = Px(0);
                    }
                    let child_size = children.with_node(0, |n| {
                        LAYOUT.with_constraints(
                            {
                                let mut c = c;
                                c.x = constraints_x;
                                c.with_less_y(insert_size.height + spacing)
                            },
                            || n.layout(wl),
                        )
                    });

                    // position
                    let (child, o) = match place {
                        ChildInsertPlace::Above => (0, insert_size.height + spacing),
                        ChildInsertPlace::Below => (1, child_size.height + spacing),
                        _ => unreachable!(),
                    };
                    let o = PxVector::new(Px(0), o);
                    if offset != o || offset_child != child {
                        offset_child = child;
                        offset = o;
                        WIDGET.render_update();
                    }

                    PxSize::new(
                        insert_size.width.max(child_size.width),
                        insert_size.height + spacing + child_size.height,
                    )
                }
                _ => {
                    unreachable!()
                }
            };
        }
        UiNodeOp::Render { frame } => children.for_each(|i, child| {
            if i as u8 == offset_child {
                frame.push_reference_frame(
                    offset_key.into(),
                    offset_key.bind(offset.into(), false),
                    TransformStyle::Flat,
                    true,
                    true,
                    |frame| {
                        child.render(frame);
                    },
                );
            } else {
                child.render(frame);
            }
        }),
        UiNodeOp::RenderUpdate { update } => {
            children.for_each(|i, child| {
                if i as u8 == offset_child {
                    update.with_transform(offset_key.update(offset.into(), false), true, |update| {
                        child.render_update(update);
                    });
                } else {
                    child.render_update(update);
                }
            });
        }
        _ => {}
    })
}

/// Insert the `insert` node in the `place` relative to the widget's child, but outside of the child layout.
///
/// This is still *inside* the parent widget, but outside of properties like padding.
///
/// This property disables inline layout for the widget.
#[property(CHILD_LAYOUT - 1, default(ChildInsertPlace::Start, NilUiNode, 0))]
pub fn child_out_insert(
    child: impl UiNode,
    place: impl IntoVar<ChildInsertPlace>,
    insert: impl UiNode,
    spacing: impl IntoVar<Length>,
) -> impl UiNode {
    child_insert(child, place, insert, spacing)
}

/// Insert `insert` to the left of the widget's child.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(NilUiNode, 0))]
pub fn child_insert_left(child: impl UiNode, insert: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    child_insert(child, ChildInsertPlace::Left, insert, spacing)
}

/// Insert `insert` to the right of the widget's child.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(NilUiNode, 0))]
pub fn child_insert_right(child: impl UiNode, insert: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    child_insert(child, ChildInsertPlace::Right, insert, spacing)
}

/// Insert `insert` above the widget's child.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(NilUiNode, 0))]
pub fn child_insert_above(child: impl UiNode, insert: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    child_insert(child, ChildInsertPlace::Above, insert, spacing)
}

/// Insert `insert` below the widget's child.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(NilUiNode, 0))]
pub fn child_insert_below(child: impl UiNode, insert: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    child_insert(child, ChildInsertPlace::Below, insert, spacing)
}

/// Insert `insert` to the left of the widget's child in LTR contexts or to the right of the widget's child in RTL contexts.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(NilUiNode, 0))]
pub fn child_insert_start(child: impl UiNode, insert: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    child_insert(child, ChildInsertPlace::Start, insert, spacing)
}

/// Insert `insert` to the right of the widget's child in LTR contexts or to the right of the widget's child in RTL contexts.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(NilUiNode, 0))]
pub fn child_insert_end(child: impl UiNode, insert: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    child_insert(child, ChildInsertPlace::End, insert, spacing)
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

            let f = frame.scale_factor().0;
            let s = WIDGET.info().bounds_info().inner_size().to_dip(f);
            if size.get() != s {
                let _ = size.set(s);
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            c.render_update(update);

            let info = WIDGET.info();
            let f = info.tree().scale_factor().0;
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

            let f = frame.scale_factor().0;
            let w = WIDGET.info().bounds_info().inner_size().width.to_dip(f);
            if width.get() != w {
                let _ = width.set(w);
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            c.render_update(update);

            let info = WIDGET.info();
            let f = info.tree().scale_factor().0;
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

            let f = frame.scale_factor().0;
            let h = WIDGET.info().bounds_info().inner_size().height.to_dip(f);
            if height.get() != h {
                let _ = height.set(h);
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            c.render_update(update);

            let info = WIDGET.info();
            let f = info.tree().scale_factor().0;
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
