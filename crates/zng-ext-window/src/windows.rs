use std::{any::Any, mem, pin::Pin, sync::Arc};

use parking_lot::Mutex;
use zng_app::{
    APP, Deadline, hn_once,
    timer::TIMERS,
    update::{InfoUpdates, LayoutUpdates, RenderUpdates, UPDATES, WidgetUpdates},
    view_process::{
        VIEW_PROCESS, VIEW_PROCESS_INITED_EVENT,
        raw_events::{RAW_WINDOW_FOCUS_EVENT, RawWindowFocusArgs},
    },
    widget::{
        WIDGET, WidgetId,
        info::{WidgetInfoTree, access::AccessEnabled},
    },
    window::{WINDOW, WINDOWS_APP, WindowId, WindowMode},
};
use zng_app_context::{RunOnDrop, app_local};
use zng_layout::unit::FrequencyUnits;
use zng_task::{ParallelIteratorExt, rayon::prelude::*};
use zng_txt::{ToTxt as _, Txt, formatx};
use zng_unique_id::{IdEntry, IdMap, IdSet};
use zng_var::{ResponderVar, ResponseVar, VARS, Var, const_var, response_done_var, response_var, var, var_default};
use zng_view_api::{
    DragDropId,
    api_extension::{ApiExtensionId, ApiExtensionPayload},
    drag_drop::{DragDropData, DragDropEffect, DragDropError},
    window::{RenderMode, WindowCapability},
};
use zng_wgt::prelude::{InteractionPath, UiNode, WidgetInfo, WidgetInfoBuilder};

use crate::{
    CloseWindowResult, NestedWindowNode, ParallelWin, ViewExtensionError, WINDOW_CLOSE_EVENT, WINDOW_CLOSE_REQUESTED_EVENT,
    WindowCloseArgs, WindowCloseRequestedArgs, WindowInstance, WindowInstanceState, WindowLoadingHandle, WindowNode, WindowRoot,
    WindowVars,
};

app_local! {
    pub(crate) static WINDOWS_SV: WindowsService = WindowsService::new();
}
pub(crate) struct WindowsService {
    exit_on_last_close: Var<bool>,
    pub(crate) default_render_mode: Var<RenderMode>,
    parallel: Var<ParallelWin>,
    pub(crate) frame_duration_from_monitor: Var<bool>,
    // Mutex for Sync
    pub(crate) root_extenders: Mutex<Vec<Box<dyn FnMut(WindowRootExtenderArgs) -> UiNode + Send + 'static>>>,
    pub(crate) open_nested_handlers: Mutex<Vec<Box<dyn FnMut(&mut OpenNestedHandlerArgs) + Send + 'static>>>,

    pub(crate) windows: IdMap<WindowId, WindowInstance>,
    widget_update_buf: Vec<(WindowId, WindowNode, Option<WindowVars>)>,

    pub(crate) focused: Var<Option<InteractionPath>>,
    focused_set: bool,
}
impl WindowsService {
    fn new() -> Self {
        WINDOWS_APP.init_info_provider(Box::new(WINDOWS));
        #[cfg(feature = "image")]
        zng_ext_image::IMAGES_WINDOW.hook_render_windows_service(Box::new(WINDOWS));
        crate::hooks::hook_events();
        let sv = Self {
            exit_on_last_close: var(true),
            default_render_mode: var_default(),
            parallel: var_default(),
            frame_duration_from_monitor: var(true),
            root_extenders: Mutex::new(vec![]),
            open_nested_handlers: Mutex::new(vec![]),

            windows: IdMap::new(),
            widget_update_buf: vec![],

            focused: const_var(None),
            focused_set: false,
        };
        // init frame_duration_from_monitor,
        // windows bind their refresh_rate to also call set_frame_duration
        sv.frame_duration_from_monitor
            .hook(|a| {
                if *a.value() {
                    WINDOWS_SV.read().set_frame_duration();
                }
                true
            })
            .perm();
        sv
    }

    /// Called to apply widget updates without locking the entire WINDOWS service, reuses a buffer
    ///
    /// Must call `finish_widget_update` to after the update
    fn start_widget_update(&mut self, clone_vars: bool) -> Vec<(WindowId, WindowNode, Option<WindowVars>)> {
        let mut buf = mem::take(&mut self.widget_update_buf);
        if clone_vars {
            buf.extend(
                self.windows
                    .iter_mut()
                    .filter_map(|(k, v)| Some((*k, v.root.take()?, v.vars.clone()))),
            );
        } else {
            buf.extend(self.windows.iter_mut().filter_map(|(k, v)| Some((*k, v.root.take()?, None))));
        }
        buf
    }

    fn finish_widget_update(&mut self, mut nodes: Vec<(WindowId, WindowNode, Option<WindowVars>)>) {
        for (id, node, _) in nodes.drain(..) {
            self.windows.get_mut(&id).unwrap().root = Some(node);
        }
        self.widget_update_buf = nodes;
    }

    pub(crate) fn set_frame_duration(&self) {
        if self.frame_duration_from_monitor.get() {
            let max = self
                .windows
                .values()
                .filter_map(|v| v.vars.as_ref())
                .map(|v| v.0.refresh_rate.get())
                .max()
                .unwrap_or(60.hertz());
            VARS.frame_duration().set(max.period());
        }
    }
}

/// Windows service.
pub struct WINDOWS;
impl WINDOWS {
    /// Defines if app process exit should be requested when the last window closes. This is `true` by default.
    ///
    /// This setting does not consider headless windows and is fully ignored in headless apps.
    ///
    /// Note that if [`APP.exit`] is requested directly the windows service will cancel it, request
    /// close for all headed and headless windows, and if all windows close request app exit again, independent
    /// of this setting.
    ///
    /// [`APP.exit`]: zng_app::APP::exit
    pub fn exit_on_last_close(&self) -> Var<bool> {
        WINDOWS_SV.read().exit_on_last_close.clone()
    }

    /// Defines the render mode of windows opened by this service.
    ///
    /// Note that this setting only affects windows opened after it is changed, also the view-process may select
    /// a different render mode if it cannot support the requested mode.
    pub fn default_render_mode(&self) -> Var<RenderMode> {
        WINDOWS_SV.read().default_render_mode.clone()
    }

