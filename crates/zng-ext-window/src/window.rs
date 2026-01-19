use std::{mem, pin::Pin, sync::Arc};

use parking_lot::Mutex;
use zng_app::{
    Deadline, hn,
    render::{FrameBuilder, FrameUpdate},
    static_id,
    update::{LayoutUpdates, RenderUpdates, UPDATES},
    view_process::{
        VIEW_PROCESS, ViewHeadless, ViewRenderer, ViewWindow,
        raw_events::{
            RAW_COLORS_CONFIG_CHANGED_EVENT, RAW_HEADLESS_OPEN_EVENT, RAW_WINDOW_OPEN_EVENT, RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT,
        },
    },
    widget::{VarLayout as _, VarSubscribe, WIDGET, WidgetCtx, info::WidgetInfoTree},
    window::{WINDOW, WindowCtx, WindowId, WindowMode},
};
use zng_color::Rgba;
use zng_layout::{
    context::LayoutPassId,
    unit::{DipToPx as _, FactorUnits as _, Length, Px, PxConstraints, PxDensity, PxSize, PxToDip as _},
};
use zng_state_map::StateId;
use zng_var::{ResponderVar, ResponseVar, Var, VarHandle, var};
use zng_view_api::{
    api_extension::{ApiExtensionId, ApiExtensionPayload},
    config::{ColorsConfig, FontAntiAliasing},
    window::{FrameId, FrameRequest, FrameUpdateRequest, FrameWaitId, HeadlessRequest, WindowRequest, WindowState, WindowStateAll},
};
use zng_wgt::prelude::{DIRECTION_VAR, LAYOUT, LayoutMetrics, UiNode, WidgetInfo, WidgetLayout};

use crate::{
    AutoSize, CloseWindowResult, MONITORS, StartPosition, WINDOWS, WINDOWS_SV, WindowInstanceState, WindowLoadingHandle, WindowRoot,
    WindowRootExtenderArgs, WindowVars,
};

/// Extensions methods for [`WINDOW`] contexts of windows open by [`WINDOWS`].
///
/// [`WINDOW`]: zng_app::window::WINDOW
#[expect(non_camel_case_types)]
pub trait WINDOW_Ext {
    /// Clone a reference to the variables that get and set window properties.
    fn vars(&self) -> WindowVars {
        WindowVars::req()
    }

    /// Enable accessibility info.
    ///
    /// If access is not already enabled, enables it in the app-process only.
    fn enable_access(&self) {
        let vars = WINDOW.vars();
        let access_enabled = &vars.0.access_enabled;
        if access_enabled.get().is_disabled() {
            access_enabled.modify(|e| **e |= zng_app::widget::info::access::AccessEnabled::APP);
        }
    }

    /// Gets a handle that stops the window from loading while the handle is alive.
    ///
    /// A window is only opened in the view-process after it is loaded, without any loading handles the window is considered loaded
    /// after the first layout pass. Nodes in the window can request a loading handle to delay the view opening to after all async resources
    /// it requires are loaded.
    ///
    /// Note that a window is only loaded after all handles are dropped or expired, you should set a reasonable `deadline`,  
    /// it is best to partially render a window after a short time than not show anything.
    ///
    /// Returns `None` if the window has already loaded.
    fn loading_handle(&self, deadline: impl Into<Deadline>) -> Option<WindowLoadingHandle> {
        WINDOWS.loading_handle(WINDOW.id(), deadline)
    }

    /// Generate an image from the current rendered frame of the window.
    ///
    /// The image is not loaded at the moment of return, it will update when it is loaded.
    #[cfg(feature = "image")]
    fn frame_image(&self, mask: Option<zng_ext_image::ImageMaskMode>) -> zng_ext_image::ImageVar {
        WINDOWS.frame_image(WINDOW.id(), mask)
    }

