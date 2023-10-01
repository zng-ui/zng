//! Properties that affect the widget render only.

use std::fmt;

use crate::core::gradient::{GradientRadius, GradientStops, LinearGradientAxis};
use crate::prelude::new_property::*;
use crate::widgets::{conic_gradient, flood, linear_gradient, radial_gradient};

use super::hit_test_mode;

/// Custom background property. Allows using any other widget as a background.
///
/// Backgrounds are not interactive, but are hit-testable, they don't influence the layout being measured and
/// arranged with the widget size, and they are always clipped to the widget bounds.
///
/// See also [`background_fn`] for use in styles.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # fn foo() -> impl UiNode { Wgt!() }
/// #
/// Container! {
///     child = foo();
///     background = Text! {
///         txt = "CUSTOM BACKGROUND";
///         font_size = 72;
///         font_color = web_colors::LIGHT_GRAY;
///         transform = rotate(45.deg());
///         align = Align::CENTER;
///     }
/// }
/// # ;
/// ```
///
/// The example renders a custom text background.
///
/// [`background_fn`]: fn@background_fn
#[property(FILL)]
pub fn background(child: impl UiNode, background: impl UiNode) -> impl UiNode {
    let background = interactive_node(background, false);
    let background = fill_node(background);

    match_node_list(ui_vec![background, child], |children, op| match op {
        UiNodeOp::Measure { wm, desired_size } => {
            *desired_size = children.with_node(1, |n| n.measure(wm));
        }
        UiNodeOp::Layout { wl, final_size } => {
            let size = children.with_node(1, |n| n.layout(wl));

            LAYOUT.with_constraints(PxConstraints2d::new_exact_size(size), || {
                children.with_node(0, |n| n.layout(wl));
            });
            *final_size = size;
        }
        _ => {}
    })
}

/// Custom background generated using a [`WidgetFn<()>`].
///
/// This is the equivalent of setting [`background`] to the [`presenter`] node, but if the property is cloned
/// in styles the `wgt_fn` will be called multiple times to create duplicates of the background nodes instead
/// of moving the node to the latest widget.
///
/// [`WidgetFn<()>`]: WidgetFn
/// [`background`]: fn@background
#[property(FILL, default(WidgetFn::nil()))]
pub fn background_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<()>>) -> impl UiNode {
    background(child, presenter((), wgt_fn))
}

/// Single color background property.
///
/// This property applies a [`flood`] as [`background`].
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # fn foo() -> impl UiNode { Wgt!() }
/// #
/// Container! {
///     child = foo();
///     background_color = hex!(#ADF0B0);
/// }
/// # ;
/// ```
///
/// [`background`]: fn@background
#[property(FILL, default(colors::BLACK.transparent()))]
pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    background(child, flood(color))
}

/// Linear gradient background property.
///
/// This property applies a [`linear_gradient`] as [`background`].
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # fn foo() -> impl UiNode { Wgt!() }
/// #
/// Container! {
///     child = foo();
///     background_gradient = {
///         axis: 90.deg(),
///         stops: [colors::BLACK, colors::WHITE],
///     }
/// }
/// # ;
/// ```
///
/// [`background`]: fn@background
#[property(FILL, default(0.deg(), {
    let c = colors::BLACK.transparent();
    crate::core::gradient::stops![c, c]
}))]
pub fn background_gradient(child: impl UiNode, axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    background(child, linear_gradient(axis, stops))
}

/// Radial gradient background property.
///
/// This property applies a [`radial_gradient`] as [`background`].
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # fn foo() -> impl UiNode { Wgt!() }
/// #
/// Container! {
///     child = foo();
///     background_radial = {
///         center: (50.pct(), 80.pct()),
///         radius: 100.pct(),
///         stops: [colors::BLACK, web_colors::DARK_ORANGE],
///     }
/// }
/// # ;
/// ```
///
/// [`background`]: fn@background
#[property(FILL, default((50.pct(), 50.pct()), 100.pct(), {
    let c = colors::BLACK.transparent();
    crate::core::gradient::stops![c, c]
}))]
pub fn background_radial(
    child: impl UiNode,
    center: impl IntoVar<Point>,
    radius: impl IntoVar<GradientRadius>,
    stops: impl IntoVar<GradientStops>,
) -> impl UiNode {
    background(child, radial_gradient(center, radius, stops))
}

