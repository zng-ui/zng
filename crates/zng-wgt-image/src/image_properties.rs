use super::*;
use std::fmt;

use node::CONTEXT_IMAGE_VAR;
use zng_app::render::ImageRendering;
use zng_ext_image::{ImageDownscale, ImageLimits};
use zng_ext_window::WINDOW_Ext as _;
use zng_wgt_window::BlockWindowLoad;

/// Image layout mode.
///
/// This layout mode can be set to all images inside a widget using [`img_fit`], the [`image_presenter`] uses this value
/// to calculate the image final size.
///
/// The image desired size is its original size, either in pixels or DIPs after cropping and scaling.
///
/// [`img_fit`]: fn@img_fit
/// [`image_presenter`]: crate::node::image_presenter
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

/// Image repeat mode.
///
/// After the image is fit, aligned, offset and clipped the final image can be repeated
/// to fill any blank space by enabling [`img_repeat`] with one of these options.
///
/// [`img_repeat`]: fn@img_repeat
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ImageRepeat {
    /// The image is only rendered once.
    None,
    /// The image is repeated to fill empty space, border copies are clipped.
    Repeat,
}
impl fmt::Debug for ImageRepeat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "ImageRepeat::")?
        }
        match self {
            Self::None => write!(f, "None"),
            Self::Repeat => write!(f, "Repeat"),
        }
    }
}
impl_from_and_into_var! {
    fn from(repeat: bool) -> ImageRepeat {
        if repeat { ImageRepeat::Repeat } else { ImageRepeat::None }
    }
}

context_var! {
    /// The Image scaling algorithm in the renderer.
    ///
    /// Is `ImageRendering::Auto` by default.
    pub static IMAGE_RENDERING_VAR: ImageRendering = ImageRendering::Auto;

    /// If the image is cached.
    ///
    /// Is `true` by default.
    pub static IMAGE_CACHE_VAR: bool = true;

    /// Widget function for the content shown when the image does not load.
    pub static IMAGE_ERROR_FN_VAR: WidgetFn<ImgErrorArgs> = WidgetFn::nil();

    /// Widget function for the content shown when the image is still loading.
    pub static IMAGE_LOADING_FN_VAR: WidgetFn<ImgLoadingArgs> = WidgetFn::nil();

    /// Custom image load and decode limits.
    ///
    /// Set to `None` to use the [`IMAGES.limits`].
    ///
    /// [`IMAGES.limits`]: zng_ext_image::IMAGES::limits
    pub static IMAGE_LIMITS_VAR: Option<ImageLimits> = None;

    /// Custom resize applied during image decode.
    ///
    /// Is `None` by default.
    pub static IMAGE_DOWNSCALE_VAR: Option<ImageDownscale> = None;

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
    /// Is `Align::CENTER` by default.
    pub static IMAGE_ALIGN_VAR: Align = Align::CENTER;

    /// Offset applied to the image after all measure and arrange.
    pub static IMAGE_OFFSET_VAR: Vector = Vector::default();

    /// Simple clip applied to the image before layout.
    ///
    /// No cropping is done by default.
    pub static IMAGE_CROP_VAR: Rect = Rect::default();

    /// Pattern repeat applied on the final image.
    ///
    /// Is `ImageRepeat::None` by default.
    pub static IMAGE_REPEAT_VAR: ImageRepeat = ImageRepeat::None;

    /// Spacing between repeated image copies.
    ///
    /// is `Size::zero` by default.
    pub static IMAGE_REPEAT_SPACING_VAR: Size = Size::zero();
}

/// Sets the [`ImageFit`] of all inner images.
///
/// This property sets the [`IMAGE_FIT_VAR`].
#[property(CONTEXT, default(IMAGE_FIT_VAR), widget_impl(Image))]
pub fn img_fit(child: impl IntoUiNode, fit: impl IntoVar<ImageFit>) -> UiNode {
    with_context_var(child, IMAGE_FIT_VAR, fit)
}

