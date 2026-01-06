//! UI nodes used for building the image widget.

use std::mem;

use zng_ext_image::{IMAGES, ImageCacheMode, ImageOptions, ImageRenderArgs};
use zng_wgt_stack::stack_nodes;

use super::image_properties::{
    IMAGE_ALIGN_VAR, IMAGE_AUTO_SCALE_VAR, IMAGE_CACHE_VAR, IMAGE_CROP_VAR, IMAGE_DOWNSCALE_VAR, IMAGE_ERROR_FN_VAR, IMAGE_FIT_VAR,
    IMAGE_LIMITS_VAR, IMAGE_LOADING_FN_VAR, IMAGE_OFFSET_VAR, IMAGE_RENDERING_VAR, IMAGE_SCALE_VAR, ImageFit, ImgErrorArgs, ImgLoadingArgs,
};
use super::*;

context_var! {
    /// Image acquired by [`image_source`], or `"no image source in context"` error by default.
    ///
    /// [`image_source`]: fn@image_source
    pub static CONTEXT_IMAGE_VAR: Img = no_context_image();
}
fn no_context_image() -> Img {
    Img::new_empty(Txt::from_static("no image source in context"))
}

/// Requests an image from [`IMAGES`] and sets [`CONTEXT_IMAGE_VAR`].
///
/// Caches the image if [`img_cache`] is `true` in the context.
///
/// The image is not rendered by this property, the [`image_presenter`] renders the image in [`CONTEXT_IMAGE_VAR`].
///
/// In a widget this should be placed inside context properties and before event properties.
///
/// [`img_cache`]: fn@crate::img_cache
/// [`IMAGES`]: zng_ext_image::IMAGES
pub fn image_source(child: impl IntoUiNode, source: impl IntoVar<ImageSource>) -> UiNode {
    let source = source.into_var();
    let ctx_img = var(Img::new_empty(Txt::default()));
    let child = with_context_var(child, CONTEXT_IMAGE_VAR, ctx_img.read_only());
    let mut img = var(Img::new_empty(Txt::default())).read_only();
    let mut _ctx_binding = None;

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var(&source)
                .sub_var(&IMAGE_CACHE_VAR)
                .sub_var(&IMAGE_DOWNSCALE_VAR)
                .sub_var(&IMAGE_ENTRIES_MODE_VAR);

            let mode = if IMAGE_CACHE_VAR.get() {
                ImageCacheMode::Cache
            } else {
                ImageCacheMode::Ignore
            };

            let mut source = source.get();
            if let ImageSource::Render(_, args) = &mut source {
                *args = Some(ImageRenderArgs::new(WINDOW.id()));
            }
            let opt = ImageOptions::new(mode, IMAGE_DOWNSCALE_VAR.get(), None, IMAGE_ENTRIES_MODE_VAR.get());
            img = IMAGES.image(source, opt, IMAGE_LIMITS_VAR.get());

            ctx_img.set_from(&img);
            _ctx_binding = Some(img.bind(&ctx_img));
        }
        UiNodeOp::Deinit => {
            child.deinit();

            ctx_img.set(no_context_image());
            img = var(no_context_image()).read_only();
            _ctx_binding = None;
        }
        UiNodeOp::Update { .. } => {
            if source.is_new() || IMAGE_DOWNSCALE_VAR.is_new() || IMAGE_ENTRIES_MODE_VAR.is_new() {
                // source update:

                let mut source = source.get();

                if let ImageSource::Render(_, args) = &mut source {
                    *args = Some(ImageRenderArgs::new(WINDOW.id()));
                }

                let mode = if IMAGE_CACHE_VAR.get() {
                    ImageCacheMode::Cache
                } else {
                    ImageCacheMode::Ignore
                };
                let opt = ImageOptions::new(mode, IMAGE_DOWNSCALE_VAR.get(), None, IMAGE_ENTRIES_MODE_VAR.get());
                img = IMAGES.image(source, opt, IMAGE_LIMITS_VAR.get());

                ctx_img.set_from(&img);
                _ctx_binding = Some(img.bind(&ctx_img));
            } else if let Some(enabled) = IMAGE_CACHE_VAR.get_new() {
                // cache-mode update:
                let is_cached = ctx_img.with(|img| IMAGES.is_cached(img));
                if enabled != is_cached {
                    img = if is_cached {
                        // must not cache, but is cached, detach from cache.

                        let img = mem::replace(&mut img, var(Img::new_empty(Txt::default())).read_only());
                        IMAGES.detach(img)
                    } else {
                        // must cache, but image is not cached, get source again.

                        let source = source.get();
                        let opt = ImageOptions::new(ImageCacheMode::Cache, IMAGE_DOWNSCALE_VAR.get(), None, IMAGE_ENTRIES_MODE_VAR.get());
                        IMAGES.image(source, opt, IMAGE_LIMITS_VAR.get())
                    };

                    ctx_img.set_from(&img);
                    _ctx_binding = Some(img.bind(&ctx_img));
                }
            }
        }
        _ => {}
    })
}