    /// Defines what window operations can run in parallel, between windows.
    ///
    /// Note that this config is for parallel execution between windows, see the `parallel` property for parallel execution
    /// within windows and widgets.
    ///
    /// See [`ParallelWin`] for the options.
    pub fn parallel(&self) -> Var<ParallelWin> {
        WINDOWS_SV.read().parallel.clone()
    }

    /// Variable that tracks the OS window manager configuration for the window chrome.
    ///
    /// The chrome (also known as window decorations) defines the title bar, window buttons and window border. Some
    /// window managers don't provide a native chrome, you can use this config with [`WindowVars::chrome`]
    /// in a [`register_root_extender`] to provide a custom fallback chrome, the main crate `zng` also provides a custom
    /// chrome fallback.
    ///
    /// [`register_root_extender`]: WINDOWS_EXTENSIONS::register_root_extender
    pub fn system_chrome(&self) -> Var<bool> {
        VIEW_PROCESS_INITED_EVENT.var_map(
            |a| Some(a.window.contains(WindowCapability::SYSTEM_CHROME)),
            || VIEW_PROCESS.info().window.contains(WindowCapability::SYSTEM_CHROME),
        )
    }

    /// Defines if the headed window in the monitor with the faster refresh rate defines the [`VARS.frame_duration`].
    ///
    /// This is `true` by default.
    ///
    /// Note that the app-process blocks anyway if the view-process is behind two frames at any window, so even if
    /// the refresh rate is very high the app will not be overwhelmed. The app will consume more power in higher refresh rate.
    ///
    /// If this is disabled the frame duration is reset to the default of 60FPS (~16.668ms).
    ///
    /// [`VARS.frame_duration`]: VARS::frame_duration
    pub fn frame_duration_from_monitor(&self) -> Var<bool> {
        WINDOWS_SV.read().frame_duration_from_monitor.clone()
    }
}
impl WINDOWS {
    /// Requests a new window.
    ///
    /// The `new_window` future runs inside the new [`WINDOW`] context.
    ///
    /// Returns a response var that will update once when the window starts building, the [`WindowVars::instance_state`] can be
    /// use to continue monitoring the window.
    ///
    /// An update cycle is processed between the end of `new_window` and the window init, this means that you
    /// can use the context [`WINDOW`] to set variables that will be read on init with the new value.
    ///
    /// Note that there are no *window handles*, the window is controlled in the service using the ID or from the inside.
    ///
    /// # Panics
    ///
    /// If the `window_id` is already assigned to an open or opening window.
    pub fn open<F: Future<Output = WindowRoot> + Send + 'static>(
        &self,
        window_id: impl Into<WindowId>,
        new_window: impl IntoFuture<IntoFuture = F>,
    ) -> ResponseVar<WindowVars> {
        self.open_impl(window_id.into(), Box::pin(new_window.into_future()), WindowMode::Headed, false)
    }

    /// Focus a window if it is open or loading, otherwise opens it focused.
    ///
    /// Returns a variable that updates once the window starts building or is already open. You can
    /// track the focused status using [`WindowVars::is_focused`].
    pub fn focus_or_open<F: Future<Output = WindowRoot> + Send + 'static>(
        &self,
        window_id: impl Into<WindowId>,
        new_window: impl IntoFuture<IntoFuture = F>,
    ) -> ResponseVar<WindowVars> {
        self.open_impl(window_id.into(), Box::pin(new_window.into_future()), WindowMode::Headed, true)
    }
    /// Requests a new headless window.
    ///
    /// This is similar to `open`, but the window will not show on screen and can optionally not even have a renderer.
    ///
    /// # Panics
    ///
    /// If the `window_id` is already assigned to an open or opening window.
    pub fn open_headless<F: Future<Output = WindowRoot> + Send + 'static>(
        &self,
        window_id: impl Into<WindowId>,
        new_window: impl IntoFuture<IntoFuture = F>,
        with_renderer: bool,
    ) -> ResponseVar<WindowVars> {
        self.open_impl(
            window_id.into(),
            Box::pin(new_window.into_future()),
            if with_renderer {
                WindowMode::HeadlessWithRenderer
            } else {
                WindowMode::Headless
            },
            false,
        )
    }
    fn open_impl(
        &self,
        window_id: WindowId,
        new_window: Pin<Box<dyn Future<Output = WindowRoot> + Send + 'static>>,
        mode: WindowMode,
        focus_existing: bool,
    ) -> ResponseVar<WindowVars> {
        let mode = match (mode, APP.window_mode()) {
            (m, WindowMode::Headed) => m,
            (m, WindowMode::HeadlessWithRenderer) => {
                if m.is_headless() {
                    m
                } else {
                    WindowMode::HeadlessWithRenderer
                }
            }
            (_, WindowMode::Headless) => WindowMode::Headless,
        };

        let mut s = WINDOWS_SV.write();
        match s.windows.entry(window_id) {
            IdEntry::Vacant(e) => {
                let (r, rsp) = response_var();
                e.insert(WindowInstance::new(window_id, mode, new_window, r));
                rsp
            }
            IdEntry::Occupied(e) => {
                if focus_existing {
                    match &e.get().vars {
                        Some(v) => {
                            v.0.focused.set(true);
                            response_done_var(v.clone())
                        }
                        None => {
                            // just requested, did not start building yet
                            let (r, rsp) = response_var();
                            UPDATES.once_update("WINDOWS wait build start", move || match WINDOWS.vars(window_id) {
                                Some(v) => r.respond(v),
                                None => tracing::error!("window {window_id:?} build did not start"),
                            });
                            rsp
                        }
                    }
                } else {
                    panic!("{window_id:?} is already open or opening");
                }
            }
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
    /// Returns `None` if the window has already loaded or is not found.
    pub fn loading_handle(
        &self,
        window_id: impl Into<WindowId>,
        deadline: impl Into<Deadline>,
        debug_name: impl Into<Txt>,
    ) -> Option<WindowLoadingHandle> {
        self.loading_handle_impl(window_id.into(), deadline.into(), debug_name.into())
    }
    fn loading_handle_impl(&self, window_id: WindowId, deadline: Deadline, debug_name: Txt) -> Option<WindowLoadingHandle> {
        let mut s = WINDOWS_SV.write();

        let window = s.windows.get_mut(&window_id)?;
        if let Some(vars) = &window.vars
            && !matches!(
                vars.0.instance_state.get(),
                WindowInstanceState::Building | WindowInstanceState::Loading
            )
        {
            tracing::debug!("cannot get loading handle `{debug_name}` for window `{window_id:?}`, already loaded");
            return None;
        }
        let h = if let Some(h) = window.pending_loading.upgrade() {
            h
        } else {
            let h: Arc<dyn Any + Send + Sync> = Arc::new(RunOnDrop::new(move || {
                if let Some(vars) = WINDOWS.vars(window_id) {
                    vars.0.instance_state.modify(move |a| {
                        if matches!(a.value(), WindowInstanceState::Loading) {
                            UPDATES.layout_window(window_id);
                        }
                    });
                }
            }));
            window.pending_loading = Arc::downgrade(&h);
            h
        };

        let handle = TIMERS.deadline(deadline);
        handle
            .hook(move |a| {
                let _hold = &h;
                if a.value().has_elapsed() {
                    tracing::debug!("loading handle `{debug_name}` timeout");
                    false
                } else {
                    true
                }
            })
            .perm();
        Some(WindowLoadingHandle(handle))
    }

    /// Starts closing a window, the operation can be canceled by listeners of
    /// [`WINDOW_CLOSE_REQUESTED_EVENT`]. If the window has children they are closed together.
    ///
    /// Returns a response var that will update once with the result of the operation.
    ///
    /// If the window is not found returns `Closed`.
    pub fn close(&self, window_id: impl Into<WindowId>) -> ResponseVar<CloseWindowResult> {
        self.close_together([window_id.into()])
    }

    /// Starts closing multiple windows together, the operation can be canceled by listeners of
    /// [`WINDOW_CLOSE_REQUESTED_EVENT`]. If canceled none of the windows are closed. Children of each window
    /// are also selected the close together.
    ///
    /// Returns a response var that will update once with the result of the operation. Returns
    /// [`Cancel`] if `windows` is empty.
    ///
    /// [`Cancel`]: CloseWindowResult::Cancel
    pub fn close_together(&self, windows: impl IntoIterator<Item = WindowId>) -> ResponseVar<CloseWindowResult> {
        self.close_together_impl(windows.into_iter().collect())
    }
    fn close_together_impl(&self, request: Vec<WindowId>) -> ResponseVar<CloseWindowResult> {
        let (r, rsp) = response_var();

        let mut s = WINDOWS_SV.write();

        // collect requests that are still building
        let mut building = vec![];
        for id in request.iter().copied() {
            if let IdEntry::Occupied(w) = s.windows.entry(id) {
                match &w.get().vars {
                    Some(v) => {
                        let state = v.instance_state();
                        if let WindowInstanceState::Building = state.get() {
                            building.push(state);
                        }
                    }
                    None => {
                        // did not start building yet, drop
                        w.remove();
                    }
                }
            }
        }
        drop(s);

        if building.is_empty() {
            close_together_all_built(request, r);
        } else {
            UPDATES
                .run(async move {
                    for b in building {
                        b.wait_match(|s| !matches!(s, WindowInstanceState::Building)).await;
                    }
                    close_together_all_built(request, r);
                })
                .perm();
        }

        rsp
    }

    /// Starts closing all open windows together, the operation can be canceled by listeners of
    /// [`WINDOW_CLOSE_REQUESTED_EVENT`]. If canceled none of the windows are closed.
    ///
    /// Returns a response var that will update once with the result of the operation. Returns
    /// [`Cancel`] if no window is open.
    ///
    /// [`Cancel`]: CloseWindowResult::Cancel
    pub fn close_all(&self) -> ResponseVar<CloseWindowResult> {
        let set: Vec<_> = WINDOWS_SV.read().windows.keys().copied().collect();
        self.close_together_impl(set)
    }
}

fn close_together_all_built(request: Vec<WindowId>, r: ResponderVar<CloseWindowResult>) {
    let s = WINDOWS_SV.read();
    let mut open = IdSet::new();
    fn collect(s: &WindowsService, request: &mut dyn Iterator<Item = WindowId>, open: &mut IdSet<WindowId>) {
        for id in request {
            if let Some(w) = s.windows.get(&id)
                && open.insert(id)
            {
                collect(s, &mut w.vars.as_ref().unwrap().children().get().into_iter(), open);
            }
        }
    }
    collect(&s, &mut request.into_iter(), &mut open);

    WINDOW_CLOSE_REQUESTED_EVENT.notify(WindowCloseRequestedArgs::now(open));
    WINDOW_CLOSE_REQUESTED_EVENT
        .on_event(
            true,
            hn_once!(|args: &WindowCloseRequestedArgs| {
                if args.propagation.is_stopped() {
                    r.respond(CloseWindowResult::Cancel);
                    return;
                }

                // notify close first, so that the closed windows receive on_close.
                WINDOW_CLOSE_EVENT.notify(WindowCloseArgs::now(args.windows.clone()));
                WINDOW_CLOSE_EVENT
                    .on_event(
                        true,
                        hn_once!(|args: &WindowCloseArgs| {
                            // deinit windows
                            let mut nodes;
                            let parallel;
                            {
                                let mut s = WINDOWS_SV.write();
                                // UPDATE includes info rebuild
                                parallel = s.parallel.get().contains(ParallelWin::UPDATE);
                                // take root nodes to allow widgets to use WINDOWS
                                nodes = s.start_widget_update(true);
                            };

                            let deinit = |(id, n, _): &mut (WindowId, WindowNode, Option<WindowVars>)| {
                                if args.windows.contains(id) {
                                    n.with_root(|n| n.deinit());
                                }
                            };
                            if parallel {
                                nodes.par_iter_mut().with_ctx().for_each(deinit);
                            } else {
                                nodes.iter_mut().for_each(deinit);
                            }

                            // drop windows
                            let mut s = WINDOWS_SV.write();
                            s.finish_widget_update(nodes);

                            for id in &args.windows {
                                if let Some(w) = s.windows.remove(id) {
                                    let vars = w.vars.unwrap();
                                    vars.0.instance_state.set(WindowInstanceState::Closed);
                                    if vars.0.focused.get() && APP.window_mode().is_headless() {
                                        // simulate focus loss in headless app (mostly for tests)
                                        RAW_WINDOW_FOCUS_EVENT.notify(RawWindowFocusArgs::now(Some(*id), None));
                                    }
                                }
                            }

                            if s.exit_on_last_close.get()
                                && !s.windows.iter().any(|w| w.1.mode.is_headed())
                                && APP.window_mode().is_headed()
                            {
                                zng_app::APP.exit();
                            }

                            r.respond(CloseWindowResult::Closed);
                        }),
                    )
                    .perm();

                // notify
                WINDOW_CLOSE_EVENT.notify(WindowCloseArgs::now(args.windows.clone()));
            }),
        )
        .perm();
}

impl WINDOWS {
    /// Gets if the window is headed or headless.
    ///
    /// Returns `None` if the window is not found.
    pub fn mode(&self, window_id: impl Into<WindowId>) -> Option<WindowMode> {
        self.mode_impl(window_id.into())
    }
    fn mode_impl(&self, window_id: WindowId) -> Option<WindowMode> {
        Some(WINDOWS_SV.read().windows.get(&window_id)?.mode)
    }

    /// Returns a shared reference the variables that control the window, if the window exists.
    pub fn vars(&self, window_id: impl Into<WindowId>) -> Option<WindowVars> {
        self.vars_impl(window_id.into())
    }
    fn vars_impl(&self, window_id: WindowId) -> Option<WindowVars> {
        WINDOWS_SV.read().windows.get(&window_id)?.vars.clone()
    }

    /// Get the latest info tree for the window.
    pub fn widget_tree(&self, id: impl Into<WindowId>) -> Option<WidgetInfoTree> {
        zng_app::window::WindowsService::widget_tree(self, id.into())
    }

    /// Search for the widget in the latest info tree of each open window.
    pub fn widget_info(&self, id: impl Into<WidgetId>) -> Option<WidgetInfo> {
        zng_app::window::WindowsService::widget_info(self, id.into())
    }

    /// Returns shared references to the widget trees of each open window.
    pub fn widget_trees(&self) -> Vec<WidgetInfoTree> {
        WINDOWS_SV.read().windows.values().filter_map(|v| v.info.clone()).collect()
    }
}
impl zng_app::window::WindowsService for WINDOWS {
    fn widget_tree(&self, id: WindowId) -> Option<WidgetInfoTree> {
        WINDOWS_SV.read().windows.get(&id)?.info.clone()
    }

    fn widget_info(&self, id: WidgetId) -> Option<WidgetInfo> {
        WINDOWS_SV.read().windows.values().find_map(|w| w.info.as_ref()?.get(id))
    }

    fn update_info(&self, updates: &mut InfoUpdates) {
        let mut nodes;
        let parallel;
        {
            let mut s = WINDOWS_SV.write();
            // fulfill delivery search
            if updates.delivery_list_mut().has_pending_search() {
                updates
                    .delivery_list_mut()
                    .fulfill_search(s.windows.values().filter_map(|w| w.info.as_ref()));
            }
            // UPDATE includes info rebuild
            parallel = s.parallel.get().contains(ParallelWin::UPDATE);
            // take root nodes to allow widgets to use WINDOWS
            nodes = s.start_widget_update(true);
        };

        // for each window
        let updates = Arc::new(mem::take(updates));
        let rebuild_info = |(id, n, vars): &mut (WindowId, WindowNode, Option<WindowVars>)| {
            if updates.delivery_list().enter_window(*id) {
                // rebuild info
                let vars = vars.as_ref().unwrap();
                let access_enabled = vars.access_enabled().get();
                let info = n.with_root(|n| {
                    let mut builder = WidgetInfoBuilder::new(
                        updates.clone(),
                        *id,
                        access_enabled,
                        WIDGET.id(),
                        WIDGET.bounds(),
                        WIDGET.border(),
                        vars.scale_factor().get(),
                    );
                    n.info(&mut builder);

                    builder.finalize(WINDOW.try_info(), true)
                });
                n.win_ctx.set_widget_tree(info.clone());
                WINDOWS_SV.write().windows.get_mut(id).unwrap().info = Some(info.clone());

                if access_enabled.contains(AccessEnabled::VIEW) {
                    // access data is send in the frame display list
                    UPDATES.render_window(n.win_ctx.id());
                }
            }
        };
        if parallel {
            nodes.par_iter_mut().with_ctx().for_each(rebuild_info);
        } else {
            nodes.iter_mut().for_each(rebuild_info);
        }

        // restore root nodes
        WINDOWS_SV.write().finish_widget_update(nodes);
    }

    fn update_widgets(&self, updates: &mut WidgetUpdates) {
        let mut nodes;
        let parallel;
        {
            let mut s = WINDOWS_SV.write();
            // fulfill delivery search
            if updates.delivery_list_mut().has_pending_search() {
                updates
                    .delivery_list_mut()
                    .fulfill_search(s.windows.values().filter_map(|w| w.info.as_ref()));
            }
            parallel = s.parallel.get().contains(ParallelWin::UPDATE);
            // take root nodes to allow widgets to use WINDOWS
            nodes = s.start_widget_update(false);
        };

        // for each window
        let update = |(id, n, _): &mut (WindowId, WindowNode, _)| {
            if updates.delivery_list().enter_window(*id) {
                if n.wgt_ctx.take_reinit() {
                    n.with_root(|n| {
                        n.deinit();
                        n.init();
                    })
                } else {
                    n.with_root(|n| n.update(updates));
                }
            }
        };
        if parallel {
            nodes.par_iter_mut().with_ctx().for_each(update);
        } else {
            nodes.iter_mut().for_each(update);
        }

        // restore root nodes
        WINDOWS_SV.write().finish_widget_update(nodes);
    }

    fn update_layout(&self, updates: &mut LayoutUpdates) {
        let mut nodes;
        let parallel;
        {
            let mut s = WINDOWS_SV.write();
            // fulfill delivery search
            if updates.delivery_list_mut().has_pending_search() {
                updates
                    .delivery_list_mut()
                    .fulfill_search(s.windows.values().filter_map(|w| w.info.as_ref()));
            }
            parallel = s.parallel.get().contains(ParallelWin::LAYOUT);
            // take root nodes to allow widgets to use WINDOWS
            nodes = s.start_widget_update(true);
        };

        // for each window
        let updates = Arc::new(mem::take(updates));
        let layout = |args: &mut (WindowId, WindowNode, Option<WindowVars>)| {
            crate::window::layout_open_view(args, &updates);
        };
        if parallel {
            nodes.par_iter_mut().with_ctx().for_each(layout);
        } else {
            nodes.iter_mut().for_each(layout);
        }

        // restore root nodes
        WINDOWS_SV.write().finish_widget_update(nodes);
    }

    fn update_render(&self, render_widgets: &mut RenderUpdates, render_update_widgets: &mut RenderUpdates) {
        let mut nodes;
        let parallel;
        {
            let mut s = WINDOWS_SV.write();
            // fulfill delivery search
            for d in [&mut *render_widgets, &mut *render_update_widgets] {
                if d.delivery_list_mut().has_pending_search() {
                    d.delivery_list_mut()
                        .fulfill_search(s.windows.values().filter_map(|w| w.info.as_ref()));
                }
            }
            parallel = s.parallel.get().contains(ParallelWin::RENDER);
            // take root nodes to allow widgets to use WINDOWS
            nodes = s.start_widget_update(true);
        };

        // for each window
        let render_widgets = Arc::new(mem::take(render_widgets));
        let render_update_widgets = Arc::new(mem::take(render_update_widgets));
        let render = |args: &mut (WindowId, WindowNode, Option<WindowVars>)| {
            crate::window::render(args, &render_widgets, &render_update_widgets);
        };
        if parallel {
            nodes.par_iter_mut().with_ctx().for_each(render);
        } else {
            nodes.iter_mut().for_each(render);
        }

        // restore root nodes
        WINDOWS_SV.write().finish_widget_update(nodes);
    }
}

#[cfg(feature = "image")]
impl WINDOWS {
    /// Generate an image from the current rendered frame of the window or the first frame of the window.
    ///
    /// The image is not loaded at the moment of return, it will update when the frame pixels are copied.
    ///
    /// If the window is not found or an error is reported in the [image error].
    ///
    /// [image error]: zng_ext_image::ImageEntry::error
    pub fn frame_image(&self, window_id: impl Into<WindowId>, mask: Option<zng_ext_image::ImageMaskMode>) -> zng_ext_image::ImageVar {
        self.frame_image_task(window_id.into(), Box::new(move |v| v.frame_image(mask)))
    }

    /// Generate an image from a rectangular selection of the current rendered frame of the window, or of the first frame of the window.
    ///
    // The image is not loaded at the moment of return, it will update when the frame pixels are copied.
    ///
    /// If the window is not found the error is reported in the [image error].
    ///
    /// [image error]: zng_ext_image::ImageEntry::error
    pub fn frame_image_rect(
        &self,
        window_id: impl Into<WindowId>,
        rect: zng_layout::unit::PxRect,
        mask: Option<zng_ext_image::ImageMaskMode>,
    ) -> zng_ext_image::ImageVar {
        self.frame_image_task(window_id.into(), Box::new(move |v| v.frame_image_rect(rect, mask)))
    }

    fn frame_image_task(
        &self,
        window_id: WindowId,
        task: Box<
            dyn FnOnce(
                    &zng_app::view_process::ViewRenderer,
                ) -> Result<zng_app::view_process::ViewImageHandle, zng_task::channel::ChannelError>
                + Send
                + Sync,
        >,
    ) -> zng_ext_image::ImageVar {
        use zng_ext_image::*;
        use zng_txt::*;
        use zng_var::*;

        let r = var(ImageEntry::new_loading());
        let rr = r.read_only();

        UPDATES.once_update("WINDOWS.frame_image", move || {
            let s = WINDOWS_SV.read();
            if let Some(w) = &s.windows.get(&window_id) {
                if !w.mode.has_renderer() {
                    return r.set(ImageEntry::new_error(formatx!("window {window_id} has no renderer")));
                }

                if let Some(n) = &w.root
                    && let Some(v) = &n.renderer
                    && n.frame_id != zng_view_api::window::FrameId::INVALID
                {
                    // already has a frame
                    return match task(v) {
                        Ok(handle) => {
                            let img = IMAGES.register(None, (handle, Default::default()));
                            img.set_bind(&r).perm();
                            r.hold(img).perm();
                        }
                        Err(e) => r.set(ImageEntry::new_error(e.to_txt())),
                    };
                }

                // first frame not available yet, await it
                use zng_app::view_process::raw_events::RAW_FRAME_RENDERED_EVENT;
                let mut task = Some(task);
                RAW_FRAME_RENDERED_EVENT
                    .hook(move |args| {
                        if args.window_id == window_id {
                            let img = WINDOWS.frame_image_task(window_id, task.take().unwrap());
                            img.set_bind(&r).perm();
                            r.hold(r.clone()).perm();
                            false
                        } else {
                            WINDOWS_SV.read().windows.contains_key(&window_id)
                        }
                    })
                    .perm();
            } else {
                r.set(ImageEntry::new_error(formatx!("window {window_id} not found")));
            }
        });

        rr
    }
}
impl WINDOWS {
    /// Move the window to the front of the operating system Z stack.
    ///
    /// Note that the window is not focused, the `FOCUS.focus_window` operation brings to top and sets focus.
    pub fn bring_to_top(&self, window_id: impl Into<WindowId>) {
        self.bring_to_top_impl(window_id.into());
    }
    fn bring_to_top_impl(&self, window_id: WindowId) {
        UPDATES.once_update("WINDOWS.bring_to_top", move || {
            if let Some(w) = WINDOWS_SV.read().windows.get(&window_id)
                && let Some(root) = &w.root
                && let Some(v) = &root.view_window
            {
                let _ = v.bring_to_top();
            } else {
                tracing::error!("cannot bring_to_top {window_id}, not open in view-process");
            }
        });
    }
}

/// Windows focus service integration.
///
/// The `FOCUS` uses this.
#[expect(non_camel_case_types)]
pub struct WINDOWS_FOCUS;
impl WINDOWS_FOCUS {
    /// Setup a var that is controlled by the focus service and tracks the focused widget.
    ///
    /// This must be called by the focus implementation only.
    pub fn hook_focus_service(&self, focused: Var<Option<InteractionPath>>) {
        let mut s = WINDOWS_SV.write();
        assert!(!s.focused_set, "focus service already hooked");
        s.focused = focused;
        let mut handler = crate::hooks::focused_widget_handler();
        s.focused
            .hook(move |a| {
                handler(a.value());
                true
            })
            .perm();
    }

    /// Request operating system focus for the window.
    ///
    /// The window will be made active and steal keyboard focus from the current focused window.
    pub fn focus(&self, window_id: impl Into<WindowId>) {
        self.focus_impl(window_id.into());
    }
    fn focus_impl(&self, window_id: WindowId) {
        UPDATES.once_update("WINDOWS.focus", move || {
            if let Some(w) = WINDOWS_SV.read().windows.get(&window_id)
                && let Some(root) = &w.root
                && let Some(v) = &root.view_window
            {
                if !RAW_WINDOW_FOCUS_EVENT.with(|a| matches!(a.latest(), Some(w) if w.new_focus == Some(window_id))) {
                    let _ = v.focus();
                } else {
                    // multiple repeated focus requests have weird effects in Windows,
                    // it may even return focus to previous window
                    tracing::debug!("skipping focus window request, already focused");
                }
            } else {
                tracing::error!("cannot focus {window_id}, not open in view-process");
            }
        });
    }
}

/// Windows extensions hooks.
#[expect(non_camel_case_types)]
pub struct WINDOWS_EXTENSIONS;

/// Arguments for [`WINDOWS_EXTENSIONS.register_root_extender`].
///
/// [`WINDOWS_EXTENSIONS.register_root_extender`]: WINDOWS_EXTENSIONS::register_root_extender
#[non_exhaustive]
pub struct WindowRootExtenderArgs {
    /// The window root content, extender must wrap this node with extension nodes or return
    /// it for no-op.
    pub root: UiNode,
}
impl WINDOWS_EXTENSIONS {
    /// Register the closure `extender` to be called with the root of every new window starting on the next update.
    ///
    /// The closure returns the new root node that will be passed to any other root extender until
    /// the actual final root node is created. The closure is called in the [`WINDOW`] context of the new window,
    /// so it can be used to modify the window context too.
    ///
    /// This is an advanced API that enables app wide features, like themes, to inject context in every new window. The
    /// extender is called in the context of the window, after the window creation future has completed.
    ///
    /// Note that the *root* node passed to the extender is the child node of the `WindowRoot` widget, not the widget itself.
    /// The extended root will be wrapped in the root widget node, that is, the final root widget will be
    /// `root(extender_nodes(CONTEXT(EVENT(..))))`, so extension nodes should operate as `CONTEXT` properties.
    ///
    /// Note that for themes the `zng-wgt-window` crate provides a `register_style_fn` API that is built over this
    /// method and more oriented for theming.
    pub fn register_root_extender(&self, extender: impl FnMut(WindowRootExtenderArgs) -> UiNode + Send + 'static) {
        self.register_root_extender_impl(Box::new(extender));
    }
    fn register_root_extender_impl(&self, extender: Box<dyn FnMut(WindowRootExtenderArgs) -> UiNode + Send + 'static>) {
        UPDATES.once_update("WINDOWS.register_root_extender", move || {
            WINDOWS_SV.write().root_extenders.get_mut().push(extender);
        });
    }

    /// Register the closure `handler` to be called for every new window starting on the next update.
    ///
    /// The closure is called in the new [`WINDOW`] context and can optionally call [`OpenNestedHandlerArgs::nest`] to convert to a nested window.
    /// Nested windows can be manipulated using the `WINDOWS` API just like other windows, but are layout and rendered inside another window.
    ///
    /// This is primarily an adapter for mobile platforms that only support one real window, it accelerates cross platform support from
    /// projects originally desktop only.
    ///
    /// Note that this API is not recommended for implementing features such as *window docking* or
    /// *tabbing*, for that you probably need to model *tabs* as objects that can outlive their host windows and use [`ArcNode`]
    /// to transfer the content between host windows.
    ///
    /// [`NestedWindowNode`]: crate::NestedWindowNode
    /// [`ArcNode`]: zng_app::widget::node::ArcNode
    pub fn register_open_nested_handler(&self, handler: impl FnMut(&mut OpenNestedHandlerArgs) + Send + 'static) {
        self.register_open_nested_handler_impl(Box::new(handler));
    }
    fn register_open_nested_handler_impl(&self, handler: Box<dyn FnMut(&mut OpenNestedHandlerArgs) + Send + 'static>) {
        UPDATES.once_update("WINDOWS.register_open_nested_handler", move || {
            WINDOWS_SV.write().open_nested_handlers.get_mut().push(handler);
        });
    }

    /// Add a view-process extension payload to the window request for the view-process.
    ///
    /// This will only work if called on the first [`UiNode::init`] and at most the first [`UiNode::layout`] of the window.
    ///
    /// The payload is dropped after it is send, this method must be called again on [`VIEW_PROCESS_INITED_EVENT`]
    /// to reinitialize the extensions after view-process respawn.
    ///
    /// [`UiNode::init`]: zng_app::widget::node::UiNode::init
    /// [`UiNode::layout`]: zng_app::widget::node::UiNode::layout
    /// [`VIEW_PROCESS_INITED_EVENT`]: zng_app::view_process::VIEW_PROCESS_INITED_EVENT
    pub fn view_extensions_init(
        &self,
        window_id: impl Into<WindowId>,
        extension_id: ApiExtensionId,
        request: ApiExtensionPayload,
    ) -> Result<(), ViewExtensionError> {
        self.view_extensions_init_impl(window_id.into(), extension_id, request)
    }
    fn view_extensions_init_impl(
        &self,
        window_id: WindowId,
        extension_id: ApiExtensionId,
        request: ApiExtensionPayload,
    ) -> Result<(), ViewExtensionError> {
        match WINDOWS_SV.write().windows.get_mut(&window_id) {
            Some(w) => {
                if matches!(w.mode, WindowMode::HeadlessWithRenderer) {
                    Err(ViewExtensionError::NotOpenInViewProcess(window_id))
                } else if let Some(exts) = &mut w.extensions_init {
                    exts.push((extension_id, request));
                    Ok(())
                } else {
                    Err(ViewExtensionError::AlreadyOpenInViewProcess(window_id))
                }
            }
            None => Err(ViewExtensionError::WindowNotFound(window_id)),
        }
    }
    pub(crate) fn take_view_extensions_init(&self, window_id: WindowId) -> Vec<(ApiExtensionId, ApiExtensionPayload)> {
        WINDOWS_SV
            .write()
            .windows
            .get_mut(&window_id)
            .and_then(|w| w.extensions_init.take())
            .unwrap_or_default()
    }

    /// Call a view-process headed window extension with custom encoded payload.
    ///
    /// Note that unlike most service methods this calls happens immediately.
    pub fn view_window_extension_raw(
        &self,
        window_id: impl Into<WindowId>,
        extension_id: ApiExtensionId,
        request: ApiExtensionPayload,
    ) -> Result<ApiExtensionPayload, ViewExtensionError> {
        self.view_window_extension_raw_impl(window_id.into(), extension_id, request)
    }
    fn view_window_extension_raw_impl(
        &self,
        window_id: WindowId,
        extension_id: ApiExtensionId,
        request: ApiExtensionPayload,
    ) -> Result<ApiExtensionPayload, ViewExtensionError> {
        if let Some(w) = WINDOWS_SV.read().windows.get(&window_id) {
            if let Some(r) = &w.root
                && let Some(v) = &r.view_window
            {
                match v.window_extension_raw(extension_id, request) {
                    Ok(r) => Ok(r),
                    Err(_) => Err(ViewExtensionError::Disconnected),
                }
            } else {
                Err(ViewExtensionError::NotOpenInViewProcess(window_id))
            }
        } else {
            Err(ViewExtensionError::WindowNotFound(window_id))
        }
    }

    /// Call a headed window extension with serialized payload.
    ///
    /// Note that unlike most service methods this call happens immediately.
    pub fn view_window_extension<I, O>(
        &self,
        window_id: impl Into<WindowId>,
        extension_id: ApiExtensionId,
        request: &I,
    ) -> Result<O, ViewExtensionError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        let window_id = window_id.into();
        if let Some(w) = WINDOWS_SV.read().windows.get(&window_id) {
            if let Some(r) = &w.root
                && let Some(v) = &r.view_window
            {
                match v.window_extension(extension_id, request) {
                    Ok(r) => match r {
                        Ok(r) => Ok(r),
                        Err(e) => Err(ViewExtensionError::Api(e)),
                    },
                    Err(_) => Err(ViewExtensionError::Disconnected),
                }
            } else {
                Err(ViewExtensionError::NotOpenInViewProcess(window_id))
            }
        } else {
            Err(ViewExtensionError::WindowNotFound(window_id))
        }
    }

    /// Call a view-process render extension with custom encoded payload for the renderer associated with the window.
    ///
    /// Note that unlike most service methods this call happens immediately.
    pub fn view_render_extension_raw(
        &self,
        window_id: impl Into<WindowId>,
        extension_id: ApiExtensionId,
        request: ApiExtensionPayload,
    ) -> Result<ApiExtensionPayload, ViewExtensionError> {
        self.view_render_extension_raw_impl(window_id.into(), extension_id, request)
    }
    fn view_render_extension_raw_impl(
        &self,
        window_id: WindowId,
        extension_id: ApiExtensionId,
        request: ApiExtensionPayload,
    ) -> Result<ApiExtensionPayload, ViewExtensionError> {
        if let Some(w) = WINDOWS_SV.read().windows.get(&window_id) {
            if let Some(r) = &w.root
                && let Some(v) = &r.renderer
            {
                match v.render_extension_raw(extension_id, request) {
                    Ok(r) => Ok(r),
                    Err(_) => Err(ViewExtensionError::Disconnected),
                }
            } else {
                Err(ViewExtensionError::NotOpenInViewProcess(window_id))
            }
        } else {
            Err(ViewExtensionError::WindowNotFound(window_id))
        }
    }

    /// Call a render extension with serialized payload for the renderer associated with the window.
    ///
    /// Note that unlike most service methods this call happens immediately.
    pub fn view_render_extension<I, O>(
        &self,
        window_id: impl Into<WindowId>,
        extension_id: ApiExtensionId,
        request: &I,
    ) -> Result<O, ViewExtensionError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        let window_id = window_id.into();
        if let Some(w) = WINDOWS_SV.read().windows.get(&window_id) {
            if let Some(r) = &w.root
                && let Some(v) = &r.renderer
            {
                match v.render_extension(extension_id, request) {
                    Ok(r) => match r {
                        Ok(r) => Ok(r),
                        Err(e) => Err(ViewExtensionError::Api(e)),
                    },
                    Err(_) => Err(ViewExtensionError::Disconnected),
                }
            } else {
                Err(ViewExtensionError::NotOpenInViewProcess(window_id))
            }
        } else {
            Err(ViewExtensionError::WindowNotFound(window_id))
        }
    }
}