/// Conic gradient background property.
///
/// This property applies a [`conic_gradient`] as [`background`].
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # fn foo() -> impl UiNode { Wgt!() }
/// #
/// Container! {
///     child = foo();
///     background_conic = {
///         center: (50.pct(), 80.pct()),
///         angle: 0.deg(),
///         stops: [colors::BLACK, web_colors::DARK_ORANGE],
///     }
/// }
/// # ;
/// ```
///
/// [`background`]: fn@background
#[property(FILL, default((50.pct(), 50.pct()), 0.deg(), {
    let c = colors::BLACK.transparent();
    crate::core::gradient::stops![c, c]
}))]
pub fn background_conic(
    child: impl UiNode,
    center: impl IntoVar<Point>,
    angle: impl IntoVar<AngleRadian>,
    stops: impl IntoVar<GradientStops>,
) -> impl UiNode {
    background(child, conic_gradient(center, angle, stops))
}

/// Custom foreground fill property. Allows using any other widget as a foreground overlay.
///
/// The foreground is rendered over the widget content and background and under the widget borders.
///
/// Foregrounds are not interactive, not hit-testable and don't influence the widget layout.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # fn foo() -> impl UiNode { Wgt!() }
/// #
/// Container! {
///     child = foo();
///     foreground = Text! {
///         txt = "TRIAL";
///         font_size = 72;
///         font_color = colors::BLACK;
///         opacity = 10.pct();
///         transform = rotate(45.deg());
///         align = Align::CENTER;
///     }
/// }
/// # ;
/// ```
///
/// The example renders a custom see-through text overlay.
#[property(FILL, default(crate::core::widget_instance::NilUiNode))]
pub fn foreground(child: impl UiNode, foreground: impl UiNode) -> impl UiNode {
    let foreground = interactive_node(foreground, false);
    let foreground = fill_node(foreground);
    let foreground = hit_test_mode(foreground, HitTestMode::Disabled);

    match_node_list(ui_vec![child, foreground], |children, op| match op {
        UiNodeOp::Measure { wm, desired_size } => {
            *desired_size = children.with_node(0, |n| n.measure(wm));
        }
        UiNodeOp::Layout { wl, final_size } => {
            let size = children.with_node(0, |n| n.layout(wl));
            LAYOUT.with_constraints(PxConstraints2d::new_exact_size(size), || {
                children.with_node(1, |n| n.layout(wl));
            });
            *final_size = size;
        }
        _ => {}
    })
}

/// Custom foreground generated using a [`WidgetFn<()>`].
///
/// This is the equivalent of setting [`foreground`] to the [`presenter`] node.
///
/// [`WidgetFn<()>`]: WidgetFn
/// [`foreground`]: fn@background
#[property(FILL, default(WidgetFn::nil()))]
pub fn foreground_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<()>>) -> impl UiNode {
    foreground(child, presenter((), wgt_fn))
}

