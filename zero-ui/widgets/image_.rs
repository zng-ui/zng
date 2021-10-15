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
        /// The image source.
        ///
        /// Can be a file path, an URI, binary included in the app and more.
        source(impl IntoVar<ImageSource>);

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

    fn new_child() -> impl UiNode {
        nodes::image_presenter()
    }

    fn new_event(child: impl UiNode, source: impl IntoVar<ImageSource>) -> impl UiNode {
        nodes::image_source(child, source)
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

    /// UI nodes used for building the image widget.
    pub mod nodes {
        use super::*;
        use std::fmt;

        context_var! {
            /// Image acquired by [`image_source`], or `Unset` by default.
            pub struct ContextImageVar: ContextImage = return &ContextImage::None;
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
        impl ContextImage {
            /// Like `Option::as_ref`.
            pub fn as_ref(&self) -> Option<&ImageVar> {
                match self {
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
        pub fn image_source(child: impl UiNode, source: impl IntoVar<ImageSource>) -> impl UiNode {
            struct ImageSourceNode<C, S> {
                child: C,
                source: S,
                image: ContextImage,
            }
            impl<C: UiNode, S: Var<ImageSource>> UiNode for ImageSourceNode<C, S> {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    let mode = if *ImageCacheVar::get(ctx) {
                        ImageCacheMode::Cache
                    } else {
                        ImageCacheMode::Ignore
                    };
                    self.image = ContextImage::Some(ctx.services.images().get(self.source.get_clone(ctx.vars), mode));
                    let child = &mut self.child;
                    ctx.vars.with_context_var(
                        ContextImageVar,
                        &self.image,
                        self.source.is_new(ctx.vars),
                        self.source.version(ctx.vars),
                        || {
                            child.init(ctx);
                        },
                    );
                }

                fn deinit(&mut self, ctx: &mut WidgetContext) {
                    let child = &mut self.child;
                    ctx.vars.with_context_var(
                        ContextImageVar,
                        &self.image,
                        self.source.is_new(ctx.vars),
                        self.source.version(ctx.vars),
                        || {
                            child.deinit(ctx);
                        },
                    );
                    self.image = ContextImage::None;
                }

                fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                    let child = &mut self.child;
                    ctx.vars.with_context_var(
                        ContextImageVar,
                        &self.image,
                        self.source.is_new(ctx.vars),
                        self.source.version(ctx.vars),
                        || {
                            child.event(ctx, args);
                        },
                    );
                }

                fn update(&mut self, ctx: &mut WidgetContext) {
                    let mut force_new = false;
                    if let Some(s) = self.source.clone_new(ctx) {
                        // source update:
                        let mode = if *ImageCacheVar::get(ctx) {
                            ImageCacheMode::Cache
                        } else {
                            ImageCacheMode::Ignore
                        };
                        self.image = ContextImage::Some(ctx.services.images().get(s, mode));
                        force_new = true;
                    } else if let Some(enabled) = ImageCacheVar::clone_new(ctx) {
                        // cache-mode update:
                        if enabled {
                            if !self.image.as_ref().unwrap().get(ctx.vars).is_cached() {
                                // must cache, but image is not cached, get source again.

                                let source = self.source.get_clone(ctx);
                                self.image = ContextImage::Some(ctx.services.images().get(source, ImageCacheMode::Cache));

                                force_new = true;
                            }
                        } else if let Some(key) = self.image.as_ref().unwrap().get(ctx.vars).cache_key().cloned() {
                            // cannot cache, but image is cached, detach from cache.

                            let source = self.source.get_clone(ctx);
                            let images = ctx.services.images();
                            self.image = ContextImage::Some(images.get(source, ImageCacheMode::Ignore));
                            images.clean(key);

                            force_new = true;
                        }
                    }

                    let child = &mut self.child;
                    ctx.vars.with_context_var(
                        ContextImageVar,
                        &self.image,
                        force_new || self.source.is_new(ctx.vars),
                        self.source.version(ctx.vars),
                        || {
                            child.update(ctx);
                        },
                    );
                }

                fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                    let child = &mut self.child;
                    ctx.vars
                        .with_context_var(ContextImageVar, &self.image, self.source.version(ctx.vars), || {
                            child.measure(ctx, available_size)
                        })
                }

                fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
                    let child = &mut self.child;
                    ctx.vars
                        .with_context_var(ContextImageVar, &self.image, self.source.version(ctx.vars), || {
                            child.arrange(ctx, final_size);
                        });
                }

                fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                    ctx.vars
                        .with_context_var(ContextImageVar, &self.image, self.source.version(ctx.vars), || {
                            self.child.render(ctx, frame);
                        });
                }

                fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                    ctx.vars
                        .with_context_var(ContextImageVar, &self.image, self.source.version(ctx.vars), || {
                            self.child.render_update(ctx, update);
                        });
                }
            }
            ImageSourceNode {
                child,
                source: source.into_var(),
                image: ContextImage::None,
            }
        }

        /// Renders the [`ContextImageVar`] if set.
        ///
        /// This is the inner-most node of an image widget. It is configured by the [`ImageRenderingVar`].
        pub fn image_presenter() -> impl UiNode {
            struct ImagePresenterNode {
                measured_image_size: PxSize,
                final_size: PxSize,
            }
            #[impl_ui_node(none)]
            impl UiNode for ImagePresenterNode {
                fn update(&mut self, ctx: &mut WidgetContext) {
                    if ContextImageVar::is_new(ctx) {
                        ctx.updates.layout();
                    } else if let Some(var) = ContextImageVar::get(ctx.vars).as_ref() {
                        if let Some(img) = var.get_new(ctx.vars) {
                            if self.measured_image_size == img.size() {
                                ctx.updates.render();
                            } else {
                                ctx.updates.layout();
                            }
                        }
                    }
                }

                fn measure(&mut self, ctx: &mut LayoutContext, _: AvailableSize) -> PxSize {
                    if let Some(var) = ContextImageVar::get(ctx.vars).as_ref() {
                        let img = var.get(ctx.vars);
                        self.measured_image_size = img.size();
                        img.layout_size(ctx)
                    } else {
                        PxSize::zero()
                    }
                }

                fn arrange(&mut self, _: &mut LayoutContext, final_size: PxSize) {
                    self.final_size = final_size;
                }

                fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                    if let Some(var) = ContextImageVar::get(ctx.vars).as_ref() {
                        let img = var.get(ctx.vars);
                        if img.is_loaded() {
                            frame.push_image(PxRect::from(self.final_size), img, *ImageRenderingVar::get(ctx.vars));
                        }
                    }
                }
            }
            ImagePresenterNode {
                measured_image_size: PxSize::zero(),
                final_size: PxSize::zero(),
            }
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
