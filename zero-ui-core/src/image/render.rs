use crate::{
    color::{rgba, Rgba},
    context::{AppContext, RenderContext, WindowContext},
    event::EventUpdateArgs,
    impl_ui_node,
    render::FrameBuilder,
    render::RenderMode,
    service::ServiceTuple,
    units::*,
    var::{types::WeakRcVar, *},
    widget_info::UpdateMask,
    window::*,
    BoxedUiNode, UiNode, WidgetId,
};

use super::{Image, ImageManager, ImageVar, Images};

impl Images {
    /// Render the `node` to an image.
    ///
    /// The `node` is inited, updated, layout and rendered to an image of its desired size. If the `node`
    /// subscribes to any variable or event it is kept alive and updating, the returned image variable then updates
    /// for every new render of the node. If the `node` does not subscribe to anything, or the returned image is dropped the
    /// `node` is deinited and dropped.
    ///
    /// The closure input is the [`WindowContext`] of the headless window used for rendering the node.
    ///
    /// Requires the [`Windows`] service.
    pub fn render<U, N>(&mut self, node: N, config: RenderConfig) -> ImageVar
    where
        U: UiNode,
        N: FnOnce(&mut WindowContext) -> U + 'static,
    {
        let result = var(Image::new_none(None));
        self.render_image(node, config, &result);
        result.into_read_only()
    }

    pub(super) fn render_image<U, N>(&mut self, node: N, config: RenderConfig, result: &RcVar<Image>)
    where
        U: UiNode,
        N: FnOnce(&mut WindowContext) -> U + 'static,
    {
        struct ImageRenderNode<C> {
            child: C,
            clear_color: Rgba,
        }
        #[impl_ui_node(child)]
        impl<C: UiNode> UiNode for ImageRenderNode<C> {
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                frame.set_clear_color(self.clear_color.into());
                self.child.render(ctx, frame);
            }
        }

        self.render.requests.push(RenderRequest {
            node: Box::new(move |ctx| {
                ImageRenderNode {
                    child: node(ctx),
                    clear_color: config.clear_color,
                }
                .boxed()
            }),
            config,
            image: result.downgrade(),
        });

        let _ = self.updates.send_update(UpdateMask::none());
    }
}

impl ImageManager {
    /// AppExtension::update
    pub(super) fn update_render(&mut self, ctx: &mut AppContext) {
        let (images, windows) = <(Images, Windows)>::req(ctx.services);

        images.render.active.retain(|r| {
            let mut retain = false;

            if let Some(img) = r.image.upgrade() {
                if img.get(ctx.vars).is_loading() {
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
                        let w = Window::new_container(
                            req.config.root_id.unwrap_or_else(WidgetId::new_unique),
                            StartPosition::Default,
                            false,
                            true,
                            req.config.render_mode,
                            HeadlessMonitor::new_scale(req.config.scale_factor.unwrap_or_else(|| 1.fct())),
                            false,
                            (req.node)(ctx),
                        );

                        let vars = ctx.window_state.req(WindowVarsKey);

                        vars.frame_capture_mode().set(ctx.vars, FrameCaptureMode::All);

                        if let Some(size) = req.config.size {
                            vars.size().set(ctx.vars, size);
                        } else {
                            vars.auto_size().set(ctx.vars, true);
                        }

                        vars.min_size().set(ctx.vars, (1.px(), 1.px()));

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
    pub(super) fn event_preview_render<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        if let Some(args) = FrameImageReadyEvent.update(args) {
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
    node: Box<dyn FnOnce(&mut WindowContext) -> BoxedUiNode>,
    config: RenderConfig,
    image: WeakRcVar<Image>,
}

/// Configuration for the [`Images::render`] operation.
#[derive(Debug, Clone, Hash, PartialEq)]
pub struct RenderConfig {
    /// Widget id for the root widget that hosts the rendering node.
    ///
    /// If `None` a random id is used.
    pub root_id: Option<WidgetId>,
    /// Size of the resulting image.
    ///
    /// If `None` the image auto-sizes to the node desired size.
    pub size: Option<DipSize>,

    /// Scale factor used during rendering and as the density of the resulting image.
    ///
    /// If `None` the parent widget can override, otherwise is `1`.
    pub scale_factor: Option<Factor>,

    /// Render backend preference. Default is `Software`.
    pub render_mode: RenderMode,

    /// Color the image is filled first before render.
    ///
    /// Is transparent black by default.
    pub clear_color: Rgba,
}
impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            root_id: None,
            size: None,
            scale_factor: None,
            render_mode: RenderMode::Software,
            clear_color: rgba(0, 0, 0, 0),
        }
    }
}
impl_from_and_into_var! {
    fn from(render_mode: RenderMode) -> RenderConfig {
        RenderConfig {
            render_mode,
            ..Default::default()
        }
    }
}
