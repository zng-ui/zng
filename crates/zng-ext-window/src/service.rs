use std::{any::Any, future::Future, mem, sync::Arc};

use parking_lot::Mutex;
use zng_app::{
    app_hn_once,
    event::AnyEventArgs,
    timer::{DeadlineHandle, TIMERS},
    update::{EventUpdate, InfoUpdates, LayoutUpdates, RenderUpdates, WidgetUpdates, UPDATES},
    view_process::{
        self,
        raw_events::{
            RAW_COLORS_CONFIG_CHANGED_EVENT, RAW_IMAGE_LOADED_EVENT, RAW_IMAGE_LOAD_ERROR_EVENT, RAW_WINDOW_CLOSE_EVENT,
            RAW_WINDOW_CLOSE_REQUESTED_EVENT, RAW_WINDOW_FOCUS_EVENT,
        },
        ViewImage, ViewRenderer, ViewWindowOrHeadless, VIEW_PROCESS, VIEW_PROCESS_INITED_EVENT,
    },
    widget::{
        info::{InteractionPath, WidgetInfo, WidgetInfoTree},
        node::{BoxedUiNode, NilUiNode, UiNode},
        UiTaskWidget, WidgetId,
    },
    window::{WindowCtx, WindowId, WindowMode, WINDOW},
    AppEventSender, Deadline, APP, EXIT_REQUESTED_EVENT,
};
use zng_app_context::app_local;

use zng_color::{colors::ACCENT_COLOR_VAR, COLOR_SCHEME_VAR};
use zng_ext_image::{ImageRenderWindowRoot, ImageRenderWindowsService, ImageVar, Img};
use zng_layout::unit::TimeUnits as _;
use zng_layout::unit::{Factor, FactorUnits, LengthUnits, PxRect};
use zng_task::{
    rayon::iter::{IntoParallelRefMutIterator, ParallelIterator},
    ParallelIteratorExt, UiTask,
};
use zng_txt::{formatx, Txt};
use zng_unique_id::{IdMap, IdSet};
use zng_var::{
    impl_from_and_into_var, response_done_var, response_var, types::WeakArcVar, var, AnyWeakVar, ArcVar, BoxedVar, LocalVar, ResponderVar,
    ResponseVar, Var, WeakVar,
};
use zng_view_api::{
    api_extension::{ApiExtensionId, ApiExtensionPayload},
    config::ColorsConfig,
    image::ImageMaskMode,
    window::{RenderMode, WindowState},
    ViewProcessOffline,
};
use zng_wgt::node::with_context_var;

use crate::{
    cmd::WindowCommands, control::WindowCtrl, CloseWindowResult, FrameCaptureMode, HeadlessMonitor, StartPosition, ViewExtensionError,
    WindowCloseArgs, WindowCloseRequestedArgs, WindowFocusChangedArgs, WindowLoadingHandle, WindowNotFound, WindowOpenArgs, WindowRoot,
    WindowVars, FRAME_IMAGE_READY_EVENT, MONITORS, WINDOW_CLOSE_EVENT, WINDOW_CLOSE_REQUESTED_EVENT, WINDOW_FOCUS_CHANGED_EVENT,
    WINDOW_LOAD_EVENT, WINDOW_VARS_ID,
};

app_local! {
    pub(super) static WINDOWS_SV: WindowsService = WindowsService::new();
    static FOCUS_SV: BoxedVar<Option<InteractionPath>> = LocalVar(None).boxed();
}
pub(super) struct WindowsService {
    exit_on_last_close: ArcVar<bool>,
    default_render_mode: ArcVar<RenderMode>,
    parallel: ArcVar<ParallelWin>,
    root_extenders: Mutex<Vec<Box<dyn FnMut(WindowRootExtenderArgs) -> BoxedUiNode + Send>>>, // Mutex for +Sync only.

    windows: IdMap<WindowId, AppWindow>,
    windows_info: IdMap<WindowId, AppWindowInfo>,

    open_loading: IdMap<WindowId, WindowLoading>,
    open_requests: Vec<OpenWindowRequest>,
    open_tasks: Vec<AppWindowTask>,

    close_requests: Vec<CloseWindowRequest>,
    close_responders: IdMap<WindowId, Vec<ResponderVar<CloseWindowResult>>>,
    exit_on_close: bool,

    focus_request: Option<WindowId>,
    bring_to_top_requests: Vec<WindowId>,

    frame_images: Vec<WeakArcVar<Img>>,

    loading_deadline: Option<DeadlineHandle>,
    latest_colors_cfg: ColorsConfig,

    view_window_tasks: Vec<ViewWindowTask>,
}
impl WindowsService {
    fn new() -> Self {
        Self {
            exit_on_last_close: var(true),
            default_render_mode: var(RenderMode::default()),
            root_extenders: Mutex::new(vec![]),
            parallel: var(ParallelWin::default()),
            windows: IdMap::default(),
            windows_info: IdMap::default(),
            open_loading: IdMap::new(),
            open_tasks: vec![],
            open_requests: Vec::with_capacity(1),
            exit_on_close: false,
            close_responders: IdMap::default(),
            close_requests: vec![],
            focus_request: None,
            bring_to_top_requests: vec![],
            frame_images: vec![],
            loading_deadline: None,
            latest_colors_cfg: ColorsConfig::default(),
            view_window_tasks: vec![],
        }
    }

    fn open_impl(&mut self, id: WindowId, new_window: UiTask<WindowRoot>, force_headless: Option<WindowMode>) -> ResponseVar<WindowId> {
        let (responder, response) = response_var();
        let request = OpenWindowRequest {
            id,
            new: Mutex::new(new_window),
            force_headless,
            responder,
        };
        self.open_requests.push(request);
        self.open_loading.insert(id, WindowLoading::new());
        UPDATES.update(None);

        response
    }

    fn loading_handle_impl(&mut self, window_id: WindowId, deadline: Deadline) -> Option<WindowLoadingHandle> {
        let mut handle = None;

        if let Some(info) = self.windows_info.get_mut(&window_id) {
            // window already opened, check if not loaded
            if !info.is_loaded {
                handle = Some(info.loading_handle.new_handle(UPDATES.sender(), deadline))
            }

            // drop timer to nearest deadline, will recreate in the next update.
            self.loading_deadline = None;
        } else if let Some(h) = self.open_loading.get_mut(&window_id) {
            // window not opened yet
            handle = Some(h.new_handle(UPDATES.sender(), deadline));
        }

        handle
    }

    fn close_together(&mut self, windows: impl IntoIterator<Item = WindowId>) -> Result<ResponseVar<CloseWindowResult>, WindowNotFound> {
        let mut group = IdSet::default();

        for w in windows {
            if !self.windows_info.contains_key(&w) {
                return Err(WindowNotFound(w));
            }
            group.insert(w);
        }

        if group.is_empty() {
            return Ok(response_done_var(CloseWindowResult::Cancel));
        }

        let (responder, response) = response_var();
        self.close_requests.push(CloseWindowRequest { responder, windows: group });
        UPDATES.update(None);

        Ok(response)
    }

    fn frame_image_impl(
        &mut self,
        window_id: WindowId,
        action: impl FnOnce(&ViewRenderer) -> std::result::Result<ViewImage, ViewProcessOffline>,
    ) -> ImageVar {
        if let Some(w) = self.windows_info.get(&window_id) {
            if let Some(r) = &w.view {
                match action(&r.renderer()) {
                    Ok(img) => {
                        let img = Img::new(img);
                        let img = var(img);
                        self.frame_images.retain(|i| i.strong_count() > 0);
                        self.frame_images.push(img.downgrade());
                        img.read_only()
                    }
                    Err(_) => var(Img::dummy(Some(formatx!("{}", WindowNotFound(window_id))))).read_only(),
                }
            } else {
                var(Img::dummy(Some(formatx!("window `{window_id}` is headless without renderer")))).read_only()
            }
        } else {
            var(Img::dummy(Some(formatx!("{}", WindowNotFound(window_id))))).read_only()
        }
    }

