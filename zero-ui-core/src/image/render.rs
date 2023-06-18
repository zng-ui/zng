use crate::{
    context::{StaticStateId, UPDATES, WIDGET, WINDOW},
    event::{AnyEventArgs, EventUpdate},
    property,
    render::RenderMode,
    units::*,
    var::{types::WeakArcVar, *},
    widget_instance::{match_node, UiNode, UiNodeOp, WidgetId},
    window::*,
};

use super::{ImageManager, ImageVar, ImagesService, Img, IMAGES, IMAGES_SV};

impl ImagesService {
    fn render<N>(&mut self, render: N) -> ImageVar
    where
        N: FnOnce() -> WindowRoot + Send + Sync + 'static,
    {
        let result = var(Img::new_none(None));
        self.render_img(
            move || {
                let r = render();
                WINDOW_CTRL.vars().frame_capture_mode().set_ne(FrameCaptureMode::All);
                r
            },
            &result,
        );
        result.read_only()
    }

    fn render_node<U, N>(&mut self, render_mode: RenderMode, scale_factor: impl Into<Factor>, render: N) -> ImageVar
    where
        U: UiNode,
        N: FnOnce() -> U + Send + Sync + 'static,
    {
        let scale_factor = scale_factor.into();
        self.render(move || {
            let node = render();
            WindowRoot::new_container(
                WidgetId::new_unique(),
                StartPosition::Default,
                false,
                true,
                Some(render_mode),
                HeadlessMonitor::new_scale(scale_factor),
                false,
                node,
            )
        })
    }

    pub(super) fn render_img<N>(&mut self, render: N, result: &ArcVar<Img>)
    where
        N: FnOnce() -> WindowRoot + Send + Sync + 'static,
    {
        self.render.requests.push(RenderRequest {
            render: Box::new(render),
            image: result.downgrade(),
        });
        UPDATES.update(None);
    }
}

impl IMAGES {
    /// Render the *window* to an image.
    ///
    /// The *window* is created as a headless surface and rendered to the returned image. You can use the
    /// [`IMAGE_RENDER.retain`] var create an image that updates with new frames, or by default only render once.
    ///
    /// The closure runs in the [`WINDOW`] context of the headless window.
    ///
    /// Requires the [`WINDOWS`] service.
    ///
    /// [`IMAGE_RENDER.retain`]: IMAGE_RENDER::retain
    pub fn render<N>(&self, render: N) -> ImageVar
    where
        N: FnOnce() -> WindowRoot + Send + Sync + 'static,
    {
        IMAGES_SV.write().render(render)
    }

    /// Render an [`UiNode`] to an image.
    ///
    /// This method is a shortcut to [`render`] a node without needing to declare the headless window, note that
    /// a headless window is still used, the node does not have the same context as the calling widget.
    ///
    /// [`render`]: Self::render
    pub fn render_node<U, N>(&self, render_mode: RenderMode, scale_factor: impl Into<Factor>, render: N) -> ImageVar
    where
        U: UiNode,
        N: FnOnce() -> U + Send + Sync + 'static,
    {
        IMAGES_SV.write().render_node(render_mode, scale_factor, render)
    }
}

impl ImageManager {
    /// AppExtension::update
    pub(super) fn update_render(&mut self) {
        let mut images = IMAGES_SV.write();

        images.render.active.retain(|r| {
            let mut retain = false;

            if let Some(img) = r.image.upgrade() {
                if img.with(Img::is_loading) {
                    retain = true;
                }
            }

            retain |= r.retain.get();

            if !retain {
                let _ = WINDOWS.close(r.window_id);
            }

            retain
        });

        for req in images.render.requests.drain(..) {
            if let Some(img) = req.image.upgrade() {
                WINDOWS.open_headless(
                    async move {
                        let ctx = ImageRenderCtx::new();
                        let retain = ctx.retain.clone();
                        WINDOW.set_state(&IMAGE_RENDER_ID, ctx);
                        let vars = WINDOW_CTRL.vars();
                        vars.auto_size().set(true);
                        vars.min_size().set((1.px(), 1.px()));

                        let w = (req.render)();

                        vars.frame_capture_mode().set(FrameCaptureMode::All);

                        let a = ActiveRenderer {
                            window_id: WINDOW.id(),
                            image: img.downgrade(),
                            retain,
                        };
                        IMAGES_SV.write().render.active.push(a);

                        w
                    },
                    true,
                );
            }
        }
    }

    /// AppExtension::event_preview
    pub(super) fn event_preview_render(&mut self, update: &EventUpdate) {
        if let Some(args) = FRAME_IMAGE_READY_EVENT.on(update) {
            if let Some(img) = &args.frame_image {
                let imgs = IMAGES_SV.read();
                if let Some(a) = imgs.render.active.iter().find(|a| a.window_id == args.window_id) {
                    if let Some(img_var) = a.image.upgrade() {
                        img_var.set(img.clone());
                    }
                    args.propagation().stop();
                }
            }
        }
    }
}

/// Fields for [`Images`] related to the render operation.
#[derive(Default)]
pub(super) struct ImagesRender {
    requests: Vec<RenderRequest>,
    active: Vec<ActiveRenderer>,
}

struct ActiveRenderer {
    window_id: WindowId,
    image: WeakArcVar<Img>,
    retain: ArcVar<bool>,
}

struct RenderRequest {
    render: Box<dyn FnOnce() -> WindowRoot + Send + Sync>,
    image: WeakArcVar<Img>,
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

static IMAGE_RENDER_ID: StaticStateId<ImageRenderCtx> = StaticStateId::new_unique();

/// Controls properties of the render window used by [`IMAGES.render`].
#[allow(non_camel_case_types)]
pub struct IMAGE_RENDER;
impl IMAGE_RENDER {
    /// If the current context is an [`IMAGES.render`] closure, window or widget.
    pub fn is_in_render(&self) -> bool {
        WINDOW.contains_state(&IMAGE_RENDER_ID)
    }

    /// If the render task is kept alive after a frame is produced, this is `false` by default
    /// meaning the image only renders once, if set to `true` the image will automatically update
    /// when the render widget requests a new frame.
    pub fn retain(&self) -> ArcVar<bool> {
        WINDOW.req_state(&IMAGE_RENDER_ID).retain
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
                actual_retain.set_from_ne(&retain);
                let handle = actual_retain.bind(&retain);
                WIDGET.push_var_handle(handle);
            } else {
                tracing::error!("can only set `render_retain` in render widgets")
            }
        }
    })
}