    /// Generate an image from a selection of the current rendered frame of the window.
    ///
    /// The image is not loaded at the moment of return, it will update when it is loaded.
    ///
    /// If the window is not found the error is reported in the image error.
    #[cfg(feature = "image")]
    fn frame_image_rect(&self, rect: zng_layout::unit::PxRect, mask: Option<zng_ext_image::ImageMaskMode>) -> zng_ext_image::ImageVar {
        WINDOWS.frame_image_rect(WINDOW.id(), rect, mask)
    }

    /// Move the window to the front of the operating system Z stack.
    ///
    /// See [`WINDOWS.bring_to_top`] for more details.
    ///
    /// [`WINDOWS.bring_to_top`]: WINDOWS::bring_to_top
    fn bring_to_top(&self) {
        WINDOWS.bring_to_top(WINDOW.id());
    }

    /// Starts closing the window, the operation can be canceled by listeners of
    /// [`WINDOW_CLOSE_REQUESTED_EVENT`]. If the window has children they are closed together.
    ///
    /// Returns a response var that will update once with the result of the operation.
    ///
    /// See [`WINDOWS.close`] for more details.
    ///
    /// [`WINDOWS.close`]: WINDOWS::close
    fn close(&self) -> ResponseVar<CloseWindowResult> {
        WINDOWS.close(WINDOW.id())
    }
}
impl WINDOW_Ext for WINDOW {}

pub(crate) struct WindowInstance {
    pub(crate) mode: WindowMode,
    pub(crate) pending_loading: Option<Var<usize>>,
    pub(crate) vars: Option<WindowVars>,
    pub(crate) info: Option<WidgetInfoTree>,
    pub(crate) extensions_init: Option<Vec<(ApiExtensionId, ApiExtensionPayload)>>,
    pub(crate) root: Option<WindowNode>,
}
impl WindowInstance {
    pub(crate) fn new(
        id: WindowId,
        mode: WindowMode,
        new_window: Pin<Box<dyn Future<Output = WindowRoot> + Send + 'static>>,
        r: ResponderVar<WindowVars>,
    ) -> Self {
        let loading = var(0);
        loading
            .on_pre_new(hn!(|a| {
                // handle loading handles dropped
                if a.value == 0
                    && let Some(w) = WINDOWS_SV.write().windows.get_mut(&id)
                    && let Some(vars) = &w.vars
                {
                    w.pending_loading = None;
                    vars.0.instance_state.modify(move |s| {
                        if matches!(s.value(), WindowInstanceState::Loading) {
                            s.set(WindowInstanceState::Loaded { has_view: false });
                            UPDATES.layout_window(id);
                        }
                    });
                }
            }))
            .perm();
        let w = Self {
            mode,
            pending_loading: Some(loading),
            vars: None,
            info: None,
            extensions_init: Some(vec![]),
            root: None,
        };
        UPDATES
            .run(async move {
                // init WINDOW context, vars
                let primary_scale_factor = match mode {
                    WindowMode::Headed => MONITORS
                        .primary_monitor()
                        .get()
                        .map(|m| m.scale_factor().get())
                        .unwrap_or_else(|| 1.fct()),
                    WindowMode::Headless | WindowMode::HeadlessWithRenderer => 1.fct(),
                };

                let system_colors = match mode {
                    WindowMode::Headed => RAW_COLORS_CONFIG_CHANGED_EVENT
                        .var_latest()
                        .get()
                        .map(|a| a.config)
                        .unwrap_or_default(),
                    WindowMode::Headless | WindowMode::HeadlessWithRenderer => ColorsConfig::default(),
                };

                let vars = {
                    let mut s = WINDOWS_SV.write();
                    let vars = WindowVars::new(s.default_render_mode.get(), primary_scale_factor, system_colors);
                    crate::hooks::init_window_hooks(id, &vars);
                    r.respond(vars.clone());
                    s.windows.get_mut(&id).unwrap().vars = Some(vars.clone());
                    vars
                };
                let mut ctx = WindowCtx::new(id, mode);
                ctx.with_state(|s| s.borrow_mut().set(*crate::WINDOW_VARS_ID, vars.clone()));

                // poll `new_window` with context
                struct CtxFut {
                    f: Pin<Box<dyn Future<Output = WindowRoot> + Send + 'static>>,
                    ctx: Option<WindowCtx>,
                }
                impl Future for CtxFut {
                    type Output = (WindowRoot, WindowCtx);

                    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
                        let s = &mut *self.as_mut();
                        let r = WINDOW.with_context(s.ctx.as_mut().unwrap(), || s.f.as_mut().poll(cx));
                        match r {
                            std::task::Poll::Ready(w) => std::task::Poll::Ready((w, s.ctx.take().unwrap())),
                            std::task::Poll::Pending => std::task::Poll::Pending,
                        }
                    }
                }
                let new_window = CtxFut {
                    f: new_window,
                    ctx: Some(ctx),
                };
                let (mut root, win_ctx) = new_window.await;

                // apply root extenders
                let mut extenders = mem::take(&mut WINDOWS_SV.write().root_extenders).into_inner();
                for ext in &mut extenders {
                    root.child = ext(WindowRootExtenderArgs { root: root.child });
                }
                {
                    let mut s = WINDOWS_SV.write();
                    extenders.append(s.root_extenders.get_mut());
                    *s.root_extenders.get_mut() = extenders;
                }

                // init and request first info update
                let mut root = WindowNode {
                    win_ctx,
                    wgt_ctx: WidgetCtx::new(root.id),
                    root: Mutex::new(root),

                    view_window: None,
                    view_headless: None,
                    renderer: None,
                    view_opening: VarHandle::dummy(),

                    layout_pass: LayoutPassId::new(),
                    frame_id: FrameId::INVALID,
                    clear_color: Rgba::default(),
                    frame_wait_id: None,
                };
                root.with_root(|n| n.init());
                UPDATES.update_info_window(id);
                UPDATES.layout_window(id);
                UPDATES.render_window(id);
                WINDOWS_SV.write().windows.get_mut(&id).unwrap().root = Some(root);
                vars.0.instance_state.set(WindowInstanceState::Loading);

                // will continue in WindowsService::update_info, called by app loop
            })
            .perm();
        w
    }
}

