use crate::{
    context::{StaticStateId, UPDATES, WIDGET, WINDOW},
    event::{AnyEventArgs, EventUpdate},
    property,
    render::RenderMode,
    ui_node,
    units::*,
    var::{types::WeakArcVar, *},
    widget_instance::{UiNode, WidgetId},
    window::*,
};

use super::{Image, ImageManager, ImageVar, ImagesService, IMAGES, IMAGES_SV};

impl ImagesService {
    fn render<N>(&mut self, render: N) -> ImageVar
    where
        N: FnOnce() -> Window + Send + Sync + 'static,
    {
        let result = var(Image::new_none(None));
        self.render_img(
            move || {
                let r = render();
                WindowVars::req().frame_capture_mode().set_ne(FrameCaptureMode::All);
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
            Window::new_container(
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

    pub(super) fn render_img<N>(&mut self, render: N, result: &ArcVar<Image>)
    where
        N: FnOnce() -> Window + Send + Sync + 'static,
    {
        self.render.requests.push(RenderRequest {
            render: Box::new(render),
            image: result.downgrade(),
        });
        UPDATES.update_ext();
    }
}

impl IMAGES {
    /// Render the *window* to an image.
    ///
    /// The *window* is created as a headless surface and rendered to the returned image. You can use the
    /// [`ImageRenderVars::retain`] var create an image that updates with new frames, or by default only render once.
    ///
    /// The closure input is the [`WindowContext`] of the headless window.
    ///
    /// Requires the [`WINDOWS`] service.
    pub fn render<N>(&self, render: N) -> ImageVar
    where
        N: FnOnce() -> Window + Send + Sync + 'static,
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
                if img.with(Image::is_loading) {
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
                    move || {
                        let vars = ImageRenderVars::new();
                        let retain = vars.retain.clone();
                        WINDOW.with_state_mut(|mut map| map.set(&IMAGE_RENDER_VARS_ID, vars));
                        let vars = WindowVars::req();
                        vars.auto_size().set(true);
                        vars.min_size().set((1.px(), 1.px()));

                        let w = (req.render)();

                        let vars = WindowVars::req();
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
    pub(super) fn event_preview_render(&mut self, update: &mut EventUpdate) {
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
    image: WeakArcVar<Image>,
    retain: ArcVar<bool>,
}

struct RenderRequest {
    render: Box<dyn FnOnce() -> Window + Send + Sync>,
    image: WeakArcVar<Image>,
}

/// Controls properties of the render window used by [`IMAGES.render`].
///
/// You can get the controller inside the closure using [`req`] or [`get`] and the `window_state`
/// in [`WindowContext`] and [`WidgetContext`].
///
/// [`WindowContext`]: crate::context::WindowContext::window_state
/// [`WidgetContext`]: crate::context::WidgetContext::window_state
/// [`Windows::vars`]: crate::window::Windows::vars
/// [`req`]: ImageRenderVars::req
/// [`get`]: ImageRenderVars::get
#[derive(Clone)]
pub struct ImageRenderVars {
    retain: ArcVar<bool>,
}
impl ImageRenderVars {
    fn new() -> Self {
        Self { retain: var(false) }
    }

    /// Require the vars from the window state.
    ///
    /// # Panics
    ///
    /// Panics if not called inside a render closure or widget.
    pub fn req() -> Self {
        WINDOW.with_state(|map| map.req(&IMAGE_RENDER_VARS_ID).clone())
    }

    /// Tries to get the window vars from the window state.
    pub fn get() -> Option<Self> {
        WINDOW.with_state(|map| map.get(&IMAGE_RENDER_VARS_ID).cloned())
    }

    /// If the render task is kept alive after a frame is produced, this is `false` by default
    /// meaning the image only renders once, if set to `true` the image will automatically update
    /// when the render widget requests a new frame.
    pub fn retain(&self) -> &ArcVar<bool> {
        &self.retain
    }
}

pub(super) static IMAGE_RENDER_VARS_ID: StaticStateId<ImageRenderVars> = StaticStateId::new_unique();

/// If the render task is kept alive after a frame is produced, this is `false` by default
/// meaning the image only renders once, if set to `true` the image will automatically update
/// when the render widget requests a new frame.
///
/// This property sets and binds `retain` to [`ImageRenderVars::retain`].
#[property(CONTEXT)]
pub fn render_retain(child: impl UiNode, retain: impl IntoVar<bool>) -> impl UiNode {
    #[ui_node(struct RenderRetainNode {
        child: impl UiNode,
        retain: impl Var<bool>,
    })]
    impl UiNode for RenderRetainNode {
        fn init(&mut self) {
            if let Some(vars) = ImageRenderVars::get() {
                vars.retain.set_ne(self.retain.get());
                let handle = self.retain.bind(vars.retain());
                WIDGET.push_var_handle(handle);
            } else {
                tracing::error!("can only set `render_retain` in render widgets");
            }

            self.child.init();
        }
    }
    RenderRetainNode {
        child,
        retain: retain.into_var(),
    }
}
