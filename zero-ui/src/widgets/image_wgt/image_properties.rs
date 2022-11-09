use super::*;
use std::fmt;

pub use crate::core::image::ImageLimits;
pub use crate::core::render::ImageRendering;
use crate::core::window::{WindowLoadingHandle, Windows};
use crate::widgets::window::nodes::BlockWindowLoad;
use nodes::CONTEXT_IMAGE_VAR;

/// Image layout mode.
///
/// This layout mode can be set to all images inside a widget using [`img_fit`], in the image widget
/// it can be set using the [`fit`] property, the [`image_presenter`] uses this value to calculate the image final size.
///
/// The image desired size is its original size, either in pixels or DIPs after cropping and scaling.
///
/// [`fit`]: mod@crate::widgets::image#wp-fit
/// [`img_fit`]: fn@img_fit
/// [`image_presenter`]: crate::widgets::image::nodes::image_presenter
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ImageFit {
    /// The image original size is preserved, the image is clipped if larger then the final size.
    None,
    /// The image is resized to fill the final size, the aspect-ratio is not preserved.
    Fill,
    /// The image is resized to fit the final size, preserving the aspect-ratio.
    Contain,
    /// The image is resized to fill the final size while preserving the aspect-ratio.
    /// If the aspect ratio of the final size differs from the image, it is clipped.
    Cover,
    /// If the image is smaller then the final size applies the [`None`] layout, if its larger applies the [`Contain`] layout.
    ///
    /// [`None`]: ImageFit::None
    /// [`Contain`]: ImageFit::Contain
    ScaleDown,
}
impl fmt::Debug for ImageFit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "ImageFit::")?
        }
        match self {
            Self::None => write!(f, "None"),
            Self::Fill => write!(f, "Fill"),
            Self::Contain => write!(f, "Contain"),
            Self::Cover => write!(f, "Cover"),
            Self::ScaleDown => write!(f, "ScaleDown"),
        }
    }
}

context_var! {
    /// The Image scaling algorithm in the renderer.
    ///
    /// Is [`ImageRendering::Auto`] by default.
    pub static IMAGE_RENDERING_VAR: ImageRendering = ImageRendering::Auto;

    /// If the image is cached.
    ///
    /// Is `true` by default.
    pub static IMAGE_CACHE_VAR: bool = true;

    /// View generator for the content shown when the image does not load.
    pub static IMAGE_ERROR_VIEW_VAR: ViewGenerator<ImageErrorArgs> = ViewGenerator::nil();

    /// View generator for the content shown when the image is still loading.
    pub static IMAGE_LOADING_VIEW_VAR: ViewGenerator<ImageLoadingArgs> = ViewGenerator::nil();

    /// Custom image load and decode limits.
    ///
    /// Set to `None` to use the [`Images::limits`].
    pub static IMAGE_LIMITS_VAR: Option<ImageLimits> = None;

    /// The image layout mode.
    ///
    /// Is [`ImageFit::Contain`] by default.
    pub static IMAGE_FIT_VAR: ImageFit = ImageFit::Contain;

    /// Scaling applied to the image desired size.
    ///
    /// Does not scale by default, `1.0`.
    pub static IMAGE_SCALE_VAR: Factor2d = Factor2d::identity();

    /// If the image desired size is scaled by the screen scale factor.
    ///
    /// Is `true` by default.
    pub static IMAGE_SCALE_FACTOR_VAR: bool = true;

    /// If the image desired size is scaled considering the image and screen PPIs.
    ///
    /// Is `false` by default.
    pub static IMAGE_SCALE_PPI_VAR: bool = false;

    /// Align of the image in relation to the image widget final size.
    ///
    /// Is [`Align::CENTER`] by default.
    pub static IMAGE_ALIGN_VAR: Align = Align::CENTER;

    /// Offset applied to the image after all measure and arrange.
    pub static IMAGE_OFFSET_VAR: Vector = Vector::default();

    /// Simple clip applied to the image before layout.
    ///
    /// No cropping is done by default.
    pub static IMAGE_CROP_VAR: Rect = Rect::default();
}

/// Sets the [`ImageFit`] of all inner images.
///
/// This property sets the [`IMAGE_FIT_VAR`].
///
/// [`fit`]: mod@crate::widgets::image#wp-fit
#[property(context, default(IMAGE_FIT_VAR))]
pub fn img_fit(child: impl UiNode, fit: impl IntoVar<ImageFit>) -> impl UiNode {
    with_context_var(child, IMAGE_FIT_VAR, fit)
}

