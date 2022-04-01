//! UI nodes used for building the image widget.

use super::properties::{
    ImageAlignVar, ImageCacheVar, ImageCropVar, ImageErrorArgs, ImageErrorViewVar, ImageFit, ImageFitVar, ImageLimitsVar, ImageLoadingArgs,
    ImageLoadingViewVar, ImageOffsetVar, ImageRenderingVar, ImageScaleFactorVar, ImageScalePpiVar, ImageScaleVar,
};
use crate::core::image::*;

use super::*;
use std::fmt;

context_var! {
    /// Image acquired by [`image_source`], or `Unset` by default.
    pub struct ContextImageVar: ContextImage = ContextImage::None;
}

/// Image set in a parent widget.
///
/// This type exists due to generics problems when using an `Option<impl Var<T>>` as the value of another variable.
/// Call [`as_ref`] to use it like `Option`.
///
/// See [`ContextImageVar`] for details.
///
/// [`as_ref`]: ContextImage::as_ref
#[derive(Clone)]
pub enum ContextImage {
    /// The context image variable.
    Some(ImageVar),
    /// No context image is set.
    None,
}
impl Default for ContextImage {
    fn default() -> Self {
        ContextImage::None
    }
}
impl ContextImage {
    /// Like `Option::as_ref`.
    pub fn as_ref(&self) -> Option<&ImageVar> {
        match self {
            ContextImage::Some(var) => Some(var),
            ContextImage::None => None,
        }
    }

