use std::{mem, pin::Pin, sync::Arc};

use parking_lot::Mutex;
use zng_app::{
    Deadline, event::EventArgs, hn_once, timer::TIMERS, update::{InfoUpdates, LayoutUpdates, RenderUpdates, UPDATES, WidgetUpdates}, view_process::raw_events::RAW_CHROME_CONFIG_CHANGED_EVENT, widget::{
        VarLayout, WIDGET, WidgetId, info::{WIDGET_TREE_CHANGED_EVENT, WidgetInfoTree, WidgetTreeChangedArgs}
    }, window::{WINDOW, WindowId, WindowMode}
};
use zng_app_context::app_local;
use zng_layout::unit::{Dip, DipToPx, Px, PxConstraints, PxSize, PxToDip};
use zng_task::rayon::prelude::*;
use zng_unique_id::{IdEntry, IdMap, IdSet};
use zng_var::{ResponderVar, ResponseVar, Var, response_done_var, response_var, var, var_default};
use zng_view_api::{
    api_extension::{ApiExtensionId, ApiExtensionPayload},
    config::ChromeConfig,
    window::{RenderMode, WindowState},
};
use zng_wgt::prelude::{DIRECTION_VAR, LAYOUT, LayoutMetrics, UiNode, WidgetInfo, WidgetInfoBuilder, WidgetLayout};

use crate::{AutoSize, CloseWindowResult, MONITORS, ParallelWin, ViewExtensionError, WINDOW_CLOSE_REQUESTED_EVENT, WINDOW_Ext as _, WindowCloseRequestedArgs, WindowInstance, WindowInstanceState, WindowLoadingHandle, WindowNode, WindowRoot, WindowVars};

app_local! {
    pub(crate) static WINDOWS_SV: WindowsService = WindowsService::new();
}
pub(crate) struct WindowsService {
    exit_on_last_close: Var<bool>,
    pub(crate) default_render_mode: Var<RenderMode>,
    parallel: Var<ParallelWin>,
    // Mutex for Sync
    pub(crate) root_extenders: Mutex<Vec<Box<dyn FnMut(WindowRootExtenderArgs) -> UiNode + Send + 'static>>>,

    pub(crate) windows: IdMap<WindowId, WindowInstance>,
    widget_update_buf: Vec<(WindowId, WindowNode)>,
}
impl WindowsService {
    fn new() -> Self {
        Self {
            exit_on_last_close: var(true),
            default_render_mode: var_default(),
            parallel: var_default(),
            root_extenders: Mutex::new(vec![]),

            windows: IdMap::new(),
            widget_update_buf: vec![],
        }
    }

    /// Called to apply widget updates without locking the entire WINDOWS service, reuses a buffer
    ///
    /// Must call `finish_widget_update` to after the update
    fn start_widget_update(&mut self) -> Vec<(WindowId, WindowNode)> {
        let mut s = WINDOWS_SV.write();
        let mut buf = mem::take(&mut s.widget_update_buf);
        buf.extend(self.windows.iter_mut().map(|(k, v)| (*k, v.root.take().unwrap())));
        buf
    }

    fn finish_widget_update(&mut self, mut nodes: Vec<(WindowId, WindowNode)>) {
        let mut s = WINDOWS_SV.write();
        for (id, node) in nodes.drain(..) {
            s.windows.get_mut(&id).unwrap().root = Some(node);
        }
        s.widget_update_buf = nodes;
    }
}

