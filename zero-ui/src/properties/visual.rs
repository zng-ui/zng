//! Properties that affect the widget render only.

use crate::core::gradient::{GradientRadius, GradientStops, LinearGradientAxis};
use crate::prelude::new_property::*;
use crate::widgets::{conic_gradient, flood, linear_gradient, radial_gradient};

use super::hit_test_mode;

/// Custom background property. Allows using any other widget as a background.
///
/// Backgrounds are not interactive, but are hit-testable, they don't influence the layout being measured and
/// arranged with the widget size, and they are always clipped to the widget bounds.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # fn foo() -> impl UiNode { wgt!() }
/// #
/// container! {
///     child = foo();
///     background = text! {
///         txt = "CUSTOM BACKGROUND";
///         font_size = 72;
///         txt_color = colors::LIGHT_GRAY;
///         transform = rotate(45.deg());
///         align = Align::CENTER;
///     }
/// }
/// # ;
/// ```
///
/// The example renders a custom text background.
#[property(FILL)]
pub fn background(child: impl UiNode, background: impl UiNode) -> impl UiNode {
    #[ui_node(struct BackgroundNode {
        children: impl UiNodeList,
    })]
    impl UiNode for BackgroundNode {
        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            self.children.with_node(1, |n| n.measure(ctx, wm))
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let size = self.children.with_node_mut(1, |n| n.layout(ctx, wl));
            ctx.with_constrains(
                |c| PxConstrains2d::new_exact_size(c.fill_size_or(size)),
                |ctx| {
                    self.children.with_node_mut(0, |n| n.layout(ctx, wl));
                },
            );
            size
        }
    }

    let background = interactive_node(background, false);
    let background = fill_node(background);

    BackgroundNode {
        children: ui_vec![background, child],
    }
}

