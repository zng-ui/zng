use crate::prelude::new_widget::*;

/// Image presenter.
///
/// This widget loads a still image from a variety of sources and presents it.
#[widget($crate::widgets::image)]
pub mod image {
    use zero_ui::core::image::{ImageCacheKey, ImageDataFormat, ImageVar};

    use super::*;
    use crate::core::task::http::Uri;
    use properties::ImageRenderingVar;
    use std::{
        fmt,
        path::{Path, PathBuf},
        sync::Arc,
    };

    /// The different inputs accepted by the [`source`] property.
    ///
    /// [`source`]: #wp-source
    #[derive(Clone)]
    pub enum ImageSource {
        /// Gets the image from [`Images`].
        /// 
        /// [`Images`]: crate::core::image::Images
        Request(ImageCacheKey),
        /// Uses an image var.
        Image(ImageVar),
    }
    impl fmt::Debug for ImageSource {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if f.alternate() {
                write!(f, "ImageSource::")?;
            }
            match self {
                ImageSource::Request(p) => f.debug_tuple("Request").field(p).finish(),
                ImageSource::Image(_) => f.debug_tuple("Image").finish(),
            }
        }
    }
    impl_from_and_into_var! {
        fn from(image: ImageVar) -> ImageSource {
            ImageSource::Image(image)
        }
        fn from(key: ImageCacheKey) -> ImageSource {
            ImageSource::Request(key)
        }
        fn from(path: PathBuf) -> ImageSource {
            ImageCacheKey::from(path).into()
        }
        fn from(path: &Path) -> ImageSource {
            ImageCacheKey::from(path).into()
        }
        fn from(uri: Uri) -> ImageSource {
            ImageCacheKey::from(uri).into()
        }
        /// See [`ImageCacheKey`] conversion from `&str`
        fn from(s: &str) -> ImageSource {
            ImageCacheKey::from(s).into()
        }
        /// Same as conversion from `&str`.
        fn from(s: String) -> ImageSource {
            ImageCacheKey::from(s).into()
        }
        /// Same as conversion from `&str`.
        fn from(s: Text) -> ImageSource {
            ImageCacheKey::from(s).into()
        }
        /// From encoded data of [`Unknown`] format.
        ///
        /// [`Unknown`]: ImageDataFormat::Unknown
        fn from(data: &'static [u8]) -> ImageSource {
            ImageCacheKey::from(data).into()
        }
        /// From encoded data of [`Unknown`] format.
        ///
        /// [`Unknown`]: ImageDataFormat::Unknown
        fn from<const N: usize>(data: &'static [u8; N]) -> ImageSource {
            ImageCacheKey::from(data).into()
        }
        /// From encoded data of [`Unknown`] format.
        ///
        /// [`Unknown`]: ImageDataFormat::Unknown
        fn from(data: Arc<Vec<u8>>) -> ImageSource {
            ImageCacheKey::from(data).into()
        }
        /// From encoded data of [`Unknown`] format.
        ///
        /// [`Unknown`]: ImageDataFormat::Unknown
        fn from(data: Vec<u8>) -> ImageSource {
            ImageCacheKey::from(data).into()
        }
        /// From encoded data of known format.
        fn from<F: Into<ImageDataFormat> + Clone>((data, format): (&'static [u8], F)) -> ImageSource {
            ImageCacheKey::from((data, format)).into()
        }
        /// From encoded data of known format.
        fn from<F: Into<ImageDataFormat> + Clone, const N: usize>((data, format): (&'static [u8; N], F)) -> ImageSource {
            ImageCacheKey::from((data, format)).into()
        }
        /// From encoded data of known format.
        fn from<F: Into<ImageDataFormat> + Clone>((data, format): (Vec<u8>, F)) -> ImageSource {
            ImageCacheKey::from((data, format)).into()
        }
        /// From encoded data of known format.
        fn from<F: Into<ImageDataFormat> + Clone>((data, format): (Arc<Vec<u8>>, F)) -> ImageSource {
            ImageCacheKey::from((data, format)).into()
        }
    }

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
    }

    fn new_child(source: impl IntoVar<ImageSource>) -> impl UiNode {
        struct ImageNode<T> {
            source: T,
            image: Option<ImageVar>,
            final_size: PxSize,
        }
        #[impl_ui_node(none)]
        impl<T: Var<ImageSource>> UiNode for ImageNode<T> {
            fn init(&mut self, ctx: &mut WidgetContext) {
                let img = match self.source.get_clone(ctx) {
                    ImageSource::Request(r) => ctx.services.images().get(r),
                    ImageSource::Image(img) => img,
                };
                self.image = Some(img);
            }
            fn deinit(&mut self, _: &mut WidgetContext) {
                self.image = None;
            }

            fn update(&mut self, ctx: &mut WidgetContext) {
                if self.source.is_new(ctx) {
                    self.init(ctx);
                } else if let Some(r) = self.image.as_ref().unwrap().get_new(ctx.vars) {
                    if let Some(e) = r.error() {
                        log::error!("{}", e);
                        if self.final_size != PxSize::zero() {
                            ctx.updates.layout();
                        }
                    } else {
                        ctx.updates.layout();
                    }
                }
            }

            fn measure(&mut self, ctx: &mut LayoutContext, _: AvailableSize) -> PxSize {
                let img = self.image.as_ref().unwrap().get(ctx.vars);
                img.layout_size(ctx)
            }

            fn arrange(&mut self, _: &mut LayoutContext, final_size: PxSize) {
                self.final_size = final_size;
            }
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                let img = self.image.as_ref().unwrap().get(ctx.vars);
                if img.is_loaded() {
                    frame.push_image(PxRect::from(self.final_size), img, *ImageRenderingVar::get(ctx.vars));
                }
            }
        }
        ImageNode {
            source: source.into_var(),
            image: None,
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
        }

        /// Sets the [`ImageRendering`] of all inner images.
        ///
        /// See the [`rendering`] property in the widget for more details.
        ///
        /// This property binds `rendering` to the [`ImageRenderingVar`] in the widget context.
        ///
        /// [`rendering`]: crate::widgets::image#wp-rendering
        #[property(context)]
        pub fn image_rendering(child: impl UiNode, rendering: impl IntoVar<ImageRendering>) -> impl UiNode {
            with_context_var(child, ImageRenderingVar, rendering)
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
pub fn image(source: impl IntoVar<image::ImageSource>) -> impl Widget {
    image! { source }
}