/// Foreground highlight border overlay.
///
/// This property draws a border contour with extra `offsets` padding as an overlay.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # fn foo() -> impl UiNode { Wgt!() }
/// Container! {
///     child = foo();
///     foreground_highlight = {
///         offsets: 3,
///         widths: 1,
///         sides: colors::BLUE,
///     }
/// }
/// # ;
/// ```
///
/// The example renders a solid blue 1 pixel border overlay, the border lines are offset 3 pixels into the container.
#[property(FILL, default(0, 0, BorderStyle::Hidden))]
pub fn foreground_highlight(
    child: impl UiNode,
    offsets: impl IntoVar<SideOffsets>,
    widths: impl IntoVar<SideOffsets>,
    sides: impl IntoVar<BorderSides>,
) -> impl UiNode {
    let offsets = offsets.into_var();
    let widths = widths.into_var();
    let sides = sides.into_var();

    let mut render_bounds = PxRect::zero();
    let mut render_widths = PxSideOffsets::zero();
    let mut render_radius = PxCornerRadius::zero();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&offsets).sub_var_layout(&widths).sub_var_render(&sides);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let size = child.layout(wl);

            let radius = BORDER.inner_radius();
            let offsets = offsets.layout();
            let radius = radius.deflate(offsets);

            let mut bounds = PxRect::zero();
            if let Some(inline) = wl.inline() {
                if let Some(first) = inline.rows.iter().find(|r| !r.size.is_empty()) {
                    bounds = *first;
                }
            }
            if bounds.size.is_empty() {
                let border_offsets = BORDER.inner_offsets();

                bounds = PxRect::new(
                    PxPoint::new(offsets.left + border_offsets.left, offsets.top + border_offsets.top),
                    size - PxSize::new(offsets.horizontal(), offsets.vertical()),
                );
            }

            let widths = LAYOUT.with_constraints(PxConstraints2d::new_exact_size(size), || widths.layout());

            if render_bounds != bounds || render_widths != widths || render_radius != radius {
                render_bounds = bounds;
                render_widths = widths;
                render_radius = radius;
                WIDGET.render();
            }

            *final_size = size;
        }
        UiNodeOp::Render { frame } => {
            child.render(frame);
            frame.push_border(render_bounds, render_widths, sides.get(), render_radius);
        }
        _ => {}
    })
}

/// Fill color overlay property.
///
/// This property applies a [`flood`] as [`foreground`].
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # fn foo() -> impl UiNode { Wgt!() }
/// #
/// Container! {
///     child = foo();
///     foreground_color = rgba(0, 240, 0, 10.pct())
/// }
/// # ;
/// ```
///
/// The example adds a green tint to the container content.
///
/// [`foreground`]: fn@foreground
#[property(FILL, default(colors::BLACK.transparent()))]
pub fn foreground_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    foreground(child, flood(color))
}

/// Linear gradient overlay property.
///
/// This property applies a [`linear_gradient`] as [`foreground`] using the [`Clamp`] extend mode.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # fn foo() -> impl UiNode { Wgt!() }
/// #
/// Container! {
///     child = foo();
///     foreground_gradient = {
///         axis: (0, 0).to(0, 10),
///         stops: [colors::BLACK, colors::BLACK.transparent()]
///     }
/// }
/// # ;
/// ```
///
/// The example adds a *shadow* gradient to a 10px strip in the top part of the container content.
///
/// [`foreground`]: fn@foreground
/// [`Clamp`]: crate::core::gradient::ExtendMode::Clamp
#[property(FILL, default(0.deg(), {
    let c = colors::BLACK.transparent();
    crate::core::gradient::stops![c, c]
}))]
pub fn foreground_gradient(child: impl UiNode, axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    foreground(child, linear_gradient(axis, stops))
}

/// Clips the widget child to the area of the widget when set to `true`.
///
/// Any content rendered outside the widget inner bounds is clipped, hit test shapes are also clipped. The clip is
/// rectangular and can have rounded corners if [`corner_radius`] is set. If the widget is inlined during layout the first
/// row advance and last row trail are also clipped.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// #
/// Container! {
///     background_color = rgb(255, 0, 0);
///     size = (200, 300);
///     corner_radius = 5;
///     clip_to_bounds = true;
///     child = Container! {
///         background_color = rgb(0, 255, 0);
///         // fixed size ignores the layout available size.
///         size = (1000, 1000);
///         child = Text!("1000x1000 green clipped to 200x300");
///     };
/// }
/// # ;
/// ```
///
/// [`corner_radius`]: fn@corner_radius
#[property(FILL, default(false))]
pub fn clip_to_bounds(child: impl UiNode, clip: impl IntoVar<bool>) -> impl UiNode {
    let clip = clip.into_var();
    let mut corners = PxCornerRadius::zero();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.layout().render();
        }
        UiNodeOp::Update { .. } => {
            if clip.is_new() {
                WIDGET.layout().render();
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            let bounds = child.layout(wl);

            if clip.get() {
                let c = BORDER.border_radius();
                if c != corners {
                    corners = c;
                    WIDGET.render();
                }
            }

            *final_size = bounds;
        }
        UiNodeOp::Render { frame } => {
            if clip.get() {
                frame.push_clips(
                    |c| {
                        let wgt_bounds = WIDGET.bounds();
                        let bounds = PxRect::from_size(wgt_bounds.inner_size());

                        if corners != PxCornerRadius::zero() {
                            c.push_clip_rounded_rect(bounds, corners, false, true);
                        } else {
                            c.push_clip_rect(bounds, false, true);
                        }

                        if let Some(inline) = wgt_bounds.inline() {
                            for r in inline.negative_space().iter() {
                                c.push_clip_rect(*r, true, true);
                            }
                        };
                    },
                    |f| child.render(f),
                );
            } else {
                child.render(frame);
            }
        }
        _ => {}
    })
}

