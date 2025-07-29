//! Color and gradient fill nodes and builders.

use zng_wgt::prelude::{gradient::*, *};

/// Gradient builder start.
///
/// Use [`gradient`] to start building.
///
/// [`gradient`]: fn@gradient
pub struct GradientBuilder {
    stops: Var<GradientStops>,
}

/// Starts building a gradient with the color stops.
pub fn gradient(stops: impl IntoVar<GradientStops>) -> GradientBuilder {
    GradientBuilder { stops: stops.into_var() }
}

/// Starts building a linear gradient with the axis and color stops.
///
/// Returns a node that is also a builder that can be used to refine the gradient definition.
pub fn linear_gradient(axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> LinearGradient {
    gradient(stops).linear(axis)
}

/// Starts building a radial gradient with the radius and color stops.
///
/// Returns a node that is also a builder that can be used to refine the gradient definition.
pub fn radial_gradient(
    center: impl IntoVar<Point>,
    radius: impl IntoVar<GradientRadius>,
    stops: impl IntoVar<GradientStops>,
) -> RadialGradient {
    gradient(stops).radial(center, radius)
}

/// Starts building a conic gradient with the angle and color stops.
///
/// Returns a node that is also a builder that can be used to refine the gradient definition.
pub fn conic_gradient(center: impl IntoVar<Point>, angle: impl IntoVar<AngleRadian>, stops: impl IntoVar<GradientStops>) -> ConicGradient {
    gradient(stops).conic(center, angle)
}

impl GradientBuilder {
    /// Builds a linear gradient.
    ///
    /// Returns a node that fills the available space with the gradient, the node type doubles
    /// as a builder that can continue building a linear gradient.
    pub fn linear(self, axis: impl IntoVar<LinearGradientAxis>) -> LinearGradient {
        LinearGradient {
            stops: self.stops,
            axis: axis.into_var(),
            extend_mode: ExtendMode::Clamp.into_var(),

            data: LinearNodeData::default(),
        }
    }

    /// Builds a radial gradient.
    ///
    /// Returns a node that fills the available space with the gradient, the node type doubles
    /// as a builder that can continue building a radial gradient.
    pub fn radial(self, center: impl IntoVar<Point>, radius: impl IntoVar<GradientRadius>) -> RadialGradient {
        RadialGradient {
            stops: self.stops,
            center: center.into_var(),
            radius: radius.into_var(),
            extend_mode: ExtendMode::Clamp.into_var(),
            data: RadialNodeData::default(),
        }
    }

    /// Builds a conic gradient.
    ///
    /// Returns a node that fills the available space with the gradient, the node type doubles
    /// as a builder that can continue building a conic gradient.
    pub fn conic(self, center: impl IntoVar<Point>, angle: impl IntoVar<AngleRadian>) -> ConicGradient {
        ConicGradient {
            stops: self.stops,
            center: center.into_var(),
            angle: angle.into_var(),
            extend_mode: ExtendMode::Clamp.into_var(),
            data: ConicNodeData::default(),
        }
    }
}

/// Linear gradient.
///
/// Can be used as a node that fills the available space with the gradient, or can continue building a linear
/// or tiled linear gradient.
///
/// Use [`gradient`] or [`linear_gradient`] to start building.
///
/// [`gradient`]: fn@gradient
pub struct LinearGradient {
    stops: Var<GradientStops>,
    axis: Var<LinearGradientAxis>,
    extend_mode: Var<ExtendMode>,

    data: LinearNodeData,
}
impl LinearGradient {
    /// Sets the extend mode of the linear gradient.
    ///
    /// By default is [`ExtendMode::Clamp`].
    ///
    /// [`ExtendMode::Clamp`]: zng_wgt::prelude::gradient::ExtendMode::Clamp
    pub fn extend_mode(self, mode: impl IntoVar<ExtendMode>) -> LinearGradient {
        LinearGradient {
            stops: self.stops,
            axis: self.axis,
            extend_mode: mode.into_var(),
            data: self.data,
        }
    }

    /// Sets the extend mode to [`ExtendMode::Repeat`].
    ///
    /// [`ExtendMode::Repeat`]: zng_wgt::prelude::gradient::ExtendMode::Repeat
    pub fn repeat(self) -> LinearGradient {
        self.extend_mode(ExtendMode::Repeat)
    }

    /// Sets the extend mode to [`ExtendMode::Reflect`].
    ///
    /// [`ExtendMode::Reflect`]: zng_wgt::prelude::gradient::ExtendMode::Reflect
    pub fn reflect(self) -> LinearGradient {
        self.extend_mode(ExtendMode::Reflect)
    }

    /// Continue building a tiled linear gradient.
    pub fn tile(self, tile_size: impl IntoVar<Size>, tile_spacing: impl IntoVar<Size>) -> TiledLinearGradient {
        TiledLinearGradient {
            stops: self.stops,
            axis: self.axis,
            extend_mode: self.extend_mode,
            tile_origin: const_var(Point::zero()),
            tile_size: tile_size.into_var(),
            tile_spacing: tile_spacing.into_var(),
            data: self.data,
            tile_data: TiledNodeData::default(),
        }
    }

    /// Continue building a tiled linear gradient.
    ///
    /// Relative values are resolved on the full available size, so settings this to `100.pct()` is
    /// the same as not tiling.
    pub fn tile_size(self, size: impl IntoVar<Size>) -> TiledLinearGradient {
        self.tile(size, Size::zero())
    }
}

/// Tiled linear gradient.
///
/// Can be used as a node that fills the available space with the gradient tiles, or can continue building a
/// tiled linear gradient.
///
/// Use [`LinearGradient::tile`] to build.
pub struct TiledLinearGradient {
    stops: Var<GradientStops>,
    axis: Var<LinearGradientAxis>,
    extend_mode: Var<ExtendMode>,
    tile_origin: Var<Point>,
    tile_size: Var<Size>,
    tile_spacing: Var<Size>,
    data: LinearNodeData,
    tile_data: TiledNodeData,
}
impl TiledLinearGradient {
    /// Set the space between tiles.
    ///
    /// Relative values are resolved on the tile size, so setting this to `100.pct()` will
    /// *skip* a tile.
    ///
    /// Leftover values are resolved on the space taken by tiles that do not
    /// fully fit in the available space, so setting this to `1.lft()` will cause the *border* tiles
    /// to always touch the full bounds and the middle filled with the maximum full tiles that fit or
    /// empty space.
    pub fn tile_spacing(self, spacing: impl IntoVar<Size>) -> TiledLinearGradient {
        TiledLinearGradient {
            stops: self.stops,
            axis: self.axis,
            extend_mode: self.extend_mode,
            tile_origin: self.tile_origin,
            tile_size: self.tile_size,
            tile_spacing: spacing.into_var(),
            data: self.data,
            tile_data: self.tile_data,
        }
    }

    /// Sets the tile offset.
    ///
    /// Relative values are resolved on the tile size, so setting this to `100.pct()` will
    /// offset a full *turn*.
    pub fn tile_origin(self, origin: impl IntoVar<Point>) -> TiledLinearGradient {
        TiledLinearGradient {
            stops: self.stops,
            axis: self.axis,
            extend_mode: self.extend_mode,
            tile_origin: origin.into_var(),
            tile_size: self.tile_size,
            tile_spacing: self.tile_spacing,
            data: self.data,
            tile_data: self.tile_data,
        }
    }
}

/// Radial gradient.
///
/// Can be used as a node that fills the available space with the gradient, or can continue building a radial
/// or tiled radial gradient.
///
/// Use [`gradient`] or [`radial_gradient`] to start building.
///  
/// [`gradient`]: fn@gradient
pub struct RadialGradient {
    stops: Var<GradientStops>,
    center: Var<Point>,
    radius: Var<GradientRadius>,
    extend_mode: Var<ExtendMode>,

    data: RadialNodeData,
}
impl RadialGradient {
    /// Sets the extend mode of the radial gradient.
    ///
    /// By default is [`ExtendMode::Clamp`].
    ///
    /// [`ExtendMode::Clamp`]: zng_wgt::prelude::gradient::ExtendMode::Clamp
    pub fn extend_mode(self, mode: impl IntoVar<ExtendMode>) -> RadialGradient {
        RadialGradient {
            stops: self.stops,
            center: self.center,
            radius: self.radius,
            extend_mode: mode.into_var(),
            data: self.data,
        }
    }

    /// Sets the extend mode to [`ExtendMode::Repeat`].
    ///
    /// [`ExtendMode::Repeat`]: zng_wgt::prelude::gradient::ExtendMode::Repeat
    pub fn repeat(self) -> RadialGradient {
        self.extend_mode(ExtendMode::Repeat)
    }

    /// Sets the extend mode to [`ExtendMode::Reflect`].
    ///
    /// [`ExtendMode::Reflect`]: zng_wgt::prelude::gradient::ExtendMode::Reflect
    pub fn reflect(self) -> RadialGradient {
        self.extend_mode(ExtendMode::Reflect)
    }

    /// Continue building a tiled radial gradient.
    pub fn tile(self, tile_size: impl IntoVar<Size>, tile_spacing: impl IntoVar<Size>) -> TiledRadialGradient {
        TiledRadialGradient {
            stops: self.stops,
            center: self.center,
            radius: self.radius,
            extend_mode: self.extend_mode,
            tile_origin: const_var(Point::zero()),
            tile_size: tile_size.into_var(),
            tile_spacing: tile_spacing.into_var(),
            data: self.data,
            tile_data: TiledNodeData::default(),
        }
    }

    /// Continue building a tiled radial gradient.
    pub fn tile_size(self, size: impl IntoVar<Size>) -> TiledRadialGradient {
        self.tile(size, Size::zero())
    }
}

/// Tiled radial gradient.
///
/// Can be used as a node that fills the available space with the gradient tiles, or can continue building the gradient.
///
/// Use [`RadialGradient::tile`] to build.
///  
/// [`gradient`]: fn@gradient
pub struct TiledRadialGradient {
    stops: Var<GradientStops>,
    center: Var<Point>,
    radius: Var<GradientRadius>,
    extend_mode: Var<ExtendMode>,
    tile_origin: Var<Point>,
    tile_size: Var<Size>,
    tile_spacing: Var<Size>,
    data: RadialNodeData,
    tile_data: TiledNodeData,
}
impl TiledRadialGradient {
    /// Set the space between tiles.
    ///
    /// Relative values are resolved on the tile size, so setting this to `100.pct()` will
    /// *skip* a tile.
    ///
    /// Leftover values are resolved on the space taken by tiles that do not
    /// fully fit in the available space, so setting this to `1.lft()` will cause the *border* tiles
    /// to always touch the full bounds and the middle filled with the maximum full tiles that fit or
    /// empty space.
    pub fn tile_spacing(self, spacing: impl IntoVar<Size>) -> TiledRadialGradient {
        TiledRadialGradient {
            stops: self.stops,
            center: self.center,
            radius: self.radius,
            extend_mode: self.extend_mode,
            tile_origin: self.tile_origin,
            tile_size: self.tile_size,
            tile_spacing: spacing.into_var(),
            data: self.data,
            tile_data: self.tile_data,
        }
    }

    /// Sets the tile offset.
    ///
    /// Relative values are resolved on the tile size, so setting this to `100.pct()` will
    /// offset a full *turn*.
    pub fn tile_origin(self, origin: impl IntoVar<Point>) -> TiledRadialGradient {
        TiledRadialGradient {
            stops: self.stops,
            center: self.center,
            radius: self.radius,
            extend_mode: self.extend_mode,
            tile_origin: origin.into_var(),
            tile_size: self.tile_size,
            tile_spacing: self.tile_spacing,
            data: self.data,
            tile_data: self.tile_data,
        }
    }
}

/// Conic gradient.
///
/// Can be used as a node that fills the available space with the gradient, or can continue building the conic
/// or a tiled conic gradient.
///
/// Use [`gradient`] or [`conic_gradient`] to start building.
///  
/// [`gradient`]: fn@gradient
pub struct ConicGradient {
    stops: Var<GradientStops>,
    center: Var<Point>,
    angle: Var<AngleRadian>,
    extend_mode: Var<ExtendMode>,

    data: ConicNodeData,
}
impl ConicGradient {
    /// Sets the extend mode of the conic gradient.
    ///
    /// By default is [`ExtendMode::Clamp`].
    ///
    /// [`ExtendMode::Clamp`]: zng_wgt::prelude::gradient::ExtendMode::Clamp
    pub fn extend_mode(self, mode: impl IntoVar<ExtendMode>) -> ConicGradient {
        ConicGradient {
            stops: self.stops,
            center: self.center,
            angle: self.angle,
            extend_mode: mode.into_var(),
            data: self.data,
        }
    }

    /// Sets the extend mode to [`ExtendMode::Repeat`].
    ///
    /// [`ExtendMode::Repeat`]: zng_wgt::prelude::gradient::ExtendMode::Repeat
    pub fn repeat(self) -> ConicGradient {
        self.extend_mode(ExtendMode::Repeat)
    }

    /// Sets the extend mode to [`ExtendMode::Reflect`].
    ///
    /// [`ExtendMode::Reflect`]: zng_wgt::prelude::gradient::ExtendMode::Reflect
    pub fn reflect(self) -> ConicGradient {
        self.extend_mode(ExtendMode::Reflect)
    }

    /// Continue building a tiled radial gradient.
    pub fn tile(self, tile_size: impl IntoVar<Size>, tile_spacing: impl IntoVar<Size>) -> TiledConicGradient {
        TiledConicGradient {
            stops: self.stops,
            center: self.center,
            angle: self.angle,
            extend_mode: self.extend_mode,
            tile_origin: const_var(Point::zero()),
            tile_size: tile_size.into_var(),
            tile_spacing: tile_spacing.into_var(),
            data: self.data,
            tile_data: TiledNodeData::default(),
        }
    }

    /// Continue building a tiled radial gradient.
    pub fn tile_size(self, size: impl IntoVar<Size>) -> TiledConicGradient {
        self.tile(size, Size::zero())
    }
}

/// Tiled conic gradient.
///
/// Can be used as a node that fills the available space with the gradient tiles, or can continue building the gradient.
///
/// Use [`ConicGradient::tile`] to build.
///  
/// [`gradient`]: fn@gradient
pub struct TiledConicGradient {
    stops: Var<GradientStops>,
    center: Var<Point>,
    angle: Var<AngleRadian>,
    extend_mode: Var<ExtendMode>,
    tile_origin: Var<Point>,
    tile_size: Var<Size>,
    tile_spacing: Var<Size>,
    data: ConicNodeData,
    tile_data: TiledNodeData,
}
impl TiledConicGradient {
    /// Set the space between tiles.
    ///
    /// Relative values are resolved on the tile size, so setting this to `100.pct()` will
    /// *skip* a tile.
    ///
    /// Leftover values are resolved on the space taken by tiles that do not
    /// fully fit in the available space, so setting this to `1.lft()` will cause the *border* tiles
    /// to always touch the full bounds and the middle filled with the maximum full tiles that fit or
    /// empty space.
    pub fn tile_spacing<TS2>(self, spacing: impl IntoVar<Size>) -> TiledConicGradient {
        TiledConicGradient {
            stops: self.stops,
            center: self.center,
            angle: self.angle,
            extend_mode: self.extend_mode,
            tile_origin: self.tile_origin,
            tile_size: self.tile_size,
            tile_spacing: spacing.into_var(),
            data: self.data,
            tile_data: self.tile_data,
        }
    }

    /// Sets the tile offset.
    ///
    /// Relative values are resolved on the tile size, so setting this to `100.pct()` will
    /// offset a full *turn*.
    pub fn tile_origin(self, origin: impl IntoVar<Point>) -> TiledConicGradient {
        TiledConicGradient {
            stops: self.stops,
            center: self.center,
            angle: self.angle,
            extend_mode: self.extend_mode,
            tile_origin: origin.into_var(),
            tile_size: self.tile_size,
            tile_spacing: self.tile_spacing,
            data: self.data,
            tile_data: self.tile_data,
        }
    }
}

#[derive(Default)]
struct LinearNodeData {
    line: PxLine,
    stops: Vec<RenderGradientStop>,
    size: PxSize,
}
#[ui_node(none)]
impl UiNode for LinearGradient {
    fn init(&mut self) {
        WIDGET.sub_var_layout(&self.axis).sub_var(&self.stops).sub_var(&self.extend_mode);
    }

    fn measure(&mut self, _: &mut WidgetMeasure) -> PxSize {
        LAYOUT.constraints().fill_size()
    }

    fn update(&mut self, _: &WidgetUpdates) {
        if self.stops.is_new() || self.extend_mode.is_new() {
            WIDGET.layout().render();
        }
    }

    fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
        let size = LAYOUT.constraints().fill_size();
        let axis = self.axis.layout();
        if self.data.size != size || self.data.line != axis {
            self.data.size = size;
            self.data.line = axis;
            WIDGET.render();
        }

        let length = self.data.line.length();
        LAYOUT.with_constraints(LAYOUT.constraints().with_new_exact_x(length), || {
            self.stops
                .with(|s| s.layout_linear(LayoutAxis::X, self.extend_mode.get(), &mut self.data.line, &mut self.data.stops))
        });

        size
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        frame.push_linear_gradient(
            PxRect::from_size(self.data.size),
            self.data.line,
            &self.data.stops,
            self.extend_mode.get().into(),
            PxPoint::zero(),
            self.data.size,
            PxSize::zero(),
        );
    }
}