/// Sets the scale applied to all inner images.
///
/// The scaling is applied after [`img_scale_ppi`] if active.
///
/// By default not scaling is done.
///
/// [`img_scale_ppi`]: fn@img_scale_ppi
/// [`scale`]: mod@crate::widgets::image#wp-scale
#[property(context, default(IMAGE_SCALE_VAR))]
pub fn img_scale(child: impl UiNode, scale: impl IntoVar<Factor2d>) -> impl UiNode {
    with_context_var(child, IMAGE_SCALE_VAR, scale)
}

/// If the image desired size is scaled by the screen scale factor.
///
/// The image desired size is its original size after [`img_crop`], it is a pixel value, but widgets are layout using
/// device independent pixels that automatically scale in higher definition displays, when this property is enabled
/// the image size is also scaled so that the image will take the same screen space in all devices, the image can end
///
/// This is enabled by default.
///
/// [`img_crop`]: fn@img_crop
#[property(context, default(IMAGE_SCALE_FACTOR_VAR))]
pub fn img_scale_factor(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, IMAGE_SCALE_FACTOR_VAR, enabled)
}

/// Sets if the image desired size is scaled considering the image and monitor PPI.
///
/// If the image desired size is scaled by PPI.
///
/// The image desired size is its original size, after [`crop`], and it can be in pixels or scaled considering
/// the image PPI, monitor PPI and scale factor.
///
/// By default this is `false`, if `true` the image is scaled in a attempt to recreate the original physical dimensions, but it
/// only works if the image and monitor PPI are set correctly. The monitor PPI can be set using the [`Monitors`] service.
///
/// [`crop`]: #wp-crop
/// [`Monitors`]: zero_ui::core::window::Monitors
///
/// [`scape_ppi`]: mod@crate::widgets::image#wp-scape_ppi
#[property(context, default(IMAGE_SCALE_PPI_VAR))]
pub fn img_scale_ppi(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, IMAGE_SCALE_PPI_VAR, enabled)
}

/// Sets the [`Align`] of all inner images within each image widget area.
///
/// If the image is smaller then the widget area it is aligned like normal, if it is larger the "viewport" it is aligned to clip,
/// for example, alignment [`BOTTOM_RIGHT`] makes a smaller image sit at the bottom-right of the widget and makes
/// a larger image bottom-right fill the widget, clipping the rest.
///
/// By default the alignment is [`CENTER`]. The [`BASELINE`] alignment is treaded the same as [`BOTTOM`].
///
/// [`BOTTOM_RIGHT`]: Align::BOTTOM_RIGHT
/// [`CENTER`]: Align::CENTER
/// [`BASELINE`]: Align::BASELINE
/// [`BOTTOM`]: Align::BOTTOM
///
/// [`img_align`]: mod@crate::widgets::image#wp-img_align
#[property(context, default(IMAGE_ALIGN_VAR))]
pub fn img_align(child: impl UiNode, fit: impl IntoVar<Align>) -> impl UiNode {
    with_context_var(child, IMAGE_ALIGN_VAR, fit)
}

/// Sets a [`Point`] that is an offset applied to all inner images within each image widget area.
///
/// Relative values are calculated from the widget final size. Note that this is different the applying the
/// [`offset`] property on the widget it-self, the widget is not moved just the image within the widget area.
///
/// This property sets the [`IMAGE_OFFSET_VAR`]. By default no offset is applied.
///
/// [`offset`]: fn@crate::properties::offset
/// [`img_offset`]: mod@crate::widgets::image#wp-img_offset
#[property(context, default(IMAGE_OFFSET_VAR))]
pub fn img_offset(child: impl UiNode, offset: impl IntoVar<Vector>) -> impl UiNode {
    with_context_var(child, IMAGE_OFFSET_VAR, offset)
}

/// Sets a [`Rect`] that is a clip applied to all inner images before their layout.
///
/// Relative values are calculated from the image pixel size, the [`img_scale_ppi`] is only considered after.
/// Note that more complex clipping can be applied after to the full widget, this property exists primarily to
/// render selections of a [texture atlas].
///
/// By default no cropping is done.
///
/// [`img_scale_ppi`]: #fn@img_scale_ppi
/// [texture atlas]: https://en.wikipedia.org/wiki/Texture_atlas///
/// [`crop`]: mod@crate::widgets::image#wp-crop
#[property(context, default(IMAGE_CROP_VAR))]
pub fn img_crop(child: impl UiNode, crop: impl IntoVar<Rect>) -> impl UiNode {
    with_context_var(child, IMAGE_CROP_VAR, crop)
}

