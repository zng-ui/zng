//! UI nodes used for building the image widget.

use std::mem;

use super::image_properties::{
    ImageErrorArgs, ImageFit, ImageLoadingArgs, IMAGE_ALIGN_VAR, IMAGE_CACHE_VAR, IMAGE_CROP_VAR, IMAGE_DOWNSCALE_VAR, IMAGE_ERROR_GEN_VAR,
    IMAGE_FIT_VAR, IMAGE_LIMITS_VAR, IMAGE_LOADING_GEN_VAR, IMAGE_OFFSET_VAR, IMAGE_RENDERING_VAR, IMAGE_SCALE_FACTOR_VAR,
    IMAGE_SCALE_PPI_VAR, IMAGE_SCALE_VAR,
};
use crate::core::image::*;

use super::*;

context_var! {
    /// Image acquired by [`image_source`], or `"no image source in context"` error by default.
    ///
    /// [`image_source`]: fn@image_source
    pub static CONTEXT_IMAGE_VAR: Img = no_context_image();
}
fn no_context_image() -> Img {
    Img::dummy(Some("no image source in context".to_owned()))
}

/// Requests an image from [`IMAGES`] and sets [`CONTEXT_IMAGE_VAR`].
///
/// Caches the image if [`img_cache`] is `true` in the context.
///
/// The image is not rendered by this property, the [`image_presenter`] renders the image in [`CONTEXT_IMAGE_VAR`].
///
/// In a widget this should be placed inside context properties and before event properties.
///
/// [`IMAGES`]: crate::core::image::IMAGES
/// [`img_cache`]: fn@crate::widgets::image::img_cache
pub fn image_source(child: impl UiNode, source: impl IntoVar<ImageSource>) -> impl UiNode {
    #[ui_node(struct ImageSourceNode {
        child: impl UiNode,
        #[var] source: impl Var<ImageSource>,

        img: ImageVar,
        ctx_img: ArcVar<Img>,
        ctx_binding: Option<VarHandle>,
    })]
    impl UiNode for ImageSourceNode {
        fn init(&mut self) {
            self.auto_subs();
            WIDGET.sub_var(&IMAGE_CACHE_VAR).sub_var(&IMAGE_DOWNSCALE_VAR);

            let mode = if IMAGE_CACHE_VAR.get() {
                ImageCacheMode::Cache
            } else {
                ImageCacheMode::Ignore
            };
            let limits = IMAGE_LIMITS_VAR.get();
            let downscale = IMAGE_DOWNSCALE_VAR.get();

            let mut source = self.source.get();
            if let ImageSource::Render(_, args) = &mut source {
                *args = Some(ImageRenderArgs { parent: Some(WINDOW.id()) });
            }
            self.img = IMAGES.image(source, mode, limits, downscale);

            self.ctx_img.set(self.img.get());
            self.ctx_binding = Some(self.img.bind(&self.ctx_img));

            self.child.init();
        }

        fn deinit(&mut self) {
            self.child.deinit();
            self.ctx_img.set(no_context_image());
            self.img = var(no_context_image()).read_only();
            self.ctx_binding = None;
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            if self.source.is_new() || IMAGE_DOWNSCALE_VAR.is_new() {
                // source update:

                let mut source = self.source.get();

                if let ImageSource::Render(_, args) = &mut source {
                    *args = Some(ImageRenderArgs { parent: Some(WINDOW.id()) });
                }

                let mode = if IMAGE_CACHE_VAR.get() {
                    ImageCacheMode::Cache
                } else {
                    ImageCacheMode::Ignore
                };
                let limits = IMAGE_LIMITS_VAR.get();
                let downscale = IMAGE_DOWNSCALE_VAR.get();

                self.img = IMAGES.image(source, mode, limits, downscale);

                self.ctx_img.set(self.img.get());
                self.ctx_binding = Some(self.img.bind(&self.ctx_img));
            } else if let Some(enabled) = IMAGE_CACHE_VAR.get_new() {
                // cache-mode update:
                let is_cached = self.ctx_img.with(|img| IMAGES.is_cached(img));
                if enabled != is_cached {
                    self.img = if is_cached {
                        // must not cache, but is cached, detach from cache.

                        let img = mem::replace(&mut self.img, var(Img::dummy(None)).read_only());
                        IMAGES.detach(img)
                    } else {
                        // must cache, but image is not cached, get source again.

                        let source = self.source.get();
                        let limits = IMAGE_LIMITS_VAR.get();
                        let downscale = IMAGE_DOWNSCALE_VAR.get();
                        IMAGES.image(source, ImageCacheMode::Cache, limits, downscale)
                    };

                    self.ctx_img.set(self.img.get());
                    self.ctx_binding = Some(self.img.bind(&self.ctx_img));
                }
            }

            self.child.update(updates);
        }
    }

    let ctx_img = var(Img::dummy(None));

    ImageSourceNode {
        child: with_context_var(child, CONTEXT_IMAGE_VAR, ctx_img.read_only()),
        img: var(Img::dummy(None)).read_only(),
        ctx_img,
        ctx_binding: None,
        source: source.into_var(),
    }
    .cfg_boxed()
}

