use std::{fmt, future::Future, mem, sync::Arc};

use parking_lot::Mutex;
use zero_ui_app::{
    app_hn_once,
    timer::{DeadlineHandle, TIMERS},
    update::{EventUpdate, InfoUpdates, LayoutUpdates, RenderUpdates, UpdateOp, WidgetUpdates, UPDATES},
    view_process::{
        self,
        raw_events::{
            RAW_COLOR_SCHEME_CHANGED_EVENT, RAW_WINDOW_CLOSE_EVENT, RAW_WINDOW_CLOSE_REQUESTED_EVENT, RAW_WINDOW_FOCUS_EVENT,
            RAW_WINDOW_OPEN_EVENT,
        },
        ViewImage, ViewRenderer, VIEW_PROCESS, VIEW_PROCESS_INITED_EVENT,
    },
    widget::{
        info::{WidgetInfo, WidgetInfoChangedArgs, WidgetInfoTree},
        instance::{BoxedUiNode, NilUiNode},
        WidgetId,
    },
    window::{WindowCtx, WindowId, WINDOW},
    AppEventSender, APP_PROCESS, EXIT_REQUESTED_EVENT,
};
use zero_ui_app_context::app_local;
use zero_ui_color::COLOR_SCHEME_VAR;
use zero_ui_ext_image::{ImageRenderWindowRoot, ImageRenderWindowsService, ImageVar, Img};
use zero_ui_layout::units::{Deadline, Factor, PxRect};
use zero_ui_task::ui::UiTask;
use zero_ui_txt::{formatx, Txt};
use zero_ui_unique_id::{IdMap, IdSet};
use zero_ui_var::{impl_from_and_into_var, response_done_var, response_var, var, ArcVar, ResponderVar, ResponseVar};
use zero_ui_view_api::{
    config::ColorScheme,
    image::ImageMaskMode,
    window::{RenderMode, WindowState},
    ViewProcessOffline,
};

use crate::{
    commands::WindowCommands, control::WindowCtrl, CloseWindowResult, FrameCaptureMode, HeadlessMonitor, StartPosition, WindowChrome,
    WindowCloseArgs, WindowCloseRequestedArgs, WindowFocusChangedArgs, WindowMode, WindowNotFound, WindowOpenArgs, WindowRoot, WindowVars,
    FRAME_IMAGE_READY_EVENT, MONITORS, WINDOW_CLOSE_EVENT, WINDOW_CLOSE_REQUESTED_EVENT, WINDOW_FOCUS_CHANGED_EVENT, WINDOW_LOAD_EVENT,
    WINDOW_VARS_ID,
};

