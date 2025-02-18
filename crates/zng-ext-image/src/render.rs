use std::{any::Any, sync::Arc};

use zng_app::{
    update::{EventUpdate, UPDATES},
    widget::{
        node::{match_node, BoxedUiNode, UiNode, UiNodeOp},
        property, WIDGET,
    },
    window::{WindowId, WINDOW},
};
use zng_layout::unit::Factor;
use zng_state_map::{static_id, StateId};
use zng_var::{types::WeakArcVar, var, ArcVar, IntoVar, Var, WeakVar};
use zng_view_api::{image::ImageMaskMode, window::RenderMode};

use crate::{ImageManager, ImageRenderArgs, ImageSource, ImageVar, ImagesService, Img, IMAGES, IMAGES_SV};

impl ImagesService {
    fn render<N, R>(&mut self, mask: Option<ImageMaskMode>, render: N) -> ImageVar
    where
        N: FnOnce() -> R + Send + Sync + 'static,
        R: ImageRenderWindowRoot,
    {
        let result = var(Img::new_none(None));
        let windows = self.render.windows();
        self.render_img(
            mask,
            move || {
                let r = render();
                windows.enable_frame_capture_in_window_context(None);
                Box::new(r)
            },
            &result,
        );
        result.read_only()
    }

    fn render_node<U, N>(&mut self, render_mode: RenderMode, scale_factor: Factor, mask: Option<ImageMaskMode>, render: N) -> ImageVar
    where
        U: UiNode,
        N: FnOnce() -> U + Send + Sync + 'static,
    {
        let scale_factor = scale_factor.into();
        let result = var(Img::new_none(None));
        let windows = self.render.windows();
        self.render_img(
            mask,
            move || {
                let node = render();
                let r = windows.new_window_root(node.boxed(), render_mode, scale_factor);
                windows.enable_frame_capture_in_window_context(mask);
                r
            },
            &result,
        );
        result.read_only()
    }

    pub(super) fn render_img<N>(&mut self, mask: Option<ImageMaskMode>, render: N, result: &ArcVar<Img>)
    where
        N: FnOnce() -> Box<dyn ImageRenderWindowRoot> + Send + Sync + 'static,
    {
        self.render.requests.push(RenderRequest {
            render: Box::new(render),
            image: result.downgrade(),
            mask,
        });
        UPDATES.update(None);
    }
}

impl ImageSource {
    /// New image from a function that generates a headless window.
    ///
    /// The function is called every time the image source is resolved and it is not found in the cache.
    ///
    /// # Examples
    ///
    /// ```
    /// # use zng_ext_image::*;
    /// # use zng_color::colors;
    /// # use std::any::Any;
    /// # struct WindowRoot;
    /// # impl ImageRenderWindowRoot for WindowRoot { fn into_any(self: Box<Self>) -> Box<dyn Any> { self } }
    /// # macro_rules! Window { ($($property:ident = $value:expr;)+) => { {$(let _ = $value;)+ WindowRoot } } }
    /// # macro_rules! Text { ($($tt:tt)*) => { () } }
    /// # fn main() { }
    /// # fn demo() {
    /// # let _ = ImageSource::render(
    ///     |args| Window! {
    ///         size = (500, 400);
    ///         parent = args.parent;
    ///         background_color = colors::GREEN;
    ///         child = Text!("Rendered!");
    ///     }
    /// )
    /// # ; }
    /// ```
    ///
    pub fn render<F, R>(new_img: F) -> Self
    where
        F: Fn(&ImageRenderArgs) -> R + Send + Sync + 'static,
        R: ImageRenderWindowRoot,
    {
        let window = IMAGES_SV.read().render.windows();
        Self::Render(
            Arc::new(Box::new(move |args| {
                if let Some(parent) = args.parent {
                    window.set_parent_in_window_context(parent);
                }
                let r = new_img(args);
                window.enable_frame_capture_in_window_context(None);
                Box::new(r)
            })),
            None,
        )
    }