/// Sets the [`ImageRendering`] of all inner images.
///
/// If the image layout size is not the same as the `source` pixel size the image must be re-scaled
/// during rendering, this property selects what algorithm is used to do this re-scaling.
///
/// Note that the algorithms used in the renderer value performance over quality and do a good
/// enough job for small or temporary changes in scale only. If the image stays at a very different scale
/// after a short time a CPU re-scale task is automatically started to generate a better quality re-scaling.
///
/// If the image is an app resource known during build time you should consider pre-scaling it to match the screen
/// size at different DPIs using mipmaps.
///
/// This is [`ImageRendering::Auto`] by default.
///
/// [`rendering`]: mod@crate::widgets::image#wp-rendering
#[property(context, default(IMAGE_RENDERING_VAR))]
pub fn img_rendering(child: impl UiNode, rendering: impl IntoVar<ImageRendering>) -> impl UiNode {
    with_context_var(child, IMAGE_RENDERING_VAR, rendering)
}

/// Sets the cache mode of all inner images.
///
/// Sets if the [`source`] is cached.
///
/// By default this is `true`, meaning the image is loaded from cache and if not present it is inserted into
/// the cache, the cache lives for the app in the [`Images`] app, the image can be manually removed from cache.
///
/// If set to `false` the image is always loaded and decoded on init or when [`source`] updates and is dropped when
/// the widget is deinited or dropped.
///
/// [`source`]: #wp-source
/// [`Images`]: zero_ui::core::image::Images
///
/// [`cache`]: mod@crate::widgets::image#wp-cache
#[property(context, default(IMAGE_CACHE_VAR))]
pub fn img_cache(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, IMAGE_CACHE_VAR, enabled)
}

/// Sets custom image load and decode limits.
///
/// If not set or set to `None` the [`Images::limits`] is used.
///
/// [`Images::limits`]: crate::core::image::Images::limits
#[property(context, default(IMAGE_LIMITS_VAR))]
pub fn img_limits(child: impl UiNode, limits: impl IntoVar<Option<ImageLimits>>) -> impl UiNode {
    with_context_var(child, IMAGE_LIMITS_VAR, limits)
}

/// If the [`CONTEXT_IMAGE_VAR`] is an error.
#[property(layout)]
pub fn is_error(child: impl UiNode, state: StateVar) -> impl UiNode {
    bind_state(child, CONTEXT_IMAGE_VAR.map(|m| m.is_error()), state)
}

/// If the [`CONTEXT_IMAGE_VAR`] is a successfully loaded image.
#[property(layout)]
pub fn is_loaded(child: impl UiNode, state: StateVar) -> impl UiNode {
    bind_state(child, CONTEXT_IMAGE_VAR.map(|m| m.is_loaded()), state)
}

/// Sets the [view generator] that is used to create a content for the error message.
///
/// [view generator]: crate::widgets::view_generator
#[property(context, default(IMAGE_ERROR_VIEW_VAR))]
pub fn img_error_view(child: impl UiNode, generator: impl IntoVar<ViewGenerator<ImageErrorArgs>>) -> impl UiNode {
    with_context_var(child, IMAGE_ERROR_VIEW_VAR, generator)
}

/// Sets the [view generator] that is used to create a content for the error message.
///
/// [view generator]: crate::widgets::view_generator
#[property(context, default(IMAGE_LOADING_VIEW_VAR))]
pub fn img_loading_view(child: impl UiNode, generator: impl IntoVar<ViewGenerator<ImageLoadingArgs>>) -> impl UiNode {
    with_context_var(child, IMAGE_LOADING_VIEW_VAR, generator)
}

/// Arguments for [`img_loading_view`].
///
/// [`img_loading_view`]: fn@img_loading_view
#[derive(Clone, Debug)]
pub struct ImageLoadingArgs {}

/// Arguments for [`on_load`].
///
/// [`on_load`]: fn@on_load
#[derive(Clone, Debug)]
pub struct ImageLoadArgs {}

/// Arguments for [`on_error`] and [`img_error_view`].
///
/// [`on_error`]: fn@on_error
/// [`img_error_view`]: fn@img_error_view
#[derive(Clone, Debug)]
pub struct ImageErrorArgs {
    /// Error message.
    pub error: Text,
}

