//! Rule line widgets.

use crate::prelude::new_widget::*;

/// Draws a horizontal or vertical rule line.
#[widget($crate::widgets::RuleLine)]
pub struct RuleLine(WidgetBase);
impl RuleLine {
    #[widget(on_start)]
    fn on_start(&mut self) {
        self.builder().push_build_action(on_build);
    }
}

/// Line orientation.
#[property(CONTEXT, capture, default(LineOrientation::Horizontal), impl(RuleLine))]
pub fn orientation(child: impl UiNode, orientation: impl IntoVar<LineOrientation>) -> impl UiNode {}

/// Line color.
#[property(CONTEXT, capture, default(rgb(0, 0, 0)), impl(RuleLine))]
pub fn color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {}

/// Line stroke thickness.
#[property(CONTEXT, capture, default(1), impl(RuleLine))]
pub fn stroke_thickness(child: impl UiNode, thickness: impl IntoVar<Length>) -> impl UiNode {}

/// Line length.
///
/// Set to [`Default`] to fill available length without requesting any length.
///
/// [`Default`]: Length::Default
#[property(CONTEXT, capture, default(Length::Default), impl(RuleLine))]
pub fn length(child: impl UiNode, length: impl IntoVar<Length>) -> impl UiNode {}

/// Line style.
#[property(CONTEXT, capture, default(LineStyle::Solid), impl(RuleLine))]
pub fn line_style(child: impl UiNode, style: impl IntoVar<LineStyle>) -> impl UiNode {}

fn on_build(wgt: &mut WidgetBuilding) {
    let child = LineNode {
        bounds: PxSize::zero(),

        orientation: wgt
            .capture_var(property_id!(orientation))
            .unwrap_or_else(|| LineOrientation::Horizontal.into_var().boxed()),

        length: wgt
            .capture_var(property_id!(length))
            .unwrap_or_else(|| LocalVar(Length::Default).boxed()),

        stroke_thickness: wgt
            .capture_var(property_id!(stroke_thickness))
            .unwrap_or_else(|| LocalVar(Length::from(1)).boxed()),

        color: wgt
            .capture_var(property_id!(color))
            .unwrap_or_else(|| LocalVar(rgb(0, 0, 0)).boxed()),

        style: wgt
            .capture_var(property_id!(line_style))
            .unwrap_or_else(|| LineStyle::Solid.into_var().boxed()),
    };
    wgt.set_child(child);
}

#[ui_node(struct LineNode {
    #[var] stroke_thickness: impl Var<Length>,
    #[var] length: impl Var<Length>,
    #[var] orientation: impl Var<LineOrientation>,
    #[var] color: impl Var<Rgba>,
    #[var] style: impl Var<LineStyle>,

    bounds: PxSize,
})]
impl UiNode for LineNode {
    fn update(&mut self, _: &WidgetUpdates) {
        if self.stroke_thickness.is_new() || self.length.is_new() || self.orientation.is_new() {
            WIDGET.layout();
        }
        if self.color.is_new() || self.style.is_new() {
            WIDGET.render();
        }
    }

    fn measure(&self, _: &mut WidgetMeasure) -> PxSize {
        let metrics = LAYOUT.metrics();
        let default_stroke = Dip::new(1).to_px(metrics.scale_factor().0);

        match self.orientation.get() {
            LineOrientation::Horizontal => PxSize::new(
                self.length.layout_dft_x(metrics.constraints().x.fill()),
                self.stroke_thickness.layout_dft_y(default_stroke),
            ),
            LineOrientation::Vertical => PxSize::new(
                self.stroke_thickness.layout_dft_x(default_stroke),
                self.length.layout_dft_y(metrics.constraints().y.fill()),
            ),
        }
    }
    fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
        let metrics = LAYOUT.metrics();
        let default_stroke = Dip::new(1).to_px(metrics.scale_factor().0);

        let bounds = match self.orientation.get() {
            LineOrientation::Horizontal => PxSize::new(
                self.length.layout_dft_x(metrics.constraints().x.fill()),
                self.stroke_thickness.layout_dft_y(default_stroke),
            ),
            LineOrientation::Vertical => PxSize::new(
                self.stroke_thickness.layout_dft_x(default_stroke),
                self.length.layout_dft_y(metrics.constraints().y.fill()),
            ),
        };

        if bounds != self.bounds {
            self.bounds = bounds;
            WIDGET.render();
        }

        bounds
    }

    fn render(&self, frame: &mut FrameBuilder) {
        let bounds = PxRect::from_size(self.bounds);
        let orientation = self.orientation.get();
        let color = self.color.get();
        let style = self.style.get();
        frame.push_line(bounds, orientation, color.into(), style);
    }
}

/// Draws an horizontal [`RuleLine!`](struct@RuleLine).
pub mod hr {
    use crate::prelude::new_widget::*;

    #[widget($crate::widgets::Hr)]
    pub struct Hr(super::RuleLine);
    impl Hr {
        #[widget(on_start)]
        fn on_start(&mut self) {
            defaults! {
                self;
                orientation = LineOrientation::Horizontal;
                color = COLOR_VAR;
                stroke_thickness  = STROKE_THICKNESS_VAR;
                line_style = LINE_STYLE_VAR;
            }
        }
    }

    context_var! {
        /// Line color, inherits from [`TEXT_COLOR_VAR`].
        pub static COLOR_VAR: Rgba = text::TEXT_COLOR_VAR;

        /// Line stroke thickness, default is `1.dip()`
        pub static STROKE_THICKNESS_VAR: Length = 1.dip();

        /// Line style, default is `Solid`.
        pub static LINE_STYLE_VAR: LineStyle = LineStyle::Solid;
    }

    /// Sets the [`COLOR_VAR`] that affects all horizontal rules inside the widget.
    #[property(CONTEXT, default(COLOR_VAR))]
    pub fn color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
        with_context_var(child, COLOR_VAR, color)
    }

    /// Sets the [`STROKE_THICKNESS_VAR`] that affects all horizontal rules inside the widget.
    #[property(CONTEXT, default(STROKE_THICKNESS_VAR))]
    pub fn stroke_thickness(child: impl UiNode, thickness: impl IntoVar<Length>) -> impl UiNode {
        with_context_var(child, STROKE_THICKNESS_VAR, thickness)
    }

    /// Sets the [`LINE_STYLE_VAR`] that affects all horizontal rules inside the widget.
    #[property(CONTEXT, default(LINE_STYLE_VAR))]
    pub fn line_style(child: impl UiNode, style: impl IntoVar<LineStyle>) -> impl UiNode {
        with_context_var(child, LINE_STYLE_VAR, style)
    }
}