/// Windows dialog service integration.
#[expect(non_camel_case_types)]
pub struct WINDOWS_DIALOG;

impl WINDOWS_DIALOG {
    /// Show a native message dialog for the window.
    ///
    /// The dialog can be modal in the view-process, in the app-process it is always async, the
    /// response var will update once when the user responds to the dialog.
    ///
    /// Consider using the `DIALOG` service instead of the method directly.
    pub fn native_message_dialog(
        &self,
        window_id: impl Into<WindowId>,
        dialog: zng_view_api::dialog::MsgDialog,
    ) -> ResponseVar<zng_view_api::dialog::MsgDialogResponse> {
        self.native_message_dialog_impl(window_id.into(), dialog)
    }
    fn native_message_dialog_impl(
        &self,
        window_id: WindowId,
        dialog: zng_view_api::dialog::MsgDialog,
    ) -> ResponseVar<zng_view_api::dialog::MsgDialogResponse> {
        let (r, rsp) = response_var();

        UPDATES.once_update("WINDOWS.native_message_dialog", move || {
            use zng_view_api::dialog::MsgDialogResponse;
            if let Some(w) = WINDOWS_SV.read().windows.get(&window_id)
                && let Some(root) = &w.root
                && let Some(v) = &root.view_window
            {
                if let Err(e) = v.message_dialog(dialog, r.clone()) {
                    r.respond(MsgDialogResponse::Error(formatx!("cannot show dialog, {e}")));
                }
            } else {
                r.respond(MsgDialogResponse::Error(formatx!(
                    "cannot show dialog, {window_id} not open in view-process"
                )));
            }
        });

        rsp
    }

