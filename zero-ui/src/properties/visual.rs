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
        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            self.children.with_node(1, |n| n.measure(wm))
        }
        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            let size = self.children.with_node_mut(1, |n| n.layout(wl));

            LAYOUT.with_constrains(
                |_| PxConstrains2d::new_exact_size(size),
                || {
                    self.children.with_node_mut(0, |n| n.layout(wl));
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
        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            self.children.with_node(0, |n| n.measure(wm))
        }
        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            let size = self.children.with_node_mut(0, |n| n.layout(wl));
            LAYOUT.with_constrains(
                |_| PxConstrains2d::new_exact_size(size),
                || {
                    self.children.with_node_mut(1, |n| n.layout(wl));
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
        fn update(&mut self, updates: &WidgetUpdates) {
            if self.offsets.is_new() || self.widths.is_new() {
                WIDGET.layout();
            } else if self.sides.is_new() {
                WIDGET.render();
            }
            self.child.update(updates);
        }

        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            self.child.measure(wm)
        }
        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            let size = self.child.layout(wl);

            let radius = BORDER.inner_radius();
            let offsets = self.offsets.layout();
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

            let widths = LAYOUT.with_constrains(|_| PxConstrains2d::new_exact_size(size), || self.widths.layout());

            if self.render_bounds != bounds || self.render_widths != widths || self.render_radius != radius {
                self.render_bounds = bounds;
                self.render_widths = widths;
                self.render_radius = radius;
                WIDGET.render();
            }

            size
        }

        fn render(&self, frame: &mut FrameBuilder) {
            self.child.render(frame);
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
        fn update(&mut self, updates: &WidgetUpdates) {
            if self.clip.is_new() {
                WIDGET.layout().render();
            }

            self.child.update(updates);
        }

        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            self.child.measure(wm)
        }
        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            let bounds = self.child.layout(wl);

            if self.clip.get() {
                let corners = BORDER.border_radius();
                if corners != self.corners {
                    self.corners = corners;
                    WIDGET.render();
                }
            }

            bounds
        }

        fn render(&self, frame: &mut FrameBuilder) {
            if self.clip.get() {
                frame.push_clips(
                    |c| {
                        let wgt_bounds = WIDGET.bounds();
                        let bounds = PxRect::from_size(wgt_bounds.inner_size());

                        if self.corners != PxCornerRadius::zero() {
                            c.push_clip_rounded_rect(bounds, self.corners, false, true);
                        } else {
                            c.push_clip_rect(bounds, false, true);
                        }

                        if let Some(inline) = wgt_bounds.inline() {
                            for r in inline.negative_space().iter() {
                                c.push_clip_rect(*r, true, true);
                            }
                        };
                    },
                    |f| self.child.render(f),
                );
            } else {
                self.child.render(frame);
            }
        }
    }
    ClipToBoundsNode {
        child,
        clip: clip.into_var(),
        corners: PxCornerRadius::zero(),
    }
}

/// Inline mode explicitly selected for a widget.
///
/// See the [`inline`] property for more details.
///
/// [`inline`]: fn@inline
#[derive(Default, Clone, Copy, PartialEq, Eq)]
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

/// Enforce an inline mode on the widget.
///
/// Set to [`InlineMode::Inline`] to use the inline layout and visual even if the widget
/// is not in an inlining parent.
///
/// Set to [`InlineMode::Block`] to ensure the widget layouts as a block item if the parent
/// is inlining.
///
/// Note that even if set to [`InlineMode::Inline`] the widget will only inline if all properties support
/// inlining.
#[property(CONTEXT-1, default(InlineMode::Allow))]
pub fn inline(child: impl UiNode, mode: impl IntoVar<InlineMode>) -> impl UiNode {
    #[ui_node(struct InlineNode {
        child: impl UiNode,
        #[var] mode: impl Var<InlineMode>,
    })]
    impl UiNode for InlineNode {
        fn update(&mut self, updates: &WidgetUpdates) {
            self.child.update(updates);
            if self.mode.is_new() {
                WIDGET.layout();
            }
        }

        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            match self.mode.get() {
                InlineMode::Allow => self.child.measure(wm),
                InlineMode::Inline => {
                    if LAYOUT.inline_constrains().is_none() {
                        // create an inline context
                        todo!("enable in `WidgetMeasure`");
                        // let c = InlineConstrainsMeasure {
                        //     first_max: LAYOUT.constrains().x.max_or(Px::MAX),
                        //     mid_clear_min: Px(0),
                        // };
                        // LAYOUT.with_inline_measure(wm, move |_| Some(c), |wm| self.child.measure(wm))
                    } else {
                        // already enabled by parent
                        self.child.measure(wm)
                    }
                }
                InlineMode::Block => {
                    // disable inline, method also disables in `WidgetMeasure`
                    LAYOUT.with_inline_measure(wm, |_| None, |wm| self.child.measure(wm))
                }
            }
        }

        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            match self.mode.get() {
                InlineMode::Allow => self.child.layout(wl),
                InlineMode::Inline => {
                    if LAYOUT.inline_constrains().is_none() {
                        todo!("compute constrains, enable in `WidgetLayout`")
                    } else {
                        // already enabled by parent
                        self.child.layout(wl)
                    }
                }
                InlineMode::Block => {
                    #[cfg(debug_assertions)]
                    if wl.inline().is_some() {
                        tracing::error!("inline enabled in `layout` when it signaled disabled in the previous `measure`")
                    }
                    self.child.layout(wl)
                }
            }
        }
    }
    InlineNode {
        child,
        mode: mode.into_var(),
    }
}
