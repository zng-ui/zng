use crate::{
    context::{state_map, AppContext, BorrowStateMap, StaticStateId, WindowContext},
    event::{AnyEventArgs, EventUpdate},
    property,
    render::RenderMode,
    service::ServiceTuple,
    ui_node,
    units::*,
    var::{types::WeakRcVar, *},
    window::*,
    UiNode, WidgetId,
};

use super::{Image, ImageManager, ImageVar, Images};

impl Images {
    /// Render the *window* to an image.
    ///
    /// The *window* is created as a headless surface and rendered to the returned image. You can use the
    /// [`ImageRenderVars::retain`] var create an image that updates with new frames, or by default only render once.
    ///
    /// The closure input is the [`WindowContext`] of the headless window.
    ///
    /// Requires the [`Windows`] service.
    pub fn render<N>(&mut self, render: N) -> ImageVar
    where
        N: FnOnce(&mut WindowContext) -> Window + 'static,
    {
        let result = var(Image::new_none(None));
        self.render_img(
            move |ctx| {
                let r = render(ctx);
                WindowVars::req(&ctx.window_state)
                    .frame_capture_mode()
                    .set_ne(ctx.vars, FrameCaptureMode::All);
                r
            },
            &result,
        );
        result.read_only()
    }

    pub(super) fn render_img<N>(&mut self, render: N, result: &RcVar<Image>)
    where
        N: FnOnce(&mut WindowContext) -> Window + 'static,
    {
        self.render.requests.push(RenderRequest {
            render: Box::new(render),
            image: result.downgrade(),
        });
        let _ = self.updates.send_ext_update();
    }

    /// Render an [`UiNode`] to an image.
    ///
    /// This method is a shortcut to [`render`] a node without needing to declare the headless window, note that
    /// a headless window is still used, the node does not have the same context as the calling widget.
    ///
    /// [`render`]: Self::render
    pub fn render_node<U, N>(&mut self, render_mode: RenderMode, scale_factor: impl Into<Factor>, render: N) -> ImageVar
    where
        U: UiNode,
        N: FnOnce(&mut WindowContext) -> U + 'static,
    {
        let scale_factor = scale_factor.into();
        self.render(move |ctx| {
            let node = render(ctx);
            Window::new_container(
                WidgetId::new_unique(),
                StartPosition::Default,
                false,
                true,
                render_mode,
                HeadlessMonitor::new_scale(scale_factor),
                false,
                node,
            )
        })
    }
}

impl ImageManager {
    /// AppExtension::update
    pub(super) fn update_render(&mut self, ctx: &mut AppContext) {
        let (images, windows) = <(Images, Windows)>::req(ctx.services);

        images.render.active.retain(|r| {
            let mut retain = false;

            if let Some(img) = r.image.upgrade() {
                if img.with(Image::is_loading) {
                    retain = true;
                }
            }

            retain |= r.retain.get();

            if !retain {
                let _ = windows.close(r.window_id);
            }

            retain
        });

        for req in images.render.requests.drain(..) {
            if let Some(img) = req.image.upgrade() {
                windows.open_headless(
                    move |ctx| {
                        let vars = ImageRenderVars::new();
                        let retain = vars.retain.clone();
                        ctx.window_state.set(&IMAGE_RENDER_VARS_ID, vars);
                        let vars = WindowVars::req(&ctx.window_state);
                        vars.auto_size().set(ctx.vars, true);
                        vars.min_size().set(ctx.vars, (1.px(), 1.px()));

                        let w = (req.render)(ctx);

                        let vars = WindowVars::req(&ctx.window_state);
                        vars.frame_capture_mode().set(ctx.vars, FrameCaptureMode::All);

                        let a = ActiveRenderer {
                            window_id: *ctx.window_id,
                            image: img.downgrade(),
                            retain,
                        };
                        Images::req(ctx.services).render.active.push(a);

                        w
                    },
                    true,
                );
            }
        }
    }

    /// AppExtension::event_preview
    pub(super) fn event_preview_render(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        if let Some(args) = FRAME_IMAGE_READY_EVENT.on(update) {
            if let Some(img) = &args.frame_image {
                let imgs = Images::req(ctx.services);
                if let Some(a) = imgs.render.active.iter().find(|a| a.window_id == args.window_id) {
                    if let Some(img_var) = a.image.upgrade() {
                        img_var.set(ctx.vars, img.clone());
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
    image: WeakRcVar<Image>,
    retain: RcVar<bool>,
}

struct RenderRequest {
    render: Box<dyn FnOnce(&mut WindowContext) -> Window>,
    image: WeakRcVar<Image>,
}

/// Controls properties of the render window used by [`Images::render`].
///
/// You can get the controller inside the closure using [`req`] or [`get`] and the `window_state`
/// in [`WindowContext`] and [`WidgetContext`].
///
/// [`WindowContext`]: crate::context::WindowContext::window_state
/// [`WidgetContext`]: crate::context::WidgetContext::window_state
/// [`Windows::vars`]: crate::window::Windows::vars
/// [`req`]: ImageRenderVars::req
/// [`get`]: ImageRenderVars::get
pub struct ImageRenderVars {
    retain: RcVar<bool>,
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
    pub fn req(window_state: &impl BorrowStateMap<state_map::Window>) -> &Self {
        window_state.borrow().req(&IMAGE_RENDER_VARS_ID)
    }

    /// Tries to get the window vars from the window state.
    pub fn get(window_state: &impl BorrowStateMap<state_map::Window>) -> Option<&Self> {
        window_state.borrow().get(&IMAGE_RENDER_VARS_ID)
    }

    /// If the render task is kept alive after a frame is produced, this is `false` by default
    /// meaning the image only renders once, if set to `true` the image will automatically update
    /// when the render widget requests a new frame.
    pub fn retain(&self) -> &RcVar<bool> {
        &self.retain
    }
}

pub(super) static IMAGE_RENDER_VARS_ID: StaticStateId<ImageRenderVars> = StaticStateId::new_unique();

/// If the render task is kept alive after a frame is produced, this is `false` by default
/// meaning the image only renders once, if set to `true` the image will automatically update
/// when the render widget requests a new frame.
///
/// This property sets and binds `retain` to [`ImageRenderVars::retain`].
#[property(context)]
pub fn render_retain(child: impl UiNode, retain: impl IntoVar<bool>) -> impl UiNode {
    #[ui_node(struct RenderRetainNode {
        child: impl UiNode,
        retain: impl Var<bool>,
    })]
    impl UiNode for RenderRetainNode {
        fn init(&mut self, ctx: &mut crate::context::WidgetContext) {
            if let Some(vars) = ImageRenderVars::get(ctx) {
                vars.retain.set_ne(ctx, self.retain.get());
                let handle = self.retain.bind(vars.retain());
                ctx.handles.push_var(handle);
            } else {
                tracing::error!("can only set `render_retain` in render widgets");
            }

            self.child.init(ctx);
        }
    }
    RenderRetainNode {
        child,
        retain: retain.into_var(),
    }
}
