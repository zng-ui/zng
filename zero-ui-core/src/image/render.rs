use crate::{
    context::{AppContext, WindowContext},
    event::{EventArgs, EventUpdate},
    render::RenderMode,
    service::ServiceTuple,
    units::*,
    var::{types::WeakRcVar, *},
    widget_info::UpdateMask,
    window::*,
    UiNode, WidgetId,
};

use super::{Image, ImageManager, ImageVar, Images};

impl Images {
    /// Render the *window* to an image.
    ///
    /// The *window* is created as a headless surface and rendered to the returned image. If the *window*
    /// subscribes to any variable or event it is kept alive and updating, the returned image variable then updates
    /// for every new render of the node. If the *window* does not subscribe to anything, or the returned image is dropped the
    /// is is closed and dropped.
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
        let _ = self.updates.send_update(UpdateMask::none());
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
                } else if let Ok(s) = windows.subscriptions(r.window_id) {
                    retain = !s.is_none();
                }
            }

            if !retain {
                let _ = windows.close(r.window_id);
            }

            retain
        });

        for req in images.render.requests.drain(..) {
            if let Some(img) = req.image.upgrade() {
                windows.open_headless(
                    move |ctx| {
                        let vars = WindowVars::req(&ctx.window_state);
                        vars.auto_size().set(ctx.vars, true);
                        vars.min_size().set(ctx.vars, (1.px(), 1.px()));

                        let w = (req.render)(ctx);

                        let vars = WindowVars::req(&ctx.window_state);
                        vars.frame_capture_mode().set(ctx.vars, FrameCaptureMode::All);

                        let a = ActiveRenderer {
                            window_id: *ctx.window_id,
                            image: img.downgrade(),
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
}

struct RenderRequest {
    render: Box<dyn FnOnce(&mut WindowContext) -> Window>,
    image: WeakRcVar<Image>,
}
