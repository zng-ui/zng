use std::{any::Any, mem, pin::Pin, sync::Arc};

use parking_lot::Mutex;
use zng_app::{
    APP, Deadline, async_hn_once,
    render::{FrameBuilder, FrameUpdate},
    static_id,
    timer::{DeadlineHandle, TIMERS},
    update::{LayoutUpdates, RenderUpdates, UPDATES},
    view_process::{
        VIEW_PROCESS, ViewHeadless, ViewRenderer, ViewWindow,
        raw_events::{
            RAW_COLORS_CONFIG_CHANGED_EVENT, RAW_HEADLESS_OPEN_EVENT, RAW_MONITORS_CHANGED_EVENT, RAW_WINDOW_FOCUS_EVENT,
            RAW_WINDOW_OPEN_EVENT, RawWindowFocusArgs,
        },
    },
    widget::{VarLayout as _, WIDGET, WidgetCtx, base::PARALLEL_VAR, info::WidgetInfoTree},
    window::{MonitorId, WINDOW, WindowCtx, WindowId, WindowMode},
};
use zng_app_context::LocalContext;
use zng_color::{COLOR_SCHEME_VAR, Rgba, colors::ACCENT_COLOR_VAR};
use zng_layout::unit::{DipSize, TimeUnits as _};
use zng_layout::{
    context::LayoutPassId,
    unit::{
        Dip, DipPoint, DipToPx as _, FactorUnits as _, Layout2d as _, Length, Px, PxConstraints, PxConstraints2d, PxDensity, PxPoint,
        PxRect, PxSize, PxToDip as _, PxVector,
    },
};
use zng_state_map::StateId;
use zng_unique_id::IdSet;
use zng_var::{ResponderVar, ResponseVar, VarHandle};
use zng_view_api::{
    api_extension::{ApiExtensionId, ApiExtensionPayload},
    config::{ColorsConfig, FontAntiAliasing},
    window::{FrameId, FrameRequest, FrameUpdateRequest, FrameWaitId, HeadlessRequest, WindowCapability, WindowRequest, WindowState},
};
use zng_wgt::{
    node::with_context_var,
    prelude::{DIRECTION_VAR, LAYOUT, LayoutMetrics, UiNode, UiNodeImpl, WidgetInfo, WidgetInfoBuilder, WidgetLayout},
};

use crate::{
    AutoSize, CloseWindowResult, MONITORS, OpenNestedHandlerArgs, SetFromLayoutTag, StartPosition, WINDOW_CLOSE_EVENT, WINDOW_LOAD_EVENT,
    WINDOW_OPEN_EVENT, WINDOWS, WINDOWS_EXTENSIONS, WINDOWS_SV, WidgetInfoImeArea as _, WindowCloseArgs, WindowInstanceState,
    WindowLoadingHandle, WindowOpenArgs, WindowRoot, WindowRootExtenderArgs, WindowVars,
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
    /// [`WINDOW_CLOSE_REQUESTED_EVENT`]: crate::WINDOW_CLOSE_REQUESTED_EVENT
    fn close(&self) -> ResponseVar<CloseWindowResult> {
        WINDOWS.close(WINDOW.id())
    }
}
impl WINDOW_Ext for WINDOW {}

