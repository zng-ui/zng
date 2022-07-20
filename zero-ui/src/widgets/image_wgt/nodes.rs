//! UI nodes used for building the image widget.

use std::mem;

use super::properties::{
    ImageAlignVar, ImageCacheVar, ImageCropVar, ImageErrorArgs, ImageErrorViewVar, ImageFit, ImageFitVar, ImageLimitsVar, ImageLoadingArgs,
    ImageLoadingViewVar, ImageOffsetVar, ImageRenderingVar, ImageScaleFactorVar, ImageScalePpiVar, ImageScaleVar,
};
use crate::core::{image::*, window::WindowVarsKey};

use super::*;

context_var! {
    /// Image acquired by [`image_source`], or `"no image source in context"` error by default.
    ///
    /// [`image_source`]: fn@image_source
    pub struct ContextImageVar: Image = Image::dummy(Some("no image source in context".to_owned()));
}

/// Requests an image from [`Images`] and sets [`ContextImageVar`].
///
/// Caches the image if [`image_cache`] is `true` in the context.
///
/// The image is not rendered by this property, the [`image_presenter`] renders the image in [`ContextImageVar`].
///
/// In a widget this should be placed inside context properties and before event properties.
///
/// [`Images`]: crate::core::image::Images
/// [`image_cache`]: mod@crate::widgets::image::properties::image_cache
pub fn image_source(child: impl UiNode, source: impl IntoVar<ImageSource>) -> impl UiNode {
    struct ImageSourceNode<C, S: Var<ImageSource>> {
        child: C,
        source: S,
        render_factor: Option<ReadOnlyRcVar<Factor>>,

        img: ImageVar,
        ctx_img: RcVar<Image>,
        ctx_binding: Option<VarBindingHandle>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, S: Var<ImageSource>> UiNode for ImageSourceNode<C, S> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.source);
            if let Some(fct) = &self.render_factor {
                subs.var(ctx, fct);
            }

            self.child.subscriptions(ctx, subs);
        }

        fn init(&mut self, ctx: &mut WidgetContext) {
            let mode = if *ImageCacheVar::get(ctx) {
                ImageCacheMode::Cache
            } else {
                ImageCacheMode::Ignore
            };
            let limits = ImageLimitsVar::get_clone(ctx);

            let mut source = self.source.get_clone(ctx.vars);
            if let ImageSource::Render(_, cfg) = &mut source {
                if cfg.scale_factor.is_none() {
                    // Render without scale_factor can be configured by us, set it to our own scale factor.
                    let fct = ctx.window_state.req(WindowVarsKey).scale_factor();
                    cfg.scale_factor = Some(fct.copy(ctx));
                    self.render_factor = Some(fct);
                }
            }

            self.img = ctx.services.images().get(source, mode, limits);

            self.ctx_img.set(ctx.vars, self.img.get_clone(ctx.vars));
            self.ctx_binding = Some(self.img.bind(ctx.vars, &self.ctx_img));

            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            self.ctx_img.set(ctx, ContextImageVar::default_value());
            self.img = var(ContextImageVar::default_value()).into_read_only();
            self.ctx_binding = None;
            self.render_factor = None;
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(mut source) = self.source.clone_new(ctx) {
                // source update:

                if let ImageSource::Render(_, cfg) = &mut source {
                    // update render factor.
                    if cfg.scale_factor.is_none() {
                        if let Some(fct) = &self.render_factor {
                            cfg.scale_factor = Some(fct.copy(ctx));
                        } else {
                            let fct = ctx.window_state.req(WindowVarsKey).scale_factor();
                            cfg.scale_factor = Some(fct.copy(ctx));
                            self.render_factor = Some(fct);
                            ctx.updates.subscriptions();
                        }
                    } else if self.render_factor.take().is_some() {
                        ctx.updates.subscriptions();
                    }
                }

                let mode = if *ImageCacheVar::get(ctx) {
                    ImageCacheMode::Cache
                } else {
                    ImageCacheMode::Ignore
                };
                let limits = ImageLimitsVar::get_clone(ctx);

                self.img = ctx.services.images().get(source, mode, limits);

                self.ctx_img.set(ctx.vars, self.img.get_clone(ctx.vars));
                self.ctx_binding = Some(self.img.bind(ctx.vars, &self.ctx_img));
            } else if let Some(enabled) = ImageCacheVar::clone_new(ctx) {
                // cache-mode update:
                let images = ctx.services.images();
                let is_cached = images.is_cached(self.ctx_img.get(ctx.vars));
                if enabled != is_cached {
                    self.img = if is_cached {
                        // must not cache, but is cached, detach from cache.

                        let img = mem::replace(&mut self.img, var(Image::dummy(None)).into_read_only());
                        images.detach(img, ctx.vars)
                    } else {
                        // must cache, but image is not cached, get source again.

                        let source = self.source.get_clone(ctx);
                        let limits = ImageLimitsVar::get_clone(ctx);
                        ctx.services.images().get(source, ImageCacheMode::Cache, limits)
                    };

                    self.ctx_img.set(ctx.vars, self.img.get_clone(ctx.vars));
                    self.ctx_binding = Some(self.img.bind(ctx.vars, &self.ctx_img));
                }
            } else if let Some(fct) = &self.render_factor {
                if let Some(fct) = fct.copy_new(ctx) {
                    let mut source = self.source.get_clone(ctx);
                    match &mut source {
                        ImageSource::Render(_, cfg) => {
                            cfg.scale_factor = Some(fct);
                        }
                        _ => unreachable!(),
                    }
                    let mode = if *ImageCacheVar::get(ctx) {
                        ImageCacheMode::Cache
                    } else {
                        ImageCacheMode::Ignore
                    };
                    let limits = ImageLimitsVar::get_clone(ctx);
                    let img = ctx.services.images().get(source, mode, limits);

                    self.ctx_img.set(ctx.vars, img.get_clone(ctx.vars));
                    self.ctx_binding = Some(img.bind(ctx.vars, &self.ctx_img));
                }
            }

            self.child.update(ctx);
        }
    }

    let ctx_img = var(Image::dummy(None));

    ImageSourceNode {
        child: with_context_var(child, ContextImageVar, ctx_img.clone().into_read_only()),
        img: var(Image::dummy(None)).into_read_only(),
        ctx_img,
        ctx_binding: None,
        source: source.into_var(),
        render_factor: None,
    }
    .cfg_boxed()
}

