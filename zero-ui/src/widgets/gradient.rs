use crate::core::gradient::*;
use crate::prelude::new_widget::*;

/// Node that fills the widget area with a linear gradient defined by angle or points.
///
/// The extend mode is [`Clamp`](ExtendMode::Clamp).
pub fn linear_gradient(axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    linear_gradient_ext(axis, stops, ExtendMode::Clamp)
}
/// Node that fills the widget area with a linear gradient with extend mode [`Repeat`](ExtendMode::Repeat).
pub fn repeating_linear_gradient(axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    linear_gradient_ext(axis, stops, ExtendMode::Repeat)
}
/// Node that fills the widget area with a Linear gradient with extend mode [`Reflect`](ExtendMode::Reflect).
pub fn reflecting_linear_gradient(axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    linear_gradient_ext(axis, stops, ExtendMode::Reflect)
}
/// Node that fills the widget area with a linear gradient with extend mode configurable.
pub fn linear_gradient_ext(
    axis: impl IntoVar<LinearGradientAxis>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
) -> impl UiNode {
    #[ui_node(struct LinearGradientNode {
        #[var] axis: impl Var<LinearGradientAxis>,
        #[var] stops: impl Var<GradientStops>,
        #[var] extend_mode: impl Var<ExtendMode>,

        render_line: PxLine,
        render_stops: Vec<RenderGradientStop>,

        final_size: PxSize,
    })]
    impl UiNode for LinearGradientNode {
        fn update(&mut self, _: &WidgetUpdates) {
            if self.axis.is_new() || self.stops.is_new() || self.extend_mode.is_new() {
                self.final_size = PxSize::zero();
                WIDGET.layout();
            }
        }

        fn measure(&mut self, _: &mut WidgetMeasure) -> PxSize {
            LAYOUT.constraints().fill_size()
        }

        fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
            let final_size = LAYOUT.constraints().fill_size();
            if self.final_size != final_size {
                self.final_size = final_size;
                self.render_line = self.axis.layout();

                let length = self.render_line.length();

                LAYOUT.with_constraints(LAYOUT.constraints().with_new_exact_x(length), || {
                    self.stops
                        .with(|s| s.layout_linear(true, self.extend_mode.get(), &mut self.render_line, &mut self.render_stops))
                });

                WIDGET.render();
            }
            final_size
        }

        fn render(&mut self, frame: &mut FrameBuilder) {
            frame.push_linear_gradient(
                PxRect::from_size(self.final_size),
                self.render_line,
                &self.render_stops,
                self.extend_mode.get().into(),
                self.final_size,
                PxSize::zero(),
            );
        }
    }
    LinearGradientNode {
        axis: axis.into_var(),
        stops: stops.into_var(),
        extend_mode: extend_mode.into_var(),

        render_line: PxLine::zero(),
        render_stops: vec![],

        final_size: PxSize::zero(),
    }
}
/// Node that fills the widget area with a linear gradient with all features configurable.
pub fn linear_gradient_full(
    axis: impl IntoVar<LinearGradientAxis>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
    tile_size: impl IntoVar<Size>,
    tile_spacing: impl IntoVar<Size>,
) -> impl UiNode {
    #[ui_node(struct LinearGradientFullNode {
        #[var] axis: impl Var<LinearGradientAxis>,
        #[var] stops: impl Var<GradientStops>,
        #[var] extend_mode: impl Var<ExtendMode>,
        #[var] tile_size: impl Var<Size>,
        #[var] tile_spacing: impl Var<Size>,

        final_line: PxLine,
        final_stops: Vec<RenderGradientStop>,

        final_size: PxSize,
        final_tile_size: PxSize,
        final_tile_spacing: PxSize,
    })]
    impl UiNode for LinearGradientFullNode {
        fn update(&mut self, _: &WidgetUpdates) {
            if self.axis.is_new()
                || self.stops.is_new()
                || self.extend_mode.is_new()
                || self.tile_size.is_new()
                || self.tile_spacing.is_new()
            {
                self.final_size = PxSize::zero();
                WIDGET.layout();
            }
        }

        fn measure(&mut self, _: &mut WidgetMeasure) -> PxSize {
            LAYOUT.constraints().fill_size()
        }
        fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
            let c = LAYOUT.constraints();
            let final_size = c.fill_size();
            if self.final_size != final_size {
                self.final_size = final_size;

                self.final_tile_size = self.tile_size.layout_dft(self.final_size);
                self.final_tile_spacing = self.tile_spacing.layout_dft(self.final_size);

                self.final_line = LAYOUT.with_constraints(c.with_exact_size(self.final_tile_size), || self.axis.layout());

                let length = self.final_line.length();
                LAYOUT.with_constraints(c.with_new_exact_x(length), || {
                    self.stops
                        .with(|s| s.layout_linear(true, self.extend_mode.get(), &mut self.final_line, &mut self.final_stops))
                });

                WIDGET.render();
            }
            self.final_size
        }

        fn render(&mut self, frame: &mut FrameBuilder) {
            frame.push_linear_gradient(
                PxRect::from_size(self.final_size),
                self.final_line,
                &self.final_stops,
                self.extend_mode.get().into(),
                self.final_tile_size,
                self.final_tile_spacing,
            );
        }
    }

    LinearGradientFullNode {
        axis: axis.into_var(),
        stops: stops.into_var(),
        extend_mode: extend_mode.into_var(),
        tile_size: tile_size.into_var(),
        tile_spacing: tile_spacing.into_var(),

        final_line: PxLine::zero(),
        final_stops: vec![],

        final_size: PxSize::zero(),
        final_tile_size: PxSize::zero(),
        final_tile_spacing: PxSize::zero(),
    }
    .cfg_boxed()
}