/// Windows service.
pub struct WINDOWS;
impl WINDOWS {
    /// Defines if app process exit should be requested when the last window closes. This is `true` by default.
    ///
    /// This setting does not consider headless windows and is fully ignored in headless apps.
    ///
    /// Note that if [`APP.exit`](APP::exit) is requested directly the windows service will cancel it, request
    /// close for all headed and headless windows, and if all windows close request app exit again, independent
    /// of this setting.
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
    /// window managers don't provide a native chrome, you can use this config with the [`WindowVars::chrome`] setting
    /// in a [`register_root_extender`] to provide a custom fallback chrome.
    ///
    /// [`register_root_extender`]: Self::register_root_extender
    pub fn system_chrome(&self) -> Var<ChromeConfig> {
        RAW_CHROME_CONFIG_CHANGED_EVENT.var_map(|args| Some(args.config), ChromeConfig::default)
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
    pub fn loading_handle(&self, window_id: impl Into<WindowId>, deadline: impl Into<Deadline>) -> Option<WindowLoadingHandle> {
        self.loading_handle_impl(window_id.into(), deadline.into())
    }
    fn loading_handle_impl(&self, window_id: WindowId, deadline: Deadline) -> Option<WindowLoadingHandle> {
        let s = WINDOWS_SV.read();
        let count = s.windows.get(&window_id)?.pending_loading.as_ref()?;
        count.modify(|a| **a += 1);
        let handle = TIMERS.deadline(deadline);
        let count_wk = count.downgrade();
        handle
            .hook(move |a| {
                let elapsed = a.value().has_elapsed();
                if elapsed && let Some(count) = count_wk.upgrade() {
                    count.modify(|c| **c -= 1);
                }
                !elapsed
            })
            .perm();
        let count_wk = count.downgrade();
        handle
            .hook_drop(move || {
                if let Some(count) = count_wk.upgrade() {
                    count.modify(|c| **c -= 1);
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
    /// Returns an error if any of the IDs is not one of the open windows or is only an open request.
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
                    },
                    None => {
                        // did not start building yet, drop
                        w.remove();
                    },
                }
            }
        }
        drop(s);

        if building.is_empty() {
            close_together_all_built(request, r);
        } else {
            UPDATES.run(async move {
                for b in building {
                    b.wait_match(|s| !matches!(s, WindowInstanceState::Building)).await;
                }
                close_together_all_built(request, r);
            }).perm();
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
        self.close_together(set)
    }
}

fn close_together_all_built(request: Vec<WindowId>, r: ResponderVar<CloseWindowResult>) {
    let s = WINDOWS_SV.read();
    let mut open = IdSet::new();
    fn collect(s: &WindowsService, request: &mut dyn Iterator<Item=WindowId>, open: &mut IdSet<WindowId>) {
        for id in request {
            if let Some(w) = s.windows.get(&id) && open.insert(id) {
                collect(s, &mut w.vars.as_ref().unwrap().children().get().into_iter(), open);
            }
        }
    }
    collect(&s, &mut request.into_iter(), &mut open);

    WINDOW_CLOSE_REQUESTED_EVENT.notify(WindowCloseRequestedArgs::now(open));
    WINDOW_CLOSE_REQUESTED_EVENT.on_event(true, hn_once!(|args: &WindowCloseRequestedArgs| {
        if args.propagation().is_stopped() {
            r.respond(CloseWindowResult::Cancel);
            return;
        }
        
        // deinit windows
        let mut nodes;
        let parallel;
        {
            let mut s = WINDOWS_SV.write();
            // UPDATE includes info rebuild
            parallel = s.parallel.get().contains(ParallelWin::UPDATE);
            // take root nodes to allow widgets to use WINDOWS
            nodes = s.start_widget_update();
        };

        let deinit = |(id, n): &mut (WindowId, WindowNode)| {
            if args.windows.contains(id) {
                n.with_root(|n| n.deinit());
            }
        };
        if parallel {
            nodes.par_iter_mut().for_each(deinit);
        } else {
            nodes.iter_mut().for_each(deinit);
        }

        // drop windows
        let mut s = WINDOWS_SV.write();
        for (id, node) in nodes.drain(..) {
            if let IdEntry::Occupied(mut e) = s.windows.entry(id) {
                if args.windows.contains(&id) {
                    e.remove();
                } else {
                    e.get_mut().root = Some(node);
                }
            }
        }
        s.finish_widget_update(nodes);

    })).perm();
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
            nodes = s.start_widget_update();
        };