    /// Like `Option::take`.
    pub fn take(&mut self) -> Option<ImageVar> {
        match std::mem::take(self) {
            ContextImage::Some(var) => Some(var),
            ContextImage::None => None,
        }
    }
}
impl fmt::Debug for ContextImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Some(_) => write!(f, "Some(_)"),
            Self::None => write!(f, "None"),
        }
    }
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
        image: ContextImage,
        source: S,
    }
    impl<C: UiNode, S: Var<ImageSource>> UiNode for ImageSourceNode<C, S> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            let mode = if *ImageCacheVar::get(ctx) {
                ImageCacheMode::Cache
            } else {
                ImageCacheMode::Ignore
            };
            let limits = ImageLimitsVar::get_clone(ctx);
            self.image = ContextImage::Some(ctx.services.images().get(self.source.get_clone(ctx.vars), mode, limits));
            ctx.vars
                .with_context_var(ContextImageVar, ContextVarData::map(ctx.vars, &self.source, &self.image), || {
                    self.child.init(ctx);
                });
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            ctx.vars
                .with_context_var(ContextImageVar, ContextVarData::map(ctx.vars, &self.source, &self.image), || {
                    self.child.deinit(ctx);
                });
            self.image = ContextImage::None;
        }

        fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            ctx.vars
                .with_context_var(ContextImageVar, ContextVarData::map(ctx.vars, &self.source, &self.image), || {
                    self.child.event(ctx, args);
                });
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(s) = self.source.clone_new(ctx) {
                // source update:
                let mode = if *ImageCacheVar::get(ctx) {
                    ImageCacheMode::Cache
                } else {
                    ImageCacheMode::Ignore
                };
                let limits = ImageLimitsVar::get_clone(ctx);
                self.image = ContextImage::Some(ctx.services.images().get(s, mode, limits));
            } else if let Some(enabled) = ImageCacheVar::clone_new(ctx) {
                // cache-mode update:
                let images = ctx.services.images();
                let is_cached = images.is_cached(self.image.as_ref().unwrap().get(ctx.vars));
                if enabled != is_cached {
                    if is_cached {
                        // must not cache, but is cached, detach from cache.

                        self.image = ContextImage::Some(images.detach(self.image.take().unwrap(), ctx.vars));
                    } else {
                        // must cache, but image is not cached, get source again.

                        let source = self.source.get_clone(ctx);
                        let limits = ImageLimitsVar::get_clone(ctx);
                        self.image = ContextImage::Some(ctx.services.images().get(source, ImageCacheMode::Cache, limits));
                    }
                }
            }

            ctx.vars
                .with_context_var(ContextImageVar, ContextVarData::map(ctx.vars, &self.source, &self.image), || {
                    self.child.update(ctx);
                });
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            ctx.vars
                .with_context_var(ContextImageVar, ContextVarData::map(ctx.vars, &self.source, &self.image), || {
                    self.child.measure(ctx, available_size)
                })
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
            ctx.vars
                .with_context_var(ContextImageVar, ContextVarData::map(ctx.vars, &self.source, &self.image), || {
                    self.child.arrange(ctx, widget_layout, final_size);
                });
        }

        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            ctx.vars.with_context_var(
                ContextImageVar,
                ContextVarData::map_read(ctx.vars, &self.source, &self.image),
                || {
                    self.child.info(ctx, info);
                },
            );
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            ctx.vars.with_context_var(
                ContextImageVar,
                ContextVarData::map_read(ctx.vars, &self.source, &self.image),
                || {
                    self.child.subscriptions(ctx, subscriptions);
                },
            );
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            ctx.vars.with_context_var(
                ContextImageVar,
                ContextVarData::map_read(ctx.vars, &self.source, &self.image),
                || {
                    self.child.render(ctx, frame);
                },
            );
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            ctx.vars.with_context_var(
                ContextImageVar,
                ContextVarData::map_read(ctx.vars, &self.source, &self.image),
                || {
                    self.child.render_update(ctx, update);
                },
            );
        }
    }
    ImageSourceNode {
        child,
        image: ContextImage::None,
        source: source.into_var(),
    }
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
                return DataUpdate::None;
            }
            if is_new {
                // init or generator changed.
                if let Some(var) = ContextImageVar::get(ctx.vars).as_ref() {
                    if let Some(e) = var.get(ctx).error() {
                        return DataUpdate::Update(ImageErrorArgs {
                            error: e.to_owned().into(),
                        });
                    }
                }
                return DataUpdate::None;
            } else if let Some(new) = ContextImageVar::get_new(ctx.vars) {
                // image var update.
                if let Some(var) = new.as_ref() {
                    if let Some(e) = var.get(ctx).error() {
                        return DataUpdate::Update(ImageErrorArgs {
                            error: e.to_owned().into(),
                        });
                    }
                }
                return DataUpdate::None;
            } else if let Some(var) = ContextImageVar::get(ctx.vars).as_ref() {
                // image update.
                if let Some(new) = var.get_new(ctx) {
                    if let Some(e) = new.error() {
                        return DataUpdate::Update(ImageErrorArgs {
                            error: e.to_owned().into(),
                        });
                    } else {
                        return DataUpdate::None;
                    }
                }
            }

            DataUpdate::Same
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
            if let Some(img) = ContextImageVar::get(vars).as_ref() {
                subscriptions.var(vars, img);
            }
        },
        |ctx, is_new| {
            if *InLoadingViewVar::get(ctx) {
                // avoid recursion.
                return DataUpdate::None;
            }
            if is_new {
                // init or generator changed.
                if let Some(var) = ContextImageVar::get(ctx.vars).as_ref() {
                    if var.get(ctx).is_loading() {
                        return DataUpdate::Update(ImageLoadingArgs {});
                    }
                }
                return DataUpdate::None;
            } else if let Some(new) = ContextImageVar::get_new(ctx.vars) {
                ctx.updates.subscriptions();
                // image var update.
                if let Some(var) = new.as_ref() {
                    if var.get(ctx).is_loading() {
                        return DataUpdate::Update(ImageLoadingArgs {});
                    }
                }
                return DataUpdate::None;
            } else if let Some(var) = ContextImageVar::get(ctx.vars).as_ref() {
                // image update.
                if let Some(new) = var.get_new(ctx) {
                    if new.is_loading() {
                        return DataUpdate::Update(ImageLoadingArgs {});
                    } else {
                        return DataUpdate::None;
                    }
                }
            }

            DataUpdate::Same
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

            if let Some(img) = ContextImageVar::get(ctx.vars).as_ref() {
                subscriptions.var(ctx, img);
            }
        }

        fn init(&mut self, ctx: &mut WidgetContext) {
            if let Some(var) = ContextImageVar::get(ctx.vars).as_ref() {
                self.img_size = var.get(ctx).size();
                self.requested_layout = true;
            } else {
                self.img_size = PxSize::zero();
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(var) = ContextImageVar::get_new(ctx.vars) {
                ctx.updates.subscriptions_layout_and_render();
                self.requested_layout = true;

                if let Some(var) = var.as_ref() {
                    self.img_size = var.get(ctx).size();
                } else {
                    self.img_size = PxSize::zero();
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

            if let Some(var) = ContextImageVar::get(ctx.vars).as_ref() {
                if let Some(img) = var.get_new(ctx.vars) {
                    let img_size = img.size();
                    if self.img_size != img_size {
                        self.img_size = img_size;
                        ctx.updates.layout();
                        self.requested_layout = true;
                    } else if img.is_loaded() {
                        ctx.updates.render();
                    }
                }
            }

            if ImageRenderingVar::is_new(ctx) {
                ctx.updates.render();
            }
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            if let Some(var) = ContextImageVar::get(ctx.vars).as_ref() {
                let img_rect = PxRect::from_size(self.img_size);

                let crop = ImageCropVar::get(ctx).to_layout(ctx, AvailableSize::from_size(self.img_size), img_rect);

                self.measure_img_size = self.img_size;
                self.measure_clip_rect = img_rect.intersection(&crop).unwrap_or_default();

                let mut scale = *ImageScaleVar::get(ctx);
                if *ImageScalePpiVar::get(ctx) {
                    let img = var.get(ctx.vars);
                    let sppi = ctx.metrics.screen_ppi;
                    let (ippi_x, ippi_y) = img.ppi().unwrap_or((sppi, sppi));
                    scale *= Factor2d::new(ippi_x / sppi, ippi_y / sppi);
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
            } else {
                // no context image
                PxSize::zero()
            }
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
            offset += ImageOffsetVar::get(ctx.vars).to_layout(ctx, AvailableSize::from_size(final_size), PxVector::zero());

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

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            if let Some(var) = ContextImageVar::get(ctx.vars).as_ref() {
                let img = var.get(ctx.vars);
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
