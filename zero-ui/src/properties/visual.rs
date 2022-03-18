//! Properties that affect the widget render only.

use crate::core::gradient::{GradientStops, LinearGradientAxis};
use crate::prelude::new_property::*;
use crate::widgets::{fill_color, linear_gradient};

use super::{hit_test_mode, interactive};

/// Custom background property. Allows using any other widget as a background.
///
/// Backgrounds are not interactive and don't influence the widget layout but they are hit-testable.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
/// # fn foo() -> impl Widget { blank!() }
///
/// container! {
///     content = foo();
///     background = text! {
///         text = "CUSTOM BACKGROUND";
///         font_size = 72;
///         color = colors::LIGHT_GRAY;
///         transform = rotate(45.deg());
///         align = Align::CENTER;
///     }
/// }
/// # ;
/// ```
///
/// The example renders a custom text background.
#[property(fill, allowed_in_when = false, default(crate::core::NilUiNode))]
pub fn background(child: impl UiNode, background: impl UiNode) -> impl UiNode {
    struct BackgroundNode<C> {
        /// [background, child]
        children: C,
    }
    #[impl_ui_node(children)]
    impl<C: UiNodeList> UiNode for BackgroundNode<C> {
        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let available_size = self.children.widget_measure(1, ctx, available_size);
            self.children.widget_measure(0, ctx, AvailableSize::finite(available_size));
            available_size
        }
    }

    let background = fill_node(background);
    let background = interactive(background, false);

    BackgroundNode {
        children: nodes![background, child],
    }
}

/// Custom background generated using a [`ViewGenerator<()>`].
///
/// This is the equivalent of setting [`background`] to the [`presenter_default`] node.
///
/// [`ViewGenerator<()>`]: ViewGenerator
/// [`background`]: fn@background
/// [`presenter_default`]: ViewGenerator::presenter_default
#[property(fill, default(ViewGenerator::nil()))]
pub fn background_gen(child: impl UiNode, generator: impl IntoVar<ViewGenerator<()>>) -> impl UiNode {
    background(child, ViewGenerator::presenter_default(generator))
}

/// Single color background property.
///
/// This property applies a [`fill_color`] as [`background`].
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
/// # fn foo() -> impl Widget { blank!() }
///
/// container! {
///     content = foo();
///     background_color = hex!(#ADF0B0);
/// }
/// # ;
/// ```
///
/// [`background`]: fn@background
#[property(fill, default(colors::BLACK.transparent()))]
pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    background(child, fill_color(color))
}

/// Linear gradient background property.
///
/// This property applies a [`linear_gradient`] as [`background`].
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
/// # fn foo() -> impl Widget { blank!() }
///
/// container! {
///     content = foo();
///     background_gradient = {
///         axis: 90.deg(),
///         stops: [colors::BLACK, colors::WHITE]
///     }
/// }
/// # ;
/// ```
///
/// [`background`]: fn@background
#[property(fill, default(0.deg(), {
    let c = colors::BLACK.transparent();
    crate::core::gradient::stops![c, c]
}))]
pub fn background_gradient(child: impl UiNode, axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    background(child, linear_gradient(axis, stops))
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
/// use zero_ui::prelude::*;
/// # fn foo() -> impl Widget { blank!() }
///
/// container! {
///     content = foo();
///     foreground = text! {
///         text = "TRIAL";
///         font_size = 72;
///         color = colors::BLACK;
///         opacity = 10.pct();
///         transform = rotate(45.deg());
///         align = Align::CENTER;
///     }
/// }
/// # ;
/// ```
///
/// The example renders a custom see-through text overlay.
#[property(fill, allowed_in_when = false, default(crate::core::NilUiNode))]
pub fn foreground(child: impl UiNode, foreground: impl UiNode) -> impl UiNode {
    struct ForegroundNode<C> {
        children: C,
    }
    #[impl_ui_node(children)]
    impl<C: UiNodeList> UiNode for ForegroundNode<C> {
        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let available_size = self.children.widget_measure(0, ctx, available_size);
            self.children.widget_measure(1, ctx, AvailableSize::finite(available_size));
            available_size
        }
    }

    let foreground = fill_node(foreground);
    let foreground = hit_test_mode(foreground, HitTestMode::Disabled);
    let foreground = interactive(foreground, false);

    ForegroundNode {
        children: nodes![child, foreground],
    }
}