#[derive(Default)]
struct TiledNodeData {
    origin: PxPoint,
    size: PxSize,
    spacing: PxSize,
}
#[ui_node(none)]
impl UiNode for TiledLinearGradient {
    fn init(&mut self) {
        WIDGET
            .sub_var_layout(&self.axis)
            .sub_var(&self.stops)
            .sub_var(&self.extend_mode)
            .sub_var_layout(&self.tile_origin)
            .sub_var_layout(&self.tile_size)
            .sub_var_layout(&self.tile_spacing);
    }

    fn update(&mut self, _: &WidgetUpdates) {
        if self.stops.is_new() || self.extend_mode.is_new() {
            WIDGET.layout().render();
        }
    }

    fn measure(&mut self, _: &mut WidgetMeasure) -> PxSize {
        LAYOUT.constraints().fill_size()
    }

    fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
        let constraints = LAYOUT.constraints();
        let size = constraints.fill_size();
        let axis = self.axis.layout();
        let tile_size = self.tile_size.layout_dft(size);

        let mut request_render = false;

        if self.data.size != size || self.data.line != axis || self.tile_data.size != tile_size {
            self.data.size = size;
            self.data.line = self.axis.layout();
            self.tile_data.size = tile_size;
            request_render = true;
        }

        LAYOUT.with_constraints(PxConstraints2d::new_exact_size(self.tile_data.size), || {
            let leftover = tile_leftover(self.tile_data.size, size);
            LAYOUT.with_leftover(Some(leftover.width), Some(leftover.height), || {
                let spacing = self.tile_spacing.layout();
                request_render |= self.tile_data.spacing != spacing;
                self.tile_data.spacing = spacing;
            });
            let origin = self.tile_origin.layout();
            request_render |= self.tile_data.origin != origin;
            self.tile_data.origin = origin;
        });