pub(crate) struct WindowInstance {
    pub(crate) mode: WindowMode,
    pub(crate) pending_loading: std::sync::Weak<dyn Any + Send + Sync>,
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
        let w = Self {
            mode,
            pending_loading: std::sync::Weak::<()>::new(),
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
                    crate::hooks::hook_window_vars_cmds(id, &vars);
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
                let (mut root, mut win_ctx) = new_window.await;
                let mut nested = None;

                // lock in kiosk mode
                if root.kiosk {
                    vars.0.chrome.set(false);
                    let chrome_wk = vars.0.chrome.downgrade();
                    vars.0
                        .chrome
                        .hook(move |a| {
                            if !a.value() {
                                tracing::error!("cannot enable chrome in kiosk mode");
                                if let Some(c) = chrome_wk.upgrade() {
                                    c.set(false);
                                }
                            }
                            true
                        })
                        .perm();

                    if !vars.0.state.get().is_fullscreen() {
                        let try_exclusive =
                            !VIEW_PROCESS.is_connected() || VIEW_PROCESS.info().window.contains(WindowCapability::EXCLUSIVE);
                        if try_exclusive {
                            vars.0.state.set(WindowState::Exclusive);
                        } else {
                            vars.0.state.set(WindowState::Fullscreen);
                        }
                        let state_wk = vars.0.state.downgrade();
                        vars.0
                            .state
                            .hook(move |a| {
                                if !a.value().is_fullscreen() {
                                    tracing::error!("cannot exit fullscreen in kiosk mode");
                                    if let Some(s) = state_wk.upgrade() {
                                        let try_exclusive = !VIEW_PROCESS.is_connected()
                                            || VIEW_PROCESS.info().window.contains(WindowCapability::EXCLUSIVE);
                                        if try_exclusive {
                                            s.set(WindowState::Exclusive);
                                        } else {
                                            s.set(WindowState::Fullscreen);
                                        }
                                    }
                                }

                                true
                            })
                            .perm();
                    }
                }

                // apply root extenders and nest handlers
                let mut extenders;
                let mut nest_handlers;
                {
                    let mut s = WINDOWS_SV.write();
                    extenders = mem::take(&mut s.root_extenders).into_inner();
                    nest_handlers = mem::take(&mut s.open_nested_handlers).into_inner();
                }
                WINDOW.with_context(&mut win_ctx, || {
                    for ext in &mut extenders {
                        root.child = ext(WindowRootExtenderArgs {
                            root: mem::replace(&mut root.child, UiNode::nil()),
                        });
                    }
                    // built-in "extenders", set context vars
                    let child = mem::replace(&mut root.child, UiNode::nil());
                    let child = with_context_var(child, ACCENT_COLOR_VAR, vars.actual_accent_color());
                    let child = with_context_var(child, COLOR_SCHEME_VAR, vars.actual_color_scheme());
                    let child = with_context_var(child, PARALLEL_VAR, vars.parallel());
                    root.child = child;

                    let mut args = OpenNestedHandlerArgs::new();
                    for nest in &mut nest_handlers {
                        nest(&mut args);
                        if args.has_nested {
                            nested = Some(NestedData {
                                pending_layout: None,
                                pending_render: None,
                            });
                            break;
                        }
                    }
                });
                {
                    let mut s = WINDOWS_SV.write();
                    extenders.append(s.root_extenders.get_mut());
                    *s.root_extenders.get_mut() = extenders;
                    nest_handlers.append(s.open_nested_handlers.get_mut());
                    *s.open_nested_handlers.get_mut() = nest_handlers;
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

                    nested,

                    layout_pass: LayoutPassId::new(),
                    frame_id: FrameId::INVALID,
                    clear_color: Rgba::default(),
                    frame_wait_id: None,
                };
                root.with_root(|n| n.init());
                UPDATES.update_info_window(id);
                UPDATES.layout_window(id);
                WINDOWS_SV.write().windows.get_mut(&id).unwrap().root = Some(root);
                vars.0.instance_state.set(WindowInstanceState::Loading);
                WINDOW_OPEN_EVENT.notify(WindowOpenArgs::now(id));

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

    pub(crate) nested: Option<NestedData>,

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

pub(crate) struct NestedData {
    pending_layout: Option<Arc<LayoutUpdates>>,
    pending_render: Option<[Arc<RenderUpdates>; 2]>,
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

    if let Some(n) = &mut n.nested {
        // nested window layout happens in the layout context of the parent window
        n.pending_layout = Some(updates.clone());
        if let Some(t) = vars.0.nest_parent.get() {
            UPDATES.layout(t);
        }
        return;
    }

    // resolve monitor
    let mut monitor_rect = PxRect::zero();
    let mut monitor_density = PxDensity::default();
    let mut scale_factor = 1.fct();
    if n.win_ctx.mode().is_headed() {
        // get real monitor data
        let monitor = vars.0.actual_monitor.get().and_then(|id| MONITORS.monitor(id));

        // if monitor query is running now
        let resolving_monitor = monitor.is_none();
        let monitor = monitor.unwrap_or_else(|| vars.0.monitor.get().select_fallback(*id));

        if monitor.id() == MonitorId::fallback() && matches!(vars.0.instance_state.get(), WindowInstanceState::Loading) {
            // window not open yet and view-process provided no monitor (probably loading)
            let handle = WINDOWS.loading_handle(*id, 1.secs());
            RAW_MONITORS_CHANGED_EVENT
                .hook(move |_| {
                    let _hold = &handle;
                    false
                })
                .perm();
        }

        monitor_rect = monitor.px_rect();
        monitor_density = monitor.density().get();
        scale_factor = monitor.scale_factor().get();

        if resolving_monitor {
            let id = monitor.id();
            vars.0.actual_monitor.modify(move |a| {
                if a.set(id) {
                    a.push_tag(SetFromLayoutTag);
                }
            });
        }
    } else {
        debug_assert!(n.win_ctx.mode().is_headless());
        // uses test monitor data
        let m = &n.root.get_mut().headless_monitor;
        if let Some(f) = m.scale_factor {
            scale_factor = f;
        }
        monitor_rect.size = m.size.to_px(scale_factor);
        monitor_density = m.density;
    }

    // metrics for layout of actual root values, relative to screen size
    let font_size_dft = Length::pt_to_px(11.0, scale_factor);
    let monitor_metrics = || {
        LayoutMetrics::new(scale_factor, monitor_rect.size, font_size_dft)
            .with_screen_density(monitor_density)
            .with_direction(DIRECTION_VAR.get())
    };

    // valid auto size config
    let auto_size = if matches!(vars.state().get(), WindowState::Normal) {
        vars.0.auto_size.get()
    } else {
        AutoSize::empty()
    };

    // layout
    n.layout_pass = n.layout_pass.next();
    let (final_size, min_size, max_size) = LAYOUT.with_root_context(n.layout_pass, monitor_metrics(), || {
        // root font size
        let font_size = vars.0.font_size.layout_dft_x(font_size_dft);
        LAYOUT.with_font_size(font_size, || {
            // root constraints
            let min_size = vars.0.min_size.layout();
            let max_size = vars.0.max_size.layout_dft(PxSize::splat(Px::MAX)).max(min_size);
            let mut size = vars.0.actual_size.get().to_px(scale_factor);
            if size.is_empty() {
                // has not open yet, use Normal size for now
                size = vars
                    .0
                    .size
                    .layout_dft(DipSize::new(Dip::new(800), Dip::new(600)).to_px(scale_factor));
            }

            let metrics = LAYOUT.metrics();
            let mut root_cons = metrics.constraints();
            let mut viewport = size;
            if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                root_cons.x = PxConstraints::new_range(min_size.width, max_size.width);
                viewport.width = monitor_rect.size.width;
            } else {
                root_cons.x = PxConstraints::new_exact(size.width);
            }
            if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                root_cons.y = PxConstraints::new_range(min_size.height, max_size.height);
                viewport.height = monitor_rect.size.height;
            } else {
                root_cons.y = PxConstraints::new_exact(size.height);
            }

            // layout
            let metrics = metrics.with_constraints(root_cons).with_viewport(viewport);
            let desired_size = LAYOUT.with_context(metrics, || {
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
    let min_size_dip = min_size.to_dip(scale_factor);
    let max_size_dip = max_size.to_dip(scale_factor);
    vars.0.actual_min_size.set(min_size_dip);
    vars.0.actual_max_size.set(max_size_dip);

    // transition to Loaded (without view)
    if matches!(vars.0.instance_state.get(), WindowInstanceState::Loading) {
        let mut s = WINDOWS_SV.write();
        let w = s.windows.get_mut(id).unwrap();
        if w.pending_loading.strong_count() > 0 {
            // wait loading handles
            tracing::debug!("skipping view-process open, active loading handles");
            return;
        }
        w.pending_loading = std::sync::Weak::<()>::new();
        vars.0.instance_state.set(WindowInstanceState::Loaded { has_view: false });
        if !n.win_ctx.mode().has_renderer() {
            WINDOW_LOAD_EVENT.notify(WindowOpenArgs::now(*id));
        }
    }

    // transition to Loaded (with view) or update view
    match n.win_ctx.mode() {
        WindowMode::Headed => {
            if let Some(view) = &n.view_window {
                // update view window size if is auto_size
                let prev_size = vars.0.actual_size.get().to_px(scale_factor);
                if !auto_size.is_empty() && prev_size != final_size {
                    let mut s = vars.window_state_all();

                    let prev_center = LAYOUT.with_root_context(
                        LayoutPassId::new(),
                        monitor_metrics().with_constraints(PxConstraints2d::new_exact_size(prev_size)),
                        || vars.0.auto_size_origin.layout(),
                    );
                    let new_center = LAYOUT.with_root_context(
                        LayoutPassId::new(),
                        monitor_metrics().with_constraints(PxConstraints2d::new_exact_size(final_size)),
                        || vars.0.auto_size_origin.layout(),
                    );
                    let offset = if prev_size.is_empty() {
                        PxVector::zero()
                    } else {
                        prev_center.to_vector() - new_center.to_vector()
                    };
                    s.restore_rect.origin += offset.to_dip(scale_factor);
                    s.restore_rect.size = size_dip;
                    s.min_size = min_size_dip;
                    s.max_size = max_size_dip;
                    vars.0.restore_rect.modify(move |a| {
                        if a.value().size != size_dip {
                            a.value_mut().size = size_dip;
                        }
                    });
                    let _ = view.set_state(s);
                }
            } else if n.view_opening.is_dummy() {
                // start opening view-process window

                if !VIEW_PROCESS.is_available() || !VIEW_PROCESS.is_connected() {
                    tracing::debug!("skipping view-process open window, no view-process connected");
                    return;
                }

                let id = n.win_ctx.id();

                n.view_opening = RAW_WINDOW_OPEN_EVENT.hook(move |a| {
                    if a.window_id != id {
                        return true;
                    }

                    let mut s = WINDOWS_SV.write();
                    if let Some(w) = s.windows.get_mut(&id) {
                        let vars = w.vars.as_ref().unwrap();
                        vars.0.instance_state.set(WindowInstanceState::Loaded { has_view: true });
                        WINDOW_LOAD_EVENT.notify(WindowOpenArgs::now(id));
                        let r = w.root.as_mut().unwrap();
                        let window = a.window.upgrade().unwrap();

                        if mem::take(&mut r.root.get_mut().start_focused) {
                            let _ = window.focus();
                        }

                        r.renderer = Some(window.renderer());
                        r.view_window = Some(window);
                        r.view_opening = VarHandle::dummy();
                        UPDATES.render_window(id);

                        vars.set_from_view(|v| &v.0.state, a.data.state.state);
                        vars.set_from_view(|v| &v.0.global_position, a.data.state.global_position);
                        vars.set_from_view(|v| &v.0.restore_rect, a.data.state.restore_rect);
                        vars.set_from_view(|v| &v.0.restore_state, a.data.state.restore_state);
                        vars.set_from_view(|v| &v.0.chrome, a.data.state.chrome_visible);

                        vars.set_from_view(|v| &v.0.actual_monitor, a.data.monitor);
                        vars.set_from_view(|v| &v.0.actual_position, a.data.position.1);
                        vars.set_from_view(|v| &v.0.global_position, a.data.position.0);
                        vars.set_from_view(|v| &v.0.actual_size, a.data.size);
                        vars.set_from_view(|v| &v.0.scale_factor, a.data.scale_factor);
                        vars.set_from_view(|v| &v.0.render_mode, a.data.render_mode);
                        vars.set_from_view(|v| &v.0.safe_padding, a.data.safe_padding);
                    }

                    false
                });

                // Layout initial position in the monitor space.
                let mut system_pos = false;
                let mut global_position = PxPoint::zero();
                let mut position = DipPoint::zero();
                match n.root.get_mut().start_position {
                    StartPosition::Default => {
                        let pos = vars.0.position.get();
                        system_pos = pos.x.is_default() || pos.y.is_default();
                        if !system_pos {
                            LAYOUT.with_root_context(n.layout_pass, monitor_metrics(), || {
                                let pos = pos.layout();
                                position = pos.to_dip(scale_factor);
                                global_position = monitor_rect.origin + pos.to_vector();
                            });
                        } else {
                            // in case system does not implement position
                            position = DipPoint::splat(Dip::new(60));
                            global_position = monitor_rect.origin + position.to_px(scale_factor).to_vector();
                        }
                    }
                    start_position => {
                        let screen_rect = match start_position {
                            StartPosition::CenterMonitor => monitor_rect,
                            StartPosition::CenterParent => {
                                if let Some(parent_id) = vars.0.parent.get()
                                    && let Some(parent_vars) = WINDOWS.vars(parent_id)
                                    && matches!(parent_vars.0.instance_state.get(), WindowInstanceState::Loaded { has_view: true })
                                {
                                    PxRect::new(parent_vars.0.global_position.get(), parent_vars.actual_size_px().get())
                                } else {
                                    monitor_rect
                                }
                            }
                            _ => unreachable!(),
                        };

                        let pos = PxPoint::new(
                            (screen_rect.size.width - final_size.width) / Px(2),
                            (screen_rect.size.height - final_size.height) / Px(2),
                        );
                        global_position = screen_rect.origin + pos.to_vector();
                        position = pos.to_dip(scale_factor);
                    }
                };
                vars.0.global_position.set(global_position);
                vars.0.position.set(position);

                let mut state_all = vars.window_state_all();
                state_all.global_position = global_position;
                state_all.restore_rect.origin = position;
                state_all.restore_rect.size = size_dip;
                state_all.min_size = min_size_dip;
                state_all.max_size = max_size_dip;

                let mut ime_area = None;
                {
                    let s = WINDOWS_SV.read();
                    s.focused.with(|f| {
                        if let Some(f) = f
                            && f.window_id() == id
                            && let Some(w) = s.windows.get(&id)
                            && let Some(i) = &w.info
                            && let Some(i) = i.get(f.widget_id())
                            && let Some(r) = i.ime_area()
                        {
                            ime_area = Some(r.to_dip(scale_factor));
                        }
                    });
                }

                let r = VIEW_PROCESS.open_window(WindowRequest::new(
                    zng_view_api::window::WindowId::from_raw(id.get()),
                    vars.0.title.get(),
                    state_all,
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
                    ime_area,
                    vars.0.enabled_buttons.get(),
                    vars.0.system_shutdown_warn.get(),
                    WINDOWS_EXTENSIONS.take_view_extensions_init(id),
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
                if APP.window_mode().is_headless() && !vars.0.instance_state.get().is_loaded() {
                    // simulate focus, for tests mostly
                    let args = RawWindowFocusArgs::now(WINDOWS_SV.read().focused.with(|p| p.as_ref().map(|p| p.window_id())), Some(*id));
                    RAW_WINDOW_FOCUS_EVENT.notify(args);
                }

                if !VIEW_PROCESS.is_connected() {
                    tracing::debug!("skipping view-process open headless, no view-process connected");
                    return;
                }

                // start opening view-process renderer
                let id = n.win_ctx.id();
                n.view_opening = RAW_HEADLESS_OPEN_EVENT.hook(move |a| {
                    if a.window_id != id {
                        return true;
                    }

                    let mut s = WINDOWS_SV.write();
                    if let Some(w) = s.windows.get_mut(&id) {
                        w.vars
                            .as_ref()
                            .unwrap()
                            .0
                            .instance_state
                            .set(WindowInstanceState::Loaded { has_view: true });
                        WINDOW_LOAD_EVENT.notify(WindowOpenArgs::now(id));

                        let r = w.root.as_mut().unwrap();
                        let surface = a.surface.upgrade().unwrap();
                        r.renderer = Some(surface.renderer());
                        r.view_headless = Some(surface);
                        r.view_opening = VarHandle::dummy();

                        UPDATES.render_window(id);
                    }

                    false
                });
                let r = VIEW_PROCESS.open_headless(HeadlessRequest::new(
                    zng_view_api::window::WindowId::from_raw(id.get()),
                    scale_factor,
                    size_dip,
                    n.root.get_mut().render_mode.unwrap_or_else(|| WINDOWS.default_render_mode().get()),
                    WINDOWS_EXTENSIONS.take_view_extensions_init(id),
                ));
                if r.is_err() {
                    tracing::error!("view-process headless surface open request failed, will retry on respawn");
                    n.view_opening = VarHandle::dummy();
                }
            }
        }
        WindowMode::Headless => {
            if APP.window_mode().is_headless() && !vars.0.instance_state.get().is_loaded() {
                // simulate focus, for tests mostly
                let args = RawWindowFocusArgs::now(WINDOWS_SV.read().focused.with(|p| p.as_ref().map(|p| p.window_id())), Some(*id));
                RAW_WINDOW_FOCUS_EVENT.notify(args);
            }
        }
    }
}

pub(crate) fn render(
    (id, n, vars): &mut (WindowId, WindowNode, Option<WindowVars>),
    render_widgets: &Arc<RenderUpdates>,
    render_update_widgets: &Arc<RenderUpdates>,
) {
    if render_widgets.delivery_list().enter_window(*id)
        || (n.frame_id == FrameId::INVALID && render_update_widgets.delivery_list().enter_window(*id))
    {
        // if is pending render or is first frame and is pending render_update

        if let Some(n) = &mut n.nested {
            // if is nested redirect to parent window
            n.pending_render = Some([render_widgets.clone(), render_update_widgets.clone()]);
            if let Some(t) = vars.take().unwrap().0.nest_parent.get() {
                UPDATES.render(t);
            }
            return;
        }

        if matches!(n.win_ctx.mode(), WindowMode::Headed | WindowMode::HeadlessWithRenderer) && n.renderer.is_none() {
            // skip until there is a window.
            tracing::debug!("skipping render, no renderer connected");
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
        }

        if n.wgt_ctx.is_pending_reinit() {
            n.with_root(|_| WIDGET.update());
        }
    } else if render_update_widgets.delivery_list().enter_window(*id) {
        if let Some(n) = &mut n.nested {
            n.pending_render = Some([render_widgets.clone(), render_update_widgets.clone()]);
            if let Some(t) = vars.take().unwrap().0.nest_parent.get() {
                UPDATES.render_update(t);
            }
            return;
        }

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
        }

        if n.wgt_ctx.is_pending_reinit() {
            n.with_root(|_| WIDGET.update());
        }
    }
}

/// UI node that hosts a nested window as defined by [`WINDOWS_EXTENSIONS.register_open_nested_handler`] handler.
///
/// [`WINDOWS_EXTENSIONS.register_open_nested_handler`]: WINDOWS_EXTENSIONS::register_open_nested_handler
pub struct NestedWindowNode {
    window_id: WindowId,
    close_deadline: DeadlineHandle,
}
impl NestedWindowNode {
    pub(crate) fn new(window_id: WindowId) -> Self {
        Self {
            window_id,
            close_deadline: DeadlineHandle::dummy(),
        }
    }

    fn take_node(&self) -> Option<WindowNode> {
        WINDOWS_SV.write().windows.get_mut(&self.window_id)?.root.take()
    }

    fn restore_node(&self, node: WindowNode) {
        if let Some(w) = WINDOWS_SV.write().windows.get_mut(&self.window_id) {
            w.root = Some(node);
        }
    }
}
// layout and render happens during parent window pass, other updates/info are disconnected
impl UiNodeImpl for NestedWindowNode {
    fn children_len(&self) -> usize {
        0
    }
    fn with_child(&mut self, _: usize, _: &mut dyn FnMut(&mut UiNode)) {}

    fn init(&mut self) {
        if let Some(mut n) = self.take_node() {
            // net parent and nest vars

            let inner_vars = WINDOW.with_context(&mut n.win_ctx, || WINDOW.vars());
            let parent_id = WINDOW.id();
            let nest_id = WIDGET.id();
            inner_vars.0.parent.set(Some(parent_id));
            inner_vars.0.nest_parent.set(Some(nest_id));

            // parent var allows modify, lock it
            let this = inner_vars.0.parent.downgrade();
            inner_vars
                .0
                .parent
                .hook(move |a| {
                    if *a.value() != Some(parent_id) {
                        this.upgrade().unwrap().set(Some(parent_id));
                    }
                    true
                })
                .perm();

            self.restore_node(n);
        }

        // cancel close in case of reinit
        self.close_deadline = DeadlineHandle::dummy();
    }

    fn deinit(&mut self) {
        // can be a temporary deinit (for reinit), wait 100ms before closing window
        let id = self.window_id;
        self.close_deadline = TIMERS.on_deadline(
            100.ms(),
            async_hn_once!(|_| {
                let r = WINDOWS.close(id).wait_rsp().await;
                if matches!(r, CloseWindowResult::Cancel) {
                    // did not close ok, force close
                    tracing::error!("nested window {id} already deinited, cannot cancel close");
                    let mut s = WINDOWS_SV.write();
                    let mut windows = IdSet::new();
                    if let Some(mut w) = s.windows.remove(&id) {
                        let vars = w.vars.take().unwrap();
                        vars.0.instance_state.set(WindowInstanceState::Closed);
                        windows.insert(id);

                        if let Some(mut r) = w.root.take() {
                            r.with_root(|n| n.deinit());
                        }
                    }
                    WINDOW_CLOSE_EVENT.notify(WindowCloseArgs::now(windows));
                }
            }),
        );
    }

    fn info(&mut self, info: &mut WidgetInfoBuilder) {
        info.set_meta(*NESTED_WINDOW_INFO_ID, self.window_id);
    }

    fn measure(&mut self, wm: &mut zng_wgt::prelude::WidgetMeasure) -> PxSize {
        let mut r = PxSize::zero();
        // reset context to app level, only parent layout constraints should affect the nested window
        if let Some(mut n) = self.take_node() {
            let mut ctx = LocalContext::capture_filtered(zng_app_context::CaptureFilter::app_only());
            ctx.with_context(|| {
                n.with_root(|n| {
                    r = wm.with_widget(|wm| n.measure(wm));
                })
            });
            self.restore_node(n);
        }
        r
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        let mut r = PxSize::zero();
        if let Some(mut n) = self.take_node() {
            let pending = n.nested.as_mut().unwrap().pending_layout.take();
            let mut ctx = LocalContext::capture_filtered(zng_app_context::CaptureFilter::app_only());
            ctx.with_context(|| {
                n.with_root(|n| {
                    r = if let Some(p) = pending {
                        wl.with_layout_updates(p, |wl| n.layout(wl))
                    } else {
                        wl.with_widget(|wl| n.layout(wl))
                    };
                });
            });
            self.restore_node(n);
        }
        r
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        if let Some(mut n) = self.take_node() {
            let [render_widgets, render_update_widgets] = n.nested.as_mut().unwrap().pending_render.take().unwrap_or_default();
            let mut ctx = LocalContext::capture_filtered(zng_app_context::CaptureFilter::app_only());
            ctx.with_context(|| {
                let root_id = n.wgt_ctx.id();
                let root_bounds = n.wgt_ctx.bounds();
                let info = n.win_ctx.widget_tree();
                n.with_root(|n| {
                    frame.with_nested_window(
                        render_widgets,
                        render_update_widgets,
                        root_id,
                        &root_bounds,
                        &info,
                        FontAntiAliasing::Default,
                        |frame| n.render(frame),
                    );
                });
            });
            self.restore_node(n);
        }
    }

    fn render_update(&mut self, update: &mut FrameUpdate) {
        if let Some(mut n) = self.take_node() {
            let [_, render_update_widgets] = n.nested.as_mut().unwrap().pending_render.take().unwrap_or_default();
            let mut ctx = LocalContext::capture_filtered(zng_app_context::CaptureFilter::app_only());
            ctx.with_context(|| {
                let root_id = n.wgt_ctx.id();
                let root_bounds = n.wgt_ctx.bounds();
                n.with_root(|n| {
                    update.with_nested_window(render_update_widgets, root_id, root_bounds, |update| {
                        n.render_update(update);
                    });
                })
            });
            self.restore_node(n);
        }
    }
}
