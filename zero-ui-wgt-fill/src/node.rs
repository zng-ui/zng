//! Color and gradient fill nodes and builders.

use zero_ui_wgt::prelude::{gradient::*, *};

/// Gradient builder start.
///
/// Use [`gradient`] to start building.
///
/// [`gradient`]: fn@gradient
pub struct GradientBuilder<S> {
    stops: S,
}

/// Starts building a gradient with the color stops.
pub fn gradient<S>(stops: S) -> GradientBuilder<S::Var>
where
    S: IntoVar<GradientStops>,
{
    GradientBuilder { stops: stops.into_var() }
}

/// Starts building a linear gradient with the axis and color stops.
///
/// Returns a node that is also a builder that can be used to refine the gradient definition.
pub fn linear_gradient<A: IntoVar<LinearGradientAxis>, S: IntoVar<GradientStops>>(
    axis: A,
    stops: S,
) -> LinearGradient<S::Var, A::Var, LocalVar<ExtendMode>> {
    gradient(stops).linear(axis)
}

/// Starts building a radial gradient with the radius and color stops.
///
/// Returns a node that is also a builder that can be used to refine the gradient definition.
pub fn radial_gradient<C, R, S>(center: C, radius: R, stops: S) -> RadialGradient<S::Var, C::Var, R::Var, LocalVar<ExtendMode>>
where
    C: IntoVar<Point>,
    R: IntoVar<GradientRadius>,
    S: IntoVar<GradientStops>,
{
    gradient(stops).radial(center, radius)
}

/// Starts building a conic gradient with the angle and color stops.
///
/// Returns a node that is also a builder that can be used to refine the gradient definition.
pub fn conic_gradient<C, A, S>(center: C, angle: A, stops: S) -> ConicGradient<S::Var, C::Var, A::Var, LocalVar<ExtendMode>>
where
    C: IntoVar<Point>,
    A: IntoVar<AngleRadian>,
    S: IntoVar<GradientStops>,
{
    gradient(stops).conic(center, angle)
}

