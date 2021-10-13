use zero_ui_core::image::ImageSource;

use crate::prelude::new_widget::*;

/// Image presenter.
///
/// This widget loads a still image from a variety of sources and presents it.
#[widget($crate::widgets::image)]
pub mod image {
    use zero_ui::core::image::{ImageCacheMode, ImageSource, ImageVar};

    use super::*;
    use properties::{ImageCacheVar, ImageRenderingVar};

    properties! {
        child {
            /// The image source.
            ///
            /// Can be a file path, an URI, binary included in the app and more.
            source(impl IntoVar<ImageSource>);
        }

        /// Sets the image scaling algorithm used to rescale the image in the renderer.
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
        properties::image_rendering as rendering;

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
        properties::image_cache as cache;
    }

    fn new_child(source: impl IntoVar<ImageSource>) -> impl UiNode {
        struct ImageNode<T> {
            source: T,
            image: Option<ImageVar>,
            measured_image_size: PxSize,
            final_size: PxSize,
        }
        #[impl_ui_node(none)]
        impl<T: Var<ImageSource>> UiNode for ImageNode<T> {
            fn init(&mut self, ctx: &mut WidgetContext) {
                let cache_mode = if *ImageCacheVar::get(ctx) {
                    ImageCacheMode::Cache
                } else {
                    ImageCacheMode::Ignore
                };
                let img = ctx.services.images().get(self.source.get_clone(ctx.vars), cache_mode);
                self.image = Some(img);
            }
            fn deinit(&mut self, _: &mut WidgetContext) {
                self.image = None;
            }

            fn update(&mut self, ctx: &mut WidgetContext) {
                if self.source.is_new(ctx) {
                    self.init(ctx);
                } else if let Some(img) = self.image.as_ref().unwrap().get_new(ctx.vars) {
                    if let Some(e) = img.error() {
                        log::error!("{}", e);
                        if self.final_size != PxSize::zero() {
                            ctx.updates.layout();
                        }
                    } else if self.measured_image_size != img.size() {
                        ctx.updates.layout();
                    } else {
                        ctx.updates.render();
                    }
                }
            }

            fn measure(&mut self, ctx: &mut LayoutContext, _: AvailableSize) -> PxSize {
                let img = self.image.as_ref().unwrap().get(ctx.vars);
                self.measured_image_size = img.size();
                img.layout_size(ctx)
            }

            fn arrange(&mut self, _: &mut LayoutContext, final_size: PxSize) {
                self.final_size = final_size;
            }
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                let img = self.image.as_ref().unwrap().get(ctx.vars);
                if img.is_loaded() {
                    frame.push_image(PxRect::from(self.final_size), img, *ImageRenderingVar::get(ctx.vars));
                } else if let Some(e) = img.error() {
                    todo!("{}", e);
                    //frame.push_text();
                }
            }
        }
        ImageNode {
            source: source.into_var(),
            image: None,
            measured_image_size: PxSize::zero(),
            final_size: PxSize::zero(),
        }
    }

    /// Properties that configure [`image!`] widgets from parent widgets.
    ///
    /// Note that this properties are already available in the [`image!`] widget directly without the `image_` prefix.
    ///
    /// [`image!`]: mod@crate::widgets::image
    pub mod properties {
        use super::*;

        pub use crate::core::render::ImageRendering;

        context_var! {
            /// The Image scaling algorithm in the renderer.
            ///
            /// Is [`ImageRendering::Auto`] by default.
            pub struct ImageRenderingVar: ImageRendering = const ImageRendering::Auto;

            /// If the image is cached.
            ///
            /// Is `true` by default.
            pub struct ImageCacheVar: bool = const true;
        }

        /// Sets the [`ImageRendering`] of all inner images.
        ///
        /// See the [`rendering`] property in the widget for more details.
        ///
        /// [`rendering`]: crate::widgets::image#wp-rendering
        #[property(context, default(ImageRendering::Auto))]
        pub fn image_rendering(child: impl UiNode, rendering: impl IntoVar<ImageRendering>) -> impl UiNode {
            with_context_var(child, ImageRenderingVar, rendering)
        }

        /// Sets the cache mode of all inner images.
        ///
        /// See the [`cache`] property in the widget for more details.
        ///
        /// [`cache`]: crate::widgets::image#wp-cache
        #[property(context, default(true))]
        pub fn image_cache(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
            with_context_var(child, ImageCacheVar, enabled)
        }
    }
}

/// Image presenter.
///
/// This function is the shorthand form of [`image!`].
///
/// # Examples
///
/// Create an image button:
///
/// ```
/// use zero_ui::prelude::*;
/// use zero_ui::widgets::image::properties::*;
///
/// # let _ =
/// button! {
///     content = image("https://httpbin.org/image");
///     image_rendering = ImageRendering::Pixelated;
/// }
/// # ;
/// ```
///
/// Note that you can only define the [`source`] property in the image widget but you can
/// still use the [`image::properties`] in the parent widget to define other properties.
///
/// [`image!`]: mod@image
/// [`source`]: mod@image#wp-source
pub fn image(source: impl IntoVar<ImageSource>) -> impl Widget {
    image! { source }
}