        let length = self.data.line.length();
        LAYOUT.with_constraints(constraints.with_new_exact_x(length), || {
            self.stops
                .with(|s| s.layout_linear(LayoutAxis::X, self.extend_mode.get(), &mut self.data.line, &mut self.data.stops))
        });

        if request_render {
            WIDGET.render();
        }

        size
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        frame.push_linear_gradient(
            PxRect::from_size(self.data.size),
            self.data.line,
            &self.data.stops,
            self.extend_mode.get().into(),
            self.tile_data.origin,
            self.tile_data.size,
            self.tile_data.spacing,
        );
    }
}

#[derive(Default)]
struct RadialNodeData {
    size: PxSize,
    center: PxPoint,
    radius: PxSize,
    stops: Vec<RenderGradientStop>,
}

#[ui_node(none)]
impl UiNode for RadialGradient {
    fn init(&mut self) {
        WIDGET
            .sub_var_layout(&self.center)
            .sub_var_layout(&self.radius)
            .sub_var(&self.stops)
            .sub_var(&self.extend_mode);
    }

    fn update(&mut self, _: &WidgetUpdates) {
        if self.stops.is_new() || self.extend_mode.is_new() {
            WIDGET.layout().render();
        }
    }

    fn measure(&mut self, _: &mut WidgetMeasure) -> PxSize {
        LAYOUT.constraints().fill_size()
    }

    fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
        let size = LAYOUT.constraints().fill_size();

        let mut request_render = size != self.data.size;

        self.data.size = size;
        LAYOUT.with_constraints(PxConstraints2d::new_fill_size(size), || {
            let center = self.center.layout_dft(size.to_vector().to_point() * 0.5.fct());
            let radius = self.radius.get().layout(center);
            request_render |= center != self.data.center || radius != self.data.radius;
            self.data.center = center;
            self.data.radius = radius;
        });

        LAYOUT.with_constraints(
            LAYOUT
                .constraints()
                .with_exact_x(self.data.radius.width.max(self.data.radius.height)),
            || {
                self.stops
                    .with(|s| s.layout_radial(LayoutAxis::X, self.extend_mode.get(), &mut self.data.stops))
            },
        );

        if request_render {
            WIDGET.render();
        }

        size
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        frame.push_radial_gradient(
            PxRect::from_size(self.data.size),
            self.data.center,
            self.data.radius,
            &self.data.stops,
            self.extend_mode.get().into(),
            PxPoint::zero(),
            self.data.size,
            PxSize::zero(),
        );
    }
}

#[ui_node(none)]
impl UiNode for TiledRadialGradient {
    fn init(&mut self) {
        WIDGET
            .sub_var_layout(&self.center)
            .sub_var_layout(&self.radius)
            .sub_var(&self.stops)
            .sub_var(&self.extend_mode)
            .sub_var_layout(&self.tile_origin)
            .sub_var_layout(&self.tile_size)
            .sub_var_layout(&self.tile_spacing);
    }