    /// Show a native file dialog for the window.
    ///
    /// The dialog can be modal in the view-process, in the app-process it is always async, the
    /// response var will update once when the user responds to the dialog.
    ///
    /// Consider using the `DIALOG` service instead of the method directly.
    pub fn native_file_dialog(
        &self,
        window_id: impl Into<WindowId>,
        dialog: zng_view_api::dialog::FileDialog,
    ) -> ResponseVar<zng_view_api::dialog::FileDialogResponse> {
        self.native_file_dialog_impl(window_id.into(), dialog)
    }
    fn native_file_dialog_impl(
        &self,
        window_id: WindowId,
        dialog: zng_view_api::dialog::FileDialog,
    ) -> ResponseVar<zng_view_api::dialog::FileDialogResponse> {
        let (r, rsp) = response_var();

        UPDATES.once_update("WINDOWS.native_file_dialog", move || {
            use zng_view_api::dialog::FileDialogResponse;
            if let Some(w) = WINDOWS_SV.read().windows.get(&window_id)
                && let Some(root) = &w.root
                && let Some(v) = &root.view_window
            {
                if let Err(e) = v.file_dialog(dialog, r.clone()) {
                    r.respond(FileDialogResponse::Error(formatx!("cannot show dialog, {e}")));
                }
            } else {
                r.respond(FileDialogResponse::Error(formatx!(
                    "cannot show dialog, {window_id} not open in view-process"
                )));
            }
        });

        rsp
    }