context_var! {
    /// Used to avoid recursion in [`image_error_presenter`].
    struct InErrorViewVar: bool = false;
    /// Used to avoid recursion in [`image_loading_presenter`].
    struct InLoadingViewVar: bool = false;
}

/// Presents the contextual [`ImageErrorViewVar`] if the [`ContextImageVar`] is an error.
///
/// The error view is rendered under the `child`.
///
/// The image widget adds this node around the [`image_presenter`] node.
pub fn image_error_presenter(child: impl UiNode) -> impl UiNode {
    let view = ViewGenerator::presenter_map(
        ImageErrorViewVar,
        |vars, subs| {
            subs.var(vars, &ContextImageVar::new());
        },
        |ctx, is_new| {
            if *InErrorViewVar::get(ctx) {
                // avoid recursion.
                DataUpdate::None
            } else if is_new {
                // init or generator changed.
                if let Some(e) = ContextImageVar::get(ctx.vars).error() {
                    DataUpdate::Update(ImageErrorArgs {
                        error: e.to_owned().into(),
                    })
                } else {
                    DataUpdate::None
                }
            } else if let Some(new) = ContextImageVar::get_new(ctx.vars) {
                // image var update.
                if let Some(e) = new.error() {
                    DataUpdate::Update(ImageErrorArgs {
                        error: e.to_owned().into(),
                    })
                } else {
                    DataUpdate::None
                }
            } else {
                DataUpdate::Same
            }
        },
        |view| with_context_var(view, InErrorViewVar, true),
    );

    stack_nodes_layout_by(nodes![view, child], 1, |constrains, _, img_size| {
        if img_size == PxSize::zero() {
            constrains
        } else {
            PxConstrains2d::new_fill_size(img_size)
        }
    })
}