context_local! {
    /// Used to avoid recursion in [`image_error_presenter`].
    static IN_ERROR_VIEW: bool = false;
    /// Used to avoid recursion in [`image_loading_presenter`].
    static IN_LOADING_VIEW: bool = false;
}

/// Presents the contextual [`IMAGE_ERROR_GEN_VAR`] if the [`CONTEXT_IMAGE_VAR`] is an error.
///
/// The error view is rendered under the `child`.
///
/// The image widget adds this node around the [`image_presenter`] node.
pub fn image_error_presenter(child: impl UiNode) -> impl UiNode {
    let mut image_handle: Option<(VarHandle, WidgetId)> = None;

    let view = WidgetFn::presenter_map(
        IMAGE_ERROR_GEN_VAR,
        move |is_new| {
            if is_new {
                if let Some((handle, id)) = &mut image_handle {
                    let current_id = WIDGET.id();
                    if *id != current_id {
                        *id = current_id;
                        *handle = CONTEXT_IMAGE_VAR.subscribe(current_id);
                    }
                } else {
                    let id = WIDGET.id();
                    image_handle = Some((CONTEXT_IMAGE_VAR.subscribe(id), id));
                }
            }

            if IN_ERROR_VIEW.get_clone() {
                // avoid recursion.
                DataUpdate::None
            } else if is_new {
                // init or fn changed.
                if let Some(e) = CONTEXT_IMAGE_VAR.get().error() {
                    DataUpdate::Update(ImageErrorArgs { error: e })
                } else {
                    DataUpdate::None
                }
            } else if let Some(new) = CONTEXT_IMAGE_VAR.get_new() {
                // image var update.
                if let Some(e) = new.error() {
                    DataUpdate::Update(ImageErrorArgs { error: e })
                } else {
                    DataUpdate::None
                }
            } else {
                DataUpdate::Same
            }
        },
        |view| with_context_local(view, &IN_ERROR_VIEW, true),
    );

    stack_nodes_layout_by(ui_vec![view, child], 1, |constraints, _, img_size| {
        if img_size == PxSize::zero() {
            constraints
        } else {
            PxConstraints2d::new_fill_size(img_size)
        }
    })
}