/// Foreground highlight border overlay.
///
/// This property draws a border contour with extra `offsets` padding as a fill overlay.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
/// # fn foo() -> impl Widget { blank!() }
/// container! {
///     content = foo();
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
#[property(fill, default(0, 0, BorderStyle::Hidden))]
pub fn foreground_highlight(
    child: impl UiNode,
    offsets: impl IntoVar<SideOffsets>,
    widths: impl IntoVar<SideOffsets>,
    sides: impl IntoVar<BorderSides>,
) -> impl UiNode {
    struct ForegroundHighlightNode<C, O, W, S> {
        child: C,
        offsets: O,
        widths: W,
        sides: S,

        final_widths: PxSideOffsets,
        final_rect: PxRect,
        final_radius: PxCornerRadius,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, O: Var<SideOffsets>, W: Var<SideOffsets>, S: Var<BorderSides>> UiNode for ForegroundHighlightNode<C, O, W, S> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.vars(ctx).var(&self.offsets).var(&self.widths).var(&self.sides);

            self.child.subscriptions(ctx, subscriptions);
        }
        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            if self.offsets.is_new(ctx) {
                ctx.updates.layout()
            }
            if self.widths.is_new(ctx) {
                ctx.updates.layout()
            }
            if self.sides.is_new(ctx) {
                ctx.updates.render();
            }
        }
        fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
            self.child.arrange(ctx, widget_layout, final_size);

            let available_size = AvailableSize::finite(final_size);
            let final_offsets = self
                .offsets
                .get(ctx.vars)
                .to_layout(ctx.metrics, available_size, PxSideOffsets::zero());

            self.final_widths = self
                .widths
                .get(ctx.vars)
                .to_layout(ctx.metrics, available_size, PxSideOffsets::zero());

            let diff = PxSize::new(final_offsets.horizontal(), final_offsets.vertical());

            let border_offsets = widget_layout.border_offsets();

            self.final_rect.origin = PxPoint::new(final_offsets.left + border_offsets.left, final_offsets.top + border_offsets.top);
            self.final_rect.size = final_size - diff;
            self.final_radius = widget_layout.corner_radius();
        }
        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.child.render(ctx, frame);
            frame.with_hit_tests_disabled(|f| {
                f.push_border(self.final_rect, self.final_widths, self.sides.copy(ctx), self.final_radius);
            })
        }
    }
    ForegroundHighlightNode {
        child,
        offsets: offsets.into_var(),
        widths: widths.into_var(),
        sides: sides.into_var(),

        final_widths: PxSideOffsets::zero(),
        final_rect: PxRect::zero(),
        final_radius: PxCornerRadius::zero(),
    }
}

/// Fill color overlay property.
///
/// This property applies a [`fill_color`] as [`foreground`].
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
/// # fn foo() -> impl Widget { blank!() }
///
/// container! {
///     content = foo();
///     foreground_color = rgba(0, 240, 0, 10.pct())
/// }
/// # ;
/// ```
///
/// The example adds a green tint to the container content.
///
/// [`foreground`]: fn@foreground
#[property(fill, default(colors::BLACK.transparent()))]
pub fn foreground_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    foreground(child, fill_color(color))
}

/// Linear gradient overlay property.
///
/// This property applies a [`linear_gradient`] as [`foreground`] using the [`Clamp`] extend mode.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
/// # fn foo() -> impl Widget { blank!() }
///
/// container! {
///     content = foo();
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
#[property(fill, default(0.deg(), {
    let c = colors::BLACK.transparent();
    crate::core::gradient::stops![c, c]
}))]
pub fn foreground_gradient(child: impl UiNode, axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    foreground(child, linear_gradient(axis, stops))
}

/// Clips the widget child to the area of the widget when set to `true`.
///
/// Any content rendered outside the widget inner bounds is clipped. The clip is
/// rectangular and can have rounded corners if [`corner_radius`] is set.
///
/// Note that this property has `fill` priority, but it is usually marked as a *child* property in
/// container widgets so it applies only to the inner widgets.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
///
/// container! {
///     background_color = rgb(255, 0, 0);
///     size = (200, 300);
///     corner_radius = 5;
///     clip_to_bounds = true;
///     content = container! {
///         background_color = rgb(0, 255, 0);
///         // fixed size ignores the layout available size.
///         size = (1000, 1000);
///         content = text("1000x1000 green clipped to 200x300");
///     };
/// }
/// # ;
/// ```
///
/// [`corner_radius`]: fn@corner_radius
#[property(fill, default(false))]
pub fn clip_to_bounds(child: impl UiNode, clip: impl IntoVar<bool>) -> impl UiNode {
    struct ClipToBoundsNode<T, S> {
        child: T,
        clip: S,
        bounds: PxSize,
        corners: PxCornerRadius,
    }

    #[impl_ui_node(child)]
    impl<T: UiNode, S: Var<bool>> UiNode for ClipToBoundsNode<T, S> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.clip);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.clip.is_new(ctx) {
                ctx.updates.render();
            }

            self.child.update(ctx);
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
            let mut changed = false;

            let border_offsets = widget_layout.border_offsets();
            let new_bounds = final_size + PxSize::new(border_offsets.horizontal(), border_offsets.vertical());
            if self.bounds != new_bounds {
                self.bounds = new_bounds;
                changed = true;
            }

            let corners = widget_layout.corner_radius().inflate(border_offsets);
            if self.corners != corners {
                self.corners = corners;
                changed = true;
            }

            if changed && self.clip.copy(ctx) {
                ctx.updates.render();
            }

            self.child.arrange(ctx, widget_layout, final_size)
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            if self.clip.copy(ctx) {
                let bounds = PxRect::from_size(self.bounds);

                if self.corners != PxCornerRadius::zero() {
                    frame.push_clip_rounded_rect(bounds, self.corners, false, |f| self.child.render(ctx, f));
                } else {
                    frame.push_clip_rect(bounds, |f| self.child.render(ctx, f));
                }
            } else {
                self.child.render(ctx, frame);
            }
        }
    }
    ClipToBoundsNode {
        child,
        clip: clip.into_var(),
        bounds: PxSize::zero(),
        corners: PxCornerRadius::zero(),
    }
}