/// Presents the contextual [`ImageLoadingViewVar`] if the [`ContextImageVar`] is loading.
///
/// The loading view is rendered under the `child`.
///
/// The image widget adds this node around the [`image_error_presenter`] node.
pub fn image_loading_presenter(child: impl UiNode) -> impl UiNode {
    let view = ViewGenerator::presenter_map(
        ImageLoadingViewVar,
        |vars, subs| {
            subs.var(vars, &ContextImageVar::new());
        },
        |ctx, is_new| {
            if *InLoadingViewVar::get(ctx) {
                // avoid recursion.
                DataUpdate::None
            } else if is_new {
                // init or generator changed.
                if ContextImageVar::get(ctx.vars).is_loading() {
                    DataUpdate::Update(ImageLoadingArgs {})
                } else {
                    DataUpdate::None
                }
            } else if let Some(new) = ContextImageVar::get_new(ctx.vars) {
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
        |view| with_context_var(view, InLoadingViewVar, true),
    );

    stack_nodes_layout_by(nodes![view, child], 1, |constrains, _, img_size| {
        if img_size == PxSize::zero() {
            constrains
        } else {
            PxConstrains2d::new_fill_size(img_size)
        }
    })
}

/// Renders the [`ContextImageVar`] if set.
///
/// This is the inner-most node of an image widget, it is fully configured by context variables:
///
/// * [`ContextImageVar`]: Defines the image to render.
/// * [`ImageCropVar`]: Clip the image before layout.
/// * [`ImageScalePpiVar`]: If the image desired size is scaled by PPI.
/// * [`ImageScaleFactorVar`]: If the image desired size is scaled by the screen scale factor.
/// * [`ImageScaleVar`]: Custom scale applied to the desired size.
/// * [`ImageFitVar`]: Defines the image final size.
/// * [`ImageAlignVar`]: Defines the image alignment in the presenter final size.
/// * [`ImageRenderingVar`]: Defines the image resize algorithm used in the GPU.
pub fn image_presenter() -> impl UiNode {
    struct ImagePresenterNode {
        requested_layout: bool,

        // pixel size of the context image.
        img_size: PxSize,

        render_clip: PxRect,
        render_img_size: PxSize,
        render_offset: PxVector,

        spatial_id: SpatialFrameId,
    }
    #[impl_ui_node(none)]
    impl UiNode for ImagePresenterNode {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.vars(ctx)
                .var(&ContextImageVar::new())
                .var(&ImageFitVar::new())
                .var(&ImageScaleVar::new())
                .var(&ImageScalePpiVar::new())
                .var(&ImageCropVar::new())
                .var(&ImageAlignVar::new())
                .var(&ImageRenderingVar::new())
                .var(&ImageOffsetVar::new());
        }

        fn init(&mut self, ctx: &mut WidgetContext) {
            let img = ContextImageVar::get(ctx.vars);
            self.img_size = img.size();
            self.requested_layout = true;
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(img) = ContextImageVar::get_new(ctx.vars) {
                let img_size = img.size();
                if self.img_size != img_size {
                    self.img_size = img_size;
                    ctx.updates.layout();
                    self.requested_layout = true;
                } else if img.is_loaded() {
                    ctx.updates.render();
                }
            }

            if ImageFitVar::is_new(ctx)
                || ImageScaleVar::is_new(ctx)
                || ImageScalePpiVar::is_new(ctx)
                || ImageCropVar::is_new(ctx)
                || ImageAlignVar::is_new(ctx)
                || ImageOffsetVar::is_new(ctx)
            {
                ctx.updates.layout();
                self.requested_layout = true;
            }

            if ImageRenderingVar::is_new(ctx) {
                ctx.updates.render();
            }
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            // Similar to `layout` Part 1.

            let mut scale = *ImageScaleVar::get(ctx);
            if *ImageScalePpiVar::get(ctx) {
                let img = ContextImageVar::get(ctx.vars);
                let sppi = ctx.metrics.screen_ppi();
                let (ippi_x, ippi_y) = img.ppi().unwrap_or((sppi, sppi));
                scale *= Factor2d::new(sppi / ippi_x, sppi / ippi_y);
            }
            if *ImageScaleFactorVar::get(ctx) {
                scale *= ctx.scale_factor();
            }

            let img_rect = PxRect::from_size(self.img_size);
            let crop = ctx.with_constrains(
                |_| PxConstrains2d::new_fill_size(self.img_size),
                |ctx| ImageCropVar::get(ctx.vars).layout(ctx.metrics, |_| img_rect),
            );
            let render_clip = img_rect.intersection(&crop).unwrap_or_default() * scale;

            ctx.constrains().fill_ratio(render_clip.size)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, _: &mut WidgetLayout) -> PxSize {
            // Part 1 - Scale & Crop
            // - Starting from the image pixel size, apply scaling then crop.

            let mut scale = *ImageScaleVar::get(ctx);
            if *ImageScalePpiVar::get(ctx) {
                let img = ContextImageVar::get(ctx.vars);
                let sppi = ctx.metrics.screen_ppi();
                let (ippi_x, ippi_y) = img.ppi().unwrap_or((sppi, sppi));
                scale *= Factor2d::new(sppi / ippi_x, sppi / ippi_y);
            }
            if *ImageScaleFactorVar::get(ctx) {
                scale *= ctx.scale_factor();
            }

            // webrender needs the full image size, we offset and clip it to render the final image.
            let mut render_img_size = self.img_size * scale;

            // crop is relative to the unscaled pixel size, then applied scaled as the clip.
            let img_rect = PxRect::from_size(self.img_size);
            let crop = ctx.with_constrains(
                |_| PxConstrains2d::new_fill_size(self.img_size),
                |ctx| ImageCropVar::get(ctx.vars).layout(ctx.metrics, |_| img_rect),
            );
            let mut render_clip = img_rect.intersection(&crop).unwrap_or_default() * scale;
            let mut render_offset = -render_clip.origin.to_vector();

            // Part 2 - Fit, Align & Clip
            // - Fit the cropped and scaled image to the constrains, add a bounds clip to the crop clip.

            let mut align = *ImageAlignVar::get(ctx);
            if align.is_baseline() {
                align.y = 1.fct();
            }

            let wgt_size = ctx.constrains().fill_ratio(render_clip.size);

            let mut fit = *ImageFitVar::get(ctx);
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
                let offset = diff * align.x;
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
                let offset = diff * align.y;
                render_offset.y += offset;
                if diff < Px(0) {
                    render_clip.origin.y -= offset;
                    render_clip.size.height += diff;
                }
            }

            // Part 3 - Custom Offset and Update
            let offset = ctx.with_constrains(
                |_| PxConstrains2d::new_fill_size(wgt_size),
                |ctx| ImageOffsetVar::get(ctx.vars).layout(ctx.metrics, |_| PxVector::zero()),
            );
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
                ctx.updates.render();
            }

            wgt_size
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let img = ContextImageVar::get(ctx.vars);
            if img.is_loaded() && !self.img_size.is_empty() && !self.render_clip.is_empty() {
                if self.render_offset != PxVector::zero() {
                    let transform = RenderTransform::translation_px(self.render_offset);
                    frame.push_reference_frame(self.spatial_id, FrameBinding::Value(transform), true, false, |frame| {
                        frame.push_image(self.render_clip, self.render_img_size, img, *ImageRenderingVar::get(ctx.vars))
                    });
                } else {
                    frame.push_image(self.render_clip, self.render_img_size, img, *ImageRenderingVar::get(ctx.vars));
                }
            }
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
