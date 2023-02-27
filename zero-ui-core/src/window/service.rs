use std::sync::Arc;
use std::{fmt, mem};

use linear_map::set::LinearSet;
use linear_map::LinearMap;
use parking_lot::Mutex;

use super::commands::WindowCommands;
use super::*;
use crate::app::raw_events::{RAW_COLOR_SCHEME_CHANGED_EVENT, RAW_WINDOW_OPEN_EVENT};
use crate::app::view_process::{VIEW_PROCESS, VIEW_PROCESS_INITED_EVENT};
use crate::app::{
    raw_events::{RAW_WINDOW_CLOSE_EVENT, RAW_WINDOW_CLOSE_REQUESTED_EVENT},
    view_process::{self, ColorScheme, ViewRenderer},
    AppEventSender,
};
use crate::app::{APP_PROCESS, EXIT_REQUESTED_EVENT};
use crate::context::{state_map, OwnedStateMap, WidgetUpdates, WindowCtx};
use crate::context::{UPDATES, WINDOW};
use crate::crate_util::IdSet;
use crate::event::{AnyEventArgs, EventUpdate};
use crate::image::{Image, ImageVar};
use crate::render::RenderMode;
use crate::timer::{DeadlineHandle, TIMERS};
use crate::widget_info::WidgetInfoTree;
use crate::{app_local, var::*};
use crate::{units::*, widget_instance::WidgetId};

app_local! {
    pub(super) static WINDOWS_SV: WindowsService = WindowsService::new();
}
pub(super) struct WindowsService {
    exit_on_last_close: ArcVar<bool>,
    default_render_mode: ArcVar<RenderMode>,

    windows: LinearMap<WindowId, AppWindow>,
    windows_info: LinearMap<WindowId, AppWindowInfo>,
    open_requests: Vec<OpenWindowRequest>,
    update_sender: Option<AppEventSender>,
    close_requests: Vec<CloseWindowRequest>,
    close_responders: LinearMap<WindowId, Vec<ResponderVar<CloseWindowResult>>>,
    focus_request: Option<WindowId>,
    bring_to_top_requests: Vec<WindowId>,
    frame_images: Vec<ArcVar<Image>>,
    loading_deadline: Option<DeadlineHandle>,
    latest_color_scheme: ColorScheme,
}
impl WindowsService {
    fn new() -> Self {
        Self {
            exit_on_last_close: var(true),
            default_render_mode: var(RenderMode::default()),
            windows: LinearMap::with_capacity(1),
            windows_info: LinearMap::with_capacity(1),
            open_requests: Vec::with_capacity(1),
            close_responders: LinearMap::with_capacity(1),
            update_sender: None,
            close_requests: vec![],
            focus_request: None,
            bring_to_top_requests: vec![],
            frame_images: vec![],
            loading_deadline: None,
            latest_color_scheme: ColorScheme::Dark,
        }
    }

    pub(super) fn init(&mut self, update_sender: AppEventSender) {
        self.update_sender = Some(update_sender);
    }

    fn open_impl(
        &mut self,
        id: WindowId,
        new_window: impl FnOnce() -> Window + Send + 'static,
        force_headless: Option<WindowMode>,
    ) -> ResponseVar<WindowOpenArgs> {
        let (responder, response) = response_var();
        let request = OpenWindowRequest {
            id,
            new: Mutex::new(Box::new(new_window)),
            force_headless,
            responder,
            loading_handle: WindowLoading::new(),
        };
        self.open_requests.push(request);
        let _ = self.update_sender.as_ref().expect("`WindowManager` not inited").send_ext_update();

        response
    }

    fn loading_handle_impl(&mut self, window_id: WindowId, deadline: Deadline) -> Option<WindowLoadingHandle> {
        let mut handle = None;

        if let Some(info) = self.windows_info.get_mut(&window_id) {
            // window already opened, check if not loaded
            if !info.is_loaded {
                handle = Some(
                    info.loading_handle
                        .new_handle(self.update_sender.as_ref().expect("`WindowManager` not inited"), deadline),
                )
            }

            // drop timer to nearest deadline, will recreate in the next update.
            self.loading_deadline = None;
        } else if let Some(request) = self.open_requests.iter_mut().find(|r| r.id == window_id) {
            // window not opened yet
            handle = Some(
                request
                    .loading_handle
                    .new_handle(self.update_sender.as_ref().expect("`WindowManager` not inited"), deadline),
            );
        }

        handle
    }