context_local! {
    /// Used to avoid recursion in [`image_error_presenter`].
    static IN_ERROR_VIEW: bool = false;
    /// Used to avoid recursion in [`image_loading_presenter`].
    static IN_LOADING_VIEW: bool = false;
}

/// Presents the contextual [`IMAGE_ERROR_FN_VAR`] if the [`CONTEXT_IMAGE_VAR`] is an error.
///
/// The error view is rendered under the `child`.
///
/// The image widget adds this node around the [`image_presenter`] node.
pub fn image_error_presenter(child: impl IntoUiNode) -> UiNode {
    let view = CONTEXT_IMAGE_VAR
        .map(|i| i.error().map(|e| ImgErrorArgs { error: e }))
        .present_opt(IMAGE_ERROR_FN_VAR.map(|f| {
            wgt_fn!(f, |e| {
                if IN_ERROR_VIEW.get_clone() {
                    UiNode::nil()
                } else {
                    with_context_local(f(e), &IN_ERROR_VIEW, true)
                }
            })
        }));

    stack_nodes(ui_vec![view, child], 1, |constraints, _, img_size| {
        if img_size == PxSize::zero() {
            constraints
        } else {
            PxConstraints2d::new_fill_size(img_size)
        }
    })
}

/// Presents the contextual [`IMAGE_LOADING_FN_VAR`] if the [`CONTEXT_IMAGE_VAR`] is loading.
///
/// The loading view is rendered under the `child`.
///
/// The image widget adds this node around the [`image_error_presenter`] node.
pub fn image_loading_presenter(child: impl IntoUiNode) -> UiNode {
    let view = CONTEXT_IMAGE_VAR
        .map(|i| if i.is_loading() { Some(ImgLoadingArgs {}) } else { None })
        .present_opt(IMAGE_LOADING_FN_VAR.map(|f| {
            wgt_fn!(f, |a| {
                if IN_LOADING_VIEW.get_clone() {
                    UiNode::nil()
                } else {
                    with_context_local(f(a), &IN_LOADING_VIEW, true)
                }
            })
        }));

    stack_nodes(ui_vec![view, child], 1, |constraints, _, img_size| {
        if img_size == PxSize::zero() {
            constraints
        } else {
            PxConstraints2d::new_fill_size(img_size)
        }
    })
}