    fn update(&mut self, _: &WidgetUpdates) {
        if self.stops.is_new() || self.extend_mode.is_new() {
            WIDGET.layout().render();
        }
    }

    fn measure(&mut self, _: &mut WidgetMeasure) -> PxSize {
        LAYOUT.constraints().fill_size()
    }

    fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
        let size = LAYOUT.constraints().fill_size();
        let tile_size = self.tile_size.layout_dft(size);

        let mut request_render = size != self.data.size || self.tile_data.size != tile_size;

        self.data.size = size;
        self.tile_data.size = tile_size;

        LAYOUT.with_constraints(PxConstraints2d::new_exact_size(self.tile_data.size), || {
            let leftover = tile_leftover(self.tile_data.size, size);
            LAYOUT.with_leftover(Some(leftover.width), Some(leftover.height), || {
                let spacing = self.tile_spacing.layout();
                request_render |= self.tile_data.spacing != spacing;
                self.tile_data.spacing = spacing;
            });

            let center = self.center.layout_dft(tile_size.to_vector().to_point() * 0.5.fct());
            let radius = self.radius.get().layout(center);
            let origin = self.tile_origin.layout();

            request_render |= self.data.center != center || self.data.radius != radius || self.tile_data.origin != origin;

            self.data.center = center;
            self.data.radius = radius;
            self.tile_data.origin = origin;
        });