pub(crate) struct WindowNode {
    pub(crate) win_ctx: WindowCtx,
    pub(crate) wgt_ctx: WidgetCtx,
    // Mutex for Sync only
    pub(crate) root: Mutex<WindowRoot>,

    pub(crate) view_window: Option<ViewWindow>,
    pub(crate) view_headless: Option<ViewHeadless>,
    pub(crate) renderer: Option<ViewRenderer>,
    pub(crate) view_opening: VarHandle,

    pub(crate) layout_pass: LayoutPassId,
    pub(crate) frame_id: FrameId,
    pub(crate) clear_color: Rgba,
    pub(crate) frame_wait_id: Option<FrameWaitId>,
}
impl WindowNode {
    pub(crate) fn with_root<R>(&mut self, f: impl FnOnce(&mut UiNode) -> R) -> R {
        WINDOW.with_context(&mut self.win_ctx, || {
            WIDGET.with_context(&mut self.wgt_ctx, zng_app::widget::WidgetUpdateMode::Bubble, || {
                f(&mut self.root.get_mut().child)
            })
        })
    }
}

static_id! {
    static ref NESTED_WINDOW_INFO_ID: StateId<WindowId>;
}

/// Extension methods for widget info about a node that hosts a nested window.
pub trait NestedWindowWidgetInfoExt {
    /// Gets the hosted window ID if the widget hosts a nested window.
    fn nested_window(&self) -> Option<WindowId>;

    /// Gets the hosted window info tree if the widget hosts a nested window that is open.
    fn nested_window_tree(&self) -> Option<WidgetInfoTree> {
        WINDOWS.widget_tree(self.nested_window()?)
    }
}

impl NestedWindowWidgetInfoExt for WidgetInfo {
    fn nested_window(&self) -> Option<WindowId> {
        self.meta().get_clone(*NESTED_WINDOW_INFO_ID)
    }
}