    fn view_window_task(&mut self, window_id: WindowId, task: impl FnOnce(Option<&view_process::ViewWindow>) + Send + 'static) {
        self.view_window_tasks.push(ViewWindowTask {
            window_id,
            task: Mutex::new(Box::new(task)),
        });
    }

    fn take_requests(
        &mut self,
    ) -> (
        Vec<OpenWindowRequest>,
        Vec<AppWindowTask>,
        Vec<CloseWindowRequest>,
        Option<WindowId>,
        Vec<WindowId>,
        Vec<ViewWindowTask>,
    ) {
        (
            mem::take(&mut self.open_requests),
            mem::take(&mut self.open_tasks),
            mem::take(&mut self.close_requests),
            self.focus_request.take(),
            mem::take(&mut self.bring_to_top_requests),
            mem::take(&mut self.view_window_tasks),
        )
    }
}

bitflags! {
    /// Defines what window operations can run in parallel, between windows.
    ///
    /// Note that this does not define parallelism inside the window, see [`WINDOWS.parallel`] for more details.
    ///
    /// [`WINDOWS.parallel`]: WINDOWS::parallel
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    pub struct ParallelWin: u8 {
        /// Windows can init, deinit, update and rebuild info in parallel.
        const UPDATE = 0b0001;
        /// Windows can handle event updates in parallel.
        const EVENT = 0b0010;
        /// Windows can layout in parallel.
        const LAYOUT = 0b0100;
        /// Windows with pending render or render update generate display lists in parallel.
        const RENDER = 0b1000;
    }
}
impl Default for ParallelWin {
    /// Is all by default.
    fn default() -> Self {
        Self::all()
    }
}
impl_from_and_into_var! {
    fn from(all: bool) -> ParallelWin {
        if all {
            ParallelWin::all()
        } else {
            ParallelWin::empty()
        }
    }
}

/// Windows service.
///
/// # Provider
///
/// This service is provided by the [`WindowManager`].
///
/// [`WindowManager`]: crate::WindowManager
pub struct WINDOWS;
impl WINDOWS {
    /// Defines if app process exit should be requested when the last window closes. This is `true` by default.
    ///
    /// This setting does not consider headless windows and is fully ignored in headless apps.
    ///
    /// Note that if [`APP.exit`](APP::exit) is requested directly the windows service will cancel it, request
    /// close for all headed and headless windows, and if all windows close request app exit again, independent
    /// of this setting.
    pub fn exit_on_last_close(&self) -> ArcVar<bool> {
        WINDOWS_SV.read().exit_on_last_close.clone()
    }

    /// Defines the render mode of windows opened by this service.
    ///
    /// Note that this setting only affects windows opened after it is changed, also the view-process may select
    /// a different render mode if it cannot support the requested mode.
    pub fn default_render_mode(&self) -> ArcVar<RenderMode> {
        WINDOWS_SV.read().default_render_mode.clone()
    }

    /// Defines what window operations can run in parallel, between windows.
    ///
    /// Note that this config is for parallel execution between windows, see the `parallel` property for parallel execution
    /// within windows and widgets.
    ///
    /// See [`ParallelWin`] for the options.
    pub fn parallel(&self) -> ArcVar<ParallelWin> {
        WINDOWS_SV.read().parallel.clone()
    }

    /// Requests a new window.
    ///
    /// The `new_window` future runs in an [`UiTask`] inside the new [`WINDOW`] context.
    ///
    /// Returns a response variable that will update once when the window is opened, note that while the [`WINDOW`] is
    /// available in the `new_window` argument already, the window is only available in this service after
    /// the returned variable updates. Also note that the window might not be fully [loaded] yet.
    ///
    /// An update cycle is processed between the end of `new_window` and the window init, this means that you
    /// can use the context [`WINDOW`] to set variables that will be read on init with the new value.
    ///
    /// [loaded]: Self::is_loaded
    /// [`UiTask`]: zng_task::UiTask
    /// [`WINDOW`]: zng_app::window::WINDOW
    pub fn open(&self, new_window: impl Future<Output = WindowRoot> + Send + 'static) -> ResponseVar<WindowId> {
        WINDOWS_SV
            .write()
            .open_impl(WindowId::new_unique(), UiTask::new(None, new_window), None)
    }

    /// Requests a new window with pre-defined ID.
    ///
    /// # Panics
    ///
    /// If the `window_id` is already assigned to an open or opening window.
    pub fn open_id(
        &self,
        window_id: impl Into<WindowId>,
        new_window: impl Future<Output = WindowRoot> + Send + 'static,
    ) -> ResponseVar<WindowId> {
        let window_id = window_id.into();
        self.assert_id_unused(window_id);
        WINDOWS_SV.write().open_impl(window_id, UiTask::new(None, new_window), None)
    }

    /// Requests a new headless window.
    ///
    /// Headless windows don't show on screen, but if `with_renderer` is `true` they will still render frames.
    ///
    /// Note that in a headless app the [`open`] method also creates headless windows, this method
    /// creates headless windows even in a headed app.
    ///
    /// [`open`]: WINDOWS::open
    pub fn open_headless(
        &self,
        new_window: impl Future<Output = WindowRoot> + Send + 'static,
        with_renderer: bool,
    ) -> ResponseVar<WindowId> {
        WINDOWS_SV.write().open_impl(
            WindowId::new_unique(),
            UiTask::new(None, new_window),
            Some(if with_renderer {
                WindowMode::HeadlessWithRenderer
            } else {
                WindowMode::Headless
            }),
        )
    }

    /// Requests a new headless window with pre-defined ID.
    ///
    /// # Panics
    ///
    /// If the `window_id` is already assigned to an open or opening window.
    pub fn open_headless_id(
        &self,
        window_id: impl Into<WindowId>,
        new_window: impl Future<Output = WindowRoot> + Send + 'static,
        with_renderer: bool,
    ) -> ResponseVar<WindowId> {
        let window_id = window_id.into();
        self.assert_id_unused(window_id);
        WINDOWS_SV.write().open_impl(
            window_id,
            UiTask::new(None, new_window),
            Some(if with_renderer {
                WindowMode::HeadlessWithRenderer
            } else {
                WindowMode::Headless
            }),
        )
    }