        LAYOUT.with_constraints(
            LAYOUT
                .constraints()
                .with_exact_x(self.data.radius.width.max(self.data.radius.height)),
            || {
                self.stops
                    .with(|s| s.layout_radial(LayoutAxis::X, self.extend_mode.get(), &mut self.data.stops))
            },
        );

        if request_render {
            WIDGET.render();
        }

        size
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        frame.push_radial_gradient(
            PxRect::from_size(self.data.size),
            self.data.center,
            self.data.radius,
            &self.data.stops,
            self.extend_mode.get().into(),
            self.tile_data.origin,
            self.tile_data.size,
            self.tile_data.spacing,
        );
    }
}

#[derive(Default)]
struct ConicNodeData {
    size: PxSize,
    center: PxPoint,
    stops: Vec<RenderGradientStop>,
}

#[ui_node(none)]
impl UiNode for ConicGradient {
    fn init(&mut self) {
        WIDGET
            .sub_var_layout(&self.center)
            .sub_var_layout(&self.angle)
            .sub_var(&self.stops)
            .sub_var(&self.extend_mode);
    }

    fn update(&mut self, _: &WidgetUpdates) {
        if self.stops.is_new() || self.extend_mode.is_new() {
            WIDGET.layout().render();
        }
    }

    fn measure(&mut self, _: &mut WidgetMeasure) -> PxSize {
        LAYOUT.constraints().fill_size()
    }

    fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
        let size = LAYOUT.constraints().fill_size();

        let mut request_render = size != self.data.size;

        self.data.size = size;
        LAYOUT.with_constraints(PxConstraints2d::new_fill_size(size), || {
            let center = self.center.layout_dft(size.to_vector().to_point() * 0.5.fct());
            request_render |= self.data.center != center;
            self.data.center = center;
        });

        let perimeter = Px({
            let a = size.width.0 as f32;
            let b = size.height.0 as f32;
            std::f32::consts::PI * 2.0 * ((a * a + b * b) / 2.0).sqrt()
        } as _);
        LAYOUT.with_constraints(LAYOUT.constraints().with_exact_x(perimeter), || {
            self.stops
                .with(|s| s.layout_radial(LayoutAxis::X, self.extend_mode.get(), &mut self.data.stops))
        });

        if request_render {
            WIDGET.render();
        }

        size
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        frame.push_conic_gradient(
            PxRect::from_size(self.data.size),
            self.data.center,
            self.angle.get(),
            &self.data.stops,
            self.extend_mode.get().into(),
            PxPoint::zero(),
            self.data.size,
            PxSize::zero(),
        );
    }
}

#[ui_node(none)]
impl UiNode for TiledConicGradient {
    fn init(&mut self) {
        WIDGET
            .sub_var_layout(&self.center)
            .sub_var_layout(&self.angle)
            .sub_var(&self.stops)
            .sub_var(&self.extend_mode)
            .sub_var_layout(&self.tile_origin)
            .sub_var_layout(&self.tile_size)
            .sub_var_layout(&self.tile_spacing);
    }