/// Presents the contextual [`IMAGE_LOADING_GEN_VAR`] if the [`CONTEXT_IMAGE_VAR`] is loading.
///
/// The loading view is rendered under the `child`.
///
/// The image widget adds this node around the [`image_error_presenter`] node.
pub fn image_loading_presenter(child: impl UiNode) -> impl UiNode {
    let mut image_handle = None;

    let view = WidgetFn::presenter_map(
        IMAGE_LOADING_GEN_VAR,
        move |is_new| {
            if is_new {
                if let Some((handle, id)) = &mut image_handle {
                    let current_id = WIDGET.id();
                    if *id != current_id {
                        *id = current_id;
                        *handle = CONTEXT_IMAGE_VAR.subscribe(current_id);
                    }
                } else {
                    let id = WIDGET.id();
                    image_handle = Some((CONTEXT_IMAGE_VAR.subscribe(id), id));
                }
            }

            if IN_LOADING_VIEW.get_clone() {
                // avoid recursion.
                DataUpdate::None
            } else if is_new {
                // init or fn changed.
                if CONTEXT_IMAGE_VAR.with(Img::is_loading) {
                    DataUpdate::Update(ImageLoadingArgs {})
                } else {
                    DataUpdate::None
                }
            } else if let Some(new) = CONTEXT_IMAGE_VAR.get_new() {
                // image var update.
                if new.is_loading() {
                    DataUpdate::Update(ImageLoadingArgs {})
                } else {
                    DataUpdate::None
                }
            } else {
                DataUpdate::Same
            }
        },
        |view| with_context_local(view, &IN_LOADING_VIEW, true),
    );

    stack_nodes_layout_by(ui_vec![view, child], 1, |constraints, _, img_size| {
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
/// * [`IMAGE_SCALE_PPI_VAR`]: If the image desired size is scaled by PPI.
/// * [`IMAGE_SCALE_FACTOR_VAR`]: If the image desired size is scaled by the screen scale factor.
/// * [`IMAGE_SCALE_VAR`]: Custom scale applied to the desired size.
/// * [`IMAGE_FIT_VAR`]: Defines the image final size.
/// * [`IMAGE_ALIGN_VAR`]: Defines the image alignment in the presenter final size.
/// * [`IMAGE_RENDERING_VAR`]: Defines the image resize algorithm used in the GPU.
/// * [`IMAGE_OFFSET_VAR`]: Defines an offset applied to the image after all measure and arrange.
pub fn image_presenter() -> impl UiNode {
    #[ui_node(struct ImagePresenterNode {
        requested_layout: bool,

        // pixel size of the context image.
        img_size: PxSize,

        render_clip: PxRect,
        render_img_size: PxSize,
        render_offset: PxVector,

        spatial_id: SpatialFrameId,
    })]
    impl UiNode for ImagePresenterNode {
        fn init(&mut self) {
            WIDGET
                .sub_var(&CONTEXT_IMAGE_VAR)
                .sub_var(&IMAGE_CROP_VAR)
                .sub_var(&IMAGE_SCALE_PPI_VAR)
                .sub_var(&IMAGE_SCALE_FACTOR_VAR)
                .sub_var(&IMAGE_SCALE_VAR)
                .sub_var(&IMAGE_FIT_VAR)
                .sub_var(&IMAGE_ALIGN_VAR)
                .sub_var(&IMAGE_RENDERING_VAR)
                .sub_var(&IMAGE_OFFSET_VAR);

            self.img_size = CONTEXT_IMAGE_VAR.with(Img::size);
            self.requested_layout = true;
        }

        fn update(&mut self, _: &WidgetUpdates) {
            if let Some(img) = CONTEXT_IMAGE_VAR.get_new() {
                let img_size = img.size();
                if self.img_size != img_size {
                    self.img_size = img_size;
                    WIDGET.layout();
                    self.requested_layout = true;
                } else if img.is_loaded() {
                    WIDGET.render();
                }
            }

            if IMAGE_FIT_VAR.is_new()
                || IMAGE_SCALE_VAR.is_new()
                || IMAGE_SCALE_FACTOR_VAR.is_new()
                || IMAGE_SCALE_PPI_VAR.is_new()
                || IMAGE_CROP_VAR.is_new()
                || IMAGE_ALIGN_VAR.is_new()
                || IMAGE_OFFSET_VAR.is_new()
            {
                WIDGET.layout();
                self.requested_layout = true;
            }

            if IMAGE_RENDERING_VAR.is_new() {
                WIDGET.render();
            }
        }

        fn measure(&self, _: &mut WidgetMeasure) -> PxSize {
            // Similar to `layout` Part 1.

            let metrics = LAYOUT.metrics();

            let mut scale = IMAGE_SCALE_VAR.get();
            if IMAGE_SCALE_PPI_VAR.get() {
                let sppi = metrics.screen_ppi();
                let (ippi_x, ippi_y) = CONTEXT_IMAGE_VAR.with(Img::ppi).unwrap_or((sppi, sppi));
                scale *= Factor2d::new(sppi / ippi_x, sppi / ippi_y);
            }
            if IMAGE_SCALE_FACTOR_VAR.get() {
                scale *= metrics.scale_factor();
            }

            let img_rect = PxRect::from_size(self.img_size);
            let crop = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(self.img_size), || {
                let mut r = IMAGE_CROP_VAR.get();
                r.replace_default(&img_rect.into());
                r.layout()
            });
            let render_clip = img_rect.intersection(&crop).unwrap_or_default() * scale;

            let min_size = metrics.constraints().clamp_size(render_clip.size);
            let wgt_ratio = metrics.constraints().with_min_size(min_size).fill_ratio(render_clip.size);
            metrics.constraints().fill_size_or(wgt_ratio)
        }
        fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
            // Part 1 - Scale & Crop
            // - Starting from the image pixel size, apply scaling then crop.

            let metrics = LAYOUT.metrics();

            let mut scale = IMAGE_SCALE_VAR.get();
            if IMAGE_SCALE_PPI_VAR.get() {
                let sppi = metrics.screen_ppi();
                let (ippi_x, ippi_y) = CONTEXT_IMAGE_VAR.with(Img::ppi).unwrap_or((sppi, sppi));
                scale *= Factor2d::new(sppi / ippi_x, sppi / ippi_y);
            }
            if IMAGE_SCALE_FACTOR_VAR.get() {
                scale *= metrics.scale_factor();
            }

            // webrender needs the full image size, we offset and clip it to render the final image.
            let mut render_img_size = self.img_size * scale;

            // crop is relative to the unscaled pixel size, then applied scaled as the clip.
            let img_rect = PxRect::from_size(self.img_size);
            let crop = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(self.img_size), || {
                let mut r = IMAGE_CROP_VAR.get();
                r.replace_default(&img_rect.into());
                r.layout()
            });
            let mut render_clip = img_rect.intersection(&crop).unwrap_or_default() * scale;
            let mut render_offset = -render_clip.origin.to_vector();

            // Part 2 - Fit, Align & Clip
            // - Fit the cropped and scaled image to the constraints, add a bounds clip to the crop clip.

            let mut align = IMAGE_ALIGN_VAR.get();

            let min_size = metrics.constraints().clamp_size(render_clip.size);
            let wgt_ratio = metrics.constraints().with_min_size(min_size).fill_ratio(render_clip.size);
            let wgt_size = metrics.constraints().fill_size_or(wgt_ratio);

            let mut fit = IMAGE_FIT_VAR.get();
            if let ImageFit::ScaleDown = fit {
                if render_clip.size.width < wgt_size.width && render_clip.size.height < wgt_size.height {
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
                    let content = render_clip.size.to_f32();
                    let scale = (container.width / content.width).min(container.height / content.height).fct();
                    render_clip *= scale;
                    render_img_size *= scale;
                    render_offset *= scale;
                }
                ImageFit::Cover => {
                    let container = wgt_size.to_f32();
                    let content = render_clip.size.to_f32();
                    let scale = (container.width / content.width).max(container.height / content.height).fct();
                    render_clip *= scale;
                    render_img_size *= scale;
                    render_offset *= scale;
                }
                ImageFit::None => {}
                ImageFit::ScaleDown => unreachable!(),
            }

            if align.is_fill_x() {
                let factor = wgt_size.width.0 as f32 / render_clip.size.width.0 as f32;
                render_clip.size.width = wgt_size.width;
                render_clip.origin.x *= factor;
                render_img_size.width *= factor;
                render_offset.x = -render_clip.origin.x;
            } else {
                let diff = wgt_size.width - render_clip.size.width;
                let offset = diff * align.x(metrics.direction());
                render_offset.x += offset;
                if diff < Px(0) {
                    render_clip.origin.x -= offset;
                    render_clip.size.width += diff;
                }
            }
            if align.is_fill_y() {
                let factor = wgt_size.height.0 as f32 / render_clip.size.height.0 as f32;
                render_clip.size.height = wgt_size.height;
                render_clip.origin.y *= factor;
                render_img_size.height *= factor;
                render_offset.y = -render_clip.origin.y;
            } else {
                let diff = wgt_size.height - render_clip.size.height;
                let offset = diff * align.y();
                render_offset.y += offset;
                if diff < Px(0) {
                    render_clip.origin.y -= offset;
                    render_clip.size.height += diff;
                }
            }

            // Part 3 - Custom Offset and Update
            let offset = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(wgt_size), || IMAGE_OFFSET_VAR.layout());
            if offset != PxVector::zero() {
                render_offset += offset;

                let screen_clip = PxRect::new(-render_offset.to_point(), wgt_size);
                render_clip.origin -= offset;
                render_clip = render_clip.intersection(&screen_clip).unwrap_or_default();
            }

            if self.render_clip != render_clip || self.render_img_size != render_img_size || self.render_offset != render_offset {
                self.render_clip = render_clip;
                self.render_img_size = render_img_size;
                self.render_offset = render_offset;
                WIDGET.render();
            }

            wgt_size
        }

        fn render(&self, frame: &mut FrameBuilder) {
            CONTEXT_IMAGE_VAR.with(|img| {
                if img.is_loaded() && !self.img_size.is_empty() && !self.render_clip.is_empty() {
                    if self.render_offset != PxVector::zero() {
                        let transform = PxTransform::from(self.render_offset);
                        frame.push_reference_frame(self.spatial_id.into(), FrameValue::Value(transform), true, false, |frame| {
                            frame.push_image(self.render_clip, self.render_img_size, img, IMAGE_RENDERING_VAR.get())
                        });
                    } else {
                        frame.push_image(self.render_clip, self.render_img_size, img, IMAGE_RENDERING_VAR.get());
                    }
                }
            })
        }
    }
    ImagePresenterNode {
        requested_layout: true,

        img_size: PxSize::zero(),

        render_clip: PxRect::zero(),
        render_img_size: PxSize::zero(),
        render_offset: PxVector::zero(),

        spatial_id: SpatialFrameId::new_unique(),
    }
}
