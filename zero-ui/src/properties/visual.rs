//! Properties that affect the widget render only.

use crate::core::gradient::{GradientStops, LinearGradientAxis};
use crate::prelude::new_property::*;
use crate::widgets::{fill_color, linear_gradient};

use super::margin;

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
#[property(inner, allowed_in_when = false, default(crate::core::NilUiNode))]
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
#[property(inner, default(ViewGenerator::nil()))]
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
#[property(inner, default(colors::BLACK.transparent()))]
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
#[property(inner, default(0.deg(), {
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
#[property(inner, allowed_in_when = false, default(crate::core::NilUiNode))]
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
///         radius: 0
///     }
/// }
/// # ;
/// ```
///
/// The example renders a solid blue 1 pixel border overlay, the border lines are inset 3 pixels in the container.
///
/// [`foreground`]: fn@foreground
/// [`border`]: fn@crate::properties::border
#[property(inner, default(0, 0, BorderStyle::Hidden, 0))]
pub fn foreground_highlight(
    child: impl UiNode,
    offsets: impl IntoVar<SideOffsets>,
    widths: impl IntoVar<SideOffsets>,
    sides: impl IntoVar<BorderSides>,
    radius: impl IntoVar<BorderRadius>,
) -> impl UiNode {
    let border = crate::properties::border(crate::core::FillUiNode, widths, sides, radius);
    foreground(child, margin(border, offsets))
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
#[property(inner, default(colors::BLACK.transparent()))]
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
#[property(inner, default(0.deg(), {
    let c = colors::BLACK.transparent();
    crate::core::gradient::stops![c, c]
}))]
pub fn foreground_gradient(child: impl UiNode, axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    foreground(child, linear_gradient(axis, stops))
}

/// Clips the widget child to the area of the widget when set to `true`.
///
/// Any content rendered outside the widget *inner size* bounds is clipped. The clip is
/// rectangular and can have rounded corners if TODO.
///
/// # Example
/// ```
/// use zero_ui::prelude::*;
///
/// container! {
///     background_color = rgb(255, 0, 0);
///     size = (200, 300);
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
#[property(inner, default(false))]
pub fn clip_to_bounds(child: impl UiNode, clip: impl IntoVar<bool>) -> impl UiNode {
    struct ClipToBoundsNode<T, S> {
        child: T,
        clip: S,
        bounds: PxSize,
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

        fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
            if self.bounds != final_size {
                self.bounds = final_size;

                if self.clip.copy(ctx) {
                    ctx.updates.render();
                }
            }
            self.child.arrange(ctx, widget_offset, final_size)
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            if self.clip.copy(ctx) {
                frame.push_simple_clip(self.bounds, |frame| self.child.render(ctx, frame));
            } else {
                self.child.render(ctx, frame);
            }
        }
    }
    ClipToBoundsNode {
        child,
        clip: clip.into_var(),
        bounds: PxSize::zero(),
    }
}

/// Renders the widget and its content in the specified layer.
#[property(context, default(0, LayerMode::default()))]
pub fn layer(child: impl UiNode, index: impl IntoVar<LayerIndex>, mode: impl IntoVar<LayerMode>) -> impl UiNode {
    use std::cell::RefCell;
    use std::rc::Rc;

    struct LayerNode<C, I, M> {
        child: Rc<RefCell<C>>,
        index: I,
        mode: M,
    }
    #[impl_ui_node(
        delegate = self.child.borrow(),
        delegate_mut = self.child.borrow_mut()
    )]
    impl<C: UiNode, I: Var<LayerIndex>, M: Var<LayerMode>> UiNode for LayerNode<C, I, M> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.index.is_new(ctx) || self.mode.is_new(ctx) {
                ctx.updates.render();
            }

            self.child.borrow_mut().update(ctx);
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            if self.index.copy(ctx) == LayerIndex::DEFAULT {
                self.child.borrow_mut().measure(ctx, available_size)
            } else {
                let mode = self.mode.copy(ctx);

                if mode.contains(LayerMode::LAYOUT) {
                    self.child.borrow_mut().measure(ctx, available_size)
                } else {
                    // .min(PxSize::new(Px(1), Px(1)))
                    todo!()
                }
            }
        }
        fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
            if self.index.copy(ctx) == LayerIndex::DEFAULT {
                self.child.borrow_mut().arrange(ctx, widget_offset, final_size)
            } else {
                let mode = self.mode.copy(ctx);

                if mode.contains(LayerMode::LAYOUT) {
                    self.child.borrow_mut().arrange(ctx, widget_offset, final_size);
                } else {
                    todo!()
                }
            }
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let index = self.index.copy(ctx);
            let mode = self.mode.copy(ctx);
            let child = Rc::clone(&self.child);

            if mode == LayerMode::DEFAULT {
                frame.in_layer(ctx, index, move |ctx, frame| child.borrow().render(ctx, frame));
            } else {
                todo!()
            }
        }
    }
    LayerNode {
        child: Rc::new(RefCell::new(child)),
        index: index.into_var(),
        mode: mode.into_var(),
    }
}

bitflags::bitflags! {
    /// Defines the render context of a [`layer`] modified widget.
    ///
    /// Widgets in different layers are inited, updated and layout when its parent is layout, but
    /// it is rendered in a different *surface* so the parent transform and filters does not affect the widget.
    /// The flags in this type instruct the [`layer`] property to recreate parts of the parent render context.
    ///
    /// # Visibility Pixel
    ///
    /// Even if the [`LAYOUT`] flag is not set the layered widget still reserves a 1x1 pixel point in its parent if it is visible, this is
    /// done because zero pixel sized widgets are considered [`Collapsed`] and not rendered by most layout widgets.
    ///
    /// [`layer`]: fn@layer
    /// [`z_stack`]: mod@crate::widgets::layout::z_stack
    /// [`LAYOUT`]: LayerMode::LAYOUT
    /// [`Collapsed`]: crate::core::Visibility::Collapsed
    pub struct LayerMode: u16 {
        /// The widget is layout as if it is the only content of the window, the available size is the window content area,
        /// it only reserves a 1x1 pixel point in the parent layout.
        const DEFAULT = 0b0;

        /// The parent stacked transform is applied to the widget origin point, so it is not scaled and
        /// rotated like the parent but it is positioned at the transform point.
        const OFFSET = 0b1;

        /// The parent stacked transform is applied the widget, this flag overrides [`OFFSET`].
        ///
        /// [`OFFSET`]: LayerMode::OFFSET
        const TRANSFORM = 0b11;

        /// The parent stacked pixel filters is copied to the widget, that is the opacity, grayscale, and other filters.
        const FILTERS = 0b100;

        /// The widget is layout normally in its parent, reserving the space.
        const LAYOUT = 0b1000;

        /// The widget reserves the [`LAYOUT`] and copied the [`TRANSFORM`] and [`FILTERS`], this causes
        /// the visual result should look as if the widget is rendered normally but it is guaranteed to be
        /// over all other widgets in the parent, the layer index behaving something like a ***Z*** *index*.
        ///
        /// [`LAYOUT`]: LayerMode::LAYOUT
        /// [`TRANSFORM`]: LayerMode::TRANSFORM
        /// [`FILTERS`]: LayerMode::FILTERS
        const Z = Self::LAYOUT.bits | Self::TRANSFORM.bits | Self::FILTERS.bits;
    }
}
impl Default for LayerMode {
    fn default() -> Self {
        LayerMode::DEFAULT
    }
}