/// Renders the [`CONTEXT_IMAGE_VAR`] if set.
///
/// This is the inner-most node of an image widget, it is fully configured by context variables:
///
/// * [`CONTEXT_IMAGE_VAR`]: Defines the image to render.
/// * [`IMAGE_CROP_VAR`]: Clip the image before layout.
/// * [`IMAGE_AUTO_SCALE_VAR`]: If the image desired size is scaled by pixel density.
/// * [`IMAGE_SCALE_VAR`]: Custom scale applied to the desired size.
/// * [`IMAGE_FIT_VAR`]: Defines the image final size.
/// * [`IMAGE_ALIGN_VAR`]: Defines the image alignment in the presenter final size.
/// * [`IMAGE_RENDERING_VAR`]: Defines the image resize algorithm used in the GPU.
/// * [`IMAGE_OFFSET_VAR`]: Defines an offset applied to the image after all measure and arrange.
pub fn image_presenter() -> UiNode {
    let mut img_size = PxSize::zero();
    let mut render_clip = PxRect::zero();
    let mut render_img_size = PxSize::zero();
    let mut render_tile_size = PxSize::zero();
    let mut render_tile_spacing = PxSize::zero();
    let mut render_offset = PxVector::zero();
    let spatial_id = SpatialFrameId::new_unique();

    match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var(&CONTEXT_IMAGE_VAR)
                .sub_var_layout(&IMAGE_CROP_VAR)
                .sub_var_layout(&IMAGE_AUTO_SCALE_VAR)
                .sub_var_layout(&IMAGE_SCALE_VAR)
                .sub_var_layout(&IMAGE_FIT_VAR)
                .sub_var_layout(&IMAGE_ALIGN_VAR)
                .sub_var_layout(&IMAGE_OFFSET_VAR)
                .sub_var_layout(&IMAGE_REPEAT_VAR)
                .sub_var_layout(&IMAGE_REPEAT_SPACING_VAR)
                .sub_var_render(&IMAGE_RENDERING_VAR);

            img_size = CONTEXT_IMAGE_VAR.with(Img::size);
        }
        UiNodeOp::Update { .. } => {
            if let Some(img) = CONTEXT_IMAGE_VAR.get_new() {
                let ig_size = img.size();
                if img_size != ig_size {
                    img_size = ig_size;
                    WIDGET.layout();
                } else if img.is_loaded() {
                    WIDGET.render();
                }
            }
        }
        UiNodeOp::Measure { desired_size, .. } => {
            // Similar to `layout` Part 1.

            let metrics = LAYOUT.metrics();

            let mut scale = IMAGE_SCALE_VAR.get();
            match IMAGE_AUTO_SCALE_VAR.get() {
                ImageAutoScale::Pixel => {}
                ImageAutoScale::Factor => {
                    scale *= metrics.scale_factor();
                }
                ImageAutoScale::Density => {
                    let screen = metrics.screen_density();
                    let image = CONTEXT_IMAGE_VAR.with(Img::density).unwrap_or(PxDensity2d::splat(screen));
                    scale *= Factor2d::new(screen.ppcm() / image.width.ppcm(), screen.ppcm() / image.height.ppcm());
                }
            }

            let img_rect = PxRect::from_size(img_size);
            let crop = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(img_size), || {
                let mut r = IMAGE_CROP_VAR.get();
                r.replace_default(&img_rect.into());
                r.layout()
            });
            let render_clip = img_rect.intersection(&crop).unwrap_or_default() * scale;

            let min_size = metrics.constraints().clamp_size(render_clip.size);
            let wgt_ratio = metrics.constraints().with_min_size(min_size).fill_ratio(render_clip.size);

            *desired_size = metrics.constraints().inner().fill_size_or(wgt_ratio);
        }
        UiNodeOp::Layout { final_size, .. } => {
            // Part 1 - Scale & Crop
            // - Starting from the image pixel size, apply scaling then crop.

            let metrics = LAYOUT.metrics();

            let mut scale = IMAGE_SCALE_VAR.get();
            match IMAGE_AUTO_SCALE_VAR.get() {
                ImageAutoScale::Pixel => {}
                ImageAutoScale::Factor => {
                    scale *= metrics.scale_factor();
                }
                ImageAutoScale::Density => {
                    let screen = metrics.screen_density();
                    let image = CONTEXT_IMAGE_VAR.with(Img::density).unwrap_or(PxDensity2d::splat(screen));
                    scale *= Factor2d::new(screen.ppcm() / image.width.ppcm(), screen.ppcm() / image.height.ppcm());
                }
            }

            // webrender needs the full image size, we offset and clip it to render the final image.
            let mut r_img_size = img_size * scale;

            // crop is relative to the unscaled pixel size, then applied scaled as the clip.
            let img_rect = PxRect::from_size(img_size);
            let crop = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(img_size), || {
                let mut r = IMAGE_CROP_VAR.get();
                r.replace_default(&img_rect.into());
                r.layout()
            });
            let mut r_clip = img_rect.intersection(&crop).unwrap_or_default() * scale;
            let mut r_offset = -r_clip.origin.to_vector();

            // Part 2 - Fit, Align & Clip
            // - Fit the cropped and scaled image to the constraints, add a bounds clip to the crop clip.

            let mut align = IMAGE_ALIGN_VAR.get();

            let constraints = metrics.constraints();
            let min_size = constraints.clamp_size(r_clip.size);
            let wgt_ratio = constraints.with_min_size(min_size).fill_ratio(r_clip.size);
            let wgt_size = constraints.inner().fill_size_or(wgt_ratio);

            let mut fit = IMAGE_FIT_VAR.get();
            if let ImageFit::ScaleDown = fit {
                if r_clip.size.width < wgt_size.width && r_clip.size.height < wgt_size.height {
                    fit = ImageFit::None;
                } else {
                    fit = ImageFit::Contain;
                }
            }
            match fit {
                ImageFit::Fill => {
                    align = Align::FILL;
                }
                ImageFit::Contain => {
                    let container = wgt_size.to_f32();
                    let content = r_clip.size.to_f32();
                    let scale = (container.width / content.width).min(container.height / content.height).fct();
                    r_clip *= scale;
                    r_img_size *= scale;
                    r_offset *= scale;
                }
                ImageFit::Cover => {
                    let container = wgt_size.to_f32();
                    let content = r_clip.size.to_f32();
                    let scale = (container.width / content.width).max(container.height / content.height).fct();
                    r_clip *= scale;
                    r_img_size *= scale;
                    r_offset *= scale;
                }
                ImageFit::None => {}
                ImageFit::ScaleDown => unreachable!(),
            }

            if align.is_fill_x() {
                let factor = wgt_size.width.0 as f32 / r_clip.size.width.0 as f32;
                r_clip.size.width = wgt_size.width;
                r_clip.origin.x *= factor;
                r_img_size.width *= factor;
                r_offset.x = -r_clip.origin.x;
            } else {
                let diff = wgt_size.width - r_clip.size.width;
                let offset = diff * align.x(metrics.direction());
                r_offset.x += offset;
                if diff < Px(0) {
                    r_clip.origin.x -= offset;
                    r_clip.size.width += diff;
                }
            }
            if align.is_fill_y() {
                let factor = wgt_size.height.0 as f32 / r_clip.size.height.0 as f32;
                r_clip.size.height = wgt_size.height;
                r_clip.origin.y *= factor;
                r_img_size.height *= factor;
                r_offset.y = -r_clip.origin.y;
            } else {
                let diff = wgt_size.height - r_clip.size.height;
                let offset = diff * align.y();
                r_offset.y += offset;
                if diff < Px(0) {
                    r_clip.origin.y -= offset;
                    r_clip.size.height += diff;
                }
            }

            // Part 3 - Custom Offset and Update
            let offset = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(wgt_size), || IMAGE_OFFSET_VAR.layout());
            if offset != PxVector::zero() {
                r_offset += offset;

                let screen_clip = PxRect::new(-r_offset.to_point(), wgt_size);
                r_clip.origin -= offset;
                r_clip = r_clip.intersection(&screen_clip).unwrap_or_default();
            }

            // Part 4 - Repeat
            let mut r_tile_size = r_img_size;
            let mut r_tile_spacing = PxSize::zero();
            if matches!(IMAGE_REPEAT_VAR.get(), ImageRepeat::Repeat) {
                r_clip = PxRect::from_size(wgt_size);
                r_tile_size = r_img_size;
                r_img_size = wgt_size;
                r_offset = PxVector::zero();

                let leftover = tile_leftover(r_tile_size, wgt_size);
                r_tile_spacing = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(r_tile_size), || {
                    LAYOUT.with_leftover(Some(leftover.width), Some(leftover.height), || IMAGE_REPEAT_SPACING_VAR.layout())
                });
            }

            if render_clip != r_clip
                || render_img_size != r_img_size
                || render_offset != r_offset
                || render_tile_size != r_tile_size
                || render_tile_spacing != r_tile_spacing
            {
                render_clip = r_clip;
                render_img_size = r_img_size;
                render_offset = r_offset;
                render_tile_size = r_tile_size;
                render_tile_spacing = r_tile_spacing;
                WIDGET.render();
            }

            *final_size = wgt_size;
        }
        UiNodeOp::Render { frame } => {
            if render_clip.is_empty() {
                return;
            }
            CONTEXT_IMAGE_VAR.with(|img| {
                img.with_best_reduce(render_tile_size, |img| {
                    if render_offset != PxVector::zero() {
                        let transform = PxTransform::from(render_offset);
                        frame.push_reference_frame(spatial_id.into(), FrameValue::Value(transform), true, false, |frame| {
                            frame.push_image(
                                render_clip,
                                render_img_size,
                                render_tile_size,
                                render_tile_spacing,
                                img,
                                IMAGE_RENDERING_VAR.get(),
                            )
                        });
                    } else {
                        frame.push_image(
                            render_clip,
                            render_img_size,
                            render_tile_size,
                            render_tile_spacing,
                            img,
                            IMAGE_RENDERING_VAR.get(),
                        );
                    }
                })
            });
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