/// Node that fills the widget area with a radial gradient defined by the center point and radius.
///
/// The extend mode is [`Clamp`](ExtendMode::Clamp).
pub fn radial_gradient(
    center: impl IntoVar<Point>,
    radius: impl IntoVar<GradientRadius>,
    stops: impl IntoVar<GradientStops>,
) -> impl UiNode {
    radial_gradient_ext(center, radius, stops, ExtendMode::Clamp)
}
/// Node that fills the widget area with a radial gradient with extend mode [`Repeat`](ExtendMode::Repeat).
pub fn repeating_radial_gradient(
    center: impl IntoVar<Point>,
    radius: impl IntoVar<GradientRadius>,
    stops: impl IntoVar<GradientStops>,
) -> impl UiNode {
    radial_gradient_ext(center, radius, stops, ExtendMode::Repeat)
}
/// Node that fills the widget area with a radial gradient with extend mode [`Reflect`](ExtendMode::Reflect).
pub fn reflecting_radial_gradient(
    center: impl IntoVar<Point>,
    radius: impl IntoVar<GradientRadius>,
    stops: impl IntoVar<GradientStops>,
) -> impl UiNode {
    radial_gradient_ext(center, radius, stops, ExtendMode::Reflect)
}
/// Node that fill the widget area with a radial gradient with extend mode configurable.
pub fn radial_gradient_ext(
    center: impl IntoVar<Point>,
    radius: impl IntoVar<GradientRadius>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
) -> impl UiNode {
    #[ui_node(struct RadialGradientNode {
        #[var] center: impl Var<Point>,
        #[var] radius: impl Var<GradientRadius>,
        #[var] stops: impl Var<GradientStops>,
        #[var] extend_mode: impl Var<ExtendMode>,

        render_stops: Vec<RenderGradientStop>,
        render_center: PxPoint,
        render_radius: PxSize,
        final_size: PxSize,
    })]
    impl UiNode for RadialGradientNode {
        fn update(&mut self, _: &WidgetUpdates) {
            if self.center.is_new() || self.radius.is_new() || self.stops.is_new() || self.extend_mode.is_new() {
                self.final_size = PxSize::zero();
                WIDGET.layout();
            }
        }

        fn measure(&mut self, _: &mut WidgetMeasure) -> PxSize {
            LAYOUT.constraints().fill_size()
        }

        fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
            let final_size = LAYOUT.constraints().fill_size();
            if self.final_size != final_size {
                self.final_size = final_size;
                LAYOUT.with_constraints(PxConstraints2d::new_fill_size(self.final_size), || {
                    self.render_center = self.center.layout_dft(self.final_size.to_vector().to_point() * 0.5.fct());
                    self.render_radius = self.radius.get().layout(self.render_center);
                });

                LAYOUT.with_constraints(
                    LAYOUT
                        .constraints()
                        .with_exact_x(self.render_radius.width.max(self.render_radius.height)),
                    || {
                        self.stops
                            .with(|s| s.layout_radial(true, self.extend_mode.get(), &mut self.render_stops))
                    },
                );

                WIDGET.render();
            }
            final_size
        }

        fn render(&mut self, frame: &mut FrameBuilder) {
            frame.push_radial_gradient(
                PxRect::from_size(self.final_size),
                self.render_center,
                self.render_radius,
                &self.render_stops,
                self.extend_mode.get().into(),
                self.final_size,
                PxSize::zero(),
            );
        }
    }
    RadialGradientNode {
        center: center.into_var(),
        radius: radius.into_var(),
        stops: stops.into_var(),
        extend_mode: extend_mode.into_var(),

        render_stops: vec![],
        render_center: PxPoint::zero(),
        render_radius: PxSize::zero(),
        final_size: PxSize::zero(),
    }
}
/// Node that fills the widget area with a radial gradient with all features configurable.
pub fn radial_gradient_full(
    center: impl IntoVar<Point>,
    radius: impl IntoVar<GradientRadius>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
    tile_size: impl IntoVar<Size>,
    tile_spacing: impl IntoVar<Size>,
) -> impl UiNode {
    #[ui_node(struct RadialGradientNode {
        #[var] center: impl Var<Point>,
        #[var] radius: impl Var<GradientRadius>,
        #[var] stops: impl Var<GradientStops>,
        #[var] extend_mode: impl Var<ExtendMode>,
        #[var] tile_size: impl Var<Size>,
        #[var] tile_spacing: impl Var<Size>,

        render_stops: Vec<RenderGradientStop>,
        render_center: PxPoint,
        render_radius: PxSize,
        final_size: PxSize,
        final_tile_size: PxSize,
        final_tile_spacing: PxSize,
    })]
    impl UiNode for RadialGradientNode {
        fn update(&mut self, _: &WidgetUpdates) {
            if self.center.is_new()
                || self.radius.is_new()
                || self.stops.is_new()
                || self.extend_mode.is_new()
                || self.tile_size.is_new()
                || self.tile_spacing.is_new()
            {
                self.final_size = PxSize::zero();
                WIDGET.layout();
            }
        }

        fn measure(&mut self, _: &mut WidgetMeasure) -> PxSize {
            LAYOUT.constraints().fill_size()
        }

        fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
            let final_size = LAYOUT.constraints().fill_size();
            if self.final_size != final_size {
                self.final_size = final_size;

                self.final_tile_size = self.tile_size.layout_dft(self.final_size);
                self.final_tile_spacing = self.tile_spacing.layout_dft(self.final_size);

                LAYOUT.with_constraints(PxConstraints2d::new_fill_size(self.final_tile_size), || {
                    // self.final_tile_size.to_vector().to_point() * 0.5.fct()
                    self.render_radius = self.radius.get().layout(self.render_center);
                });

                LAYOUT.with_constraints(
                    LAYOUT
                        .constraints()
                        .with_exact_x(self.render_radius.width.max(self.render_radius.height)),
                    || {
                        self.stops
                            .with(|s| s.layout_radial(true, self.extend_mode.get(), &mut self.render_stops))
                    },
                );

                WIDGET.render();
            }
            final_size
        }

        fn render(&mut self, frame: &mut FrameBuilder) {
            frame.push_radial_gradient(
                PxRect::from_size(self.final_size),
                self.render_center,
                self.render_radius,
                &self.render_stops,
                self.extend_mode.get().into(),
                self.final_tile_size,
                self.final_tile_spacing,
            );
        }
    }
    RadialGradientNode {
        center: center.into_var(),
        radius: radius.into_var(),
        stops: stops.into_var(),
        extend_mode: extend_mode.into_var(),
        tile_size: tile_size.into_var(),
        tile_spacing: tile_spacing.into_var(),

        render_stops: vec![],
        render_center: PxPoint::zero(),
        render_radius: PxSize::zero(),
        final_size: PxSize::zero(),
        final_tile_size: PxSize::zero(),
        final_tile_spacing: PxSize::zero(),
    }
}