/// Sets the scale applied to all inner images.
///
/// The scaling is applied after [`img_scale_ppi`] if active.
///
/// By default not scaling is done.
///
/// [`img_scale_ppi`]: fn@img_scale_ppi
#[property(CONTEXT, default(IMAGE_SCALE_VAR), widget_impl(Image))]
pub fn img_scale(child: impl IntoUiNode, scale: impl IntoVar<Factor2d>) -> UiNode {
    with_context_var(child, IMAGE_SCALE_VAR, scale)
}

/// Sets if the image desired size is scaled by the screen scale factor.
///
/// The image desired size is its original size after [`img_crop`], it is a pixel value, but widgets are layout using
/// device independent pixels that automatically scale in higher definition displays, when this property is enabled
/// the image size is also scaled so that the image will take the same screen space in all devices.
///
/// This is enabled by default.
///
/// [`img_crop`]: fn@img_crop
#[property(CONTEXT, default(IMAGE_SCALE_FACTOR_VAR), widget_impl(Image))]
pub fn img_scale_factor(child: impl IntoUiNode, enabled: impl IntoVar<bool>) -> UiNode {
    with_context_var(child, IMAGE_SCALE_FACTOR_VAR, enabled)
}

/// Sets if the image desired size is scaled considering the image and monitor PPI.
///
/// The image desired size is its original size, after [`img_crop`], and it can be in pixels or scaled considering
/// the image PPI, monitor PPI and scale factor.
///
/// By default this is `false`, if `true` the image is scaled in a attempt to recreate the original physical dimensions, this
/// only works if the image and monitor PPI are set correctly. The monitor PPI can be set using the [`MONITORS`] service.
///
/// [`img_crop`]: fn@img_crop
/// [`MONITORS`]: zng_ext_window::MONITORS
#[property(CONTEXT, default(IMAGE_SCALE_PPI_VAR), widget_impl(Image))]
pub fn img_scale_ppi(child: impl IntoUiNode, enabled: impl IntoVar<bool>) -> UiNode {
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
/// [`BOTTOM_RIGHT`]: zng_wgt::prelude::Align::BOTTOM_RIGHT
/// [`CENTER`]: zng_wgt::prelude::Align::CENTER
/// [`BASELINE`]: zng_wgt::prelude::Align::BASELINE
/// [`BOTTOM`]: zng_wgt::prelude::Align::BOTTOM
/// [`Align`]: zng_wgt::prelude::Align
/// [`img_align`]: fn@crate::img_align
#[property(CONTEXT, default(IMAGE_ALIGN_VAR), widget_impl(Image))]
pub fn img_align(child: impl IntoUiNode, align: impl IntoVar<Align>) -> UiNode {
    with_context_var(child, IMAGE_ALIGN_VAR, align)
}

/// Sets a [`Point`] that is an offset applied to all inner images within each image widget area.
///
/// Relative values are calculated from the widget final size. Note that this is different the applying the
/// `offset` property on the widget itself, the widget is not moved just the image within the widget area.
///
/// This property sets the [`IMAGE_OFFSET_VAR`]. By default no offset is applied.
///
/// [`img_offset`]: fn@crate::img_offset
/// [`Point`]: zng_wgt::prelude::Point
#[property(CONTEXT, default(IMAGE_OFFSET_VAR), widget_impl(Image))]
pub fn img_offset(child: impl IntoUiNode, offset: impl IntoVar<Vector>) -> UiNode {
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
/// [texture atlas]: https://en.wikipedia.org/wiki/Texture_atlas
/// [`Rect`]: zng_wgt::prelude::Rect
#[property(CONTEXT, default(IMAGE_CROP_VAR), widget_impl(Image))]
pub fn img_crop(child: impl IntoUiNode, crop: impl IntoVar<Rect>) -> UiNode {
    with_context_var(child, IMAGE_CROP_VAR, crop)
}

/// Sets the [`ImageRepeat`] of all inner images.
///
/// Note that `repeat` converts from `bool` so you can set this property to `img_repeat = true;` to
/// enable repeat in all inner images.
///
/// See also [`img_repeat_spacing`] to control the space between repeated tiles.
///
/// This property sets the [`IMAGE_REPEAT_VAR`].
///
/// [`img_repeat_spacing`]: fn@img_repeat_spacing
#[property(CONTEXT, default(IMAGE_REPEAT_VAR), widget_impl(Image))]
pub fn img_repeat(child: impl IntoUiNode, repeat: impl IntoVar<ImageRepeat>) -> UiNode {
    with_context_var(child, IMAGE_REPEAT_VAR, repeat)
}

/// Sets the spacing between copies of the image if it is repeated.
///
/// Relative lengths are computed on the size of a single repeated tile image, so `100.pct()` is *skips*
/// an entire image of space. The leftover size is set to the space taken by tile images that do not fully
/// fit inside the clip area, `1.lft()` will insert space to cause only fully visible tiles to remain on screen.
///
/// This property sets the [`IMAGE_REPEAT_SPACING_VAR`].
#[property(CONTEXT, default(IMAGE_REPEAT_SPACING_VAR), widget_impl(Image))]
pub fn img_repeat_spacing(child: impl IntoUiNode, spacing: impl IntoVar<Size>) -> UiNode {
    with_context_var(child, IMAGE_REPEAT_SPACING_VAR, spacing)
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
/// [`ImageRendering`]: zng_app::render::ImageRendering
/// [`ImageRendering::Auto`]: zng_app::render::ImageRendering::Auto
#[property(CONTEXT, default(IMAGE_RENDERING_VAR), widget_impl(Image))]
pub fn img_rendering(child: impl IntoUiNode, rendering: impl IntoVar<ImageRendering>) -> UiNode {
    with_context_var(child, IMAGE_RENDERING_VAR, rendering)
}

/// Sets the cache mode of all inner images.
///
/// Sets if the [`source`] is cached.
///
/// By default this is `true`, meaning the image is loaded from cache and if not present it is inserted into
/// the cache, the cache lives for the app in the [`IMAGES`] service, the image can be manually removed from cache.
///
/// If set to `false` the image is always loaded and decoded on init or when [`source`] updates and is dropped when
/// the widget is deinited or dropped.
///
/// [`source`]: fn@crate::source
/// [`IMAGES`]: zng_ext_image::IMAGES
#[property(CONTEXT, default(IMAGE_CACHE_VAR), widget_impl(Image))]
pub fn img_cache(child: impl IntoUiNode, enabled: impl IntoVar<bool>) -> UiNode {
    with_context_var(child, IMAGE_CACHE_VAR, enabled)
}

/// Sets custom image load and decode limits.
///
/// If not set or set to `None` the [`IMAGES.limits`] is used.
///
/// See also [`img_downscale`] for a way to still display unexpected large images.
///
/// [`IMAGES.limits`]: zng_ext_image::IMAGES::limits
/// [`img_downscale`]: fn@img_downscale
#[property(CONTEXT, default(IMAGE_LIMITS_VAR), widget_impl(Image))]
pub fn img_limits(child: impl IntoUiNode, limits: impl IntoVar<Option<ImageLimits>>) -> UiNode {
    with_context_var(child, IMAGE_LIMITS_VAR, limits)
}

/// Custom pixel resize applied during image load/decode.
///
/// Note that this resize affects the image actual pixel size directly when it is loading to force the image pixels to be within an expected size.
/// This property primary use is as error recover before the [`img_limits`] error happens, you set the limits to the size that should not even
/// be processed and set this property to the maximum size expected.
///
/// Changing this value after an image is already loaded or loading will cause the image to reload, image cache allocates different
/// entries for different downscale values, this means that this property should never be used for responsive resize,use the widget
/// size and other properties to efficiently resize an image on screen.
///
/// [`IMAGES.limits`]: zng_ext_image::IMAGES::limits
/// [`img_limits`]: fn@img_limits
#[property(CONTEXT, default(IMAGE_DOWNSCALE_VAR), widget_impl(Image))]
pub fn img_downscale(child: impl IntoUiNode, downscale: impl IntoVar<Option<ImageDownscale>>) -> UiNode {
    with_context_var(child, IMAGE_DOWNSCALE_VAR, downscale)
}

/// If the [`CONTEXT_IMAGE_VAR`] is an error.
#[property(LAYOUT, widget_impl(Image))]
pub fn is_error(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    bind_state(child, CONTEXT_IMAGE_VAR.map(|m| m.is_error()), state)
}

/// If the [`CONTEXT_IMAGE_VAR`] has successfully loaded.
#[property(LAYOUT, widget_impl(Image))]
pub fn is_loaded(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    bind_state(child, CONTEXT_IMAGE_VAR.map(|m| m.is_loaded()), state)
}

/// Gets the [`CONTEXT_IMAGE_VAR`].
#[property(LAYOUT, widget_impl(Image))]
pub fn get_img(child: impl IntoUiNode, state: impl IntoVar<Option<Img>>) -> UiNode {
    bind_state(child, CONTEXT_IMAGE_VAR.map_into(), state)
}

/// Gets the [`CONTEXT_IMAGE_VAR`] ideal size.
#[property(LAYOUT, widget_impl(Image))]
pub fn get_img_layout_size(child: impl IntoUiNode, state: impl IntoVar<PxSize>) -> UiNode {
    let state = state.into_var();
    match_node(child, move |_, op| {
        if let UiNodeOp::Layout { .. } = op {
            let size = CONTEXT_IMAGE_VAR.with(|img| img.layout_size(&LAYOUT.metrics()));
            if state.get() != size {
                state.set(size);
            }
        }
    })
}

/// Sets the [`wgt_fn!`] that is used to create a content for the error message.
///
/// [`wgt_fn!`]: zng_wgt::wgt_fn
#[property(CONTEXT, default(IMAGE_ERROR_FN_VAR), widget_impl(Image))]
pub fn img_error_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<ImgErrorArgs>>) -> UiNode {
    with_context_var(child, IMAGE_ERROR_FN_VAR, wgt_fn)
}

/// Sets the [`wgt_fn!`] that is used to create a content for the loading message.
///
/// [`wgt_fn!`]: zng_wgt::wgt_fn
#[property(CONTEXT, default(IMAGE_LOADING_FN_VAR), widget_impl(Image))]
pub fn img_loading_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<ImgLoadingArgs>>) -> UiNode {
    with_context_var(child, IMAGE_LOADING_FN_VAR, wgt_fn)
}

/// Arguments for [`img_loading_fn`].
///
/// [`img_loading_fn`]: fn@img_loading_fn
#[derive(Clone, Default, Debug, PartialEq)]
#[non_exhaustive]
pub struct ImgLoadingArgs {}

/// Arguments for [`on_load`].
///
/// [`on_load`]: fn@on_load
#[derive(Clone, Default, Debug)]
#[non_exhaustive]
pub struct ImgLoadArgs {}

/// Arguments for [`on_error`] and [`img_error_fn`].
///
/// [`on_error`]: fn@on_error
/// [`img_error_fn`]: fn@img_error_fn
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ImgErrorArgs {
    /// Error message.
    pub error: Txt,
}

impl ImgErrorArgs {
    /// New args.
    pub fn new(error: impl Into<Txt>) -> Self {
        Self { error: error.into() }
    }
}

/// Image load or decode error event.
///
/// This property calls `handler` every time the [`CONTEXT_IMAGE_VAR`] updates with a different error or on the first update
/// after init if the image is already in error on init.
///
/// # Handlers
///
/// This property accepts any [`WidgetHandler`], including the async handlers. Use one of the handler macros, [`hn!`],
/// [`hn_once!`], [`async_hn!`] or [`async_hn_once!`], to declare a handler closure.
///
/// # Route
///
/// This property is not routed, it works only inside a widget that loads images. There is also no *preview* event.
///
/// [`WidgetHandler`]: zng_wgt::prelude::WidgetHandler
/// [`hn!`]: zng_wgt::prelude::hn!
/// [`hn_once!`]: zng_wgt::prelude::hn_once!
/// [`async_hn!`]: zng_wgt::prelude::async_hn!
/// [`async_hn_once!`]: zng_wgt::prelude::async_hn_once!
#[property(EVENT, widget_impl(Image))]
pub fn on_error(child: impl IntoUiNode, handler: Handler<ImgErrorArgs>) -> UiNode {
    let mut handler = handler.into_wgt_runner();
    let mut error = Txt::from_str("");
    let mut first_update = false;

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&CONTEXT_IMAGE_VAR);

            if CONTEXT_IMAGE_VAR.with(Img::is_error) {
                first_update = true;
                WIDGET.update();
            }
        }
        UiNodeOp::Deinit => {
            handler.deinit();
        }
        UiNodeOp::Update { .. } => {
            if let Some(new_img) = CONTEXT_IMAGE_VAR.get_new() {
                first_update = false;
                if let Some(e) = new_img.error() {
                    if error != e {
                        error = e;
                        handler.event(&ImgErrorArgs { error: error.clone() });
                    }
                } else {
                    error = "".into();
                }
            } else if std::mem::take(&mut first_update) {
                CONTEXT_IMAGE_VAR.with(|i| {
                    if let Some(e) = i.error() {
                        error = e;
                        handler.event(&ImgErrorArgs { error: error.clone() });
                    }
                });
            }

            handler.update();
        }
        _ => {}
    })
}

