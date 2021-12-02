use zero_ui_core::image::ImageSource;

use crate::prelude::new_widget::*;

/// Image presenter.
///
/// This widget loads a still image from a variety of sources and presents it.
#[widget($crate::widgets::image)]
pub mod image {
    use zero_ui::core::image::{ImageCacheMode, ImageSource, ImageVar};

    use super::*;
    use properties::{ImageCacheVar, ImageErrorViewVar, ImageFit, ImageLoadingViewVar, ImageRenderingVar};

    properties! {
        /// The image source.
        ///
        /// Can be a file path, an URI, binary included in the app and more.
        source(impl IntoVar<ImageSource>);

        /// Sets the image final size mode.
        ///
        /// By default the [`Contain`] mode is used.
        ///
        /// [`Contain`]: ImageFit::Contain
        properties::image_fit as fit;

        /// Alignment of the image after the final size is calculated.
        ///
        /// If the image is smaller then the widget area it is aligned like normal, if it is larger the "viewport" is aligned,
        /// so for examples, alignment [`BOTTOM_RIGHT`] makes a smaller image sit at the bottom-right of the widget and makes
        /// a larger image bottom-right fill the widget, clipping the rest.
        ///
        /// By default the alignment is [`CENTER`].
        ///
        /// [`BOTTOM_RIGHT`]: Alignment::BOTTOM_RIGHT
        /// [`CENTER`]: Alignment::CENTER
        properties::image_align;

        /// Offset applied to the image after the final size and alignment.
        ///
        /// Relative values are calculated from the widget final size. Note that this is different the applying the
        /// [`offset`] property on the widget it-self, the widget is not moved just the image within the widget area.
        ///
        /// By default no offset is applied.
        ///
        /// [`offset`]: crate::properties::offset
        properties::image_offset;

        /// Simple clip rectangle applied to the image before all layout.
        ///
        /// Relative values are calculated from the image pixel size, the [`scale_ppi`] is only considered after.
        /// Note that more complex clipping can be applied after to the full widget, this property exists primarily to
        /// render selections of a [texture atlas].
        ///
        /// By default no cropping is done.
        ///
        /// [`scale_ppi`]: #wp-scale_ppi
        /// [texture atlas]: https://en.wikipedia.org/wiki/Texture_atlas
        properties::image_crop as crop;

        /// Scale applied to the image desired size.
        ///
        /// The scaling is applied after [`scale_ppi`] if active.
        ///
        /// By default not scaling is done.
        ///
        /// [`scale_ppi`]: #wp-scale_ppi
        properties::image_scale;

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

        /// If the image desired size is scaled by the screen scale factor.
        ///
        /// The image desired size is its original size after [`crop`], it is a pixel value, but widgets are layout using
        /// device independent pixels that automatically scale in higher definition displays, when this property is enabled
        /// the image size is also scaled so that the image will take the same screen space in all devices, the image can end
        ///
        /// This is enabled by default.
        ///
        /// [`crop`]: #wp-crop
        properties::image_scale_factor as scale_factor;

        /// If the image desired size is scaled by PPI.
        ///
        /// The image desired size is its original size, after [`crop`], and it can be in pixels or scaled considering
        /// the image PPi, monitor PPI and scale factor.
        ///
        /// By default this is `false`, if `true` the image is scaled in a attempt to recreate the original physical dimensions, but it
        /// only works if the image and monitor PPI are set correctly. The monitor PPI can be set using the [`Monitors`] service.
        ///
        /// [`crop`]: #wp-crop
        /// [`Monitors`]: zero_ui::core::window::Monitors
        properties::image_scale_ppi as scale_ppi;

        /// View generator that creates the loading content.
        properties::image_loading_view as loading_view;

        /// View generator that creates the error content when the image failed to load.
        properties::image_error_view as error_view;

        /// Sets custom image load and decode limits.
        ///
        /// If not set or set to `None` the [`Images::limits`] is used.
        properties::image_limits as limits;

        /// If the image successfully loaded.
        properties::is_loaded;

        /// Event called when the images successfully loads.
        properties::on_load;

        /// If the image failed to load.
        properties::is_error;

        /// Event called when the image fails to load.
        properties::on_error;
    }