/// Node that fills the widget area with a conic gradient defined by center point and start angle.
///
/// The extend mode is [`Clamp`](ExtendMode::Clamp).
pub fn conic_gradient(center: impl IntoVar<Point>, angle: impl IntoVar<AngleRadian>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    conic_gradient_ext(center, angle, stops, ExtendMode::Clamp)
}
/// Node that fills the widget area with a conic gradient with extend mode [`Repeat`](ExtendMode::Repeat).
pub fn repeating_conic_gradient(
    center: impl IntoVar<Point>,
    angle: impl IntoVar<AngleRadian>,
    stops: impl IntoVar<GradientStops>,
) -> impl UiNode {
    conic_gradient_ext(center, angle, stops, ExtendMode::Repeat)
}
/// Node that fills the widget area with a conic gradient with extend mode [`Reflect`](ExtendMode::Reflect).
pub fn reflecting_conic_gradient(
    center: impl IntoVar<Point>,
    angle: impl IntoVar<AngleRadian>,
    stops: impl IntoVar<GradientStops>,
) -> impl UiNode {
    conic_gradient_ext(center, angle, stops, ExtendMode::Reflect)
}
/// Node that fill the widget area with a conic gradient with extend mode configurable.
pub fn conic_gradient_ext(
    center: impl IntoVar<Point>,
    angle: impl IntoVar<AngleRadian>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
) -> impl UiNode {
    #[ui_node(struct ConicGradientNode {
        #[var] angle: impl Var<AngleRadian>,
        #[var] center: impl Var<Point>,
        #[var] stops: impl Var<GradientStops>,
        #[var] extend_mode: impl Var<ExtendMode>,

        render_stops: Vec<RenderGradientStop>,
        render_center: PxPoint,
        final_size: PxSize,
    })]
    impl UiNode for ConicGradientNode {
        fn update(&mut self, _: &WidgetUpdates) {
            if self.center.is_new() || self.angle.is_new() || self.stops.is_new() || self.extend_mode.is_new() {
                self.final_size = PxSize::zero();
                WIDGET.layout();
            }
        }

        fn measure(&mut self, _: &mut WidgetMeasure) -> PxSize {
            LAYOUT.constraints().fill_size()
        }

        fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
            let final_size = LAYOUT.constraints().fill_size();
            if self.final_size != final_size {
                self.final_size = final_size;
                LAYOUT.with_constraints(PxConstraints2d::new_fill_size(self.final_size), || {
                    self.render_center = self.center.layout_dft(self.final_size.to_vector().to_point() * 0.5.fct());
                });

                let perimeter = Px({
                    let a = self.final_size.width.0 as f32;
                    let b = self.final_size.height.0 as f32;
                    std::f32::consts::PI * 2.0 * ((a * a + b * b) / 2.0).sqrt()
                } as _);
                LAYOUT.with_constraints(LAYOUT.constraints().with_exact_x(perimeter), || {
                    self.stops
                        .with(|s| s.layout_radial(true, self.extend_mode.get(), &mut self.render_stops))
                });

                WIDGET.render();
            }
            final_size
        }

        fn render(&mut self, frame: &mut FrameBuilder) {
            frame.push_conic_gradient(
                PxRect::from_size(self.final_size),
                self.render_center,
                self.angle.get(),
                &self.render_stops,
                self.extend_mode.get().into(),
                self.final_size,
                PxSize::zero(),
            );
        }
    }
    ConicGradientNode {
        angle: angle.into_var(),
        center: center.into_var(),
        stops: stops.into_var(),
        extend_mode: extend_mode.into_var(),

        render_stops: vec![],
        render_center: PxPoint::zero(),
        final_size: PxSize::zero(),
    }
}
/// Node that fills the widget area with a conic gradient with all features configurable.
pub fn conic_gradient_full(
    center: impl IntoVar<Point>,
    angle: impl IntoVar<AngleRadian>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
    tile_size: impl IntoVar<Size>,
    tile_spacing: impl IntoVar<Size>,
) -> impl UiNode {
    #[ui_node(struct ConicGradientFullNode {
        #[var] angle: impl Var<AngleRadian>,
        #[var] center: impl Var<Point>,
        #[var] stops: impl Var<GradientStops>,
        #[var] extend_mode: impl Var<ExtendMode>,
        #[var] tile_size: impl Var<Size>,
        #[var] tile_spacing: impl Var<Size>,

        render_stops: Vec<RenderGradientStop>,
        render_center: PxPoint,
        final_size: PxSize,
        final_tile_size: PxSize,
        final_tile_spacing: PxSize,
    })]
    impl UiNode for ConicGradientFullNode {
        fn update(&mut self, _: &WidgetUpdates) {
            if self.center.is_new() || self.angle.is_new() || self.stops.is_new() || self.extend_mode.is_new() {
                self.final_size = PxSize::zero();
                WIDGET.layout();
            }
        }

        fn measure(&mut self, _: &mut WidgetMeasure) -> PxSize {
            LAYOUT.constraints().fill_size()
        }

        fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
            let final_size = LAYOUT.constraints().fill_size();
            if self.final_size != final_size {
                self.final_size = final_size;

                self.final_tile_size = self.tile_size.layout_dft(self.final_size);
                self.final_tile_spacing = self.tile_spacing.layout_dft(self.final_size);

                LAYOUT.with_constraints(PxConstraints2d::new_fill_size(self.final_tile_size), || {
                    self.render_center = self
                        .center
                        .get()
                        .layout_dft(self.final_tile_size.to_vector().to_point() * 0.5.fct());
                });

                let perimeter = Px({
                    let a = self.final_tile_size.width.0 as f32;
                    let b = self.final_tile_size.height.0 as f32;
                    std::f32::consts::PI * 2.0 * ((a * a + b * b) / 2.0).sqrt()
                } as _);
                LAYOUT.with_constraints(LAYOUT.constraints().with_exact_x(perimeter), || {
                    self.stops
                        .with(|s| s.layout_radial(true, self.extend_mode.get(), &mut self.render_stops))
                });

                WIDGET.render();
            }
            final_size
        }

        fn render(&mut self, frame: &mut FrameBuilder) {
            frame.push_conic_gradient(
                PxRect::from_size(self.final_size),
                self.render_center,
                self.angle.get(),
                &self.render_stops,
                self.extend_mode.get().into(),
                self.final_tile_size,
                self.final_tile_spacing,
            );
        }
    }
    ConicGradientFullNode {
        angle: angle.into_var(),
        center: center.into_var(),
        stops: stops.into_var(),
        extend_mode: extend_mode.into_var(),
        tile_size: tile_size.into_var(),
        tile_spacing: tile_spacing.into_var(),

        render_stops: vec![],
        render_center: PxPoint::zero(),
        final_size: PxSize::zero(),
        final_tile_size: PxSize::zero(),
        final_tile_spacing: PxSize::zero(),
    }
}