    #[track_caller]
    fn assert_id_unused(&self, id: WindowId) {
        let w = WINDOWS_SV.read();
        if w.windows_info.contains_key(&id) || w.open_loading.contains_key(&id) {
            panic!("window id `{id:?}` is already in use")
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
        WINDOWS_SV.write().loading_handle_impl(window_id.into(), deadline.into())
    }

    /// Starts closing a window, the operation can be canceled by listeners of
    /// [`WINDOW_CLOSE_REQUESTED_EVENT`]. If the window has children they are closed together.
    ///
    /// Returns a response var that will update once with the result of the operation.
    ///
    /// Returns an error if the `window_id` is not one of the open windows or is only an open request.
    pub fn close(&self, window_id: impl Into<WindowId>) -> Result<ResponseVar<CloseWindowResult>, WindowNotFound> {
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
    pub fn close_together(&self, windows: impl IntoIterator<Item = WindowId>) -> Result<ResponseVar<CloseWindowResult>, WindowNotFound> {
        WINDOWS_SV.write().close_together(windows)
    }

    /// Starts closing all open windows together, the operation can be canceled by listeners of
    /// [`WINDOW_CLOSE_REQUESTED_EVENT`]. If canceled none of the windows are closed.
    ///
    /// Returns a response var that will update once with the result of the operation. Returns
    /// [`Cancel`] if no window is open.
    ///
    /// [`Cancel`]: CloseWindowResult::Cancel
    pub fn close_all(&self) -> ResponseVar<CloseWindowResult> {
        let set: Vec<_> = WINDOWS_SV.read().windows_info.keys().copied().collect();
        self.close_together(set).unwrap()
    }

    /// Get the window [mode].
    ///
    /// This value indicates if the window is headless or not.
    ///
    /// Returns an error if the `window_id` is not one of the open windows or is only an open request.
    ///
    /// [mode]: WindowMode
    pub fn mode(&self, window_id: impl Into<WindowId>) -> Result<WindowMode, WindowNotFound> {
        let window_id = window_id.into();
        WINDOWS_SV
            .read()
            .windows_info
            .get(&window_id)
            .map(|w| w.mode)
            .ok_or(WindowNotFound(window_id))
    }

    /// Returns a shared reference to the latest widget tree info for the window.
    ///
    /// Returns an error if the `window_id` is not one of the open windows or is only an open request.
    pub fn widget_tree(&self, window_id: impl Into<WindowId>) -> Result<WidgetInfoTree, WindowNotFound> {
        let window_id = window_id.into();
        WINDOWS_SV
            .read()
            .windows_info
            .get(&window_id)
            .map(|w| w.widget_tree.clone())
            .ok_or(WindowNotFound(window_id))
    }

    /// Search for the widget info in all windows.
    pub fn widget_info(&self, widget_id: impl Into<WidgetId>) -> Option<WidgetInfo> {
        let widget_id = widget_id.into();
        WINDOWS_SV.read().windows_info.values().find_map(|w| w.widget_tree.get(widget_id))
    }

    /// Generate an image from the current rendered frame of the window.
    ///
    /// The image is not loaded at the moment of return, it will update when it is loaded.
    ///
    /// If the window is not found the error is reported in the [image error].
    ///
    /// [image error]: zng_ext_image::Img::error
    pub fn frame_image(&self, window_id: impl Into<WindowId>, mask: Option<ImageMaskMode>) -> ImageVar {
        WINDOWS_SV
            .write()
            .frame_image_impl(window_id.into(), move |vr| vr.frame_image(mask))
    }

    /// Generate an image from a rectangular selection of the current rendered frame of the window.
    ///
    /// The image is not loaded at the moment of return, it will update when it is loaded.
    ///
    /// If the window is not found the error is reported in the image error.
    ///
    /// [image error]: zng_ext_image::Img::error
    pub fn frame_image_rect(&self, window_id: impl Into<WindowId>, rect: PxRect, mask: Option<ImageMaskMode>) -> ImageVar {
        WINDOWS_SV
            .write()
            .frame_image_impl(window_id.into(), |vr| vr.frame_image_rect(rect, mask))
    }

    /// Returns a shared reference the variables that control the window.
    ///
    /// Returns an error if the `window_id` is not one of the open windows or is only an open request.
    pub fn vars(&self, window_id: impl Into<WindowId>) -> Result<WindowVars, WindowNotFound> {
        let window_id = window_id.into();
        WINDOWS_SV
            .read()
            .windows_info
            .get(&window_id)
            .map(|w| w.vars.clone())
            .ok_or(WindowNotFound(window_id))
    }

    /// Gets if the window is focused in the operating system.
    ///
    /// Returns an error if the `window_id` is not one of the open windows, returns `false` if the `window_id` is
    /// one of the open requests.
    pub fn is_focused(&self, window_id: impl Into<WindowId>) -> Result<bool, WindowNotFound> {
        let window_id = window_id.into();
        let w = WINDOWS_SV.read();
        if let Some(w) = w.windows_info.get(&window_id) {
            Ok(w.is_focused)
        } else if w.open_loading.contains_key(&window_id) {
            Ok(false)
        } else {
            Err(WindowNotFound(window_id))
        }
    }

    /// Returns shared references to the widget trees of each open window.
    pub fn widget_trees(&self) -> Vec<WidgetInfoTree> {
        WINDOWS_SV.read().windows_info.values().map(|w| w.widget_tree.clone()).collect()
    }

    /// Gets the id of the window that is focused in the operating system.
    pub fn focused_window_id(&self) -> Option<WindowId> {
        WINDOWS_SV.read().windows_info.values().find(|w| w.is_focused).map(|w| w.id)
    }

    /// Returns a shared reference to the focused window's info.
    pub fn focused_info(&self) -> Option<WidgetInfoTree> {
        WINDOWS_SV
            .read()
            .windows_info
            .values()
            .find(|w| w.is_focused)
            .map(|w| w.widget_tree.clone())
    }

    /// Returns `true` if the window is open.
    pub fn is_open(&self, window_id: impl Into<WindowId>) -> bool {
        WINDOWS_SV.read().windows_info.contains_key(&window_id.into())
    }

    /// Returns `true` if the `window_id` is associated with a pending window open request or open task.
    ///
    /// Window open requests start polling after each update.
    pub fn is_opening(&self, window_id: impl Into<WindowId>) -> bool {
        let window_id = window_id.into();
        let sv = WINDOWS_SV.read();
        sv.open_loading.contains_key(&window_id)
    }

    /// Returns `true` if the window is not open or has pending loading handles.
    pub fn is_loading(&self, window_id: impl Into<WindowId>) -> bool {
        let window_id = window_id.into();
        let sv = WINDOWS_SV.read();
        sv.open_loading.contains_key(&window_id) || sv.windows_info.get(&window_id).map(|i| !i.is_loaded).unwrap_or(false)
    }

    /// Returns `true` if the window is open and has no pending loading handles.
    pub fn is_loaded(&self, window_id: impl Into<WindowId>) -> bool {
        let window_id = window_id.into();
        WINDOWS_SV.read().windows_info.get(&window_id).map(|i| i.is_loaded).unwrap_or(false)
    }

    /// Wait until the window is loaded or closed.
    ///
    /// If `wait_event` is `true` also awaits for the [`WINDOW_LOAD_EVENT`] to finish notifying.
    ///
    /// Returns `true` if the window iis open and has no pending loading handles.
    pub fn wait_loaded(&self, window_id: impl Into<WindowId>, wait_event: bool) -> impl Future<Output = bool> + Send + Sync + 'static {
        Self::wait_loaded_impl(window_id.into(), wait_event)
    }
    async fn wait_loaded_impl(window_id: WindowId, wait_event: bool) -> bool {
        if Self.is_loaded(window_id) {
            if wait_event {
                // unlikely, but it can just have loaded and the event is ongoing.
                zng_task::yield_now().await;
            }
            return true;
        }

        // start receiving before loading check otherwise could load after check and before receiver creation.
        let recv = WINDOW_LOAD_EVENT.receiver();
        while Self.is_loading(window_id) {
            while let Ok(msg) = zng_task::with_deadline(recv.recv_async(), 1.secs()).await {
                if let Ok(args) = msg {
                    if args.window_id == window_id {
                        if wait_event {
                            zng_task::yield_now().await;
                        }
                        return true;
                    }
                }
            }
            // deadline, rare case window closes before load
        }

        if Self.is_loaded(window_id) {
            if wait_event {
                zng_task::yield_now().await;
            }
            return true;
        }
        false
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
    pub fn focus(&self, window_id: impl Into<WindowId>) -> Result<(), WindowNotFound> {
        let window_id = window_id.into();
        if !self.is_focused(window_id)? {
            let mut w = WINDOWS_SV.write();
            w.focus_request = Some(window_id);
            UPDATES.update(None);
        }
        Ok(())
    }

    /// Focus a window if it is open or opening, otherwise opens it focused.
    pub fn focus_or_open(
        &self,
        window_id: impl Into<WindowId>,
        open: impl Future<Output = WindowRoot> + Send + 'static,
    ) -> Option<ResponseVar<WindowId>> {
        let window_id = window_id.into();
        if self.focus(window_id).is_ok() {
            None
        } else {
            let r = self.open_id(window_id, async move {
                let w = open.await;
                // keep the request as close to the actual open as possible
                WINDOWS.focus(WINDOW.id()).unwrap();
                w
            });
            Some(r)
        }
    }

    /// Move the window to the front of the operating system Z stack.
    ///
    /// Note that the window is not focused, the [`focus`] operation also moves the window to the front.
    ///
    /// [`always_on_top`]: WindowVars::always_on_top
    /// [`focus`]: Self::focus
    pub fn bring_to_top(&self, window_id: impl Into<WindowId>) -> Result<(), WindowNotFound> {
        let window_id = window_id.into();
        let mut w = WINDOWS_SV.write();
        if w.windows_info.contains_key(&window_id) {
            w.bring_to_top_requests.push(window_id);
            UPDATES.update(None);
            Ok(())
        } else {
            Err(WindowNotFound(window_id))
        }
    }

    /// Register the closure `extender` to be called with the root of every new window starting on the next update.
    ///
    /// The closure returns the new root node that will be passed to any other root extender until
    /// the actual final root node is created.
    ///
    /// This is an advanced API that enables app wide features, like themes, to inject context in every new window. The
    /// extender is called in the context of the window, after the window creation future has completed.
    ///
    /// Note that the *root* node passed to the extender is the child node of the `WindowRoot` widget, not the widget itself.
    /// The extended root will be wrapped in the root widget node, that is, the final root widget will be
    /// `root(extender_nodes(CONTEXT(EVENT(..))))`, so extension nodes should operate as `CONTEXT` properties.
    pub fn register_root_extender<E>(&self, mut extender: impl FnMut(WindowRootExtenderArgs) -> E + Send + 'static)
    where
        E: zng_app::widget::node::UiNode,
    {
        WINDOWS_SV
            .write()
            .root_extenders
            .get_mut()
            .push(Box::new(move |a| extender(a).boxed()))
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
    ) -> Result<(), WindowNotFound> {
        let window_id = window_id.into();
        match WINDOWS_SV.write().windows_info.get_mut(&window_id) {
            Some(i) => {
                i.extensions.push((extension_id, request));
                Ok(())
            }
            None => Err(WindowNotFound(window_id)),
        }
    }

    pub(super) fn system_colors_config(&self) -> ColorsConfig {
        WINDOWS_SV.read().latest_colors_cfg
    }

    pub(super) fn take_view_extensions_init(&self, id: WindowId) -> Vec<(ApiExtensionId, ApiExtensionPayload)> {
        std::mem::take(&mut WINDOWS_SV.write().windows_info.get_mut(&id).unwrap().extensions)
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
        let window_id = window_id.into();
        let sv = WINDOWS_SV.read();
        match WINDOWS_SV.read().windows_info.get(&window_id) {
            Some(i) => match &i.view {
                Some(r) => match r {
                    ViewWindowOrHeadless::Window(r) => {
                        let r = r.clone();
                        drop(sv);
                        r.window_extension_raw(extension_id, request)
                            .map_err(ViewExtensionError::ViewProcessOffline)
                    }
                    ViewWindowOrHeadless::Headless(_) => Err(ViewExtensionError::WindowNotHeaded(window_id)),
                },
                None => Err(ViewExtensionError::NotOpenInViewProcess(window_id)),
            },
            None => Err(ViewExtensionError::WindowNotFound(WindowNotFound(window_id))),
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
        let sv = WINDOWS_SV.read();
        match sv.windows_info.get(&window_id) {
            Some(i) => match &i.view {
                Some(r) => match r {
                    ViewWindowOrHeadless::Window(r) => {
                        let r = r.clone();
                        drop(sv);
                        let r = r
                            .window_extension(extension_id, request)
                            .map_err(ViewExtensionError::ViewProcessOffline)?;
                        r.map_err(ViewExtensionError::Api)
                    }
                    ViewWindowOrHeadless::Headless(_) => Err(ViewExtensionError::WindowNotHeaded(window_id)),
                },
                None => Err(ViewExtensionError::NotOpenInViewProcess(window_id)),
            },
            None => Err(ViewExtensionError::WindowNotFound(WindowNotFound(window_id))),
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
        let window_id = window_id.into();
        let sv = WINDOWS_SV.read();
        match WINDOWS_SV.read().windows_info.get(&window_id) {
            Some(i) => match &i.view {
                Some(r) => {
                    let r = r.renderer();
                    drop(sv);
                    r.render_extension_raw(extension_id, request)
                        .map_err(ViewExtensionError::ViewProcessOffline)
                }
                None => Err(ViewExtensionError::NotOpenInViewProcess(window_id)),
            },
            None => Err(ViewExtensionError::WindowNotFound(WindowNotFound(window_id))),
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
        let sv = WINDOWS_SV.read();
        match sv.windows_info.get(&window_id) {
            Some(i) => match &i.view {
                Some(r) => {
                    let r = r.renderer();
                    drop(sv);
                    let r = r
                        .render_extension(extension_id, request)
                        .map_err(ViewExtensionError::ViewProcessOffline)?;
                    r.map_err(ViewExtensionError::Api)
                }
                None => Err(ViewExtensionError::NotOpenInViewProcess(window_id)),
            },
            None => Err(ViewExtensionError::WindowNotFound(WindowNotFound(window_id))),
        }
    }

    /// Update the reference to view window the renderer associated with the window.
    pub(super) fn set_view(&self, id: WindowId, view: ViewWindowOrHeadless) {
        if let Some(info) = WINDOWS_SV.write().windows_info.get_mut(&id) {
            info.view = Some(view);
        }
    }

    /// Update widget info tree associated with the window.
    pub(super) fn set_widget_tree(&self, info_tree: WidgetInfoTree) {
        if let Some(info) = WINDOWS_SV.write().windows_info.get_mut(&info_tree.window_id()) {
            info.widget_tree = info_tree;
        }
    }

    /// Change window state to loaded if there are no load handles active.
    ///
    /// Returns `true` if loaded.
    pub(super) fn try_load(&self, window_id: WindowId) -> bool {
        if let Some(info) = WINDOWS_SV.write().windows_info.get_mut(&window_id) {
            info.is_loaded = info.loading_handle.try_load(window_id);

            if info.is_loaded && !info.vars.0.is_loaded.get() {
                info.vars.0.is_loaded.set(true);
                WINDOW_LOAD_EVENT.notify(WindowOpenArgs::now(info.id));
            }

            info.is_loaded
        } else {
            unreachable!()
        }
    }

    pub(super) fn on_pre_event(update: &EventUpdate) {
        if let Some(args) = RAW_WINDOW_FOCUS_EVENT.on(update) {
            let mut wns = WINDOWS_SV.write();

            let mut prev = None;
            let mut new = None;

            if let Some(prev_focus) = args.prev_focus {
                if let Some(window) = wns.windows_info.get_mut(&prev_focus) {
                    if window.is_focused {
                        window.is_focused = false;
                        prev = Some(prev_focus);
                    }
                }
            }
            if let Some(new_focus) = args.new_focus {
                if prev.is_none() {
                    if let Some((&id, window)) = wns.windows_info.iter_mut().find(|w| w.1.is_focused) {
                        if new_focus != id {
                            window.is_focused = false;
                            prev = Some(id);
                        }
                    }
                }

                if let Some(window) = wns.windows_info.get_mut(&new_focus) {
                    if !window.is_focused {
                        window.is_focused = true;
                        window.vars.focus_indicator().set(None);
                        new = Some(new_focus);
                    }
                }
            }

            if prev.is_some() || new.is_some() {
                let args = WindowFocusChangedArgs::new(args.timestamp, args.propagation().clone(), prev, new, false);
                WINDOW_FOCUS_CHANGED_EVENT.notify(args);
            }
        } else if let Some(args) = RAW_WINDOW_CLOSE_REQUESTED_EVENT.on(update) {
            let _ = WINDOWS.close(args.window_id);
        } else if let Some(args) = RAW_WINDOW_CLOSE_EVENT.on(update) {
            if WINDOWS_SV.read().windows.contains_key(&args.window_id) {
                tracing::error!("view-process closed window without request");
                let mut windows = IdSet::default();
                windows.insert(args.window_id);
                let args = WindowCloseArgs::new(args.timestamp, args.propagation().clone(), windows);
                WINDOW_CLOSE_EVENT.notify(args);
            }
        } else if let Some(args) = RAW_COLORS_CONFIG_CHANGED_EVENT.on(update) {
            WINDOWS_SV.write().latest_colors_cfg = args.config;
        } else if let Some(args) = VIEW_PROCESS_INITED_EVENT.on(update) {
            WINDOWS_SV.write().latest_colors_cfg = args.colors_config;

            // we skipped request fulfillment until this event.
            UPDATES.update(None);
        } else if let Some(args) = RAW_IMAGE_LOADED_EVENT.on(update).or_else(|| RAW_IMAGE_LOAD_ERROR_EVENT.on(update)) {
            // update ready frame images.
            let mut sv = WINDOWS_SV.write();
            sv.frame_images.retain(|i| {
                if let Some(i) = i.upgrade() {
                    if Some(&args.image) == i.get().view() {
                        i.update();
                        false
                    } else {
                        true
                    }
                } else {
                    false
                }
            });
        }

        Self::with_detached_windows(|windows, parallel| {
            if windows.len() > 1 && parallel.contains(ParallelWin::EVENT) {
                windows.par_iter_mut().with_ctx().for_each(|(_, window)| {
                    window.pre_event(update);
                });
            } else {
                for (_, window) in windows.iter_mut() {
                    window.pre_event(update);
                }
            }
        })
    }

    pub(super) fn on_ui_event(update: &mut EventUpdate) {
        if update.delivery_list_mut().has_pending_search() {
            update
                .delivery_list_mut()
                .fulfill_search(WINDOWS_SV.read().windows_info.values().map(|w| &w.widget_tree));
        }
        Self::with_detached_windows(|windows, parallel| {
            if windows.len() > 1 && parallel.contains(ParallelWin::EVENT) {
                windows.par_iter_mut().with_ctx().for_each(|(_, window)| {
                    window.ui_event(update);
                });
            } else {
                for (_, window) in windows.iter_mut() {
                    window.ui_event(update);
                }
            }
        });
    }

    pub(super) fn on_event(update: &mut EventUpdate) {
        if let Some(args) = WINDOW_CLOSE_REQUESTED_EVENT.on(update) {
            let key = args.windows.iter().next().unwrap();
            let mut sv = WINDOWS_SV.write();
            if let Some(rsp) = sv.close_responders.remove(key) {
                if !args.propagation().is_stopped() {
                    // close requested by us and not canceled.
                    WINDOW_CLOSE_EVENT.notify(WindowCloseArgs::now(args.windows.clone()));
                    for r in rsp {
                        r.respond(CloseWindowResult::Closed);
                    }
                } else {
                    for r in rsp {
                        r.respond(CloseWindowResult::Cancel);
                    }
                    // already cancelled exit request
                    sv.exit_on_close = false;
                }
            }
        } else if let Some(args) = WINDOW_CLOSE_EVENT.on(update) {
            // finish close, this notifies `UiNode::deinit` and drops the window
            // causing the ViewWindow to drop and close.

            for w in args.windows.iter() {
                let w = WINDOWS_SV.write().windows.remove(w);
                if let Some(w) = w {
                    let id = w.ctx.id();
                    w.close();

                    let info = WINDOWS_SV.write().windows_info.remove(&id).unwrap();

                    info.vars.0.is_open.set(false);

                    if info.is_focused {
                        let args = WindowFocusChangedArgs::now(Some(info.id), None, true);
                        WINDOW_FOCUS_CHANGED_EVENT.notify(args)
                    }
                }
            }

            let is_headless_app = zng_app::APP.window_mode().is_headless();
            let mut wns = WINDOWS_SV.write();

            // if windows closed because of app exit request
            // OR
            // if set to exit on last headed window close in a headed app,
            // AND there is no more open headed window OR request for opening a headed window.
            if mem::take(&mut wns.exit_on_close)
                || (wns.exit_on_last_close.get()
                    && !is_headless_app
                    && !wns.windows.values().any(|w| matches!(w.ctx.mode(), WindowMode::Headed))
                    && !wns
                        .open_requests
                        .iter()
                        .any(|w| matches!(w.force_headless, None | Some(WindowMode::Headed)))
                    && !wns.open_tasks.iter().any(|t| matches!(t.mode, WindowMode::Headed)))
            {
                // fulfill `exit_on_close` or `exit_on_last_close`
                APP.exit();
            }
        } else if let Some(args) = EXIT_REQUESTED_EVENT.on(update) {
            if !args.propagation().is_stopped() {
                let mut windows = WINDOWS_SV.write();
                if !windows.windows_info.is_empty() {
                    args.propagation().stop();
                    windows.exit_on_close = true;
                    drop(windows);
                    WINDOWS.close_all();
                }
            }
        }
    }

    pub(super) fn on_ui_update(update_widgets: &mut WidgetUpdates) {
        if update_widgets.delivery_list_mut().has_pending_search() {
            update_widgets
                .delivery_list_mut()
                .fulfill_search(WINDOWS_SV.read().windows_info.values().map(|w| &w.widget_tree));
        }

        Self::with_detached_windows(|windows, parallel| {
            if windows.len() > 1 && parallel.contains(ParallelWin::UPDATE) {
                windows.par_iter_mut().with_ctx().for_each(|(_, window)| {
                    window.update(update_widgets);
                });
            } else {
                for (_, window) in windows.iter_mut() {
                    window.update(update_widgets);
                }
            }
        });
    }

    pub(super) fn on_update() {
        Self::fulfill_requests();
    }

    fn fulfill_requests() {
        if VIEW_PROCESS.is_available() && !VIEW_PROCESS.is_online() {
            // wait ViewProcessInitedEvent
            return;
        }

        let ((open, mut open_tasks, close, focus, bring_to_top, view_tasks), colors_cfg) = {
            let mut wns = WINDOWS_SV.write();
            (wns.take_requests(), wns.latest_colors_cfg)
        };

        let window_mode = zng_app::APP.window_mode();

        // fulfill open requests.
        for r in open {
            let window_mode = match (window_mode, r.force_headless) {
                (WindowMode::Headed | WindowMode::HeadlessWithRenderer, Some(mode)) => {
                    debug_assert!(!matches!(mode, WindowMode::Headed));
                    mode
                }
                (mode, _) => mode,
            };

            let colors_cfg = match window_mode {
                WindowMode::Headed => colors_cfg,
                WindowMode::Headless | WindowMode::HeadlessWithRenderer => ColorsConfig::default(),
            };

            let task = AppWindowTask::new(r.id, window_mode, colors_cfg, r.new.into_inner(), r.responder);
            open_tasks.push(task);
        }

        // update open tasks.
        let mut any_ready = false;
        for task in &mut open_tasks {
            let ready = task.update();
            any_ready |= ready;
        }
        if any_ready {
            for mut task in open_tasks {
                if task.is_ready() {
                    let window_id = task.ctx.id();

                    let mut wns = WINDOWS_SV.write();
                    let loading = wns.open_loading.remove(&window_id).unwrap();
                    let mut root_extenders = mem::take(&mut wns.root_extenders);
                    drop(wns);
                    let (window, info, responder) = task.finish(loading, &mut root_extenders.get_mut()[..]);

                    let mut wns = WINDOWS_SV.write();
                    root_extenders.get_mut().append(wns.root_extenders.get_mut());
                    wns.root_extenders = root_extenders;

                    if wns.windows.insert(window_id, window).is_some() {
                        // id conflict resolved on request.
                        unreachable!();
                    }
                    wns.windows_info.insert(info.id, info);

                    responder.respond(window_id);
                    // WINDOW_OPEN_EVENT.notify happens after init, so that handlers
                    // on the window itself can subscribe to the event.
                } else {
                    let mut wns = WINDOWS_SV.write();
                    wns.open_tasks.push(task);
                }
            }
        } else {
            let mut wns = WINDOWS_SV.write();
            debug_assert!(wns.open_tasks.is_empty());
            wns.open_tasks = open_tasks;
        }

        // notify close requests, the request is fulfilled or canceled
        // in the `event` handler.

        {
            let mut wns = WINDOWS_SV.write();
            let wns = &mut *wns;

            let mut close_wns = IdSet::default();

            for r in close {
                for w in r.windows {
                    if let Some(info) = wns.windows_info.get(&w) {
                        if close_wns.insert(w) {
                            wns.close_responders
                                .entry(w)
                                .or_insert_with(Default::default)
                                .push(r.responder.clone());

                            info.vars.0.children.with(|c| {
                                for &c in c.iter() {
                                    if wns.windows_info.contains_key(&c) && close_wns.insert(c) {
                                        wns.close_responders
                                            .entry(c)
                                            .or_insert_with(Default::default)
                                            .push(r.responder.clone());
                                    }
                                }
                            });
                        }
                    }
                }
            }
            if !close_wns.is_empty() {
                let args = WindowCloseRequestedArgs::now(close_wns);
                WINDOW_CLOSE_REQUESTED_EVENT.notify(args);
            }
        }

        // fulfill focus request
        if let Some(w_id) = focus {
            Self::with_detached_windows(|windows, _| {
                if let Some(w) = windows.get_mut(&w_id) {
                    w.focus();
                }
            });
        }

        for w_id in bring_to_top {
            Self::with_detached_windows(|windows, _| {
                if let Some(w) = windows.get_mut(&w_id) {
                    w.bring_to_top();
                }
            });
        }

        for view_task in view_tasks {
            let task = view_task.task.into_inner();
            Self::with_detached_windows(|windows, _| {
                if let Some(w) = windows.get_mut(&view_task.window_id) {
                    w.view_task(task);
                } else {
                    task(None);
                }
            })
        }
    }

    pub(super) fn on_info(info_widgets: &mut InfoUpdates) {
        if info_widgets.delivery_list_mut().has_pending_search() {
            info_widgets
                .delivery_list_mut()
                .fulfill_search(WINDOWS_SV.read().windows_info.values().map(|w| &w.widget_tree));
        }

        let info_widgets_arc = Arc::new(mem::take(info_widgets));

        Self::with_detached_windows(|windows, parallel| {
            if windows.len() > 1 && parallel.contains(ParallelWin::LAYOUT) {
                windows.par_iter_mut().with_ctx().for_each(|(_, window)| {
                    window.info(info_widgets_arc.clone());
                })
            } else {
                for (_, window) in windows.iter_mut() {
                    window.info(info_widgets_arc.clone());
                }
            }
        });

        match Arc::try_unwrap(info_widgets_arc) {
            Ok(w) => {
                *info_widgets = w;
            }
            Err(_) => {
                tracing::error!("info_widgets not released by window")
            }
        }
    }

    pub(super) fn on_layout(layout_widgets: &mut LayoutUpdates) {
        if layout_widgets.delivery_list_mut().has_pending_search() {
            layout_widgets
                .delivery_list_mut()
                .fulfill_search(WINDOWS_SV.read().windows_info.values().map(|w| &w.widget_tree));
        }

        let layout_widgets_arc = Arc::new(mem::take(layout_widgets));

        Self::with_detached_windows(|windows, parallel| {
            if windows.len() > 1 && parallel.contains(ParallelWin::LAYOUT) {
                windows.par_iter_mut().with_ctx().for_each(|(_, window)| {
                    window.layout(layout_widgets_arc.clone());
                })
            } else {
                for (_, window) in windows.iter_mut() {
                    window.layout(layout_widgets_arc.clone());
                }
            }
        });

        match Arc::try_unwrap(layout_widgets_arc) {
            Ok(w) => {
                *layout_widgets = w;
            }
            Err(_) => {
                tracing::error!("layout_widgets not released by window")
            }
        }
    }

    pub(super) fn on_render(render_widgets: &mut RenderUpdates, render_update_widgets: &mut RenderUpdates) {
        for list in [&mut *render_widgets, &mut *render_update_widgets] {
            if list.delivery_list_mut().has_pending_search() {
                list.delivery_list_mut()
                    .fulfill_search(WINDOWS_SV.read().windows_info.values().map(|w| &w.widget_tree));
            }
        }

        let render_widgets_arc = Arc::new(mem::take(render_widgets));
        let render_update_widgets_arc = Arc::new(mem::take(render_update_widgets));

        Self::with_detached_windows(|windows, parallel| {
            if windows.len() > 1 && parallel.contains(ParallelWin::RENDER) {
                windows.par_iter_mut().with_ctx().for_each(|(_, window)| {
                    window.render(render_widgets_arc.clone(), render_update_widgets_arc.clone());
                });
            } else {
                for (_, window) in windows.iter_mut() {
                    window.render(render_widgets_arc.clone(), render_update_widgets_arc.clone());
                }
            }
        });

        match Arc::try_unwrap(render_widgets_arc) {
            Ok(w) => {
                *render_widgets = w;
            }
            Err(_) => {
                tracing::error!("render_widgets not released by window")
            }
        }
        match Arc::try_unwrap(render_update_widgets_arc) {
            Ok(w) => {
                *render_update_widgets = w;
            }
            Err(_) => {
                tracing::error!("render_update_widgets not released by window")
            }
        }
    }

    /// Takes ownership of [`Windows::windows`] for the duration of the call to `f`.
    ///
    /// The windows map is empty for the duration of `f` and should not be used, this is for
    /// mutating the window content while still allowing it to query the `Windows::windows_info`.
    fn with_detached_windows(f: impl FnOnce(&mut IdMap<WindowId, AppWindow>, ParallelWin)) {
        let (mut windows, parallel) = {
            let mut w = WINDOWS_SV.write();
            (mem::take(&mut w.windows), w.parallel.get())
        };
        f(&mut windows, parallel);
        let mut wns = WINDOWS_SV.write();
        debug_assert!(wns.windows.is_empty());
        wns.windows = windows;
    }
}

/// Native dialogs.
impl WINDOWS {
    /// Show a native message dialog for the window.
    ///
    /// The dialog can be modal in the view-process, in the app-process it is always async, the
    /// response var will update once when the user responds to the dialog.
    ///
    /// Consider using the `DIALOG` service instead of the method directly.
    pub fn native_message_dialog(
        &self,
        window_id: WindowId,
        dialog: zng_view_api::dialog::MsgDialog,
    ) -> ResponseVar<zng_view_api::dialog::MsgDialogResponse> {
        let (responder, rsp) = response_var();
        WINDOWS_SV.write().view_window_task(window_id, move |win| match win {
            Some(win) => {
                if let Err(e) = win.message_dialog(dialog, responder.clone()) {
                    responder.respond(zng_view_api::dialog::MsgDialogResponse::Error(formatx!("{e}")))
                }
            }
            None => responder.respond(zng_view_api::dialog::MsgDialogResponse::Error(Txt::from_static(
                "native window not found",
            ))),
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
        window_id: WindowId,
        dialog: zng_view_api::dialog::FileDialog,
    ) -> ResponseVar<zng_view_api::dialog::FileDialogResponse> {
        let (responder, rsp) = response_var();
        WINDOWS_SV.write().view_window_task(window_id, move |win| match win {
            Some(win) => {
                if let Err(e) = win.file_dialog(dialog, responder.clone()) {
                    responder.respond(zng_view_api::dialog::FileDialogResponse::Error(formatx!("{e}")))
                }
            }
            None => responder.respond(zng_view_api::dialog::FileDialogResponse::Error(Txt::from_static(
                "native window not found",
            ))),
        });
        rsp
    }
}

/// Window data visible in [`Windows`], detached so we can make the window visible inside the window content.
struct AppWindowInfo {
    id: WindowId,
    mode: WindowMode,
    view: Option<ViewWindowOrHeadless>,
    extensions: Vec<(ApiExtensionId, ApiExtensionPayload)>,
    vars: WindowVars,

    widget_tree: WidgetInfoTree,
    // focus tracked by the raw focus events.
    is_focused: bool,

    loading_handle: WindowLoading,
    is_loaded: bool,
}
impl AppWindowInfo {
    pub fn new(id: WindowId, root_id: WidgetId, mode: WindowMode, vars: WindowVars, loading_handle: WindowLoading) -> Self {
        Self {
            id,
            mode,
            view: None,
            extensions: vec![],
            vars,
            widget_tree: WidgetInfoTree::wgt(id, root_id),
            is_focused: false,
            loading_handle,
            is_loaded: false,
        }
    }
}
struct OpenWindowRequest {
    id: WindowId,
    new: Mutex<UiTask<WindowRoot>>, // never locked, makes `OpenWindowRequest: Sync`
    force_headless: Option<WindowMode>,
    responder: ResponderVar<WindowId>,
}
struct CloseWindowRequest {
    responder: ResponderVar<CloseWindowResult>,
    windows: IdSet<WindowId>,
}

struct AppWindowTask {
    ctx: WindowCtx,
    mode: WindowMode,
    task: Mutex<UiTask<WindowRoot>>, // never locked, used to make `AppWindowTask: Sync`
    responder: ResponderVar<WindowId>,
}
impl AppWindowTask {
    fn new(id: WindowId, mode: WindowMode, colors_cfg: ColorsConfig, new: UiTask<WindowRoot>, responder: ResponderVar<WindowId>) -> Self {
        let primary_scale_factor = match mode {
            WindowMode::Headed => MONITORS
                .primary_monitor()
                .map(|m| m.scale_factor().get())
                .unwrap_or_else(|| 1.fct()),
            WindowMode::Headless | WindowMode::HeadlessWithRenderer => 1.fct(),
        };

        let mut ctx = WindowCtx::new(id, mode);

        let vars = WindowVars::new(WINDOWS_SV.read().default_render_mode.get(), primary_scale_factor, colors_cfg);
        ctx.with_state(|s| s.borrow_mut().set(*WINDOW_VARS_ID, vars.clone()));

        Self {
            ctx,
            mode,
            responder,
            task: Mutex::new(new),
        }
    }

    fn is_ready(&mut self) -> bool {
        self.task.get_mut().is_ready()
    }

    fn update(&mut self) -> bool {
        WINDOW.with_context(&mut self.ctx, || {
            self.task.get_mut().update();
        });
        self.task.get_mut().is_ready()
    }

    fn finish(
        self,
        loading: WindowLoading,
        extenders: &mut [Box<dyn FnMut(WindowRootExtenderArgs) -> BoxedUiNode + Send>],
    ) -> (AppWindow, AppWindowInfo, ResponderVar<WindowId>) {
        let mut window = self.task.into_inner().into_result().unwrap_or_else(|_| panic!());
        let mut ctx = self.ctx;

        WINDOW.with_context(&mut ctx, || {
            for ext in extenders.iter_mut().rev() {
                let root = mem::replace(&mut window.child, NilUiNode.boxed());
                window.child = ext(WindowRootExtenderArgs { root });
            }
            let child = mem::replace(&mut window.child, NilUiNode.boxed());
            let vars = WINDOW.vars();
            let child = with_context_var(child, ACCENT_COLOR_VAR, vars.actual_accent_color());
            let child = with_context_var(child, COLOR_SCHEME_VAR, vars.actual_color_scheme());
            window.child = child.boxed();
        });

        let mode = self.mode;
        let id = ctx.id();

        ctx.set_widget_tree(WidgetInfoTree::wgt(id, window.id));

        let vars = ctx.with_state(|s| s.borrow().get_clone(*WINDOW_VARS_ID)).unwrap();

        if window.kiosk {
            vars.chrome().set(false);
            vars.visible().set(true);
            if !vars.state().get().is_fullscreen() {
                vars.state().set(WindowState::Exclusive);
            }
        }

        let commands = WindowCommands::new(id);

        let root_id = window.id;
        let ctrl = WindowCtrl::new(&vars, commands, mode, window);

        let window = AppWindow {
            ctrl: Mutex::new(ctrl),
            ctx,
        };
        let info = AppWindowInfo::new(id, root_id, mode, vars, loading);

        (window, info, self.responder)
    }
}

struct ViewWindowTask {
    window_id: WindowId,
    task: Mutex<Box<dyn FnOnce(Option<&view_process::ViewWindow>) + Send>>, // never locked, for :Async only
}

/// Window context owner.
struct AppWindow {
    ctrl: Mutex<WindowCtrl>, // never locked, makes `AppWindow: Sync`.
    ctx: WindowCtx,
}
impl AppWindow {
    fn ctrl_in_ctx<R>(&mut self, action: impl FnOnce(&mut WindowCtrl) -> R) -> R {
        WINDOW.with_context(&mut self.ctx, || action(self.ctrl.get_mut()))
    }

    pub fn pre_event(&mut self, update: &EventUpdate) {
        self.ctrl_in_ctx(|ctrl| ctrl.pre_event(update))
    }

    pub fn ui_event(&mut self, update: &EventUpdate) {
        self.ctrl_in_ctx(|ctrl| ctrl.ui_event(update))
    }

    pub fn update(&mut self, update_widgets: &WidgetUpdates) {
        self.ctrl_in_ctx(|ctrl| ctrl.update(update_widgets));
    }

    pub fn info(&mut self, info_widgets: Arc<InfoUpdates>) {
        let info_update = self.ctrl_in_ctx(|ctrl| ctrl.info(info_widgets));
        if let Some(new) = info_update {
            self.ctx.set_widget_tree(new);
        }
    }

    pub fn layout(&mut self, layout_widgets: Arc<LayoutUpdates>) {
        self.ctrl_in_ctx(|ctrl| ctrl.layout(layout_widgets));
    }

    pub fn render(&mut self, render_widgets: Arc<RenderUpdates>, render_update_widgets: Arc<RenderUpdates>) {
        self.ctrl_in_ctx(|ctrl| ctrl.render(render_widgets, render_update_widgets));
    }

    pub fn focus(&mut self) {
        self.ctrl_in_ctx(|ctrl| ctrl.focus());
    }

    pub fn bring_to_top(&mut self) {
        self.ctrl_in_ctx(|ctrl| ctrl.bring_to_top());
    }

    pub fn close(mut self) {
        self.ctrl_in_ctx(|ctrl| ctrl.close());
    }

    fn view_task(&mut self, task: Box<dyn FnOnce(Option<&view_process::ViewWindow>) + Send>) {
        self.ctrl_in_ctx(|ctrl| ctrl.view_task(task));
    }
}

struct WindowLoading {
    handles: Vec<std::sync::Weak<crate::WindowLoadingHandleData>>,
    timer: Option<DeadlineHandle>,
}
impl WindowLoading {
    pub fn new() -> Self {
        WindowLoading {
            handles: vec![],
            timer: None,
        }
    }

    /// Returns `true` if the window can load.
    pub fn try_load(&mut self, window_id: WindowId) -> bool {
        let mut deadline = Deadline::MAX;
        self.handles.retain(|h| match h.upgrade() {
            Some(h) => {
                if h.deadline.has_elapsed() {
                    false
                } else {
                    deadline = deadline.min(h.deadline);
                    true
                }
            }
            None => false,
        });

        if self.handles.is_empty() {
            self.timer = None;
            true
        } else {
            if let Some(t) = &self.timer {
                if t.deadline() != deadline {
                    self.timer = None;
                }
            }
            if self.timer.is_none() {
                let t = TIMERS.on_deadline(
                    deadline,
                    app_hn_once!(|_| {
                        UPDATES.update_window(window_id).layout_window(window_id).render_window(window_id);
                    }),
                );
                self.timer = Some(t);
            }

            false
        }
    }

    pub fn new_handle(&mut self, update: AppEventSender, deadline: Deadline) -> WindowLoadingHandle {
        let h = Arc::new(crate::WindowLoadingHandleData { update, deadline });
        self.handles.push(Arc::downgrade(&h));
        WindowLoadingHandle(h)
    }
}

/// Extensions methods for [`WINDOW`] contexts of windows open by [`WINDOWS`].
///
/// [`WINDOW`]: zng_app::window::WINDOW
#[allow(non_camel_case_types)]
pub trait WINDOW_Ext {
    /// Clone a reference to the variables that get and set window properties.
    fn vars(&self) -> super::WindowVars {
        WindowVars::req()
    }

    /// Returns `true` if the window is open.
    fn is_open(&self) -> bool {
        WINDOWS.is_open(WINDOW.id())
    }

    /// Returns `true` if the window is open and loaded.
    fn is_loaded(&self) -> bool {
        WINDOWS.is_loaded(WINDOW.id())
    }

    /// Enable accessibility info.
    ///
    /// If access is not already enabled, enables it in the app-process only.
    fn enable_access(&self) {
        let vars = WINDOW.vars();
        let access_enabled = &vars.0.access_enabled;
        if access_enabled.get().is_disabled() {
            access_enabled.modify(|e| *e.to_mut() |= zng_app::widget::info::access::AccessEnabled::APP);
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
    fn frame_image(&self, mask: Option<ImageMaskMode>) -> ImageVar {
        WINDOWS.frame_image(WINDOW.id(), mask)
    }

    /// Generate an image from a selection of the current rendered frame of the window.
    ///
    /// The image is not loaded at the moment of return, it will update when it is loaded.
    ///
    /// If the window is not found the error is reported in the image error.
    fn frame_image_rect(&self, rect: PxRect, mask: Option<ImageMaskMode>) -> ImageVar {
        WINDOWS.frame_image_rect(WINDOW.id(), rect, mask)
    }

    /// Move the window to the front of the operating system Z stack.
    ///
    /// See [`WINDOWS.bring_to_top`] for more details.
    ///
    /// [`WINDOWS.bring_to_top`]: WINDOWS::bring_to_top
    fn bring_to_top(&self) {
        WINDOWS.bring_to_top(WINDOW.id()).ok();
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
        WINDOWS.close(WINDOW.id()).unwrap()
    }
}
impl WINDOW_Ext for WINDOW {}

/// Arguments for [`WINDOWS.register_root_extender`].
///
/// [`WINDOWS.register_root_extender`]: WINDOWS::register_root_extender
pub struct WindowRootExtenderArgs {
    /// The window root content, extender must wrap this node with extension nodes or return
    /// it for no-op.
    pub root: BoxedUiNode,
}

impl ImageRenderWindowRoot for WindowRoot {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

#[doc(hidden)]
impl ImageRenderWindowsService for WINDOWS {
    fn new_window_root(&self, node: BoxedUiNode, render_mode: RenderMode, scale_factor: Option<Factor>) -> Box<dyn ImageRenderWindowRoot> {
        Box::new(WindowRoot::new_container(
            WidgetId::new_unique(),
            StartPosition::Default,
            false,
            true,
            Some(render_mode),
            scale_factor.map(HeadlessMonitor::new_scale).unwrap_or_default(),
            false,
            node,
        ))
    }

    fn enable_frame_capture_in_window_context(&self, mask: Option<ImageMaskMode>) {
        let mode = if let Some(mask) = mask {
            FrameCaptureMode::AllMask(mask)
        } else {
            FrameCaptureMode::All
        };
        WINDOW.vars().frame_capture_mode().set(mode);
    }

    fn set_parent_in_window_context(&self, parent_id: WindowId) {
        let vars = WINDOW.vars();
        vars.parent().set(parent_id);
    }

    fn open_headless_window(&self, new_window_root: Box<dyn FnOnce() -> Box<dyn ImageRenderWindowRoot> + Send>) {
        WINDOWS.open_headless(
            async move {
                let w = *new_window_root()
                    .into_any()
                    .downcast::<WindowRoot>()
                    .expect("expected `WindowRoot` in image render window");
                let vars = WINDOW.vars();
                vars.auto_size().set(true);
                vars.min_size().set((1.px(), 1.px()));
                w
            },
            true,
        );
    }

    fn on_frame_image_ready(&self, update: &EventUpdate) -> Option<(WindowId, Img)> {
        if let Some(args) = FRAME_IMAGE_READY_EVENT.on(update) {
            if let Some(img) = &args.frame_image {
                return Some((args.window_id, img.clone()));
            }
        }
        None
    }

    fn close_window(&self, window_id: WindowId) {
        let _ = WINDOWS.close(window_id);
    }

    fn clone_boxed(&self) -> Box<dyn ImageRenderWindowsService> {
        Box::new(WINDOWS)
    }
}

/// Window focused widget hook.
#[allow(non_camel_case_types)]
pub struct WINDOW_FOCUS;
impl WINDOW_FOCUS {
    /// Setup a var that is controlled by the focus service and tracks the focused widget.
    ///
    /// This must be called by the focus implementation only.
    pub fn hook_focus_service(&self, focused: BoxedVar<Option<InteractionPath>>) {
        *FOCUS_SV.write() = focused;
    }

    pub(crate) fn focused(&self) -> BoxedVar<Option<InteractionPath>> {
        FOCUS_SV.get()
    }
}