pub(crate) fn layout_open_view((id, n, vars): &mut (WindowId, WindowNode, Option<WindowVars>), updates: &Arc<LayoutUpdates>) {
    if !updates.delivery_list().enter_window(*id) {
        return;
    }

    let vars = vars.take().unwrap();

    // root metrics
    let scale_factor = vars.scale_factor().get();

    // resolve monitor
    let monitor = vars.0
        .actual_monitor
        .get()
        .and_then(|id| MONITORS.monitor(id));
    let resolving_monitor = monitor.is_none();
    let monitor = monitor.unwrap_or_else(|| vars.0.monitor.get().select_fallback(*id));
    if resolving_monitor {
        vars.0.actual_monitor.set(monitor.id());
    }

    // metrics for layout of actual root values, relative to screen size
    let font_size_dft = Length::pt_to_px(11.0, scale_factor);
    let metrics = LayoutMetrics::new(scale_factor, monitor.size().get(), font_size_dft)
        .with_screen_density(monitor.density().get())
        .with_direction(DIRECTION_VAR.get());

    // valid auto size config
    let auto_size = if matches!(vars.state().get(), WindowState::Normal) {
        vars.auto_size().get()
    } else {
        AutoSize::empty()
    };

    // layout
    n.layout_pass = n.layout_pass.next();
    let (final_size, min_size, max_size) = LAYOUT.with_root_context(n.layout_pass, metrics, || {
        // root font size
        let font_size = vars.font_size().layout_dft_x(font_size_dft);
        LAYOUT.with_font_size(font_size, || {
            // root constraints
            let min_size = vars.min_size().layout();
            let max_size = vars.max_size().layout_dft(PxSize::splat(Px::MAX)).max(min_size);                        
            let size = vars.actual_size().get().to_px(scale_factor);
            let mut root_cons = LAYOUT.constraints();
            if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                root_cons.x = PxConstraints::new_range(min_size.width, max_size.width);
            } else {
                root_cons.x = PxConstraints::new_exact(size.width);
            }
            if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                root_cons.y = PxConstraints::new_range(min_size.height, max_size.height);
            } else {
                root_cons.y = PxConstraints::new_exact(size.height);
            }

            // layout
            let desired_size = LAYOUT.with_constraints(root_cons, || {
                n.with_root(|n| WidgetLayout::with_root_widget(updates.clone(), |wl| n.layout(wl)))
            });

            // clamp desired_size
            let mut final_size = size;
            if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                final_size.width = desired_size.width.max(min_size.width).min(max_size.width);
            }
            if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                final_size.height = desired_size.height.max(min_size.height).min(max_size.height);
            }            

            (final_size, min_size, max_size)
        })
    });

    if n.wgt_ctx.is_pending_reinit() {
        n.with_root(|_| WIDGET.update());
    }

    let size_dip = final_size.to_dip(scale_factor);
    if !auto_size.is_empty() {
        // on view resize will skip layout again because size matches
        vars.0.actual_size.set(size_dip);
    }

    // transition to Loaded (without view)
    if matches!(vars.0.instance_state.get(), WindowInstanceState::Loading) {
        let mut s = WINDOWS_SV.write();
        let w = s.windows.get_mut(id).unwrap();
        if let Some(l) = &w.pending_loading
            && l.get() > 1
        {
            // wait loading handles
            tracing::debug!("skipping view-process open, active loading handles");
            return;
        }
        w.pending_loading = None;
        vars.0.instance_state.set(WindowInstanceState::Loaded { has_view: false });
    }

     // transition to Loaded (with view) or update view
    match n.win_ctx.mode() {
        WindowMode::Headed => {
            if let Some(view) = &n.view_window {
                // update view window size if is auto_size
                if !auto_size.is_empty() {
                    let mut s = vars.window_state_all(min_size, max_size);
                    s.restore_rect.size = size_dip;
                    vars.restore_rect().modify(move |a| {
                        if a.value().size != size_dip {
                            a.value_mut().size = size_dip;
                        }
                    });
                    let _ = view.set_state(s);
                }
            } else if n.view_opening.is_dummy() {
                // start opening view-process window

                if !VIEW_PROCESS.is_connected() {
                    tracing::debug!("skipping view-process open window, no view-process connected");
                    return;
                }

                let id = n.win_ctx.id();

                // fatal error if view-process fails to open, there is no way to recover from this,
                // its not a view-process crash as that causes a respawn, its an invalid request, maybe
                // the implementation only supports one window or something like that
                let error_handle = RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT.hook(move |a| {
                    if a.window_id != id {
                        return true;
                    }
                    panic!("view-process failed to open window {id}, {}", a.error);
                });
                n.view_opening = RAW_WINDOW_OPEN_EVENT.hook(move |a| {
                    let _hold = &error_handle;

                    if a.window_id != id {
                        return true;
                    }

                    let mut s = WINDOWS_SV.write();
                    if let Some(w) = s.windows.get_mut(&id) {
                        w.vars.as_ref().unwrap().0.instance_state.set(WindowInstanceState::Loaded { has_view: true });
                        w.root.as_mut().unwrap().view_window = Some(a.window); // !!: TODO renderer, window data, event holding handle
                    }

                    false
                });

                // Layout initial position in the monitor space.
                let mut system_pos = false;
                let position = match n.root.get_mut().start_position {
                    StartPosition::Default => {} // !!: TODO
                    StartPosition::CenterMonitor => {}
                    StartPosition::CenterParent => {}
                };

                let r = VIEW_PROCESS.open_window(WindowRequest::new(
                    zng_view_api::window::WindowId::from_raw(id.get()),
                    vars.0.title.get(),
                    vars.window_state_all(min_size, max_size),
                    n.root.get_mut().kiosk,
                    system_pos,
                    vars.0.video_mode.get(),
                    vars.0.visible.get(),
                    vars.0.taskbar_visible.get(),
                    vars.0.always_on_top.get(),
                    vars.0.movable.get(),
                    vars.0.resizable.get(),
                    #[cfg(feature = "image")]
                    vars.0.actual_icon.with(|w| w.as_ref().map(|w| w.view_handle().image_id())),
                    #[cfg(not(feature = "image"))]
                    None,
                    vars.0.cursor.with(|c| c.icon()),
                    #[cfg(feature = "image")]
                    vars.0
                        .actual_cursor_img
                        .with(|i| i.as_ref().map(|(i, p)| (i.view_handle().image_id(), *p))),
                    #[cfg(not(feature = "image"))]
                    None,
                    n.root.get_mut().transparent,
                    #[cfg(feature = "image")]
                    matches!(vars.0.frame_capture_mode.get(), crate::FrameCaptureMode::All),
                    #[cfg(not(feature = "image"))]
                    false,
                    n.root.get_mut().render_mode.unwrap_or_else(|| WINDOWS.default_render_mode().get()),
                    vars.0.focus_indicator.get(),
                    vars.0.focused.get(),
                    None, // !!: TODO, from info tree
                    vars.0.enabled_buttons.get(),
                    vars.0.system_shutdown_warn.get(),
                    WINDOWS.take_view_extensions_init(id),
                ));
                if r.is_err() {
                    tracing::error!("view-process window open request failed, will retry on respawn");
                    n.view_opening = VarHandle::dummy();
                }
            }
        }
        WindowMode::HeadlessWithRenderer => {
            if let Some(view) = &n.view_headless {
                if !auto_size.is_empty() {
                    let _ = view.set_size(size_dip, scale_factor);
                }
            } else if n.view_opening.is_dummy() {
                if !VIEW_PROCESS.is_connected() {
                    tracing::debug!("skipping view-process open headless, no view-process connected");
                    return;
                }

                // start opening view-process renderer
                let id = n.win_ctx.id();
                let error_handle = RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT.hook(move |a| {
                    if a.window_id != id {
                        return true;
                    }
                    tracing::error!("failed to open headless surface {id}, {}", a.error);
                    false
                });
                n.view_opening = RAW_HEADLESS_OPEN_EVENT.hook(move |a| {
                    let _hold = &error_handle;

                    if a.window_id != id {
                        return true;
                    }

                    false
                });
                let r = VIEW_PROCESS.open_headless(HeadlessRequest::new(
                    zng_view_api::window::WindowId::from_raw(id.get()),
                    scale_factor,
                    size_dip,
                    n.root.get_mut().render_mode.unwrap_or_else(|| WINDOWS.default_render_mode().get()),
                    WINDOWS.take_view_extensions_init(id),
                ));
                if r.is_err() {
                    tracing::error!("view-process headless surface open request failed, will retry on respawn");
                    n.view_opening = VarHandle::dummy();
                }
            }
        }
        WindowMode::Headless => {}
    }
}