    /// Window operations supported by the current view-process instance for headed windows.
    ///
    /// Not all window operations may be available, depending on the operating system and build. When an operation
    /// is not available an error is logged and otherwise ignored.
    pub fn available_operations(&self) -> WindowCapability {
        VIEW_PROCESS.info().window
    }
}

/// Arguments for the [`WINDOWS_EXTENSIONS.register_open_nested_handler`] handler.
///
/// [`WINDOWS_EXTENSIONS.register_open_nested_handler`]: WINDOWS_EXTENSIONS::register_open_nested_handler
pub struct OpenNestedHandlerArgs {
    pub(crate) has_nested: bool,
}
impl OpenNestedHandlerArgs {
    pub(crate) fn new() -> Self {
        Self { has_nested: true }
    }

    /// Instantiate a node that layouts and renders the window content.
    ///
    /// Calling this will stop the normal window chrome from opening, the caller is responsible for inserting the node into the
    /// main window layout.
    ///
    /// Note that the window will notify *open* like normal, but it will only be visible on this node.
    pub fn nest(&mut self) -> NestedWindowNode {
        NestedWindowNode::new(WINDOW.id())
    }
}

/// Raw drag&drop API.
#[allow(non_camel_case_types)]
pub struct WINDOWS_DRAG_DROP;
impl WINDOWS_DRAG_DROP {
    /// Start of drag&drop from the window.
    ///
    /// Note that unlike normal service methods this applies immediately.
    pub fn start_drag_drop(
        &self,
        window_id: WindowId,
        data: Vec<DragDropData>,
        allowed_effects: DragDropEffect,
    ) -> Result<DragDropId, DragDropError> {
        if let Some(w) = WINDOWS_SV.read().windows.get(&window_id)
            && let Some(root) = &w.root
            && let Some(v) = &root.view_window
        {
            v.start_drag_drop(data, allowed_effects)
                .map_err(|e| DragDropError::CannotStart(e.to_txt()))?
        } else {
            Err(DragDropError::CannotStart(formatx!("window {window_id} not open in view-process")))
        }
    }