app_local! {
    pub(super) static WINDOWS_SV: WindowsService = WindowsService::new();
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

    focus_request: Option<WindowId>,
    bring_to_top_requests: Vec<WindowId>,

    frame_images: Vec<ArcVar<Img>>,

    loading_deadline: Option<DeadlineHandle>,
    latest_color_scheme: ColorScheme,

    view_window_tasks: Vec<ViewWindowTask>,
}
impl WindowsService {
    fn new() -> Self {
        Self {
            exit_on_last_close: var(true),
            default_render_mode: var(RenderMode::default()),
            root_extenders: Mutex::new(vec![Box::new(|a| {
                with_context_var_init(a.root, COLOR_SCHEME_VAR, || WINDOW.vars().actual_color_scheme().boxed()).boxed()
            })]),
            parallel: var(ParallelWin::default()),
            windows: IdMap::default(),
            windows_info: IdMap::default(),
            open_loading: IdMap::new(),
            open_tasks: vec![],
            open_requests: Vec::with_capacity(1),
            close_responders: IdMap::default(),
            close_requests: vec![],
            focus_request: None,
            bring_to_top_requests: vec![],
            frame_images: vec![],
            loading_deadline: None,
            latest_color_scheme: ColorScheme::Dark,
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
            if let Some(r) = &w.renderer {
                match action(r) {
                    Ok(img) => {
                        let img = Img::new(img);
                        let img = var(img);
                        self.frame_images.push(img.clone());
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
    /// Defines what parts of windows can be updated in parallel.
    ///
    /// See [`WINDOWS.parallel`] for more details.
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
pub struct WINDOWS;
impl WINDOWS {
    /// If app process exit is requested when a window closes and there are no more windows open, `true` by default.
    ///
    /// This setting is ignored in headless apps, in headed apps the exit happens when all headed windows
    /// are closed, headless windows are ignored.
    ///
    /// If app exit is requested directly and there are headed windows open the exit op is canceled, the windows request close
    /// and this is set to `true` so that another exit request is made after the windows close.
    pub fn exit_on_last_close(&self) -> ArcVar<bool> {
        WINDOWS_SV.read().exit_on_last_close.clone()
    }

    /// Default render mode of windows opened by this service, the initial value is [`RenderMode::default`].
    ///
    /// Note that this setting only affects windows opened after it is changed, also the view-process may select
    /// a different render mode if it cannot support the requested mode.
    pub fn default_render_mode(&self) -> ArcVar<RenderMode> {
        WINDOWS_SV.read().default_render_mode.clone()
    }

    /// Defines what parts of windows can update in parallel.
    ///
    /// All parallel is enabled by default. See [`ParallelWin`] for details of what parts of the windows can update in parallel.
    ///
    /// Note that this config is for parallel execution between windows, see the [`parallel`] property for parallel execution
    /// within windows and widgets.
    ///
    /// [`parallel`]: fn@crate::widget_base::parallel
    pub fn parallel(&self) -> ArcVar<ParallelWin> {
        WINDOWS_SV.read().parallel.clone()
    }

    /// Requests a new window.
    ///
    /// The `new_window` future runs in a [`UiTask`] inside the new [`WINDOW`] context.
    ///
    /// Returns a response variable that will update once when the window is opened, note that while the [`WINDOW`] is
    /// available in the `new_window` argument already, the window is only available in this service after
    /// the returned variable updates. Also note that the window might not be fully [loaded] yet.
    ///
    /// An update cycle is processed between the end of `new_window` and the window init, this means that you
    /// can use the context [`WINDOW`] to set variables that will be read on init with the new value.
    ///
    /// [loaded]: Self::is_loaded
    pub fn open(&self, new_window: impl Future<Output = WindowRoot> + Send + 'static) -> ResponseVar<WindowId> {
        WINDOWS_SV
            .write()
            .open_impl(WindowId::new_unique(), UiTask::new(None, new_window), None)
    }

    /// Requests a new window with pre-defined ID.
    ///
    /// # Panics
    ///
    /// if the `window_id` is already assigned to an open or opening window.
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
    /// if the `window_id` is already assigned to an open or opening window.
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

    /// Gets a handle that stops the window from loading while it exists.
    ///
    /// A window is only opened in the view-process after it is loaded, without any loading handles the window is considered *loaded*
    /// after the first layout pass. Nodes in the window can request a loading handle to delay the view opening to after all async resources
    /// it requires to render correctly are loaded.
    ///
    /// Note that a window is only loaded after all handles are dropped or expired, you should set a reasonable `deadline`    
    /// after a time it is best to partially render a window than not showing anything.
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
    /// Returns [`WindowNotFound`] if the `window_id` is not one of the open windows or is only an open request.
    pub fn close(&self, window_id: impl Into<WindowId>) -> Result<ResponseVar<CloseWindowResult>, WindowNotFound> {
        self.close_together([window_id.into()])
    }

    /// Requests closing multiple windows together, the operation can be canceled by listeners of the
    /// [`WINDOW_CLOSE_REQUESTED_EVENT`]. If canceled none of the windows are closed. Children of each window
    /// are added to the close together set.
    ///
    /// Returns a response var that will update once with the result of the operation. Returns
    /// [`Cancel`] if `windows` is empty or only contained windows that already requested close
    /// during this update.
    ///
    /// Returns [`WindowNotFound`] if any of the IDs is not one of the open windows or is only an open request.
    ///
    /// [`Cancel`]: CloseWindowResult::Cancel
    pub fn close_together(&self, windows: impl IntoIterator<Item = WindowId>) -> Result<ResponseVar<CloseWindowResult>, WindowNotFound> {
        WINDOWS_SV.write().close_together(windows)
    }

    /// Requests close of all open windows together, the operation can be canceled by listeners of
    /// the [`WINDOW_CLOSE_REQUESTED_EVENT`]. If canceled none of the windows are closed.
    ///
    /// Returns a response var that will update once with the result of the operation, Returns
    /// [`Cancel`] if no window is open or if close was already requested to all of the windows.
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
    /// Returns [`WindowNotFound`] if the `window_id` is not one of the open windows or is only an open request.
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

    /// Reference the metadata about the window's widgets.
    ///
    /// Returns [`WindowNotFound`] if the `window_id` is not one of the open windows or is only an open request.
    pub fn widget_tree(&self, window_id: impl Into<WindowId>) -> Result<WidgetInfoTree, WindowNotFound> {
        let window_id = window_id.into();
        WINDOWS_SV
            .read()
            .windows_info
            .get(&window_id)
            .map(|w| w.widget_tree.clone())
            .ok_or(WindowNotFound(window_id))
    }

    /// Search for the widget in all windows.
    ///
    /// Returns [`WindowNotFound`] if none of the current windows contains the `widget_id`.
    pub fn widget_info(&self, widget_id: impl Into<WidgetId>) -> Option<WidgetInfo> {
        let widget_id = widget_id.into();
        WINDOWS_SV.read().windows_info.values().find_map(|w| w.widget_tree.get(widget_id))
    }

    /// Generate an image from the current rendered frame of the window.
    ///
    /// The image is not loaded at the moment of return, it will update when it is loaded.
    ///
    /// If the window is not found the error is reported in the image error.
    pub fn frame_image(&self, window_id: impl Into<WindowId>, mask: Option<ImageMaskMode>) -> ImageVar {
        WINDOWS_SV
            .write()
            .frame_image_impl(window_id.into(), move |vr| vr.frame_image(mask))
    }

    /// Generate an image from a selection of the current rendered frame of the window.
    ///
    /// The image is not loaded at the moment of return, it will update when it is loaded.
    ///
    /// If the window is not found the error is reported in the image error.
    pub fn frame_image_rect(&self, window_id: impl Into<WindowId>, rect: PxRect, mask: Option<ImageMaskMode>) -> ImageVar {
        WINDOWS_SV
            .write()
            .frame_image_impl(window_id.into(), |vr| vr.frame_image_rect(rect, mask))
    }

    /// Reference the [`WindowVars`] for the window.
    ///
    /// Returns [`WindowNotFound`] if the `window_id` is not one of the open windows or is only an open request.
    pub fn vars(&self, window_id: impl Into<WindowId>) -> Result<WindowVars, WindowNotFound> {
        let window_id = window_id.into();
        WINDOWS_SV
            .read()
            .windows_info
            .get(&window_id)
            .map(|w| w.vars.clone())
            .ok_or(WindowNotFound(window_id))
    }

    /// Gets if the window is focused in the OS.
    ///
    /// Returns [`WindowNotFound`] if the `window_id` is not one of the open windows, returns `false` if the `window_id` is
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

    /// Gets a reference to the widget trees of each open window.
    pub fn widget_trees(&self) -> Vec<WidgetInfoTree> {
        WINDOWS_SV.read().windows_info.values().map(|w| w.widget_tree.clone()).collect()
    }

    /// Gets the id of the window that is focused in the OS.
    pub fn focused_window_id(&self) -> Option<WindowId> {
        WINDOWS_SV.read().windows_info.values().find(|w| w.is_focused).map(|w| w.id)
    }

    /// Gets the latest frame for the focused window.
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

    /// Returns `true` if a pending window open request or awaiting open task is associated with the ID.
    ///
    /// Window open requests start polling after each update.
    pub fn is_opening(&self, window_id: impl Into<WindowId>) -> bool {
        let window_id = window_id.into();
        let sv = WINDOWS_SV.read();
        sv.open_loading.contains_key(&window_id)
    }

    /// Returns `true` if the window is not open or has not finished loading.
    pub fn is_loading(&self, window_id: impl Into<WindowId>) -> bool {
        let window_id = window_id.into();
        let sv = WINDOWS_SV.read();
        sv.open_loading.contains_key(&window_id) || sv.windows_info.get(&window_id).map(|i| i.is_loaded).unwrap_or(false)
    }

    /// Returns `true` if the window is open and loaded.
    pub fn is_loaded(&self, window_id: impl Into<WindowId>) -> bool {
        let window_id = window_id.into();
        WINDOWS_SV.read().windows_info.get(&window_id).map(|i| i.is_loaded).unwrap_or(false)
    }

    /// Requests that the window be made the foreground keyboard focused window.
    ///
    /// Prefer using the [`FOCUS`] service and advanced [`FocusRequest`] configs instead of using this method directly.
    ///
    /// This operation can steal keyboard focus from other apps disrupting the user, be careful with it.
    ///
    /// If the `window_id` is only associated with an open request it is modified to focus the window on open.
    ///
    /// If more than one focus request is made in the same update cycle only the last request is processed.
    ///
    /// [`FOCUS`]: crate::focus::FOCUS
    /// [`FocusRequest`]: crate::focus::FocusRequest
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

    /// Move the window to the front of the Z stack.
    ///
    /// This is ignored if the window is [`always_on_top`], the window is also not focused, the [`focus`] operation
    /// also moves the window to the front.
    ///
    /// Multiple requests can be made in the same update cycle, they are processed in order.
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

    /// Register a closure `extender` to be called with the root of every new window starting on the next update.
    ///
    /// The closure must returns the new root node that will be passed to previous registered root extenders until
    /// the final root node is created.
    ///
    /// This is an advanced API that enables features such as themes to inject context in every new window. The
    /// extender is called in the context of the window, after the window creation future has completed.
    ///
    /// Note that the *root* node passed to the extender is not the root widget, the extended root will be wrapped
    /// in the root widget node, that is, the final root widget will be `root(extender_nodes(CONTEXT(EVENT(..))))`,
    /// so extension nodes should operate as `CONTEXT` properties.
    pub fn register_root_extender<E>(&self, mut extender: impl FnMut(WindowRootExtenderArgs) -> E + Send + 'static)
    where
        E: zero_ui_app::widget::instance::UiNode,
    {
        WINDOWS_SV
            .write()
            .root_extenders
            .get_mut()
            .push(Box::new(move |a| extender(a).boxed()))
    }

    /// Update the reference to the renderer associated with the window, we need
    /// the render to enable the hit-test function.
    pub(super) fn set_renderer(&self, id: WindowId, renderer: ViewRenderer) {
        if let Some(info) = WINDOWS_SV.write().windows_info.get_mut(&id) {
            info.renderer = Some(renderer);
        }
    }

    /// Update widget info tree associated with the window.
    pub(super) fn set_widget_tree(&self, info_tree: WidgetInfoTree) {
        if let Some(info) = WINDOWS_SV.write().windows_info.get_mut(&info_tree.window_id()) {
            let prev_tree = info.widget_tree.clone();
            info.widget_tree = info_tree.clone();
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
        } else if let Some(args) = RAW_WINDOW_OPEN_EVENT.on(update) {
            WINDOWS_SV.write().latest_color_scheme = args.data.color_scheme;
        } else if let Some(args) = RAW_COLOR_SCHEME_CHANGED_EVENT.on(update) {
            WINDOWS_SV.write().latest_color_scheme = args.color_scheme;
        } else if let Some(args) = VIEW_PROCESS_INITED_EVENT.on(update) {
            WINDOWS_SV.write().latest_color_scheme = args.color_scheme;

            // we skipped request fulfillment until this event.
            UPDATES.update(None);
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
            if let Some(rsp) = WINDOWS_SV.write().close_responders.remove(key) {
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
                }
            }
        } else if let Some(args) = WINDOW_CLOSE_EVENT.on(update) {
            // finish close, this notifies  `UiNode::deinit` and drops the window
            // causing the ViewWindow to drop and close.

            for w in args.windows.iter() {
                let mut wns = WINDOWS_SV.write();
                if let Some(w) = wns.windows.remove(w) {
                    let id = w.ctx.id();
                    w.close();

                    let info = wns.windows_info.remove(&id).unwrap();

                    info.vars.0.is_open.set(false);

                    if info.is_focused {
                        let args = WindowFocusChangedArgs::now(Some(info.id), None, true);
                        WINDOW_FOCUS_CHANGED_EVENT.notify(args)
                    }
                }
            }

            let is_headless_app = zero_ui_app::App::window_mode().is_headless();
            let wns = WINDOWS_SV.read();

            // if set to exit on last headed window close in a headed app,
            // AND there is no more open headed window OR request for opening a headed window.
            if wns.exit_on_last_close.get()
                && !is_headless_app
                && !wns.windows.values().any(|w| matches!(w.ctx.mode(), WindowMode::Headed))
                && !wns
                    .open_requests
                    .iter()
                    .any(|w| matches!(w.force_headless, None | Some(WindowMode::Headed)))
                && !wns.open_tasks.iter().any(|t| matches!(t.mode, WindowMode::Headed))
            {
                // fulfill `exit_on_last_close`
                APP_PROCESS.exit();
            }
        } else if let Some(args) = EXIT_REQUESTED_EVENT.on(update) {
            if !args.propagation().is_stopped() {
                let windows = WINDOWS_SV.read();
                if windows.windows_info.values().any(|w| w.mode == WindowMode::Headed) {
                    args.propagation().stop();
                    windows.exit_on_last_close.set(true);
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
        Self::fullfill_requests();
    }

    fn fullfill_requests() {
        if VIEW_PROCESS.is_available() && !VIEW_PROCESS.is_online() {
            // wait ViewProcessInitedEvent
            return;
        }

        let ((open, mut open_tasks, close, focus, bring_to_top, view_tasks), color_scheme) = {
            let mut wns = WINDOWS_SV.write();
            (wns.take_requests(), wns.latest_color_scheme)
        };

        let window_mode = zero_ui_app::App::window_mode();

        // fulfill open requests.
        for r in open {
            let window_mode = match (window_mode, r.force_headless) {
                (WindowMode::Headed | WindowMode::HeadlessWithRenderer, Some(mode)) => {
                    debug_assert!(!matches!(mode, WindowMode::Headed));
                    mode
                }
                (mode, _) => mode,
            };

            let task = AppWindowTask::new(r.id, window_mode, color_scheme, r.new.into_inner(), r.responder);
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
    /// The dialog maybe modal in the view-process, in the app-process (caller) it is always async, the
    /// response var will update once when the user responds to the dialog.
    pub fn native_message_dialog(
        &self,
        window_id: WindowId,
        dialog: zero_ui_view_api::dialog::MsgDialog,
    ) -> ResponseVar<zero_ui_view_api::dialog::MsgDialogResponse> {
        let (responder, rsp) = response_var();
        WINDOWS_SV.write().view_window_task(window_id, move |win| match win {
            Some(win) => {
                if let Err(e) = win.message_dialog(dialog, responder.clone()) {
                    responder.respond(zero_ui_view_api::dialog::MsgDialogResponse::Error(formatx!("{e}")))
                }
            }
            None => responder.respond(zero_ui_view_api::dialog::MsgDialogResponse::Error(Txt::from_static(
                "native window not found",
            ))),
        });
        rsp
    }

    /// Show a native file dialog for the window.
    ///
    /// The dialog maybe modal in the view-process, in the app-process (caller) it is always async, the
    /// response var will update once when the user responds to the dialog.
    pub fn native_file_dialog(
        &self,
        window_id: WindowId,
        dialog: zero_ui_view_api::dialog::FileDialog,
    ) -> ResponseVar<zero_ui_view_api::dialog::FileDialogResponse> {
        let (responder, rsp) = response_var();
        WINDOWS_SV.write().view_window_task(window_id, move |win| match win {
            Some(win) => {
                if let Err(e) = win.file_dialog(dialog, responder.clone()) {
                    responder.respond(zero_ui_view_api::dialog::FileDialogResponse::Error(formatx!("{e}")))
                }
            }
            None => responder.respond(zero_ui_view_api::dialog::FileDialogResponse::Error(Txt::from_static(
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
    renderer: Option<ViewRenderer>,
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
            renderer: None,
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
    fn new(id: WindowId, mode: WindowMode, color_scheme: ColorScheme, new: UiTask<WindowRoot>, responder: ResponderVar<WindowId>) -> Self {
        let primary_scale_factor = MONITORS
            .primary_monitor()
            .map(|m| m.scale_factor().get())
            .unwrap_or_else(|| 1.fct());

        let mut ctx = WindowCtx::new(id, mode);

        let vars = WindowVars::new(WINDOWS_SV.read().default_render_mode.get(), primary_scale_factor, color_scheme);
        ctx.with_state(|s| s.borrow_mut().set(&WINDOW_VARS_ID, vars.clone()));

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
        });

        let mode = self.mode;
        let id = ctx.id();

        ctx.set_widget_tree(WidgetInfoTree::wgt(id, window.id));

        let vars = ctx.with_state(|s| s.borrow().get_clone(&WINDOW_VARS_ID)).unwrap();

        if window.kiosk {
            vars.chrome().set(WindowChrome::None);
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
        WINDOW.with_context(&mut self.ctx, || {
            self.ctrl.get_mut().close();
        });
    }

    fn view_task(&mut self, task: Box<dyn FnOnce(Option<&view_process::ViewWindow>) + Send>) {
        self.ctrl_in_ctx(|ctrl| ctrl.view_task(task));
    }
}

struct WindowLoading {
    handles: Vec<std::sync::Weak<WindowLoadingHandleData>>,
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
        let mut deadline = Deadline::timeout(1.hours());
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
        let h = Arc::new(WindowLoadingHandleData { update, deadline });
        self.handles.push(Arc::downgrade(&h));
        WindowLoadingHandle(h)
    }
}

/// Represents a handle that stops a window from opening while it exists.
///
/// A handle can be retrieved using [`WINDOWS.loading_handle`] or [`WINDOW.loading_handle`], the window does not
/// open until all handles are dropped.
///
/// [`WINDOWS.loading_handle`]: WINDOWS::loading_handle
/// [`WINDOW.loading_handle`]: WINDOW::loading_handle
#[derive(Clone)]
pub struct WindowLoadingHandle(Arc<WindowLoadingHandleData>);
impl WindowLoadingHandle {
    /// Handle expiration deadline.
    pub fn deadline(&self) -> Deadline {
        self.0.deadline
    }
}
struct WindowLoadingHandleData {
    update: AppEventSender,
    deadline: Deadline,
}
impl Drop for WindowLoadingHandleData {
    fn drop(&mut self) {
        let _ = self.update.send_update(UpdateOp::Update, None);
    }
}
impl PartialEq for WindowLoadingHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for WindowLoadingHandle {}
impl std::hash::Hash for WindowLoadingHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (Arc::as_ptr(&self.0) as usize).hash(state);
    }
}
impl fmt::Debug for WindowLoadingHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WindowLoadingHandle(_)")
    }
}

/// Extensions methods for [`WINDOW`] contexts of windows open by [`WINDOWS`].
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

    /// Gets a handle that stops the window from loading while it exists.
    ///
    /// The window is only opened in the view-process after it is loaded, without any loading handles the window is considered *loaded*
    /// after the first layout pass. Nodes in the window can request a loading handle to delay the view opening to after all async resources
    /// it requires to render correctly are loaded.
    ///
    /// Note that a window is only loaded after all handles are dropped or expired, you should set a reasonable `deadline`    
    /// after a time it is best to partially render a window than not showing anything.
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

    /// Move the window to the front of the Z stack.
    ///
    /// See [`WINDOWS.bring_to_top`] for more details.
    ///
    /// [`WINDOWS.bring_to_top`]: WINDOWS::bring_to_top
    fn bring_to_top(&self) {
        WINDOWS.bring_to_top(WINDOW.id()).ok();
    }

    /// Starts closing a window, the operation can be canceled by listeners of
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

impl ImageRenderWindowRoot for WindowRoot {}

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

    fn open_headless_window(&self, new_window_root: Box<dyn FnOnce() -> Box<dyn ImageRenderWindowRoot>>) {
        WINDOWS.open_headless(
            async move {
                let w = new_window_root();
                let vars = WINDOW.vars();
                vars.auto_size().set(true);
                vars.min_size().set((1.px(), 1.px()));
                w
            },
            true,
        )
    }

    fn on_frame_image_ready(&self, update: &EventUpdate) -> (WindowId, Img) {
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