pub(crate) fn render(
    (id, n, vars): &mut (WindowId, WindowNode, Option<WindowVars>),
    render_widgets: &Arc<RenderUpdates>,
    render_update_widgets: &Arc<RenderUpdates>,
) {
    if render_widgets.delivery_list().enter_window(*id) {
        // skip until there is a window.
        if matches!(n.win_ctx.mode(), WindowMode::Headed | WindowMode::Headless) && n.renderer.is_none() {
            return;
        }

        let vars = vars.take().unwrap();
        let info = n.win_ctx.widget_tree();

        // render
        n.frame_id = n.frame_id.next();
        let mut frame = FrameBuilder::new(
            render_widgets.clone(),
            render_update_widgets.clone(),
            n.frame_id,
            n.wgt_ctx.id(),
            &n.wgt_ctx.bounds(),
            &info,
            n.renderer.clone(),
            vars.0.scale_factor.get(),
            FontAntiAliasing::Default,
        );
        n.with_root(|n| {
            n.render(&mut frame);
        });
        let frame = frame.finalize(&info);
        n.clear_color = frame.clear_color;

        let capture = vars.take_frame_capture();
        let wait_id = n.frame_wait_id.take();
        if let Some(r) = &n.renderer {
            // send frame to view-process
            let _ = r.render(FrameRequest::new(n.frame_id, n.clear_color, frame.display_list, capture, wait_id));
        } else {
            // simulate for headless without renderer
            #[cfg(feature = "image")]
            crate::FRAME_IMAGE_READY_EVENT.notify(crate::FrameImageReadyArgs::now(n.win_ctx.id(), n.frame_id, None));
        }

        if n.wgt_ctx.is_pending_reinit() {
            n.with_root(|_| WIDGET.update());
        }
    } else if render_update_widgets.delivery_list().enter_window(*id) {
        // update
        n.frame_id = n.frame_id.next_update();
        let mut update = FrameUpdate::new(
            render_update_widgets.clone(),
            n.frame_id,
            n.wgt_ctx.id(),
            n.wgt_ctx.bounds(),
            n.clear_color,
        );
        n.with_root(|n| {
            n.render_update(&mut update);
        });
        let update = update.finalize(&n.win_ctx.widget_tree());
        if let Some(c) = update.clear_color {
            n.clear_color = c;
        }
        let vars = vars.take().unwrap();
        let capture = vars.take_frame_capture();
        let wait_id = n.frame_wait_id.take();

        if let Some(r) = &n.renderer {
            // send updates to view-process
            let _ = r.render_update(FrameUpdateRequest::new(
                n.frame_id,
                update.transforms,
                update.floats,
                update.colors,
                update.clear_color,
                capture,
                wait_id,
                update.extensions,
            ));
        } else {
            // simulate for headless without renderer
            #[cfg(feature = "image")]
            crate::FRAME_IMAGE_READY_EVENT.notify(crate::FrameImageReadyArgs::now(n.win_ctx.id(), n.frame_id, None));
        }

        if n.wgt_ctx.is_pending_reinit() {
            n.with_root(|_| WIDGET.update());
        }
    }
}