    fn close_together(&mut self, windows: impl IntoIterator<Item = WindowId>) -> Result<ResponseVar<CloseWindowResult>, WindowNotFound> {
        let mut group = LinearSet::new();

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
        let _ = self.update_sender.as_ref().expect("`WindowManager` not inited").send_ext_update();

        Ok(response)
    }

    fn frame_image_impl(
        &mut self,
        window_id: WindowId,
        action: impl FnOnce(&ViewRenderer) -> std::result::Result<view_process::ViewImage, view_process::ViewProcessOffline>,
    ) -> ImageVar {
        if let Some(w) = self.windows_info.get(&window_id) {
            if let Some(r) = &w.renderer {
                match action(r) {
                    Ok(img) => {
                        let img = Image::new(img);
                        let img = var(img);
                        self.frame_images.push(img.clone());
                        img.read_only()
                    }
                    Err(_) => var(Image::dummy(Some(format!("{}", WindowNotFound(window_id))))).read_only(),
                }
            } else {
                var(Image::dummy(Some(format!("window `{window_id}` is headless without renderer")))).read_only()
            }
        } else {
            var(Image::dummy(Some(format!("{}", WindowNotFound(window_id))))).read_only()
        }
    }

    fn take_requests(&mut self) -> (Vec<OpenWindowRequest>, Vec<CloseWindowRequest>, Option<WindowId>, Vec<WindowId>) {
        (
            mem::take(&mut self.open_requests),
            mem::take(&mut self.close_requests),
            self.focus_request.take(),
            mem::take(&mut self.bring_to_top_requests),
        )
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

    // Requests a new window.
    ///
    /// The `new_window` argument is the [`WindowContext`] of the new window.
    ///
    /// Returns a response variable that will update once when the window is opened, note that while the `window_id` is
    /// available in the `new_window` argument already, the window is only available in this service after
    /// the returned variable updates.
    ///
    /// An update cycle is processed between the call to `new_window` and the window init, this means that you
    /// can use the input [`WindowContext`] to set variables that will be read on init with the new value.
    pub fn open(&self, new_window: impl FnOnce() -> Window + Send + 'static) -> ResponseVar<WindowOpenArgs> {
        WINDOWS_SV.write().open_impl(WindowId::new_unique(), new_window, None)
    }

    /// Requests a new window with pre-defined ID.
    ///
    /// # Panics
    ///
    /// if the `window_id` is already assigned to an open or opening window.
    pub fn open_id(
        &self,
        window_id: impl Into<WindowId>,
        new_window: impl FnOnce() -> Window + Send + 'static,
    ) -> ResponseVar<WindowOpenArgs> {
        let window_id = window_id.into();
        self.assert_id_unused(window_id);
        WINDOWS_SV.write().open_impl(window_id, new_window, None)
    }

    /// Requests a new headless window.
    ///
    /// Headless windows don't show on screen, but if `with_renderer` is `true` they will still render frames.
    ///
    /// Note that in a headless app the [`open`] method also creates headless windows, this method
    /// creates headless windows even in a headed app.
    ///
    /// [`open`]: WINDOWS::open
    pub fn open_headless(&self, new_window: impl FnOnce() -> Window + Send + 'static, with_renderer: bool) -> ResponseVar<WindowOpenArgs> {
        WINDOWS_SV.write().open_impl(
            WindowId::new_unique(),
            new_window,
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
        new_window: impl FnOnce() -> Window + Send + 'static,
        with_renderer: bool,
    ) -> ResponseVar<WindowOpenArgs> {
        let window_id = window_id.into();
        self.assert_id_unused(window_id);
        WINDOWS_SV.write().open_impl(
            window_id,
            new_window,
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
        if w.windows_info.contains_key(&id) || w.open_requests.iter().any(|r| r.id == id) {
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

    /// Generate an image from the current rendered frame of the window.
    ///
    /// The image is not loaded at the moment of return, it will update when it is loaded.
    ///
    /// If the window is not found the error is reported in the image error.
    pub fn frame_image(&self, window_id: impl Into<WindowId>) -> ImageVar {
        WINDOWS_SV.write().frame_image_impl(window_id.into(), |vr| vr.frame_image())
    }

    /// Generate an image from a selection of the current rendered frame of the window.
    ///
    /// The image is not loaded at the moment of return, it will update when it is loaded.
    ///
    /// If the window is not found the error is reported in the image error.
    pub fn frame_image_rect(&self, window_id: impl Into<WindowId>, rect: PxRect) -> ImageVar {
        WINDOWS_SV
            .write()
            .frame_image_impl(window_id.into(), |vr| vr.frame_image_rect(rect))
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
        } else if w.open_requests.iter().any(|r| r.id == window_id) {
            Ok(false)
        } else {
            Err(WindowNotFound(window_id))
        }
    }

    /// Iterate over the widget trees of each open window.
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

    /// Returns `true` if a pending window open request is associated with the ID.
    ///
    /// Window open requests are processed after each update.
    pub fn is_open_request(&self, window_id: impl Into<WindowId>) -> bool {
        let window_id = window_id.into();
        WINDOWS_SV.read().open_requests.iter().any(|r| r.id == window_id)
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
            let _ = w.update_sender.as_ref().expect("`WindowManager` not inited").send_ext_update();
        }
        Ok(())
    }

    /// Focus a window if it is open or opening, otherwise opens it focused.
    pub fn focus_or_open(
        &self,
        window_id: impl Into<WindowId>,
        open: impl FnOnce() -> Window + Send + 'static,
    ) -> Option<ResponseVar<WindowOpenArgs>> {
        let window_id = window_id.into();
        if self.focus(window_id).is_ok() {
            None
        } else {
            let r = self.open_id(window_id, open);
            WINDOWS_SV.write().focus_request = Some(window_id);
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
            let _ = w.update_sender.as_ref().expect("`WindowManager` not inited").send_ext_update();
            Ok(())
        } else {
            Err(WindowNotFound(window_id))
        }
    }

    /// Update the reference to the renderer associated with the window, we need
    /// the render to enable the hit-test function.
    pub(super) fn set_renderer(&self, id: WindowId, renderer: ViewRenderer) {
        if let Some(info) = WINDOWS_SV.write().windows_info.get_mut(&id) {
            info.renderer = Some(renderer);
        }
    }

    /// Update widget info tree associated with the window.
    pub(super) fn set_widget_tree(&self, info_tree: WidgetInfoTree, pending_layout: bool, pending_render: bool) {
        if let Some(info) = WINDOWS_SV.write().windows_info.get_mut(&info_tree.window_id()) {
            let prev_tree = info.widget_tree.clone();
            info.widget_tree = info_tree.clone();

            let args = WidgetInfoChangedArgs::now(
                info_tree.window_id(),
                prev_tree.clone(),
                info_tree.clone(),
                pending_layout,
                pending_render,
            );
            WIDGET_INFO_CHANGED_EVENT.notify(args);

            let mut targets = IdSet::default();
            INTERACTIVITY_CHANGED_EVENT.visit_subscribers(|wid| {
                if let Some(wgt) = info_tree.get(wid) {
                    let prev = prev_tree.get(wid).map(|w| w.interactivity());
                    let new_int = wgt.interactivity();
                    if prev != Some(new_int) {
                        targets.insert(wid);
                    }
                }
            });
            if !targets.is_empty() {
                let args = InteractivityChangedArgs::now(prev_tree, info_tree, targets);
                INTERACTIVITY_CHANGED_EVENT.notify(args);
            }
        }
    }

    /// Change window state to loaded if there are no load handles active.
    ///
    /// Returns `true` if loaded.
    pub(super) fn try_load(&self, window_id: WindowId) -> bool {
        if let Some(info) = WINDOWS_SV.write().windows_info.get_mut(&window_id) {
            info.is_loaded = info.loading_handle.try_load();

            if info.is_loaded && !info.vars.0.is_loaded.get() {
                info.vars.0.is_loaded.set_ne(true);
                WINDOW_LOAD_EVENT.notify(WindowOpenArgs::now(info.id));
            }

            info.is_loaded
        } else {
            unreachable!()
        }
    }

    pub(super) fn on_pre_event(update: &mut EventUpdate) {
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
                        window.vars.focus_indicator().set_ne(None);
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
                let mut windows = LinearSet::with_capacity(1);
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
            UPDATES.update_ext();
        }

        Self::with_detached_windows(|windows| {
            for (_, window) in windows {
                window.pre_event(update);
            }
        })
    }

    pub(super) fn on_ui_event(update: &mut EventUpdate) {
        if update.delivery_list().has_pending_search() {
            update.fulfill_search(WINDOWS_SV.read().windows_info.values().map(|w| &w.widget_tree));
        }
        Self::with_detached_windows(|windows| {
            for (_, window) in windows {
                window.ui_event(update);
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

            for w in &args.windows {
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

            let is_headless_app = app::App::window_mode().is_headless();
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

    pub(super) fn on_ui_update(updates: &mut WidgetUpdates) {
        Self::fullfill_requests();

        if updates.delivery_list().has_pending_search() {
            updates.fulfill_search(WINDOWS_SV.read().windows_info.values().map(|w| &w.widget_tree));
        }

        Self::with_detached_windows(|windows| {
            for (_, window) in windows {
                window.update(updates);
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

        let ((open, close, focus, bring_to_top), color_scheme) = {
            let mut wns = WINDOWS_SV.write();
            (wns.take_requests(), wns.latest_color_scheme)
        };

        let window_mode = app::App::window_mode();

        // fulfill open requests.
        for r in open {
            let window_mode = match (window_mode, r.force_headless) {
                (WindowMode::Headed | WindowMode::HeadlessWithRenderer, Some(mode)) => {
                    debug_assert!(!matches!(mode, WindowMode::Headed));
                    mode
                }
                (mode, _) => mode,
            };

            let (window, info) = AppWindow::new(r.id, window_mode, color_scheme, r.new.into_inner(), r.loading_handle);

            let args = WindowOpenArgs::now(window.ctx.id());
            {
                let mut wns = WINDOWS_SV.write();
                if wns.windows.insert(window.ctx.id(), window).is_some() {
                    // id conflict resolved on request.
                    unreachable!();
                }
                wns.windows_info.insert(info.id, info);
            }

            r.responder.respond(args.clone());
            WINDOW_OPEN_EVENT.notify(args);
        }

        // notify close requests, the request is fulfilled or canceled
        // in the `event` handler.

        {
            let mut wns = WINDOWS_SV.write();
            let wns = &mut *wns;

            let mut close_wns = LinearSet::new();

            for r in close {
                for w in r.windows {
                    if let Some(info) = wns.windows_info.get(&w) {
                        if close_wns.insert(w) {
                            wns.close_responders
                                .entry(w)
                                .or_insert_with(Default::default)
                                .push(r.responder.clone());

                            info.vars.0.children.with(|c| {
                                for &c in c {
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
            Self::with_detached_windows(|windows| {
                if let Some(w) = windows.get_mut(&w_id) {
                    w.focus();
                }
            });
        }

        for w_id in bring_to_top {
            Self::with_detached_windows(|windows| {
                if let Some(w) = windows.get_mut(&w_id) {
                    w.bring_to_top();
                }
            });
        }
    }

    pub(super) fn on_layout() {
        Self::with_detached_windows(|windows| {
            for (_, window) in windows {
                window.layout();
            }
        });
    }

    pub(super) fn on_render() {
        Self::with_detached_windows(|windows| {
            for (_, window) in windows {
                window.render();
            }
        });
    }

    /// Takes ownership of [`Windows::windows`] for the duration of the call to `f`.
    ///
    /// The windows map is empty for the duration of `f` and should not be used, this is for
    /// mutating the window content while still allowing it to query the `Windows::windows_info`.
    fn with_detached_windows(f: impl FnOnce(&mut LinearMap<WindowId, AppWindow>)) {
        let mut windows = mem::take(&mut WINDOWS_SV.write().windows);
        f(&mut windows);
        let mut wns = WINDOWS_SV.write();
        debug_assert!(wns.windows.is_empty());
        wns.windows = windows;
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
    new: Mutex<Box<dyn FnOnce() -> Window + Send>>, // never locked, makes `OpenWindowRequest: Sync`
    force_headless: Option<WindowMode>,
    responder: ResponderVar<WindowOpenArgs>,
    loading_handle: WindowLoading,
}
struct CloseWindowRequest {
    responder: ResponderVar<CloseWindowResult>,
    windows: LinearSet<WindowId>,
}

/// Window context owner.
struct AppWindow {
    ctrl: Mutex<WindowCtrl>, // never locked, makes `AppWindow: Sync`.
    pub(super) ctx: WindowCtx,
}
impl AppWindow {
    pub fn new(
        id: WindowId,
        mode: WindowMode,
        color_scheme: ColorScheme,
        new: Box<dyn FnOnce() -> Window>,
        loading: WindowLoading,
    ) -> (Self, AppWindowInfo) {
        let primary_scale_factor = MONITORS
            .primary_monitor()
            .map(|m| m.scale_factor().get())
            .unwrap_or_else(|| 1.fct());

        let mut ctx = WindowCtx::new(id, mode);

        let vars = WindowVars::new(WINDOWS_SV.read().default_render_mode.get(), primary_scale_factor, color_scheme);
        ctx.state().borrow_mut().set(&WINDOW_VARS_ID, vars.clone());

        let window = WINDOW.with_context(&ctx, new);

        ctx.set_widget_tree(WidgetInfoTree::wgt(id, window.id));

        if window.kiosk {
            vars.chrome().set_ne(WindowChrome::None);
            vars.visible().set_ne(true);
            if !vars.state().get().is_fullscreen() {
                vars.state().set(WindowState::Exclusive);
            }
        }

        let commands = WindowCommands::new(id);

        let root_id = window.id;
        let ctrl = WindowCtrl::new(id, &vars, commands, mode, window);

        let window = Self {
            ctrl: Mutex::new(ctrl),
            ctx,
        };
        let info = AppWindowInfo::new(id, root_id, mode, vars, loading);

        (window, info)
    }

    fn ctrl_in_ctx(&mut self, action: impl FnOnce(&mut WindowCtrl)) {
        WINDOW.with_context(&self.ctx, || {
            action(self.ctrl.get_mut());
        });
        self.ctrl.get_mut().window_updates()
    }

    pub fn pre_event(&mut self, update: &mut EventUpdate) {
        self.ctrl_in_ctx(|ctrl| ctrl.pre_event(update))
    }

    pub fn ui_event(&mut self, update: &mut EventUpdate) {
        self.ctrl_in_ctx(|ctrl| ctrl.ui_event(update))
    }

    pub fn update(&mut self, updates: &mut WidgetUpdates) {
        self.ctrl_in_ctx(|ctrl| ctrl.update(updates));
    }

    pub fn layout(&mut self) {
        self.ctrl_in_ctx(|ctrl| ctrl.layout());
    }

    pub fn render(&mut self) {
        self.ctrl_in_ctx(|ctrl| ctrl.render());
    }

    pub fn focus(&mut self) {
        self.ctrl_in_ctx(|ctrl| ctrl.focus());
    }

    pub fn bring_to_top(&mut self) {
        self.ctrl_in_ctx(|ctrl| ctrl.bring_to_top());
    }

    pub fn close(mut self) {
        WINDOW.with_context(&self.ctx, || {
            self.ctrl.get_mut().close();
        });
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
    pub fn try_load(&mut self) -> bool {
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
                        UPDATES.update_ext();
                    }),
                );
                self.timer = Some(t);
            }

            false
        }
    }

    pub fn new_handle(&mut self, update: &AppEventSender, deadline: Deadline) -> WindowLoadingHandle {
        let h = Arc::new(WindowLoadingHandleData {
            update: update.clone(),
            deadline,
        });
        self.handles.push(Arc::downgrade(&h));
        WindowLoadingHandle(h)
    }
}

/// Represents a handle that stops a window from opening while it exists.
///
/// A handle can be retrieved using [`WINDOWS.loading_handle`], the window does not
/// open until all handles are dropped.
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
        let _ = self.update.send_ext_update();
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