    /// Notify the drag source of what effect was applied for a received drag&drop.
    ///
    /// Note that unlike normal service methods this applies immediately.
    pub fn drag_dropped(&self, window_id: WindowId, drop_id: DragDropId, applied: DragDropEffect) {
        if let Some(w) = WINDOWS_SV.read().windows.get(&window_id)
            && let Some(root) = &w.root
            && let Some(v) = &root.view_window
        {
            let _ = v.drag_dropped(drop_id, applied);
        }
    }
}

#[cfg(feature = "image")]
impl zng_ext_image::ImageRenderWindowsService for WINDOWS {
    fn clone_boxed(&self) -> Box<dyn zng_ext_image::ImageRenderWindowsService> {
        Box::new(WINDOWS)
    }

    fn new_window_root(&self, node: UiNode, render_mode: RenderMode) -> Box<dyn zng_ext_image::ImageRenderWindowRoot> {
        Box::new(WindowRoot::new_container(
            WidgetId::new_unique(),
            crate::StartPosition::Default,
            false,
            true,
            Some(render_mode),
            crate::HeadlessMonitor::default(),
            false,
            node,
        ))
    }

    fn set_parent_in_window_context(&self, parent_id: WindowId) {
        use crate::WINDOW_Ext as _;

        WINDOW.vars().0.parent.set(parent_id);
    }

