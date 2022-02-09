//! Properties that affect the widget render only.

use crate::core::gradient::{GradientStops, LinearGradientAxis};
use crate::prelude::new_property::*;
use crate::widgets::{fill_color, linear_gradient};

use super::side_offsets;

/// Custom background property. Allows using any other widget as a background.
///
/// Backgrounds don't influence the widget layout.
///
/// # Example
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
///         align = Alignment::CENTER;
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
        background_offset: PxVector,
        background_clip: (PxSize, PxCornerRadius),
        spatial_id: SpatialFrameId,
    }
    #[impl_ui_node(children)]
    impl<C: UiNodeList> UiNode for BackgroundNode<C> {
        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let available_size = self.children.widget_measure(1, ctx, available_size);
            self.children.widget_measure(0, ctx, AvailableSize::finite(available_size));
            available_size
        }
        fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
            self.children.widget_arrange(1, ctx, widget_layout, final_size);
            let border_offsets = widget_layout.border_offsets();
            //let border_diff = PxSize::new(
            //    border_offsets.left + border_offsets.right,
            //    border_offsets.top + border_offsets.bottom,
            //);

            self.background_offset = PxVector::new(border_offsets.left, border_offsets.top);
            self.background_clip = (final_size, widget_layout.corner_radius());

            widget_layout.with_custom_transform(&RenderTransform::translation_px(self.background_offset), |wl| {
                self.children.widget_arrange(0, ctx, wl, final_size)
            });
        }
        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let mut render_background = |frame: &mut FrameBuilder| {
                let (clip_bounds, clip_corners) = self.background_clip;
                let clip_bounds = PxRect::from_size(clip_bounds);

                if clip_corners != PxCornerRadius::zero() {
                    frame.push_clip_rounded_rect(clip_bounds, clip_corners, false, |f| self.children.widget_render(0, ctx, f))
                } else {
                    frame.push_clip_rect(clip_bounds, |f| self.children.widget_render(0, ctx, f))
                }
            };
            if self.background_offset != PxVector::zero() {
                frame.push_reference_frame(
                    self.spatial_id,
                    FrameBinding::Value(RenderTransform::translation_px(self.background_offset)),
                    true,
                    render_background,
                );
            } else {
                render_background(frame);
            }

            self.children.widget_render(1, ctx, frame);
        }
    }
    BackgroundNode {
        children: nodes![background, child],
        background_offset: PxVector::zero(),
        background_clip: (PxSize::zero(), PxCornerRadius::zero()),
        spatial_id: SpatialFrameId::new_unique(),
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
/// # Example
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
/// # Example
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

/// Custom foreground property. Allows using any other widget as a foreground overlay.
///
/// Foregrounds are not focusable, not hit-testable and don't influence the widget layout.
///
/// # Example
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
///         align = Alignment::CENTER;
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
    ForegroundNode {
        children: nodes![child, foreground],
    }
}

/// Foreground highlight border overlay.
///
/// This property draws a [`border`] with extra `offsets` control
/// as a [`foreground`] overlay. The border has no content.
///
/// # Example
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
/// The example renders a solid blue 1 pixel border overlay, the border lines are inset 3 pixels in the container.
///
/// [`foreground`]: fn@foreground
/// [`border`]: fn@crate::properties::border
#[property(fill, default(0, 0, BorderStyle::Hidden))]
pub fn foreground_highlight(
    child: impl UiNode,
    offsets: impl IntoVar<SideOffsets>,
    widths: impl IntoVar<SideOffsets>,
    sides: impl IntoVar<BorderSides>,
) -> impl UiNode {
    let border = crate::properties::border(crate::core::FillUiNode, widths, sides);
    foreground(child, side_offsets(border, offsets))
}

/// Fill color overlay property.
///
/// This property applies a [`fill_color`] as [`foreground`].
///
/// # Example
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
/// # Example
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
/// [`Clamp`]: ExtendMode::Clamp
#[property(fill, default(0.deg(), {
    let c = colors::BLACK.transparent();
    crate::core::gradient::stops![c, c]
}))]
pub fn foreground_gradient(child: impl UiNode, axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    foreground(child, linear_gradient(axis, stops))
}

/// Clips the widget child to the area of the widget when set to `true`.
///
/// Any content rendered outside the widget *inner size* bounds is clipped. The clip is
/// rectangular and can have rounded corners if [`corner_radius`] is set.
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

            if self.bounds != final_size {
                self.bounds = final_size;
                changed = true;
            }

            let corners = widget_layout.corner_radius();
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