/// Image load or decode error event.
///
/// This property calls `handler` every time the [`CONTEXT_IMAGE_VAR`] updates with a different error.
///
/// # Handlers
///
/// This property accepts any [`WidgetHandler`], including the async handlers. Use one of the handler macros, [`hn!`],
/// [`hn_once!`], [`async_hn!`] or [`async_hn_once!`], to declare a handler closure.
///
/// # Route
///
/// This property is not routed, it works only inside a widget that loads images. There is also no *preview* event.
#[property(event)]
pub fn on_error(child: impl UiNode, handler: impl WidgetHandler<ImageErrorArgs>) -> impl UiNode {
    #[ui_node(struct OnErrorNode {
        child: impl UiNode,
        handler: impl WidgetHandler<ImageErrorArgs>,
        error: Text,
    })]
    impl UiNode for OnErrorNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            ctx.sub_var(&CONTEXT_IMAGE_VAR);

            CONTEXT_IMAGE_VAR.with(|i| {
                if let Some(error) = i.error() {
                    self.error = error.to_owned().into();
                    self.handler.event(ctx, &ImageErrorArgs { error: self.error.clone() });
                }
            });
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if let Some(new_img) = CONTEXT_IMAGE_VAR.get_new(ctx.vars) {
                if let Some(error) = new_img.error() {
                    if self.error != error {
                        self.error = error.to_owned().into();
                        self.handler.event(ctx, &ImageErrorArgs { error: self.error.clone() });
                    }
                } else {
                    self.error = "".into();
                }
            }

            self.handler.update(ctx);
            self.child.update(ctx, updates);
        }
    }
    OnErrorNode {
        child,
        handler,
        error: "".into(),
    }
}

/// Image loaded event.
///
/// This property calls `handler` every time the [`CONTEXT_IMAGE_VAR`] updates with a successfully loaded image.
///
/// # Handlers
///
/// This property accepts any [`WidgetHandler`], including the async handlers. Use one of the handler macros, [`hn!`],
/// [`hn_once!`], [`async_hn!`] or [`async_hn_once!`], to declare a handler closure.
///
/// # Route
///
/// This property is not routed, it works only inside a widget that loads images. There is also no *preview* event.
#[property(event)]
pub fn on_load(child: impl UiNode, handler: impl WidgetHandler<ImageLoadArgs>) -> impl UiNode {
    #[ui_node(struct OnLoadNode {
        child: impl UiNode,
        handler: impl WidgetHandler<ImageLoadArgs>,
    })]
    impl UiNode for OnLoadNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            ctx.sub_var(&CONTEXT_IMAGE_VAR);

            if CONTEXT_IMAGE_VAR.with(Image::is_loaded) {
                self.handler.event(ctx, &ImageLoadArgs {});
            }
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if let Some(new_img) = CONTEXT_IMAGE_VAR.get_new(ctx.vars) {
                if new_img.is_loaded() {
                    self.handler.event(ctx, &ImageLoadArgs {});
                }
            }

            self.handler.update(ctx);
            self.child.update(ctx, updates);
        }
    }
    OnLoadNode { child, handler }
}

/// Block window load until image is loaded.
///
/// If the image widget is in the initial window content a [`WindowLoadingHandle`] is used to delay the window
/// visually opening until the source loads, fails to load or a timeout elapses. By default `true` sets the timeout to 1 second.
#[property(layout, default(false))]
pub fn img_block_window_load(child: impl UiNode, enabled: impl IntoValue<BlockWindowLoad>) -> impl UiNode {
    #[ui_node(struct ImageBlockWindowLoadNode {
        child: impl UiNode,
        enabled: BlockWindowLoad,
        block: Option<WindowLoadingHandle>,
    })]
    impl UiNode for ImageBlockWindowLoadNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            ctx.sub_var(&CONTEXT_IMAGE_VAR);

            if let Some(delay) = self.enabled.deadline() {
                self.block = Windows::req(ctx.services).loading_handle(ctx.path.window_id(), delay);
            }
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.block.is_some() && !CONTEXT_IMAGE_VAR.with(Image::is_loading) {
                self.block = None;
            }
            self.child.update(ctx, updates);
        }
    }
    ImageBlockWindowLoadNode {
        child: child.cfg_boxed(),
        enabled: enabled.into(),
        block: None,
    }
    .cfg_boxed()
}