        // for each window
        let updates = Arc::new(mem::take(updates));
        let rebuild_info = |(id, n): &mut (WindowId, WindowNode)| {
            if updates.delivery_list().enter_window(*id) {
                // rebuild info
                let info = n.with_root(|n| {
                    let mut builder = WidgetInfoBuilder::new(
                        updates.clone(),
                        *id,
                        WINDOW.vars().access_enabled().get(),
                        WIDGET.id(),
                        WIDGET.bounds(),
                        WIDGET.border(),
                        WINDOW.vars().scale_factor().get(),
                    );
                    n.info(&mut builder);

                    builder.finalize(Some(WINDOW.info()), true)
                });

                // apply and notify
                WINDOWS_SV.write().windows.get_mut(id).unwrap().info = Some(info.clone());
                WIDGET_TREE_CHANGED_EVENT.notify(WidgetTreeChangedArgs::now(info, false));
            }
        };
        if parallel {
            nodes.par_iter_mut().for_each(rebuild_info);
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
            nodes = s.start_widget_update();
        };

        // for each window
        let update = |(id, n): &mut (WindowId, WindowNode)| {
            if updates.delivery_list().enter_window(*id) {
                n.with_root(|n| n.update(updates));
            }
        };
        if parallel {
            nodes.par_iter_mut().for_each(update);
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
            nodes = s.start_widget_update();
        };

        // for each window
        let updates = Arc::new(mem::take(updates));
        let layout = |(id, n): &mut (WindowId, WindowNode)| {
            if !updates.delivery_list().enter_window(*id) {
                return;
            }

            let vars = WINDOW.vars();

            // root metrics
            let scale_factor = vars.scale_factor().get();
            let font_size = vars.font_size().layout_dft_x(Dip::new(12).to_px(scale_factor));
            let screen_density = vars.actual_monitor().get().and_then(|id| MONITORS.monitor(id)).map(|m| m.density().get()).unwrap_or_default();
            let size = vars.actual_size().get().to_px(scale_factor);
            let metrics = LayoutMetrics::new(scale_factor, size, font_size)
                .with_screen_density(screen_density)
                .with_direction(DIRECTION_VAR.get());

            // valid auto size config
            let auto_size = if matches!(vars.state().get(), WindowState::Normal) {
                vars.auto_size().get()
            } else {
                AutoSize::empty()
            };

            // layout                
            n.layout_pass = n.layout_pass.next();
            let final_size = LAYOUT.with_root_context(n.layout_pass, metrics, || {
                // root constraints
                let min_size = vars.min_size().layout();
                let max_size = vars.max_size().layout_dft(PxSize::splat(Px::MAX));
                let mut root_cons = LAYOUT.constraints();
                if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                        root_cons.x = PxConstraints::new_range(min_size.width, max_size.width);
                    }
                    if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                        root_cons.y = PxConstraints::new_range(min_size.height, max_size.height);
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

                if n.wgt_ctx.is_pending_reinit() {
                    n.with_root(|_| WIDGET.update());
                }

                final_size
            });

            // apply auto size
            if !auto_size.is_empty() {
                // !!: TODO tag this to avoid requesting another layout
                vars.size().set(final_size.to_dip(scale_factor));
            }

            // !!: TODO open view on first layout
        };
        if parallel {
            nodes.par_iter_mut().for_each(layout);
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
            if render_widgets.delivery_list_mut().has_pending_search() {
                render_widgets
                    .delivery_list_mut()
                    .fulfill_search(s.windows.values().filter_map(|w| w.info.as_ref()));
            }
            if render_update_widgets.delivery_list_mut().has_pending_search() {
                render_update_widgets
                    .delivery_list_mut()
                    .fulfill_search(s.windows.values().filter_map(|w| w.info.as_ref()));
            }
            parallel = s.parallel.get().contains(ParallelWin::RENDER);
            // take root nodes to allow widgets to use WINDOWS
            nodes = s.start_widget_update();
        };

        // for each window
        let render = |(id, n): &mut (WindowId, WindowNode)| {
            if render_widgets.delivery_list().enter_window(*id) {
                n.with_root(|n| {
                    // !!: TODO, same as layout, maybe keep frame in a var too, that allows respawn without layout?
                    todo!()
                })
            } else if render_update_widgets.delivery_list().enter_window(*id) {
                n.with_root(|n| todo!())
            }
        };
        if parallel {
            nodes.par_iter_mut().for_each(render);
        } else {
            nodes.iter_mut().for_each(render);
        }

        // restore root nodes
        WINDOWS_SV.write().finish_widget_update(nodes);
    }
}