/// Image loaded event.
///
/// This property calls `handler` every time the [`CONTEXT_IMAGE_VAR`] updates with a successfully loaded image or on the first
/// update after init if the image is already loaded on init.
///
/// # Handlers
///
/// This property accepts any [`WidgetHandler`], including the async handlers. Use one of the handler macros, [`hn!`],
/// [`hn_once!`], [`async_hn!`] or [`async_hn_once!`], to declare a handler closure.
///
/// # Route
///
/// This property is not routed, it works only inside a widget that loads images. There is also no *preview* event.
///
/// [`WidgetHandler`]: zng_wgt::prelude::WidgetHandler
/// [`hn!`]: zng_wgt::prelude::hn!
/// [`hn_once!`]: zng_wgt::prelude::hn_once!
/// [`async_hn!`]: zng_wgt::prelude::async_hn!
/// [`async_hn_once!`]: zng_wgt::prelude::async_hn_once!
#[property(EVENT, widget_impl(Image))]
pub fn on_load(child: impl IntoUiNode, handler: Handler<ImgLoadArgs>) -> UiNode {
    let mut handler = handler.into_wgt_runner();
    let mut first_update = false;

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&CONTEXT_IMAGE_VAR);

            if CONTEXT_IMAGE_VAR.with(Img::is_loaded) {
                first_update = true;
                WIDGET.update();
            }
        }
        UiNodeOp::Deinit => {
            handler.deinit();
        }
        UiNodeOp::Update { .. } => {
            if let Some(new_img) = CONTEXT_IMAGE_VAR.get_new() {
                first_update = false;
                if new_img.is_loaded() {
                    handler.event(&ImgLoadArgs {});
                }
            } else if std::mem::take(&mut first_update) && CONTEXT_IMAGE_VAR.with(Img::is_loaded) {
                handler.event(&ImgLoadArgs {});
            }

            handler.update();
        }
        _ => {}
    })
}

/// Block window load until image is loaded.
///
/// If the image widget is in the initial window content a [`WindowLoadingHandle`] is used to delay the window
/// visually opening until the source loads, fails to load or a timeout elapses. By default `true` sets the timeout to 1 second.
///
/// [`WindowLoadingHandle`]: zng_ext_window::WindowLoadingHandle
#[property(LAYOUT, default(false), widget_impl(Image))]
pub fn img_block_window_load(child: impl IntoUiNode, enabled: impl IntoValue<BlockWindowLoad>) -> UiNode {
    let enabled = enabled.into();
    let mut block = None;

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&CONTEXT_IMAGE_VAR);

            if let Some(delay) = enabled.deadline() {
                block = WINDOW.loading_handle(delay);
            }
        }
        UiNodeOp::Update { .. } => {
            if block.is_some() && !CONTEXT_IMAGE_VAR.with(Img::is_loading) {
                block = None;
            }
        }
        _ => {}
    })
}