    fn update(&mut self, _: &WidgetUpdates) {
        if self.stops.is_new() || self.extend_mode.is_new() {
            WIDGET.layout().render();
        }
    }

    fn measure(&mut self, _: &mut WidgetMeasure) -> PxSize {
        LAYOUT.constraints().fill_size()
    }

    fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
        let size = LAYOUT.constraints().fill_size();
        let tile_size = self.tile_size.layout_dft(size);

        let mut request_render = size != self.data.size || tile_size != self.tile_data.size;

        self.data.size = size;
        self.tile_data.size = tile_size;

        LAYOUT.with_constraints(PxConstraints2d::new_exact_size(tile_size), || {
            let leftover = tile_leftover(tile_size, size);
            LAYOUT.with_leftover(Some(leftover.width), Some(leftover.height), || {
                let spacing = self.tile_spacing.layout();
                request_render |= self.tile_data.spacing != spacing;
                self.tile_data.spacing = spacing;
            });
            let center = self.center.get().layout_dft(tile_size.to_vector().to_point() * 0.5.fct());
            let origin = self.tile_origin.layout();
            request_render |= self.data.center != center || self.tile_data.origin != origin;
            self.data.center = center;
            self.tile_data.origin = origin;
        });

        let perimeter = Px({
            let a = self.tile_data.size.width.0 as f32;
            let b = self.tile_data.size.height.0 as f32;
            std::f32::consts::PI * 2.0 * ((a * a + b * b) / 2.0).sqrt()
        } as _);
        LAYOUT.with_constraints(LAYOUT.constraints().with_exact_x(perimeter), || {
            self.stops
                .with(|s| s.layout_radial(LayoutAxis::X, self.extend_mode.get(), &mut self.data.stops))
        });

        if request_render {
            WIDGET.render();
        }

        size
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        frame.push_conic_gradient(
            PxRect::from_size(self.data.size),
            self.data.center,
            self.angle.get(),
            &self.data.stops,
            self.extend_mode.get().into(),
            self.tile_data.origin,
            self.tile_data.size,
            self.tile_data.spacing,
        );
    }
}

/// Node that fills the widget area with a color.
///
/// Note that this node is not a full widget, it can be used as part of a widget without adding to the info tree.
pub fn flood(color: impl IntoVar<Rgba>) -> impl UiNode {
    let color = color.into_var();
    let mut render_size = PxSize::zero();
    let frame_key = FrameValueKey::new_unique();

    match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render_update(&color);
        }
        UiNodeOp::Measure { desired_size, .. } => {
            *desired_size = LAYOUT.constraints().fill_size();
        }
        UiNodeOp::Layout { final_size, .. } => {
            *final_size = LAYOUT.constraints().fill_size();
            if *final_size != render_size {
                render_size = *final_size;
                WIDGET.render();
            }
        }
        UiNodeOp::Render { frame } => {
            if !render_size.is_empty() {
                frame.push_color(PxRect::from_size(render_size), frame_key.bind_var(&color, |&c| c));
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            if !render_size.is_empty() {
                update.update_color_opt(frame_key.update_var(&color, |&c| c));
            }
        }
        _ => {}
    })
}

fn tile_leftover(tile_size: PxSize, wgt_size: PxSize) -> PxSize {
    if tile_size.is_empty() || wgt_size.is_empty() {
        return PxSize::zero();
    }

    let full_leftover_x = wgt_size.width % tile_size.width;
    let full_leftover_y = wgt_size.height % tile_size.height;
    let full_tiles_x = wgt_size.width / tile_size.width;
    let full_tiles_y = wgt_size.height / tile_size.height;
    let spaces_x = full_tiles_x - Px(1);
    let spaces_y = full_tiles_y - Px(1);
    PxSize::new(
        if spaces_x > Px(0) { full_leftover_x / spaces_x } else { Px(0) },
        if spaces_y > Px(0) { full_leftover_y / spaces_y } else { Px(0) },
    )
}