/// Arguments for [`WINDOWS.register_root_extender`].
///
/// [`WINDOWS.register_root_extender`]: WINDOWS::register_root_extender
#[non_exhaustive]
pub struct WindowRootExtenderArgs {
    /// The window root content, extender must wrap this node with extension nodes or return
    /// it for no-op.
    pub root: UiNode,
}

#[cfg(feature = "image")]
impl WINDOWS {
    /// Generate an image from the current rendered frame of the window.
    ///
    /// The image is not loaded at the moment of return, it will update when it is loaded.
    ///
    /// If the window is not found the error is reported in the [image error].
    ///
    /// [image error]: zng_ext_image::ImageEntry::error
    pub fn frame_image(&self, window_id: impl Into<WindowId>, mask: Option<zng_ext_image::ImageMaskMode>) -> zng_ext_image::ImageVar {
        todo!()
    }

    /// Generate an image from a rectangular selection of the current rendered frame of the window.
    ///
    /// The image is not loaded at the moment of return, it will update when it is loaded.
    ///
    /// If the window is not found the error is reported in the image error.
    ///
    /// [image error]: zng_ext_image::ImageEntry::error
    pub fn frame_image_rect(
        &self,
        window_id: impl Into<WindowId>,
        mut rect: zng_layout::unit::PxRect,
        mask: Option<zng_ext_image::ImageMaskMode>,
    ) -> zng_ext_image::ImageVar {
        todo!()
    }
}

impl WINDOWS {
    /// Move the window to the front of the operating system Z stack.
    ///
    /// Note that the window is not focused, the [`focus`] operation also moves the window to the front.
    ///
    /// [`always_on_top`]: WindowVars::always_on_top
    /// [`focus`]: Self::focus
    pub fn bring_to_top(&self, window_id: impl Into<WindowId>) {
        todo!()
    }

    /// Request operating system focus for the window.
    ///
    /// The window will be made active and steal keyboard focus from the current focused window.
    ///
    /// Prefer using the `FOCUS` service and advanced `FocusRequest` configs instead of using this method directly, they integrate
    /// with the in app widget focus and internally still use this method.
    ///
    /// If the `window_id` is only associated with an open request it is modified to focus the window on open.
    /// If more than one focus request is made in the same update cycle only the last request is processed.
    pub fn focus(&self, window_id: impl Into<WindowId>) {
        todo!()
    }
}

impl WINDOWS {
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
    /// The closure can use the args to inspect the new window context and optionally convert the request to a [`NestedWindowNode`].
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
    pub fn register_open_nested_handler(&self, handler: impl FnMut(&mut crate::OpenNestedHandlerArgs) + Send + 'static) {
        todo!()
    }

    /// Gets the parent actual window and widget that hosts `maybe_nested` if it is open and nested.
    pub fn nest_parent(&self, maybe_nested: impl Into<WindowId>) -> Option<(WindowId, WidgetId)> {
        todo!()
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
        todo!()
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
        todo!()
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
        todo!()
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
        todo!()
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
        todo!()
    }
}

/// Arguments for the [`WINDOWS.register_open_nested_handler`] handler.
///
/// [`WINDOWS.register_open_nested_handler`]: WINDOWS::register_open_nested_handler
pub struct OpenNestedHandlerArgs {}