    fn new_child() -> impl UiNode {
        let node = nodes::image_presenter();
        let node = nodes::image_error_presenter(node);
        nodes::image_loading_presenter(node)
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
        use std::fmt;

        pub use crate::core::image::ImageLimits;
        pub use crate::core::render::ImageRendering;
        use nodes::ContextImageVar;

        /// Image layout mode.
        ///
        /// This layout mode can be set to all images inside a widget using [`image_fit`], in the image widget
        /// it can be set using the [`fit`] property, the [`image_presenter`] uses this value to calculate the image final size.
        ///
        /// The image desired size is its original size, either in pixels or DIPs after cropping and scaling.
        ///
        /// [`fit`]: crate::widgets::image#wp-fit
        /// [`image_fit`]: fn@image_fit
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
            pub struct ImageRenderingVar: ImageRendering = ImageRendering::Auto;

            /// If the image is cached.
            ///
            /// Is `true` by default.
            pub struct ImageCacheVar: bool = true;

            /// View generator for the content shown when the image does not load.
            pub struct ImageErrorViewVar: ViewGenerator<ImageErrorArgs> = ViewGenerator::nil();

            /// View generator for the content shown when the image is still loading.
            pub struct ImageLoadingViewVar: ViewGenerator<ImageLoadingArgs> = ViewGenerator::nil();

            /// Custom image load and decode limits.
            ///
            /// Set to `None` to use the [`Images::limits`].
            pub struct ImageLimitsVar: Option<ImageLimits> = None;

            /// The image layout mode.
            ///
            /// Is [`ImageFit::Contain`] by default.
            pub struct ImageFitVar: ImageFit = ImageFit::Contain;

            /// Scaling applied to the image desired size.
            ///
            /// Does not scale by default, `1.0`.
            pub struct ImageScaleVar: Scale2d = Scale2d::identity();

            /// If the image desired size is scaled by the screen scale factor.
            ///
            /// Is `true` by default.
            pub struct ImageScaleFactorVar: bool = true;

            /// If the image desired size is scaled considering the image and screen PPIs.
            ///
            /// Is `false` by default.
            pub struct ImageScalePpiVar: bool = false;

            /// Alignment of the image in relation to the image widget final size.
            ///
            /// Is [`Alignment::CENTER`] by default.
            pub struct ImageAlignVar: Alignment = Alignment::CENTER;

            /// Offset applied to the image after all measure and arrange.
            pub struct ImageOffsetVar: Vector = Vector::default();

            /// Simple clip applied to the image before layout.
            ///
            /// No cropping is done by default.
            pub struct ImageCropVar: Rect = Rect::default();
        }

        /// Sets the [`ImageFit`] of all inner images.
        ///
        /// See the [`fit`] property in the widget for more details.
        ///
        /// [`fit`]: crate::widgets::image#wp-fit
        #[property(context, default(ImageFit::Contain))]
        pub fn image_fit(child: impl UiNode, fit: impl IntoVar<ImageFit>) -> impl UiNode {
            with_context_var(child, ImageFitVar, fit)
        }

        /// Sets the scale applied to all inner images.
        ///
        /// See the [`scale`] property in the widget for more details.
        ///
        /// [`fit`]: crate::widgets::image#wp-fit
        #[property(context, default(Scale2d::identity()))]
        pub fn image_scale(child: impl UiNode, scale: impl IntoVar<Scale2d>) -> impl UiNode {
            with_context_var(child, ImageScaleVar, scale)
        }

        /// Sets if the image desired size is scaled by the screen scale factor.
        #[property(context, default(true))]
        pub fn image_scale_factor(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
            with_context_var(child, ImageScaleFactorVar, enabled)
        }