    /// New image from a function that generates a new [`UiNode`].
    ///
    /// The function is called every time the image source is resolved and it is not found in the cache.
    ///
    /// Note that the generated [`UiNode`] is not a child of the widget that renders the image, it is the root widget of a headless
    /// surface, not a part of the context where it is rendered. See [`IMAGES.render`] for more information.
    ///
    /// # Examples
    ///
    /// ```
    /// # use zng_ext_image::*;
    /// # use zng_view_api::window::RenderMode;
    /// # use std::any::Any;
    /// # struct WindowRoot;
    /// # impl ImageRenderWindowRoot for WindowRoot { fn into_any(self: Box<Self>) -> Box<dyn Any> { self } }
    /// # macro_rules! Container { ($($tt:tt)*) => { zng_app::widget::node::FillUiNode } }
    /// # fn main() { }
    /// # fn demo() {
    /// # let _ =
    /// ImageSource::render_node(
    ///     RenderMode::Software,
    ///     |_args| Container! {
    ///         size = (500, 400);
    ///         background_color = colors::GREEN;
    ///         child = Text!("Rendered!");
    ///     }
    /// )
    /// # ; }
    /// ```
    ///
    /// [`IMAGES.render`]: crate::IMAGES::render
    /// [`UiNode`]: zng_app::widget::node::UiNode
    pub fn render_node<U, N>(render_mode: RenderMode, render: N) -> Self
    where
        U: UiNode,
        N: Fn(&ImageRenderArgs) -> U + Send + Sync + 'static,
    {
        let window = IMAGES_SV.read().render.windows();
        Self::Render(
            Arc::new(Box::new(move |args| {
                if let Some(parent) = args.parent {
                    window.set_parent_in_window_context(parent);
                }
                let node = render(args);
                window.enable_frame_capture_in_window_context(None);
                window.new_window_root(node.boxed(), render_mode, None)
            })),
            None,
        )
    }
}

impl IMAGES {
    /// Render the *window* generated by `render` to an image.
    ///
    /// The *window* is created as a headless surface and rendered to the returned image. You can use the
    /// [`IMAGE_RENDER.retain`] var create an image that updates with new frames, or by default only render once.
    ///
    /// The closure runs in the [`WINDOW`] context of the headless window.
    ///
    /// [`IMAGE_RENDER.retain`]: IMAGE_RENDER::retain
    /// [`WINDOW`]: zng_app::window::WINDOW
    pub fn render<N, R>(&self, mask: Option<ImageMaskMode>, render: N) -> ImageVar
    where
        N: FnOnce() -> R + Send + Sync + 'static,
        R: ImageRenderWindowRoot,
    {
        IMAGES_SV.write().render(mask, render)
    }

    /// Render an [`UiNode`] to an image.
    ///
    /// This method is a shortcut to [`render`] a node without needing to declare the headless window, note that
    /// a headless window is still used, the node does not have the same context as the calling widget.
    ///
    /// [`render`]: Self::render
    /// [`UiNode`]: zng_app::widget::node::UiNode
    pub fn render_node<U, N>(
        &self,
        render_mode: RenderMode,
        scale_factor: impl Into<Factor>,
        mask: Option<ImageMaskMode>,
        render: N,
    ) -> ImageVar
    where
        U: UiNode,
        N: FnOnce() -> U + Send + Sync + 'static,
    {
        IMAGES_SV.write().render_node(render_mode, scale_factor.into(), mask, render)
    }
}

/// Images render window hook.
#[expect(non_camel_case_types)]
pub struct IMAGES_WINDOW;
impl IMAGES_WINDOW {
    /// Sets the windows service used to manage the headless windows used to render images.
    ///
    /// This must be called by the windows implementation only.
    pub fn hook_render_windows_service(&self, service: Box<dyn ImageRenderWindowsService>) {
        let mut img = IMAGES_SV.write();
        assert!(img.render.windows.is_none());
        img.render.windows = Some(service);
    }
}

impl ImageManager {
    /// AppExtension::update
    pub(super) fn update_render(&mut self) {
        let mut images = IMAGES_SV.write();

        if !images.render.active.is_empty() {
            let windows = images.render.windows();

            images.render.active.retain(|r| {
                let mut retain = false;
                if let Some(img) = r.image.upgrade() {
                    retain = img.with(Img::is_loading) || r.retain.get();
                }

                if !retain {
                    windows.close_window(r.window_id);
                }

                retain
            });
        }

        if !images.render.requests.is_empty() {
            let windows = images.render.windows();

            for req in images.render.requests.drain(..) {
                if let Some(img) = req.image.upgrade() {
                    let windows_in = windows.clone_boxed();
                    windows.open_headless_window(Box::new(move || {
                        let ctx = ImageRenderCtx::new();
                        let retain = ctx.retain.clone();
                        WINDOW.set_state(*IMAGE_RENDER_ID, ctx);

                        let w = (req.render)();

                        windows_in.enable_frame_capture_in_window_context(req.mask);

                        let a = ActiveRenderer {
                            window_id: WINDOW.id(),
                            image: img.downgrade(),
                            retain,
                        };
                        IMAGES_SV.write().render.active.push(a);

                        w
                    }));
                }
            }
        }
    }

    /// AppExtension::event_preview
    pub(super) fn event_preview_render(&mut self, update: &EventUpdate) {
        let imgs = IMAGES_SV.read();
        if !imgs.render.active.is_empty() {
            if let Some((id, img)) = imgs.render.windows().on_frame_image_ready(update) {
                if let Some(a) = imgs.render.active.iter().find(|a| a.window_id == id) {
                    if let Some(img_var) = a.image.upgrade() {
                        img_var.set(img.clone());
                    }
                }
            }
        }
    }
}