    fn enable_frame_capture_in_window_context(&self, mask: Option<zng_ext_image::ImageMaskMode>) {
        use crate::WINDOW_Ext as _;

        let mode = if let Some(mask) = mask {
            crate::FrameCaptureMode::AllMask(mask)
        } else {
            crate::FrameCaptureMode::All
        };
        WINDOW.vars().0.frame_capture_mode.set(mode);
    }

    fn open_headless_window(&self, new_window_root: Box<dyn FnOnce() -> Box<dyn zng_ext_image::ImageRenderWindowRoot> + Send>) {
        WINDOWS.open_headless(
            WindowId::new_unique(),
            async move {
                use crate::WINDOW_Ext as _;

                let root: Box<dyn std::any::Any> = new_window_root();
                let w = *root.downcast::<WindowRoot>().expect("expected `WindowRoot` in image render window");
                let vars = WINDOW.vars();
                vars.auto_size().set(true);
                vars.min_size().set(zng_layout::unit::Length::Px(zng_layout::unit::Px(1)));
                w
            },
            true,
        );
    }

    fn close_window(&self, window_id: WindowId) {
        WINDOWS.close(window_id);
    }
}
#[cfg(feature = "image")]
impl zng_ext_image::ImageRenderWindowRoot for WindowRoot {}