/// Inline mode explicitly selected for a widget.
///
/// See the [`inline`] property for more details.
///
/// [`inline`]: fn@inline
#[derive(Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum InlineMode {
    /// Widget does inline if requested by the parent widget layout and is composed only of properties that support inline.
    ///
    /// This is the default behavior.
    #[default]
    Allow,
    /// Widget always does inline.
    ///
    /// If the parent layout does not setup an inline layout environment the widget it-self will. This
    /// can be used to force the inline visual, such as background clipping or any other special visual
    /// that is only enabled when the widget is inlined.
    ///
    /// Note that the widget will only inline if composed only of properties that support inline.
    Inline,
    /// Widget disables inline.
    ///
    /// If the parent widget requests inline the request does not propagate for child nodes and
    /// inline is disabled on the widget.
    Block,
}
impl fmt::Debug for InlineMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "InlineMode::")?;
        }
        match self {
            Self::Allow => write!(f, "Allow"),
            Self::Inline => write!(f, "Inline"),
            Self::Block => write!(f, "Block"),
        }
    }
}
impl_from_and_into_var! {
    fn from(inline: bool) -> InlineMode {
        if inline {
            InlineMode::Inline
        } else {
            InlineMode::Block
        }
    }
}

/// Enforce an inline mode on the widget.
///
/// Set to [`InlineMode::Inline`] to use the inline layout and visual even if the widget
/// is not in an inlining parent. Note that the widget will still not inline if it has properties
/// that disable inlining.
///
/// Set to [`InlineMode::Block`] to ensure the widget layouts as a block item if the parent
/// is inlining.
///
/// Note that even if set to [`InlineMode::Inline`] the widget will only inline if all properties support
/// inlining.
#[property(WIDGET, default(InlineMode::Allow))]
pub fn inline(child: impl UiNode, mode: impl IntoVar<InlineMode>) -> impl UiNode {
    let mode = mode.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&mode);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            *desired_size = match mode.get() {
                InlineMode::Allow => child.measure(wm),
                InlineMode::Inline => {
                    if LAYOUT.inline_constraints().is_none() {
                        // enable inline for content.
                        wm.with_inline_visual(|wm| child.measure(wm))
                    } else {
                        // already enabled by parent
                        child.measure(wm)
                    }
                }
                InlineMode::Block => {
                    // disable inline, method also disables in `WidgetMeasure`
                    LAYOUT.disable_inline(wm, child)
                }
            };
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = match mode.get() {
                InlineMode::Allow => child.layout(wl),
                InlineMode::Inline => {
                    if LAYOUT.inline_constraints().is_none() {
                        wl.to_measure(None).with_inline_visual(|wm| child.measure(wm));
                        wl.with_inline_visual(|wl| child.layout(wl))
                    } else {
                        // already enabled by parent
                        child.layout(wl)
                    }
                }
                InlineMode::Block => {
                    if wl.inline().is_some() {
                        tracing::error!("inline enabled in `layout` when it signaled disabled in the previous `measure`");
                        LAYOUT.layout_block(wl, child)
                    } else {
                        child.layout(wl)
                    }
                }
            };
        }
        _ => {}
    })
}