/// Fields for [`Images`] related to the render operation.
#[derive(Default)]
pub(super) struct ImagesRender {
    windows: Option<Box<dyn ImageRenderWindowsService>>,
    requests: Vec<RenderRequest>,
    active: Vec<ActiveRenderer>,
}
impl ImagesRender {
    fn windows(&self) -> Box<dyn ImageRenderWindowsService> {
        self.windows.as_ref().expect("render windows service not inited").clone_boxed()
    }
}

struct ActiveRenderer {
    window_id: WindowId,
    image: WeakArcVar<Img>,
    retain: ArcVar<bool>,
}

struct RenderRequest {
    render: Box<dyn FnOnce() -> Box<dyn ImageRenderWindowRoot> + Send + Sync>,
    image: WeakArcVar<Img>,
    mask: Option<ImageMaskMode>,
}

#[derive(Clone)]
struct ImageRenderCtx {
    retain: ArcVar<bool>,
}
impl ImageRenderCtx {
    fn new() -> Self {
        Self { retain: var(false) }
    }
}

static_id! {
    static ref IMAGE_RENDER_ID: StateId<ImageRenderCtx>;
}

/// Controls properties of the render window used by [`IMAGES.render`].
///
/// [`IMAGES.render`]: IMAGES::render
#[expect(non_camel_case_types)]
pub struct IMAGE_RENDER;
impl IMAGE_RENDER {
    /// If the current context is an [`IMAGES.render`] closure, window or widget.
    ///
    /// [`IMAGES.render`]: IMAGES::render
    pub fn is_in_render(&self) -> bool {
        WINDOW.contains_state(*IMAGE_RENDER_ID)
    }

    /// If the render task is kept alive after a frame is produced, this is `false` by default
    /// meaning the image only renders once, if set to `true` the image will automatically update
    /// when the render widget requests a new frame.
    pub fn retain(&self) -> ArcVar<bool> {
        WINDOW.req_state(*IMAGE_RENDER_ID).retain
    }
}

/// If the render task is kept alive after a frame is produced, this is `false` by default
/// meaning the image only renders once, if set to `true` the image will automatically update
/// when the render widget requests a new frame.
///
/// This property sets and binds `retain` to [`IMAGE_RENDER.retain`].
///
/// [`IMAGE_RENDER.retain`]: IMAGE_RENDER::retain
#[property(CONTEXT, default(false))]
pub fn render_retain(child: impl UiNode, retain: impl IntoVar<bool>) -> impl UiNode {
    let retain = retain.into_var();
    match_node(child, move |_, op| {
        if let UiNodeOp::Init = op {
            if IMAGE_RENDER.is_in_render() {
                let actual_retain = IMAGE_RENDER.retain();
                actual_retain.set_from(&retain);
                let handle = actual_retain.bind(&retain);
                WIDGET.push_var_handle(handle);
            } else {
                tracing::error!("can only set `render_retain` in render widgets")
            }
        }
    })
}

/// Reference to a windows manager service that [`IMAGES`] can use to render images.
///
/// This service must be implemented by the window implementer, the `WINDOWS` service implements it.
pub trait ImageRenderWindowsService: Send + Sync + 'static {
    /// Clone the service reference.
    fn clone_boxed(&self) -> Box<dyn ImageRenderWindowsService>;

    /// Create a window root that presents the node.
    fn new_window_root(&self, node: BoxedUiNode, render_mode: RenderMode, scale_factor: Option<Factor>) -> Box<dyn ImageRenderWindowRoot>;

    /// Set parent window for the headless render window.
    fn set_parent_in_window_context(&self, parent_id: WindowId);

    /// Enable frame capture for the window.
    ///
    /// If `mask` is set captures only the given channel, if not set will capture the full BGRA image.
    ///
    /// Called inside the [`WINDOW`] context for the new window.
    ///
    /// [`WINDOW`]: zng_app::window::WINDOW
    fn enable_frame_capture_in_window_context(&self, mask: Option<ImageMaskMode>);

    /// Open the window.
    ///
    /// The `new_window_root` is called inside the [`WINDOW`] context for the new window.
    ///
    /// [`WINDOW`]: zng_app::window::WINDOW
    fn open_headless_window(&self, new_window_root: Box<dyn FnOnce() -> Box<dyn ImageRenderWindowRoot> + Send>);

    /// Returns the rendered frame image if it is ready for reading.
    fn on_frame_image_ready(&self, update: &EventUpdate) -> Option<(WindowId, Img)>;

    /// Close the window, does nothing if the window is not found.
    fn close_window(&self, window_id: WindowId);
}

/// Implemented for the root window type.
///
/// This is implemented for the `WindowRoot` type.
pub trait ImageRenderWindowRoot: Send + Any + 'static {
    /// Cast to `Box<dyn Any>`.
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}
