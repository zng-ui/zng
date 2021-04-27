//! Properties that affect the widget render only.

use crate::core::gradient::{GradientStops, LinearGradientAxis};
use crate::prelude::new_property::*;
use crate::widgets::{fill_color, linear_gradient};

use super::margin;

/// Custom background property. Allows using any other widget as a background.
///
/// Backgrounds are not focusable and don't influence the widget layout.
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
#[property(inner, allowed_in_when = false)]
pub fn background(child: impl UiNode, background: impl UiNode) -> impl UiNode {
    struct BackgroundNode<T: UiNode, B: UiNode> {
        child: T,
        background: B,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, B: UiNode> UiNode for BackgroundNode<T, B> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.background.init(ctx);
            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.background.deinit(ctx);
            self.child.deinit(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.background.update(ctx);
            self.child.update(ctx);
        }
        fn update_hp(&mut self, ctx: &mut WidgetContext) {
            self.background.update_hp(ctx);
            self.child.update_hp(ctx);
        }

        fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
            let available_size = self.child.measure(available_size, ctx);
            self.background.measure(available_size, ctx);
            available_size
        }

        fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
            self.background.arrange(final_size, ctx);
            self.child.arrange(final_size, ctx);
        }

        fn render(&self, frame: &mut FrameBuilder) {
            self.background.render(frame); // TODO, disable events and focus for this?
            self.child.render(frame);
        }
    }
    BackgroundNode { child, background }
}

/// Single color background property.
///
/// This property applies a [`fill_color`] as [`background`](fn@background).
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
#[property(inner)]
pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    background(child, fill_color(color))
}

/// Linear gradient background property.
///
/// This property applies a [`linear_gradient`] as [`background`](fn@background).
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
#[property(inner)]
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
#[property(inner, allowed_in_when = false)]
pub fn foreground(child: impl UiNode, foreground: impl UiNode) -> impl UiNode {
    struct ForegroundNode<T: UiNode, B: UiNode> {
        child: T,
        foreground: B,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, B: UiNode> UiNode for ForegroundNode<T, B> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);
            self.foreground.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            self.foreground.deinit(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);
            self.foreground.update(ctx);
        }
        fn update_hp(&mut self, ctx: &mut WidgetContext) {
            self.child.update_hp(ctx);
            self.foreground.update_hp(ctx);
        }

        fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
            let available_size = self.child.measure(available_size, ctx);
            self.foreground.measure(available_size, ctx);
            available_size
        }

        fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
            self.foreground.arrange(final_size, ctx);
            self.child.arrange(final_size, ctx);
        }

        fn render(&self, frame: &mut FrameBuilder) {
            self.child.render(frame);
            self.foreground.render(frame); // TODO, disable events and focus for this?
        }
    }
    ForegroundNode { child, foreground }
}

/// Foreground highlight border overlay.
///
/// This property draws a [`border`](fn@crate::properties::border) with extra `offsets` control 
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
///         details: colors::BLUE
///     }
/// }
/// # ;
/// ```
///
/// The example renders a solid blue 1 pixel border overlay, the border lines are inset 3 pixels in the container.
#[property(inner)]
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
#[property(inner)]
pub fn foreground_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    foreground(child, fill_color(color))
}

/// Linear gradient overlay property.
///
/// This property applies a [`linear_gradient`] as [`foreground`] using the [`Clamp`](ExtendMode::Clamp) extend mode.
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
#[property(inner)]
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
///     size = (200.0, 300.0);
///     clip_to_bounds = true;
///     content = container! {
///         background_color = rgb(0, 255, 0);
///         // fixed size ignores the layout available size.
///         size = (1000.0, 1000.0);
///         content = text("1000x1000 green clipped to 200x300");
///     };
/// }
/// # ;
/// ```
#[property(inner)]
pub fn clip_to_bounds(child: impl UiNode, clip: impl IntoVar<bool>) -> impl UiNode {
    struct ClipToBoundsNode<T: UiNode, S: VarLocal<bool>> {
        child: T,
        clip: S,
        bounds: LayoutSize,
    }

    #[impl_ui_node(child)]
    impl<T: UiNode, S: VarLocal<bool>> UiNode for ClipToBoundsNode<T, S> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.clip.init_local(ctx.vars);
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.clip.update_local(ctx.vars).is_some() {
                ctx.updates.render();
            }

            self.child.update(ctx);
        }

        fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
            self.bounds = final_size;
            self.child.arrange(final_size, ctx)
        }

        fn render(&self, frame: &mut FrameBuilder) {
            if *self.clip.get_local() {
                frame.push_simple_clip(self.bounds, |frame| self.child.render(frame));
            } else {
                self.child.render(frame);
            }
        }
    }
    ClipToBoundsNode {
        child,
        clip: clip.into_local(),
        bounds: LayoutSize::zero(),
    }
}
