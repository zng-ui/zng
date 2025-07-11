//! Gradient image render service.
//!

use std::hash::Hash as _;

use zng_ext_image::{IMAGES, ImageHash, ImageMaskMode, ImageVar};
use zng_ext_window::RenderMode;
use zng_wgt::prelude::{gradient::*, *};

/// Gradient images service.
///
/// Note that you can render gradients directly for most use cases and that is much faster,
/// this service is a helper for cases where you do need an image, such as for use in [`mask_image`].
///
/// Simple axis aligned linear gradients are rendered directly, other gradients are rendered using the [`IMAGES.render_node`] service.
/// The gradient images are cached keyed on the gradient parameters.
///
/// [`mask_image`]: fn@crate::mask::mask_image
/// [`IMAGES.render_node`]: IMAGES::render_node
#[expect(non_camel_case_types)]
pub struct GRADIENT_IMAGES;

impl GRADIENT_IMAGES {
    /// Linear gradient.
    ///
    /// Axis aligned gradients are rendered directly, other gradients are rendered on the view-process.
    pub fn linear(
        &self,
        size: impl Into<Size>,
        scale_factor: impl Into<Factor>,
        mask: Option<ImageMaskMode>,
        axis: impl Into<LinearGradientAxis>,
        stops: impl Into<GradientStops>,
    ) -> ImageVar {
        self.linear_impl(size.into(), scale_factor.into(), mask, axis.into(), stops.into())
    }
    fn linear_impl(
        &self,
        size: Size,
        scale_factor: Factor,
        mask: Option<ImageMaskMode>,
        axis: LinearGradientAxis,
        stops: GradientStops,
    ) -> ImageVar {
        let mut key = ImageHash::hasher();
        size.hash(&mut key);
        scale_factor.hash(&mut key);
        mask.hash(&mut key);
        axis.hash(&mut key);
        stops.hash(&mut key);

        IMAGES.render_node(RenderMode::Software, scale_factor, mask, move || {
            let child = zng_wgt_fill::node::linear_gradient(axis, stops);
            size_node(child, size)
        })
    }

    /// Standard vertical linear gradient.
    ///
    /// The returned image is `(1, 500)` sized, it can *fill* most use cases, specially if used as mask.
    /// The image is rendered directly, it will load faster.
    ///
    /// The gradient `stops` are from top-to-bottom order.
    pub fn linear_vertical(&self, mask: Option<ImageMaskMode>, stops: impl Into<GradientStops>) -> ImageVar {
        self.linear((1, 500), 1.fct(), mask, 180.deg(), stops)
    }

    /// Standard horizontal linear gradient.
    ///
    /// The returned image is `(500, 1)` sized, it can *fill* most use cases, specially if used as mask.
    /// The image is rendered directly, it will load faster.
    ///
    /// The gradient `stops` are from left-to-right order.
    pub fn linear_horizontal(&self, mask: Option<ImageMaskMode>, stops: impl Into<GradientStops>) -> ImageVar {
        self.linear((1, 500), 1.fct(), mask, 90.deg(), stops)
    }

    /// Radial gradient.
    pub fn radial(
        &self,
        size: impl Into<Size>,
        scale_factor: impl Into<Factor>,
        mask: Option<ImageMaskMode>,
        center: impl Into<Point>,
        radius: impl Into<GradientRadius>,
        stops: impl Into<GradientStops>,
    ) -> ImageVar {
        self.radial_impl(size.into(), scale_factor.into(), mask, center.into(), radius.into(), stops.into())
    }
    fn radial_impl(
        &self,
        size: Size,
        scale_factor: Factor,
        mask: Option<ImageMaskMode>,
        center: Point,
        radius: GradientRadius,
        stops: GradientStops,
    ) -> ImageVar {
        IMAGES.render_node(RenderMode::Software, scale_factor, mask, move || {
            let child = zng_wgt_fill::node::radial_gradient(center, radius, stops);
            size_node(child, size)
        })
    }

    /// Conic gradient.
    pub fn conic(
        &self,
        size: impl Into<Size>,
        scale_factor: impl Into<Factor>,
        mask: Option<ImageMaskMode>,
        center: impl Into<Point>,
        angle: impl Into<AngleRadian>,
        stops: impl Into<GradientStops>,
    ) -> ImageVar {
        self.conic_impl(size.into(), scale_factor.into(), mask, center.into(), angle.into(), stops.into())
    }
    fn conic_impl(
        &self,
        size: Size,
        scale_factor: Factor,
        mask: Option<ImageMaskMode>,
        center: Point,
        angle: AngleRadian,
        stops: GradientStops,
    ) -> ImageVar {
        IMAGES.render_node(RenderMode::Software, scale_factor, mask, move || {
            let child = zng_wgt_fill::node::conic_gradient(center, angle, stops);
            size_node(child, size)
        })
    }
}

// adapted from `zng_wgt_size_offset::size`.
fn size_node(child: impl UiNode, size: Size) -> impl UiNode {
    match_node(child, move |child, op| match op {
        UiNodeOp::Measure { desired_size, .. } => {
            child.delegated();

            let parent_constraints = LAYOUT.constraints();

            *desired_size = with_fill_metrics(parent_constraints.with_new_min(Px(0), Px(0)), |d| size.layout_dft(d));
            *desired_size = Align::TOP_LEFT.measure(*desired_size, parent_constraints);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let parent_constraints = LAYOUT.constraints();
            let constraints = parent_constraints.with_new_min(Px(0), Px(0));

            let size = with_fill_metrics(constraints, |d| size.layout_dft(d));
            let size = constraints.clamp_size(size);
            LAYOUT.with_constraints(PxConstraints2d::new_exact_size(size), || child.layout(wl));

            *final_size = Align::TOP_LEFT.measure(size, parent_constraints);
        }
        _ => {}
    })
}
fn with_fill_metrics<R>(c: PxConstraints2d, f: impl FnOnce(PxSize) -> R) -> R {
    let dft = c.fill_size();
    LAYOUT.with_constraints(c.with_fill_vector(c.is_bounded()), || f(dft))
}