/// Custom background generated using a [`WidgetGenerator<()>`].
///
/// This is the equivalent of setting [`background`] to the [`presenter_default`] node.
///
/// [`WidgetGenerator<()>`]: WidgetGenerator
/// [`background`]: fn@background
/// [`presenter_default`]: WidgetGenerator::presenter_default
#[property(FILL, default(WidgetGenerator::nil()))]
pub fn background_gen(child: impl UiNode, generator: impl IntoVar<WidgetGenerator<()>>) -> impl UiNode {
    background(child, WidgetGenerator::presenter_default(generator))
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
/// # fn foo() -> impl UiNode { wgt!() }
/// #
/// container! {
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
/// # fn foo() -> impl UiNode { wgt!() }
/// #
/// container! {
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
/// # fn foo() -> impl UiNode { wgt!() }
/// #
/// container! {
///     child = foo();
///     background_radial = {
///         center: (50.pct(), 80.pct()),
///         radius: 100.pct(),
///         stops: [colors::BLACK, colors::DARK_ORANGE],
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
/// # fn foo() -> impl UiNode { wgt!() }
/// #
/// container! {
///     child = foo();
///     background_conic = {
///         center: (50.pct(), 80.pct()),
///         angle: 0.deg(),
///         stops: [colors::BLACK, colors::DARK_ORANGE],
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
/// # fn foo() -> impl UiNode { wgt!() }
/// #
/// container! {
///     child = foo();
///     foreground = text! {
///         txt = "TRIAL";
///         font_size = 72;
///         txt_color = colors::BLACK;
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
    #[ui_node(struct ForegroundNode {
        children: impl UiNodeList,
    })]
    impl UiNode for ForegroundNode {
        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            self.children.with_node(0, |n| n.measure(ctx, wm))
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let size = self.children.with_node_mut(0, |n| n.layout(ctx, wl));
            ctx.with_constrains(
                |c| PxConstrains2d::new_exact_size(c.fill_size_or(size)),
                |ctx| {
                    self.children.with_node_mut(1, |n| n.layout(ctx, wl));
                },
            );
            size
        }
    }

    let foreground = interactive_node(foreground, false);
    let foreground = fill_node(foreground);
    let foreground = hit_test_mode(foreground, HitTestMode::Disabled);

    ForegroundNode {
        children: ui_vec![child, foreground],
    }
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
/// # fn foo() -> impl UiNode { wgt!() }
/// container! {
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
    #[ui_node(struct ForegroundHighlightNode {
        child: impl UiNode,
        #[var] offsets: impl Var<SideOffsets>,
        #[var] widths: impl Var<SideOffsets>,
        #[var] sides: impl Var<BorderSides>,

        render_bounds: PxRect,
        render_widths: PxSideOffsets,
        render_radius: PxCornerRadius,
    })]
    impl UiNode for ForegroundHighlightNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.offsets.is_new() || self.widths.is_new() {
                WIDGET.layout();
            } else if self.sides.is_new() {
                WIDGET.render();
            }
            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            self.child.measure(ctx, wm)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let size = self.child.layout(ctx, wl);

            let radius = ContextBorders::inner_radius(ctx);
            let offsets = self.offsets.get().layout(ctx.metrics, |_| PxSideOffsets::zero());
            let radius = radius.deflate(offsets);

            let mut bounds = PxRect::zero();
            if let Some(inline) = wl.inline() {
                if let Some(first) = inline.rows.iter().find(|r| !r.size.is_empty()) {
                    bounds = *first;
                }
            }
            if bounds.size.is_empty() {
                let border_offsets = ContextBorders::inner_offsets(ctx.path.widget_id());

                bounds = PxRect::new(
                    PxPoint::new(offsets.left + border_offsets.left, offsets.top + border_offsets.top),
                    size - PxSize::new(offsets.horizontal(), offsets.vertical()),
                );
            }

            let widths = ctx.with_constrains(
                |c| PxConstrains2d::new_exact_size(c.fill_size_or(size)),
                |ctx| self.widths.get().layout(ctx.metrics, |_| PxSideOffsets::zero()),
            );

            if self.render_bounds != bounds || self.render_widths != widths || self.render_radius != radius {
                self.render_bounds = bounds;
                self.render_widths = widths;
                self.render_radius = radius;
                WIDGET.render();
            }

            size
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.child.render(ctx, frame);
            frame.push_border(self.render_bounds, self.render_widths, self.sides.get(), self.render_radius);
        }
    }
    ForegroundHighlightNode {
        child: child.cfg_boxed(),
        offsets: offsets.into_var(),
        widths: widths.into_var(),
        sides: sides.into_var(),

        render_bounds: PxRect::zero(),
        render_widths: PxSideOffsets::zero(),
        render_radius: PxCornerRadius::zero(),
    }
    .cfg_boxed()
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
/// # fn foo() -> impl UiNode { wgt!() }
/// #
/// container! {
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
/// # fn foo() -> impl UiNode { wgt!() }
/// #
/// container! {
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
/// container! {
///     background_color = rgb(255, 0, 0);
///     size = (200, 300);
///     corner_radius = 5;
///     clip_to_bounds = true;
///     child = container! {
///         background_color = rgb(0, 255, 0);
///         // fixed size ignores the layout available size.
///         size = (1000, 1000);
///         child = text!("1000x1000 green clipped to 200x300");
///     };
/// }
/// # ;
/// ```
///
/// [`corner_radius`]: fn@corner_radius
#[property(FILL, default(false))]
pub fn clip_to_bounds(child: impl UiNode, clip: impl IntoVar<bool>) -> impl UiNode {
    #[ui_node(struct ClipToBoundsNode {
        child: impl UiNode,
        #[var] clip: impl Var<bool>,
        corners: PxCornerRadius,
    })]
    impl UiNode for ClipToBoundsNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.clip.is_new() {
                WIDGET.layout().render();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            self.child.measure(ctx, wm)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let bounds = self.child.layout(ctx, wl);

            if self.clip.get() {
                let corners = ContextBorders::border_radius(ctx);
                if corners != self.corners {
                    self.corners = corners;
                    WIDGET.render();
                }
            }

            bounds
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            if self.clip.get() {
                frame.push_clips(
                    |c| {
                        let bounds = PxRect::from_size(ctx.widget_info.bounds.inner_size());

                        if self.corners != PxCornerRadius::zero() {
                            c.push_clip_rounded_rect(bounds, self.corners, false, true);
                        } else {
                            c.push_clip_rect(bounds, false, true);
                        }

                        if let Some(inline) = ctx.widget_info.bounds.inline() {
                            for r in inline.negative_space().iter() {
                                c.push_clip_rect(*r, true, true);
                            }
                        }
                    },
                    |f| self.child.render(ctx, f),
                );
            } else {
                self.child.render(ctx, frame);
            }
        }
    }
    ClipToBoundsNode {
        child,
        clip: clip.into_var(),
        corners: PxCornerRadius::zero(),
    }
}

/// Force widget to do inline layout when it is not inside a parent doing inline layout.
///
/// Widgets that support inlining can have different visuals when inlined, such as multiple *row* backgrounds. This
/// property forces the widget to enter this mode by enabling inlining in the layout context if it is not already.
#[property(CONTEXT-1, default(false))]
pub fn inline(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    #[ui_node(struct InlineNode {
        child: impl UiNode,
        enabled: impl Var<bool>,
    })]
    impl UiNode for InlineNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);
            if self.enabled.is_new() {
                WIDGET.layout();
            }
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            if self.enabled.get() && ctx.inline_constrains().is_none() {
                let c = InlineConstrainsMeasure {
                    first_max: ctx.constrains().x.max_or(Px::MAX),
                    mid_clear_min: Px(0),
                };
                ctx.with_inline_constrains(wm, move |_| Some(c), |ctx, wm| self.child.measure(ctx, wm))
            } else {
                self.child.measure(ctx, wm)
            }
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            if self.enabled.get() && ctx.inline_constrains().is_none() {
                if let Some(c) = ctx.widget_info.bounds.measure_inline() {
                    let c = InlineConstrainsLayout {
                        first: PxRect::from_size(c.first),
                        mid_clear: Px(0),
                        last: PxRect::from_size(c.last),
                        first_segs: Default::default(),
                        last_segs: Default::default(),
                    };
                    return ctx.with_inline_constrains(move |_| Some(c), |ctx| self.child.layout(ctx, wl));
                }
            }
            self.child.layout(ctx, wl)
        }
    }
    InlineNode {
        child,
        enabled: enabled.into_var(),
    }
}
