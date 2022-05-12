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
/// In an widget this should be placed inside context properties and before event properties.
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
        ctx_binding: VarBindingHandle,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, S: Var<ImageSource>> UiNode for ImageSourceNode<C, S> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.source);
            if let Some(fct) = &self.render_factor {
                subscriptions.var(ctx, fct);
            }

            self.child.subscriptions(ctx, subscriptions);
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
            self.ctx_binding = self.img.bind(ctx.vars, &self.ctx_img);

            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            self.ctx_img.set(ctx, ContextImageVar::default_value());
            self.img = var(ContextImageVar::default_value()).into_read_only();
            self.ctx_binding = VarBindingHandle::dummy();
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
                self.ctx_binding = self.img.bind(ctx.vars, &self.ctx_img);
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
                    self.ctx_binding = self.img.bind(ctx.vars, &self.ctx_img);
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
                    self.ctx_binding = img.bind(ctx.vars, &self.ctx_img);
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
        ctx_binding: VarBindingHandle::dummy(),
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
        |vars, subscriptions| {
            subscriptions.var(vars, &ContextImageVar::new());
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

    stack_nodes(nodes![view, child])
}

/// Presents the contextual [`ImageLoadingViewVar`] if the [`ContextImageVar`] is loading.
///
/// The loading view is rendered under the `child`.
///
/// The image widget adds this node around the [`image_error_presenter`] node.
pub fn image_loading_presenter(child: impl UiNode) -> impl UiNode {
    let view = ViewGenerator::presenter_map(
        ImageLoadingViewVar,
        |vars, subscriptions| {
            subscriptions.var(vars, &ContextImageVar::new());
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

    stack_nodes(nodes![view, child])
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

        // pixel size of the last image presented.
        img_size: PxSize,

        // raw size of the image the last time a full `measure` was done.
        measure_img_size: PxSize,
        // last computed clip-rect in the `measure` pass.
        measure_clip_rect: PxRect,
        // desired-size (pre-available) the last time a full `measure` was done.
        desired_size: PxSize,
        // `final_size` of the last processed `arrange`.
        final_size: PxSize,

        render_clip_rect: PxRect,
        render_img_size: PxSize,
        render_offset: PxVector,

        spatial_id: SpatialFrameId,
    }
    #[impl_ui_node(none)]
    impl UiNode for ImagePresenterNode {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions
                .vars(ctx)
                .var(&ContextImageVar::new())
                .var(&ImageFitVar::new())
                .var(&ImageScaleVar::new())
                .var(&ImageScalePpiVar::new())
                .var(&ImageCropVar::new())
                .var(&ImageAlignVar::new())
                .var(&ImageRenderingVar::new());
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
            {
                ctx.updates.layout();
                self.requested_layout = true;
            }

            if ImageRenderingVar::is_new(ctx) {
                ctx.updates.render();
            }
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            // TODO !!: reimplement, wait until most of the others are done
        }

        /*
        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let img_rect = PxRect::from_size(self.img_size);

            let crop = ImageCropVar::get(ctx).layout(ctx, AvailableSize::from_size(self.img_size), img_rect);

            self.measure_img_size = self.img_size;
            self.measure_clip_rect = img_rect.intersection(&crop).unwrap_or_default();

            let mut scale = *ImageScaleVar::get(ctx);
            if *ImageScalePpiVar::get(ctx) {
                let img = ContextImageVar::get(ctx.vars);
                let sppi = ctx.metrics.screen_ppi;
                let (ippi_x, ippi_y) = img.ppi().unwrap_or((sppi, sppi));
                scale *= Factor2d::new(sppi / ippi_x, sppi / ippi_y);
            }

            if *ImageScaleFactorVar::get(ctx) {
                scale *= ctx.scale_factor;
            }
            self.measure_img_size *= scale;
            self.measure_clip_rect *= scale;

            self.requested_layout |= self.measure_clip_rect.size != self.desired_size;
            self.desired_size = self.measure_clip_rect.size;

            if let ImageFit::Fill = *ImageFitVar::get(ctx) {
                match (available_size.width, available_size.height) {
                    (AvailablePx::Infinite, AvailablePx::Finite(h)) if self.desired_size.height > Px(0) => {
                        let scale = h.0 as f32 / self.desired_size.height.0 as f32;
                        self.desired_size.width *= scale;
                        self.desired_size.height = h;
                    }
                    (AvailablePx::Finite(w), AvailablePx::Infinite) if self.desired_size.width > Px(0) => {
                        let scale = w.0 as f32 / self.desired_size.width.0 as f32;
                        self.desired_size.height *= scale;
                        self.desired_size.width = w;
                    }
                    _ => {}
                }
            }
            available_size.clip(self.desired_size)
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, _: &mut WidgetLayout, final_size: PxSize) {
            self.requested_layout |= final_size != self.final_size;

            if !self.requested_layout {
                return;
            }

            self.final_size = final_size;
            self.requested_layout = false;

            let mut f_img_size = self.measure_img_size;
            let mut f_clip_rect = self.measure_clip_rect;
            // let f_offset;

            // 1 - fit crop-rect:

            let mut align_offset = PxVector::zero();
            let mut crop_size = self.measure_clip_rect.size;

            let align = *ImageAlignVar::get(ctx.vars);
            let mut fit = *ImageFitVar::get(ctx);
            loop {
                match fit {
                    ImageFit::None => {
                        align_offset = align.solve_offset(crop_size, final_size);
                        break;
                    }
                    ImageFit::Fill => {
                        crop_size = final_size;
                        break;
                    }
                    ImageFit::Contain => {
                        let container = final_size.to_f32();
                        let content = crop_size.to_f32();
                        let scale = (container.width / content.width).min(container.height / content.height).fct();
                        crop_size *= scale;
                        align_offset = align.solve_offset(crop_size, final_size);
                        break;
                    }
                    ImageFit::Cover => {
                        let container = final_size.to_f32();
                        let content = crop_size.to_f32();
                        let scale = (container.width / content.width).max(container.height / content.height).fct();
                        crop_size *= scale;
                        align_offset = align.solve_offset(crop_size, final_size);
                        break;
                    }
                    ImageFit::ScaleDown => {
                        if crop_size.width < final_size.width && crop_size.height < final_size.height {
                            fit = ImageFit::None;
                        } else {
                            fit = ImageFit::Contain;
                        }
                    }
                }
            }

            // 2 - scale image to new crop size:
            let scale_x = crop_size.width.0 as f32 / f_clip_rect.size.width.0 as f32;
            let scale_y = crop_size.height.0 as f32 / f_clip_rect.size.height.0 as f32;
            let scale = Factor2d::new(scale_x, scale_y);

            f_img_size *= scale;
            f_clip_rect.origin *= scale;
            f_clip_rect.size = crop_size;

            // 3 - offset to align + user image_offset:
            let mut offset = PxVector::zero();
            offset += align_offset;
            offset += ImageOffsetVar::get(ctx.vars).layout(ctx, AvailableSize::from_size(final_size), PxVector::zero());

            // 4 - adjust clip_rect to clip content to container final_size:
            let top_left_clip = -offset.min(PxVector::zero());
            f_clip_rect.origin += top_left_clip;
            f_clip_rect.size -= top_left_clip.to_size();
            offset += top_left_clip;
            // bottom-right clip
            f_clip_rect.size = f_clip_rect.size.min(final_size - offset.to_size());

            // 5 - adjust offset so that clip_rect.origin is at widget (0, 0):
            let f_offset = offset;
            offset -= f_clip_rect.origin.to_vector();

            if f_img_size != self.render_img_size || f_clip_rect != self.render_clip_rect || f_offset != self.render_offset {
                self.render_img_size = f_img_size;
                self.render_clip_rect = f_clip_rect;
                self.render_offset = offset;
                ctx.updates.render();
            }
        }
        */

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let img = ContextImageVar::get(ctx.vars);
            if img.is_loaded() && !self.img_size.is_empty() && !self.render_clip_rect.is_empty() {
                if self.render_offset != PxVector::zero() {
                    let transform = RenderTransform::translation_px(self.render_offset);
                    frame.push_reference_frame(self.spatial_id, FrameBinding::Value(transform), true, |frame| {
                        frame.push_image(self.render_clip_rect, self.render_img_size, img, *ImageRenderingVar::get(ctx.vars))
                    });
                } else {
                    frame.push_image(self.render_clip_rect, self.render_img_size, img, *ImageRenderingVar::get(ctx.vars));
                }
            }
        }
    }
    ImagePresenterNode {
        requested_layout: true,

        img_size: PxSize::zero(),

        measure_clip_rect: PxRect::zero(),
        measure_img_size: PxSize::zero(),
        desired_size: PxSize::zero(),

        final_size: PxSize::zero(),

        render_clip_rect: PxRect::zero(),
        render_img_size: PxSize::zero(),
        render_offset: PxVector::zero(),

        spatial_id: SpatialFrameId::new_unique(),
    }
}