        /// Sets if the image desired size is scaled considering the image and monitor PPI.
        ///
        /// See the [`scape_ppi`] property in the widget for more details.
        ///
        /// [`scape_ppi`]: crate::widgets::image#wp-scape_ppi
        #[property(context, default(false))]
        pub fn image_scale_ppi(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
            with_context_var(child, ImageScalePpiVar, enabled)
        }

        /// Sets the [`Alignment`] of all inner images within each image widget area.
        ///
        /// See the [`image_align`] property in the widget for more details.
        ///
        /// [`image_align`]: crate::widgets::image#wp-image_align
        #[property(context, default(Alignment::CENTER))]
        pub fn image_align(child: impl UiNode, fit: impl IntoVar<Alignment>) -> impl UiNode {
            with_context_var(child, ImageAlignVar, fit)
        }

        /// Sets a [`Point`] that is an offset applied to all inner images within each image widget area.
        ///
        /// See the [`image_offset`] property in the widget for more details.
        ///
        /// [`image_offset`]: crate::widgets::image#wp-image_offset
        #[property(context, default(Vector::default()))]
        pub fn image_offset(child: impl UiNode, offset: impl IntoVar<Vector>) -> impl UiNode {
            with_context_var(child, ImageOffsetVar, offset)
        }

        /// Sets a [`Rect`] that is a clip applied to all inner images before their layout.
        ///
        /// See the [`crop`] property in the widget for more details.
        ///
        /// [`crop`]: crate::widgets::image#wp-crop
        #[property(context, default(Rect::default()))]
        pub fn image_crop(child: impl UiNode, crop: impl IntoVar<Rect>) -> impl UiNode {
            with_context_var(child, ImageCropVar, crop)
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

        /// Sets custom image load and decode limits.
        ///
        /// If not set or set to `None` the [`Images::limits`] is used.
        #[property(context, default(None))]
        pub fn image_limits(child: impl UiNode, limits: impl IntoVar<Option<ImageLimits>>) -> impl UiNode {
            with_context_var(child, ImageLimitsVar, limits)
        }

        /// If the [`ContextImageVar`] is an error.
        #[property(outer)]
        pub fn is_error(child: impl UiNode, state: StateVar) -> impl UiNode {
            struct IsErrorNode<C> {
                child: C,
                state: StateVar,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode> UiNode for IsErrorNode<C> {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    if let Some(var) = ContextImageVar::get(ctx.vars).as_ref() {
                        let is_error = var.get(ctx.vars).is_error();
                        self.state.set_ne(ctx.vars, is_error);
                    } else {
                        self.state.set_ne(ctx.vars, false);
                    }
                    self.child.init(ctx);
                }

                fn update(&mut self, ctx: &mut WidgetContext) {
                    if let Some(new_var) = ContextImageVar::get_new(ctx.vars) {
                        let is_error = new_var.as_ref().map(|v| v.get(ctx.vars).is_error()).unwrap_or(false);
                        self.state.set_ne(ctx.vars, is_error);
                    } else if let Some(var) = ContextImageVar::get(ctx.vars).as_ref() {
                        if let Some(img) = var.get_new(ctx.vars) {
                            self.state.set_ne(ctx.vars, img.is_error());
                        }
                    }
                    self.child.update(ctx);
                }
            }
            IsErrorNode { child, state }
        }

        /// If the [`ContextImageVar`] is a successfully loaded image.
        #[property(outer)]
        pub fn is_loaded(child: impl UiNode, state: StateVar) -> impl UiNode {
            struct IsLoadedNode<C> {
                child: C,
                state: StateVar,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode> UiNode for IsLoadedNode<C> {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    if let Some(var) = ContextImageVar::get(ctx.vars).as_ref() {
                        let is_loaded = var.get(ctx.vars).is_loaded();
                        self.state.set_ne(ctx.vars, is_loaded);
                    } else {
                        self.state.set_ne(ctx.vars, false);
                    }
                    self.child.init(ctx);
                }

                fn update(&mut self, ctx: &mut WidgetContext) {
                    if let Some(new_var) = ContextImageVar::get_new(ctx.vars) {
                        let is_loaded = new_var.as_ref().map(|v| v.get(ctx.vars).is_loaded()).unwrap_or(false);
                        self.state.set_ne(ctx.vars, is_loaded);
                    } else if let Some(var) = ContextImageVar::get(ctx.vars).as_ref() {
                        if let Some(img) = var.get_new(ctx.vars) {
                            self.state.set_ne(ctx.vars, img.is_loaded());
                        }
                    }
                    self.child.update(ctx);
                }
            }
            IsLoadedNode { child, state }
        }

        /// Sets the [view generator] that is used to create a content for the error message.
        ///
        /// [view generator]: crate::widgets::view_generator
        #[property(context)]
        pub fn image_error_view(child: impl UiNode, generator: impl IntoVar<ViewGenerator<ImageErrorArgs>>) -> impl UiNode {
            with_context_var(child, ImageErrorViewVar, generator)
        }

        /// Sets the [view generator] that is used to create a content for the error message.
        ///
        /// [view generator]: crate::widgets::view_generator
        #[property(context)]
        pub fn image_loading_view(child: impl UiNode, generator: impl IntoVar<ViewGenerator<ImageLoadingArgs>>) -> impl UiNode {
            with_context_var(child, ImageLoadingViewVar, generator)
        }

        /// Arguments for [`image_loading_view`].
        ///
        /// [`image_loading_view`]: fn@image_loading_view
        #[derive(Clone, Debug)]
        pub struct ImageLoadingArgs {}

        /// Arguments for [`on_load`].
        ///
        /// [`on_load`]: fn@on_load
        #[derive(Clone, Debug)]
        pub struct ImageLoadArgs {}

        /// Arguments for [`on_error`] and [`image_error_view`].
        ///
        /// [`on_error`]: fn@on_error
        /// [`image_error_view`]: fn@image_error_view
        #[derive(Clone, Debug)]
        pub struct ImageErrorArgs {
            /// Error message.
            pub error: Text,
        }

        /// Image load or decode error event.
        ///
        /// This property calls `handler` every time the [`ContextImageVar`] updates with a different error.
        ///
        /// # Handlers
        ///
        /// This property accepts any [`WidgetHandler`], including the async handlers. Use one of the handler macros, [`hn!`],
        /// [`hn_once!`], [`async_hn!`] or [`async_hn_once!`], to declare a handler closure.
        ///
        /// # Route
        ///
        /// This property is not routed, it works only inside an widget that loads images. There is also no *preview* event.
        #[property(event, default( hn!(|_, _|{}) ))]
        pub fn on_error(child: impl UiNode, handler: impl WidgetHandler<ImageErrorArgs>) -> impl UiNode {
            struct OnErrorNode<C, H> {
                child: C,
                handler: H,
                error: Text,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode, H: WidgetHandler<ImageErrorArgs>> UiNode for OnErrorNode<C, H> {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    if let Some(var) = ContextImageVar::get(ctx.vars).as_ref() {
                        if let Some(error) = var.get(ctx.vars).error() {
                            self.error = error.to_owned().into();
                            self.handler.event(ctx, &ImageErrorArgs { error: self.error.clone() });
                        }
                    }
                    self.child.init(ctx);
                }

                fn update(&mut self, ctx: &mut WidgetContext) {
                    if let Some(new_var) = ContextImageVar::get_new(ctx.vars) {
                        if let Some(error) = new_var.as_ref().and_then(|v| v.get(ctx.vars).error()) {
                            if self.error != error {
                                self.error = error.to_owned().into();
                                self.handler.event(ctx, &ImageErrorArgs { error: self.error.clone() });
                            }
                        } else {
                            self.error = "".into();
                        }
                    } else if let Some(var) = ContextImageVar::get(ctx.vars).as_ref() {
                        if let Some(img) = var.get_new(ctx.vars) {
                            if let Some(error) = img.error() {
                                if self.error != error {
                                    self.error = error.to_owned().into();
                                    self.handler.event(ctx, &ImageErrorArgs { error: self.error.clone() });
                                }
                            } else {
                                self.error = "".into();
                            }
                        }
                    }

                    self.handler.update(ctx);
                    self.child.update(ctx);
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
        /// This property calls `handler` every time the [`ContextImageVar`] updates with a successfully loaded image.
        ///
        /// # Handlers
        ///
        /// This property accepts any [`WidgetHandler`], including the async handlers. Use one of the handler macros, [`hn!`],
        /// [`hn_once!`], [`async_hn!`] or [`async_hn_once!`], to declare a handler closure.
        ///
        /// # Route
        ///
        /// This property is not routed, it works only inside an widget that loads images. There is also no *preview* event.
        #[property(event, default( hn!(|_, _|{}) ))]
        pub fn on_load(child: impl UiNode, handler: impl WidgetHandler<ImageLoadArgs>) -> impl UiNode {
            struct OnLoadNode<C, H> {
                child: C,
                handler: H,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode, H: WidgetHandler<ImageLoadArgs>> UiNode for OnLoadNode<C, H> {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    if let Some(var) = ContextImageVar::get(ctx.vars).as_ref() {
                        if var.get(ctx.vars).is_loaded() {
                            self.handler.event(ctx, &ImageLoadArgs {});
                        }
                    }

                    self.child.init(ctx);
                }

                fn update(&mut self, ctx: &mut WidgetContext) {
                    if let Some(var_opt) = ContextImageVar::get_new(ctx.vars) {
                        if let Some(var) = var_opt.as_ref() {
                            if var.get(ctx.vars).is_loaded() {
                                self.handler.event(ctx, &ImageLoadArgs {});
                            }
                        }
                    } else if let Some(var) = ContextImageVar::get(ctx.vars).as_ref() {
                        if let Some(img) = var.get_new(ctx.vars) {
                            if img.is_loaded() {
                                self.handler.event(ctx, &ImageLoadArgs {});
                            }
                        }
                    }

                    self.handler.update(ctx);
                    self.child.update(ctx);
                }
            }
            OnLoadNode { child, handler }
        }
    }

    /// UI nodes used for building the image widget.
    pub mod nodes {
        use super::properties::{
            ImageAlignVar, ImageCropVar, ImageErrorArgs, ImageFitVar, ImageLimitsVar, ImageLoadingArgs, ImageOffsetVar,
            ImageScaleFactorVar, ImageScalePpiVar, ImageScaleVar,
        };
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
        /// [`image_cache`]: fn@crate::widgets::image::properties::image_cache
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
                    let limits = *ImageLimitsVar::get(ctx);
                    self.image = ContextImage::Some(ctx.services.images().get(self.source.get_clone(ctx.vars), mode, limits));
                    let child = &mut self.child;
                    ctx.vars.with_context_var(
                        ContextImageVar,
                        &self.image,
                        self.source.is_new(ctx.vars),
                        self.source.version(ctx.vars),
                        self.source.update_mask(ctx.vars),
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
                        self.source.update_mask(ctx.vars),
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
                        self.source.update_mask(ctx.vars),
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
                        let limits = *ImageLimitsVar::get(ctx);
                        self.image = ContextImage::Some(ctx.services.images().get(s, mode, limits));
                        force_new = true;
                    } else if let Some(enabled) = ImageCacheVar::clone_new(ctx) {
                        // cache-mode update:
                        let images = ctx.services.images();
                        let is_cached = images.is_cached(self.image.as_ref().unwrap().get(ctx.vars));
                        if enabled != is_cached {
                            force_new = true;

                            if is_cached {
                                // must not cache but is cached, detach from cache.

                                self.image = ContextImage::Some(images.detach(self.image.take().unwrap(), ctx.vars));
                            } else {
                                // must cache, but image is not cached, get source again.

                                let source = self.source.get_clone(ctx);
                                let limits = *ImageLimitsVar::get(ctx);
                                self.image = ContextImage::Some(ctx.services.images().get(source, ImageCacheMode::Cache, limits));
                            }
                        }
                    }

                    let child = &mut self.child;
                    ctx.vars.with_context_var(
                        ContextImageVar,
                        &self.image,
                        force_new || self.source.is_new(ctx.vars),
                        self.source.version(ctx.vars),
                        self.source.update_mask(ctx.vars),
                        || {
                            child.update(ctx);
                        },
                    );
                }

                fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                    let child = &mut self.child;
                    ctx.vars.with_context_var(
                        ContextImageVar,
                        &self.image,
                        self.source.is_new(ctx.vars),
                        self.source.version(ctx.vars),
                        self.source.update_mask(ctx.vars),
                        || child.measure(ctx, available_size),
                    )
                }

                fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
                    let child = &mut self.child;
                    ctx.vars.with_context_var(
                        ContextImageVar,
                        &self.image,
                        self.source.is_new(ctx.vars),
                        self.source.version(ctx.vars),
                        self.source.update_mask(ctx.vars),
                        || {
                            child.arrange(ctx, widget_offset, final_size);
                        },
                    );
                }

                fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                    ctx.vars.with_context_var(
                        ContextImageVar,
                        &self.image,
                        self.source.version(ctx.vars),
                        self.source.update_mask(ctx.vars),
                        || {
                            self.child.info(ctx, info);
                        },
                    );
                }

                fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                    ctx.vars.with_context_var(
                        ContextImageVar,
                        &self.image,
                        self.source.version(ctx.vars),
                        self.source.update_mask(ctx.vars),
                        || {
                            self.child.render(ctx, frame);
                        },
                    );
                }

                fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                    ctx.vars.with_context_var(
                        ContextImageVar,
                        &self.image,
                        self.source.version(ctx.vars),
                        self.source.update_mask(ctx.vars),
                        || {
                            self.child.render_update(ctx, update);
                        },
                    );
                }
            }
            ImageSourceNode {
                child,
                source: source.into_var(),
                image: ContextImage::None,
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

            z_stack(nodes![view, child])
        }

        /// Presents the contextual [`ImageLoadingViewVar`] if the [`ContextImageVar`] is loading.
        ///
        /// The loading view is rendered under the `child`.
        ///
        /// The image widget adds this node around the [`image_error_presenter`] node.
        pub fn image_loading_presenter(child: impl UiNode) -> impl UiNode {
            let view = ViewGenerator::presenter_map(
                ImageLoadingViewVar,
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

            z_stack(nodes![view, child])
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
                prev_img_size: PxSize,

                // raw size of the image the last time a full `measure` was done.
                measure_img_size: PxSize,
                // last computed clip-rect in the `measure` pass.
                measure_clip_rect: PxRect,
                // desired-size (pre-available) the last time a full `measure` was done.
                desired_size: PxSize,
                // `final_size` of the last processed `arrange`.
                prev_final_size: PxSize,

                render_clip_rect: PxRect,
                render_img_size: PxSize,
                render_offset: PxPoint,

                spatial_id: SpatialFrameId,
            }
            #[impl_ui_node(none)]
            impl UiNode for ImagePresenterNode {
                fn update(&mut self, ctx: &mut WidgetContext) {
                    if let Some(var) = ContextImageVar::get_new(ctx.vars) {
                        ctx.updates.layout_and_render();
                        self.requested_layout = true;

                        if let Some(var) = var.as_ref() {
                            self.prev_img_size = var.get(ctx).size();
                        } else {
                            self.prev_img_size = PxSize::zero();
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
                            if self.prev_img_size != img_size {
                                self.prev_img_size = img_size;
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
                        let img_rect = PxRect::from_size(self.prev_img_size);

                        let crop = ImageCropVar::get(ctx).to_layout(ctx, AvailableSize::from_size(self.prev_img_size), img_rect);

                        self.measure_img_size = self.prev_img_size;
                        self.measure_clip_rect = img_rect.intersection(&crop).unwrap_or_default();

                        let mut scale = *ImageScaleVar::get(ctx);
                        if *ImageScalePpiVar::get(ctx) {
                            let img = var.get(ctx.vars);
                            let sppi = ctx.metrics.screen_ppi;
                            let (ippi_x, ippi_y) = img.ppi().unwrap_or((sppi, sppi));
                            scale *= Scale2d::new(ippi_x / sppi, ippi_y / sppi);
                        }

                        if *ImageScaleFactorVar::get(ctx) {
                            scale *= ctx.scale_factor;
                        }
                        self.measure_img_size *= scale;
                        self.measure_clip_rect *= scale;

                        self.requested_layout |= self.measure_clip_rect.size != self.desired_size;
                        self.desired_size = self.measure_clip_rect.size;

                        available_size.clip(self.desired_size)
                    } else {
                        // no context image
                        PxSize::zero()
                    }
                }

                fn arrange(&mut self, ctx: &mut LayoutContext, _: &mut WidgetOffset, final_size: PxSize) {
                    self.requested_layout |= final_size != self.prev_final_size;

                    if !self.requested_layout {
                        return;
                    }

                    self.prev_final_size = final_size;
                    self.requested_layout = false;

                    let mut f_img_size = self.measure_img_size;
                    let mut f_clip_rect = self.measure_clip_rect;
                    let f_offset;

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
                    let scale = Scale2d::new(scale_x, scale_y);

                    f_img_size *= scale;
                    f_clip_rect.origin *= scale;
                    f_clip_rect.size = crop_size;

                    // 3 - offset to align + user image_offset:
                    let mut offset = PxPoint::zero();
                    offset += align_offset;
                    offset += ImageOffsetVar::get(ctx.vars).to_layout(ctx, AvailableSize::from_size(final_size), PxVector::zero());

                    // 4 - adjust clip_rect to clip content to container final_size:
                    let top_left_clip = -offset.to_vector().min(PxVector::zero());
                    f_clip_rect.origin += top_left_clip;
                    f_clip_rect.size -= top_left_clip.to_size();
                    offset += top_left_clip;
                    // bottom-right clip
                    f_clip_rect.size = f_clip_rect.size.min(final_size - offset.to_vector().to_size());

                    // 5 - adjust offset so that clip_rect.origin is at widget (0, 0):
                    f_offset = offset;
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
                        if img.is_loaded() && !self.prev_img_size.is_empty() && !self.render_clip_rect.is_empty() {
                            if self.render_offset != PxPoint::zero() {
                                frame.push_reference_frame(self.spatial_id, self.render_offset, |frame| {
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

                prev_img_size: PxSize::zero(),

                measure_clip_rect: PxRect::zero(),
                measure_img_size: PxSize::zero(),
                desired_size: PxSize::zero(),

                prev_final_size: PxSize::zero(),

                render_clip_rect: PxRect::zero(),
                render_img_size: PxSize::zero(),
                render_offset: PxPoint::zero(),

                spatial_id: SpatialFrameId::new_unique(),
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

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn error_view_recursion() {
        let mut app = App::default().run_headless(false);
        app.ctx().services.images().load_in_headless = true;
        let ok = Rc::new(Cell::new(false));
        app.open_window(clone_move!(ok, |_| {
            window! {
                content = image! {
                    source = "";
                    error_view = view_generator!(ok, |_, _| {
                        ok.set(true);
                        image! {
                            source = "";
                        }
                    });
                }
            }
        }));

        let _ = app.update(false);

        assert!(ok.get());
    }
}