impl<S> GradientBuilder<S>
where
    S: Var<GradientStops>,
{
    /// Builds a linear gradient.
    ///
    /// Returns a node that fills the available space with the gradient, the node type doubles
    /// as a builder that can continue building a linear gradient.
    pub fn linear<A>(self, axis: A) -> LinearGradient<S, A::Var, LocalVar<ExtendMode>>
    where
        A: IntoVar<LinearGradientAxis>,
    {
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
    pub fn radial<C, R>(self, center: C, radius: R) -> RadialGradient<S, C::Var, R::Var, LocalVar<ExtendMode>>
    where
        C: IntoVar<Point>,
        R: IntoVar<GradientRadius>,
    {
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
    pub fn conic<C: IntoVar<Point>, A: IntoVar<AngleRadian>>(
        self,
        center: C,
        angle: A,
    ) -> ConicGradient<S, C::Var, A::Var, LocalVar<ExtendMode>> {
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
/// gradient.
///
/// Use [`gradient`] or [`linear_gradient`] to build.
///
/// [`gradient`]: fn@gradient
pub struct LinearGradient<S, A, E> {
    stops: S,
    axis: A,
    extend_mode: E,

    data: LinearNodeData,
}
impl<S, A, E> LinearGradient<S, A, E>
where
    S: Var<GradientStops>,
    A: Var<LinearGradientAxis>,
    E: Var<ExtendMode>,
{
    /// Sets the extend mode of the linear gradient.
    ///
    /// By default is [`ExtendMode::Clamp`].
    pub fn extend_mode<E2: IntoVar<ExtendMode>>(self, mode: E2) -> LinearGradient<S, A, E2::Var> {
        LinearGradient {
            stops: self.stops,
            axis: self.axis,
            extend_mode: mode.into_var(),
            data: self.data,
        }
    }

    /// Sets the extend mode to [`ExtendMode::Repeat`].
    pub fn repeat(self) -> LinearGradient<S, A, LocalVar<ExtendMode>> {
        self.extend_mode(ExtendMode::Repeat)
    }

    /// Sets the extend mode to [`ExtendMode::Reflect`].
    pub fn reflect(self) -> LinearGradient<S, A, LocalVar<ExtendMode>> {
        self.extend_mode(ExtendMode::Reflect)
    }

    /// Continue building a tiled linear gradient.
    pub fn tile<T, TS>(self, tile_size: T, tile_spacing: TS) -> TiledLinearGradient<S, A, E, LocalVar<Point>, T::Var, TS::Var>
    where
        T: IntoVar<Size>,
        TS: IntoVar<Size>,
    {
        TiledLinearGradient {
            stops: self.stops,
            axis: self.axis,
            extend_mode: self.extend_mode,
            tile_origin: LocalVar(Point::zero()),
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
    pub fn tile_size<T>(self, size: T) -> TiledLinearGradient<S, A, E, LocalVar<Point>, T::Var, LocalVar<Size>>
    where
        T: IntoVar<Size>,
    {
        self.tile(size, Size::zero())
    }
}

/// Tiled linear gradient.
///
/// Can be used as a node that fills the available space with the gradient tiles, or can continue building a
/// repeating linear gradient.
///
///
/// Use [`gradient`], [`linear_gradient`] to build.
///
/// [`gradient`]: fn@gradient
pub struct TiledLinearGradient<S, A, E, O, T, TS> {
    stops: S,
    axis: A,
    extend_mode: E,
    tile_origin: O,
    tile_size: T,
    tile_spacing: TS,
    data: LinearNodeData,
    tile_data: TiledNodeData,
}
impl<S, A, E, O, T, TS> TiledLinearGradient<S, A, E, O, T, TS>
where
    S: Var<GradientStops>,
    A: Var<LinearGradientAxis>,
    E: Var<ExtendMode>,
    O: Var<Point>,
    T: Var<Size>,
    TS: Var<Size>,
{
    /// Set the space between tiles.
    ///
    /// Relative values are resolved on the tile size, so setting this to `100.pct()` will
    /// *skip* a tile.
    ///
    /// Leftover values are resolved on the space taken by tiles that do not
    /// fully fit in the available space, so setting this to `1.lft()` will cause the *border* tiles
    /// to always touch the full bounds and the middle filled with the maximum full tiles that fit or
    /// empty space.
    pub fn tile_spacing<TS2>(self, spacing: TS2) -> TiledLinearGradient<S, A, E, O, T, TS2::Var>
    where
        TS2: IntoVar<Size>,
    {
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
    pub fn tile_origin<O2>(self, origin: O2) -> TiledLinearGradient<S, A, E, O2::Var, T, TS>
    where
        O2: IntoVar<Point>,
    {
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
/// Can be used as a node that fills the available space with the gradient, or can continue building a linear
/// gradient.
///
/// Use [`gradient`] or [`radial_gradient`] to build.
///  
/// [`gradient`]: fn@gradient
pub struct RadialGradient<S, C, R, E> {
    stops: S,
    center: C,
    radius: R,
    extend_mode: E,

    data: RadialNodeData,
}
impl<S, C, R, E> RadialGradient<S, C, R, E>
where
    S: Var<GradientStops>,
    C: Var<Point>,
    R: Var<GradientRadius>,
    E: Var<ExtendMode>,
{
    /// Sets the extend mode of the radial gradient.
    ///
    /// By default is [`ExtendMode::Clamp`].
    pub fn extend_mode<E2: IntoVar<ExtendMode>>(self, mode: E2) -> RadialGradient<S, C, R, E2::Var> {
        RadialGradient {
            stops: self.stops,
            center: self.center,
            radius: self.radius,
            extend_mode: mode.into_var(),
            data: self.data,
        }
    }

    /// Sets the extend mode to [`ExtendMode::Repeat`].
    pub fn repeat(self) -> RadialGradient<S, C, R, LocalVar<ExtendMode>> {
        self.extend_mode(ExtendMode::Repeat)
    }

    /// Sets the extend mode to [`ExtendMode::Reflect`].
    pub fn reflect(self) -> RadialGradient<S, C, R, LocalVar<ExtendMode>> {
        self.extend_mode(ExtendMode::Reflect)
    }

    /// Continue building a tiled radial gradient.
    pub fn tile<T, TS>(self, tile_size: T, tile_spacing: TS) -> TiledRadialGradient<S, C, R, E, LocalVar<Point>, T::Var, TS::Var>
    where
        T: IntoVar<Size>,
        TS: IntoVar<Size>,
    {
        TiledRadialGradient {
            stops: self.stops,
            center: self.center,
            radius: self.radius,
            extend_mode: self.extend_mode,
            tile_origin: LocalVar(Point::zero()),
            tile_size: tile_size.into_var(),
            tile_spacing: tile_spacing.into_var(),
            data: self.data,
            tile_data: TiledNodeData::default(),
        }
    }

    /// Continue building a tiled radial gradient.
    pub fn tile_size<T>(self, size: T) -> TiledRadialGradient<S, C, R, E, LocalVar<Point>, T::Var, LocalVar<Size>>
    where
        T: IntoVar<Size>,
    {
        self.tile(size, Size::zero())
    }
}

/// Tiled radial gradient.
///
/// Can be used as a node that fills the available space with the gradient tiles, or can continue building the gradient.
///
///
/// Use [`gradient`], [`radial_gradient`] to build.
///  
/// [`gradient`]: fn@gradient
pub struct TiledRadialGradient<S, C, R, E, O, T, TS> {
    stops: S,
    center: C,
    radius: R,
    extend_mode: E,
    tile_origin: O,
    tile_size: T,
    tile_spacing: TS,
    data: RadialNodeData,
    tile_data: TiledNodeData,
}
impl<S, C, R, E, O, T, TS> TiledRadialGradient<S, C, R, E, O, T, TS>
where
    S: Var<GradientStops>,
    C: Var<Point>,
    R: Var<GradientRadius>,
    E: Var<ExtendMode>,
    O: Var<Point>,
    T: Var<Size>,
    TS: Var<Size>,
{
    /// Set the space between tiles.
    ///
    /// Relative values are resolved on the tile size, so setting this to `100.pct()` will
    /// *skip* a tile.
    ///
    /// Leftover values are resolved on the space taken by tiles that do not
    /// fully fit in the available space, so setting this to `1.lft()` will cause the *border* tiles
    /// to always touch the full bounds and the middle filled with the maximum full tiles that fit or
    /// empty space.
    pub fn tile_spacing<TS2>(self, spacing: TS2) -> TiledRadialGradient<S, C, R, E, O, T, TS2::Var>
    where
        TS2: IntoVar<Size>,
    {
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
    pub fn tile_origin<O2>(self, origin: O2) -> TiledRadialGradient<S, C, R, E, O2::Var, T, TS>
    where
        O2: IntoVar<Point>,
    {
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
/// Can be used as a node that fills the available space with the gradient, or can continue building a linear
/// gradient.
///
/// Use [`gradient`] or [`conic_gradient`] to build.
///  
/// [`gradient`]: fn@gradient
pub struct ConicGradient<S, C, A, E> {
    stops: S,
    center: C,
    angle: A,
    extend_mode: E,

    data: ConicNodeData,
}
impl<S, C, A, E> ConicGradient<S, C, A, E>
where
    S: Var<GradientStops>,
    C: Var<Point>,
    A: Var<AngleRadian>,
    E: Var<ExtendMode>,
{
    /// Sets the extend mode of the conic gradient.
    ///
    /// By default is [`ExtendMode::Clamp`].
    pub fn extend_mode<E2: IntoVar<ExtendMode>>(self, mode: E2) -> ConicGradient<S, C, A, E2::Var> {
        ConicGradient {
            stops: self.stops,
            center: self.center,
            angle: self.angle,
            extend_mode: mode.into_var(),
            data: self.data,
        }
    }

    /// Sets the extend mode to [`ExtendMode::Repeat`].
    pub fn repeat(self) -> ConicGradient<S, C, A, LocalVar<ExtendMode>> {
        self.extend_mode(ExtendMode::Repeat)
    }

    /// Sets the extend mode to [`ExtendMode::Reflect`].
    pub fn reflect(self) -> ConicGradient<S, C, A, LocalVar<ExtendMode>> {
        self.extend_mode(ExtendMode::Reflect)
    }

    /// Continue building a tiled radial gradient.
    pub fn tile<T, TS>(self, tile_size: T, tile_spacing: TS) -> TiledConicGradient<S, C, A, E, LocalVar<Point>, T::Var, TS::Var>
    where
        T: IntoVar<Size>,
        TS: IntoVar<Size>,
    {
        TiledConicGradient {
            stops: self.stops,
            center: self.center,
            angle: self.angle,
            extend_mode: self.extend_mode,
            tile_origin: LocalVar(Point::zero()),
            tile_size: tile_size.into_var(),
            tile_spacing: tile_spacing.into_var(),
            data: self.data,
            tile_data: TiledNodeData::default(),
        }
    }

    /// Continue building a tiled radial gradient.
    pub fn tile_size<T>(self, size: T) -> TiledConicGradient<S, C, A, E, LocalVar<Point>, T::Var, LocalVar<Size>>
    where
        T: IntoVar<Size>,
    {
        self.tile(size, Size::zero())
    }
}

/// Tiled conic gradient.
///
/// Can be used as a node that fills the available space with the gradient tiles, or can continue building the gradient.
///
/// Use [`gradient`], [`conic_gradient`] to build.
///  
/// [`gradient`]: fn@gradient
pub struct TiledConicGradient<S, C, A, E, O, T, TS> {
    stops: S,
    center: C,
    angle: A,
    extend_mode: E,
    tile_origin: O,
    tile_size: T,
    tile_spacing: TS,
    data: ConicNodeData,
    tile_data: TiledNodeData,
}
impl<S, C, A, E, O, T, TS> TiledConicGradient<S, C, A, E, O, T, TS>
where
    S: Var<GradientStops>,
    C: Var<Point>,
    A: Var<AngleRadian>,
    E: Var<ExtendMode>,
    O: Var<Point>,
    T: Var<Size>,
    TS: Var<Size>,
{
    /// Set the space between tiles.
    ///
    /// Relative values are resolved on the tile size, so setting this to `100.pct()` will
    /// *skip* a tile.
    ///
    /// Leftover values are resolved on the space taken by tiles that do not
    /// fully fit in the available space, so setting this to `1.lft()` will cause the *border* tiles
    /// to always touch the full bounds and the middle filled with the maximum full tiles that fit or
    /// empty space.
    pub fn tile_spacing<TS2>(self, spacing: TS2) -> TiledConicGradient<S, C, A, E, O, T, TS2::Var>
    where
        TS2: IntoVar<Size>,
    {
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
    pub fn tile_origin<O2>(self, origin: O2) -> TiledConicGradient<S, C, A, E, O2::Var, T, TS>
    where
        O2: IntoVar<Point>,
    {
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
impl<S, A, E> UiNode for LinearGradient<S, A, E>
where
    S: Var<GradientStops>,
    A: Var<LinearGradientAxis>,
    E: Var<ExtendMode>,
{
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
impl<S, A, E, O, T, TS> UiNode for TiledLinearGradient<S, A, E, O, T, TS>
where
    S: Var<GradientStops>,
    A: Var<LinearGradientAxis>,
    E: Var<ExtendMode>,
    O: Var<Point>,
    T: Var<Size>,
    TS: Var<Size>,
{
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
impl<S, C, R, E> UiNode for RadialGradient<S, C, R, E>
where
    S: Var<GradientStops>,
    C: Var<Point>,
    R: Var<GradientRadius>,
    E: Var<ExtendMode>,
{
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
impl<S, C, R, E, O, T, TS> UiNode for TiledRadialGradient<S, C, R, E, O, T, TS>
where
    S: Var<GradientStops>,
    C: Var<Point>,
    R: Var<GradientRadius>,
    E: Var<ExtendMode>,
    O: Var<Point>,
    T: Var<Size>,
    TS: Var<Size>,
{
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
impl<S, C, A, E> UiNode for ConicGradient<S, C, A, E>
where
    S: Var<GradientStops>,
    C: Var<Point>,
    A: Var<AngleRadian>,
    E: Var<ExtendMode>,
{
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
impl<S, C, A, E, O, T, TS> UiNode for TiledConicGradient<S, C, A, E, O, T, TS>
where
    S: Var<GradientStops>,
    C: Var<Point>,
    A: Var<AngleRadian>,
    E: Var<ExtendMode>,
    O: Var<Point>,
    T: Var<Size>,
    TS: Var<Size>,
{
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
/// Note that this node is not a full widget, it can be used as part of an widget without adding to the info tree.
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
            frame.push_color(PxRect::from_size(render_size), frame_key.bind_var(&color, |&c| c.into()));
        }
        UiNodeOp::RenderUpdate { update } => {
            update.update_color_opt(frame_key.update_var(&color, |&c| c.into()));
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
