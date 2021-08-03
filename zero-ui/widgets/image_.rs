use crate::prelude::new_widget::*;

#[widget($crate::widgets::image)]
pub mod image {
    use zero_ui::core::image::{ImageCacheKey, ImageRequestVar};

    use super::*;
    use crate::core::task::http::Uri;
    use std::{convert::TryFrom, path::PathBuf};

    /// The different inputs accepted by the [`source`] property.
    ///
    /// [`source`]: #wp-source
    #[derive(Clone, Debug)]
    pub enum ImageSource {
        /// Reads the image from file.
        Read(PathBuf),
        /// Downloads the image using an HTTP GET request.
        Download(Uri),
        /// Uses the already created image.
        Image(Image),
    }
    impl_from_and_into_var! {
        fn from(image: Image) -> ImageSource {
            ImageSource::Image(image)
        }
        fn from(path: PathBuf) -> ImageSource {
            ImageSource::Read(path)
        }
        fn from(uri: Uri) -> ImageSource {
            ImageSource::Download(uri)
        }
        fn from(key: ImageCacheKey) -> ImageSource {
            match key {
                ImageCacheKey::Read(path) => ImageSource::Read(path),
                ImageCacheKey::Download(uri)  => ImageSource::Download(uri)
            }
        }
        fn from(s: &str) -> ImageSource {
            use crate::core::task::http::uri::*;
            if let Ok(uri) = Uri::try_from(s) {
                if let Some(scheme) = uri.scheme() {
                    if scheme == &Scheme::HTTPS || scheme == &Scheme::HTTP {
                        return ImageSource::Download(uri);
                    } else if scheme.as_str() == "file" {
                        return PathBuf::from(uri.path()).into();
                    }
                }
            }
            PathBuf::from(s).into()
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
            image: Option<ImageRequestVar>,
            final_size: LayoutSize,
        }
        #[impl_ui_node(none)]
        impl<T: Var<ImageSource>> UiNode for ImageNode<T> {
            fn init(&mut self, ctx: &mut WidgetContext) {
                let img = match self.source.get_clone(ctx) {
                    ImageSource::Read(path) => ctx.services.images().read(ctx.vars, path),
                    ImageSource::Download(uri) => ctx.services.images().download(ctx.vars, uri),
                    ImageSource::Image(img) => todo!(),
                };
                self.image = Some(img);
            }
            fn deinit(&mut self, _: &mut WidgetContext) {
                self.image = None;
            }

            fn update(&mut self, ctx: &mut WidgetContext) {
                if self.source.is_new(ctx) {
                    self.init(ctx);
                } else if self.image.as_ref().unwrap().is_new(ctx) {
                    ctx.updates.layout();
                }
            }

            fn measure(&mut self, ctx: &mut LayoutContext, _: LayoutSize) -> LayoutSize {
                if let Some(Ok(img)) = self.image.as_ref().unwrap().rsp(ctx) {
                    let (w, h) = img.size();
                    LayoutSize::new(w as f32, h as f32)
                } else {
                    LayoutSize::zero()
                }
            }

            fn arrange(&mut self, _: &mut LayoutContext, final_size: LayoutSize) {
                self.final_size = final_size;
            }
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                if let Some(Ok(img)) = self.image.as_ref().unwrap().rsp(ctx) {
                    frame.push_image(LayoutRect::from(self.final_size), img, ImageRendering::Pixelated);
                }
            }
        }
        ImageNode {
            source: source.into_var(),
            image: None,
            final_size: LayoutSize::zero(),
        }
    }

    /// Properties that configure [`image!`] widgets from parent widgets.
    ///
    /// Note that this properties are already available in the [`image!`] widget directly without the `image_` prefix.
    ///
    /// [`image!`]: mod@crate::widgets::image
    pub mod properties {
        use super::*;

        context_var! {
            /// The Image scaling algorithm in the renderer.
            ///
            /// Is [`ImageRendering::Auto`] by default.
            pub struct ImageRenderingVar: ImageRendering = const ImageRendering::Auto;
        }

        /// Sets the [`ImageRendering`] of all inner images.
        ///
        /// This property binds `rendering` to the [`ImageRenderingVar`] in the widget context.
        #[property(context)]
        pub fn image_rendering(child: impl UiNode, rendering: impl IntoVar<ImageRendering>) -> impl UiNode {
            with_context_var(child, ImageRenderingVar, rendering)
        }
    }
}

///
pub fn image(source: impl IntoVar<image::ImageSource>) -> impl Widget {
    image! { source }
}
