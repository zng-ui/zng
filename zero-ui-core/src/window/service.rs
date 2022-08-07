use std::sync::Arc;
use std::{fmt, mem};

use linear_map::set::LinearSet;
use linear_map::LinearMap;

use super::commands::WindowCommands;
use super::*;
use crate::app::view_process::{ViewProcess, ViewProcessInitedEvent};
use crate::app::{AppProcess, ExitRequestedEvent};
use crate::context::{state_map, OwnedStateMap};
use crate::event::EventUpdateArgs;
use crate::image::{Image, ImageVar};
use crate::render::RenderMode;
use crate::service::Service;
use crate::var::*;
use crate::widget_info::{WidgetInfoTree, WidgetSubscriptions};
use crate::{
    app::{
        raw_events::{RawWindowCloseEvent, RawWindowCloseRequestedEvent},
        view_process::{self, ViewRenderer},
        AppEventSender,
    },
    event::Events,
};
use crate::{units::*, WidgetId};

/// Windows service.
///
/// # Provider
///
/// This service is provided by the [`WindowManager`].
#[derive(Service)]
pub struct Windows {
    /// If app process exit is requested when a window closes and there are no more windows open, `true` by default.
    ///
    /// This setting is ignored in headless apps, in headed apps the exit happens when all headed windows
    /// are closed, headless windows are ignored.
    ///
    /// If app exit is requested directly and there are headed windows open the exit op is canceled, the windows request close
    /// and this is set to `true` so that another exit request is made after the windows close.
    pub exit_on_last_close: bool,

    /// Default render mode of windows opened by this service, the initial value is [`RenderMode::default`].
    ///
    /// Note that this setting only affects windows opened after it is changed, also the view-process may select
    /// a different render mode if it cannot support the requested mode.
    pub default_render_mode: RenderMode,

    windows: LinearMap<WindowId, AppWindow>,
    windows_info: LinearMap<WindowId, AppWindowInfo>,
    open_requests: Vec<OpenWindowRequest>,
    update_sender: AppEventSender,
    close_requests: Vec<CloseWindowRequest>,
    close_responders: LinearMap<WindowId, ResponderVar<CloseWindowResult>>,
    focus_request: Option<WindowId>,
    frame_images: Vec<RcVar<Image>>,
}
impl Windows {
    pub(super) fn new(update_sender: AppEventSender) -> Self {
        Windows {
            exit_on_last_close: true,
            default_render_mode: RenderMode::default(),
            windows: LinearMap::with_capacity(1),
            windows_info: LinearMap::with_capacity(1),
            open_requests: Vec::with_capacity(1),
            close_responders: LinearMap::with_capacity(1),
            update_sender,
            close_requests: vec![],
            focus_request: None,
            frame_images: vec![],
        }
    }

    // Requests a new window.
    ///
    /// The `new_window` argument is the [`WindowContext`] of the new window.
    ///
    /// Returns a response variable that will update once when the window is opened, note that while the `window_id` is
    /// available in the `new_window` argument already, the window is only available in this service after
    /// the returned variable updates.
    pub fn open(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) -> ResponseVar<WindowOpenArgs> {
        self.open_impl(WindowId::new_unique(), new_window, None)
    }

    /// Requests a new window with pre-defined ID.
    ///
    /// # Panics
    ///
    /// if the `window_id` is already assigned to an open or opening window.
    pub fn open_id(
        &mut self,
        window_id: impl Into<WindowId>,
        new_window: impl FnOnce(&mut WindowContext) -> Window + 'static,
    ) -> ResponseVar<WindowOpenArgs> {
        let window_id = window_id.into();
        self.assert_id_unused(window_id);
        self.open_impl(window_id, new_window, None)
    }

    /// Requests a new headless window.
    ///
    /// Headless windows don't show on screen, but if `with_renderer` is `true` they will still render frames.
    ///
    /// Note that in a headless app the [`open`] method also creates headless windows, this method
    /// creates headless windows even in a headed app.
    ///
    /// [`open`]: Windows::open
    pub fn open_headless(
        &mut self,
        new_window: impl FnOnce(&mut WindowContext) -> Window + 'static,
        with_renderer: bool,
    ) -> ResponseVar<WindowOpenArgs> {
        self.open_impl(
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
        &mut self,
        window_id: impl Into<WindowId>,
        new_window: impl FnOnce(&mut WindowContext) -> Window + 'static,
        with_renderer: bool,
    ) -> ResponseVar<WindowOpenArgs> {
        let window_id = window_id.into();
        self.assert_id_unused(window_id);
        self.open_impl(
            window_id,
            new_window,
            Some(if with_renderer {
                WindowMode::HeadlessWithRenderer
            } else {
                WindowMode::Headless
            }),
        )
    }

    fn assert_id_unused(&self, id: WindowId) {
        if self.windows_info.contains_key(&id) || self.open_requests.iter().any(|r| r.id == id) {
            panic!("window id `{id:?}` is already in use")
        }
    }

    fn open_impl(
        &mut self,
        id: WindowId,
        new_window: impl FnOnce(&mut WindowContext) -> Window + 'static,
        force_headless: Option<WindowMode>,
    ) -> ResponseVar<WindowOpenArgs> {
        let (responder, response) = response_var();
        let request = OpenWindowRequest {
            id,
            new: Box::new(new_window),
            force_headless,
            responder,
            loading_handle: WindowLoading::new(),
        };
        self.open_requests.push(request);
        let _ = self.update_sender.send_ext_update();

        response
    }

    /// Gets a handle that stops the window from loading while it exists.
    ///
    /// A window is only opened in the view-process after it is loaded, without any loading handles the window is considered *loaded*
    /// after the first layout pass. Nodes in the window can request a loading handle to delay the view opening to after all async resources
    /// it requires to render correctly are loaded.
    ///
    /// Note that a window is only loaded after all handles are dropped, in practice a timeout should be used to avoid awaiting for too long,
    /// after a time it is best to partially render a window than not showing anything.
    ///
    /// Returns `None` if the window has already loaded or is not found.
    pub fn loading_handle(&mut self, window_id: impl Into<WindowId>, deadline: impl Into<Deadline>) -> Option<WindowLoadingHandle> {
        self.loading_handle_impl(window_id.into(), deadline.into())
    }
    fn loading_handle_impl(&mut self, window_id: WindowId, deadline: Deadline) -> Option<WindowLoadingHandle> {
        if let Some(info) = self.windows_info.get_mut(&window_id) {
            if info.is_loaded {
                None
            } else {
                Some(info.loading_handle.handle(&self.update_sender, deadline))
            }
        } else if let Some(request) = self.open_requests.iter_mut().find(|r| r.id == window_id) {
            Some(request.loading_handle.handle(&self.update_sender, deadline))
        } else {
            None
        }
    }

    /// Starts closing a window, the operation can be canceled by listeners of
    /// [`WindowCloseRequestedEvent`].
    ///
    /// Returns a response var that will update once with the result of the operation.
    ///
    /// Returns [`WindowNotFound`] if the `window_id` is not one of the open windows or is only an open request.
    pub fn close(&mut self, window_id: impl Into<WindowId>) -> Result<ResponseVar<CloseWindowResult>, WindowNotFound> {
        self.close_impl(window_id.into())
    }
    fn close_impl(&mut self, window_id: WindowId) -> Result<ResponseVar<CloseWindowResult>, WindowNotFound> {
        if self.windows_info.contains_key(&window_id) {
            for req in &self.close_requests {
                if req.windows.contains(&window_id) {
                    return Ok(req.responder.response_var());
                }
            }

            let (responder, response) = response_var();
            self.close_requests.push(CloseWindowRequest {
                responder,
                windows: [window_id].into_iter().collect(),
            });
            let _ = self.update_sender.send_ext_update();

            Ok(response)
        } else {
            Err(WindowNotFound(window_id))
        }
    }

    /// Requests closing multiple windows together, the operation can be canceled by listeners of the
    /// [`WindowCloseRequestedEvent`]. If canceled none of the windows are closed.
    ///
    /// Returns a response var that will update once with the result of the operation. Returns
    /// [`Cancel`] if `windows` is empty or only contained windows that already requested close
    /// during this update.
    ///
    /// Returns [`WindowNotFound`] if any of the IDs is not one of the open windows or is only an open request.
    ///
    /// [`Cancel`]: CloseWindowResult::Cancel
    pub fn close_together(
        &mut self,
        windows: impl IntoIterator<Item = WindowId>,
    ) -> Result<ResponseVar<CloseWindowResult>, WindowNotFound> {
        let mut group = LinearSet::new();

        for w in windows {
            if !self.windows_info.contains_key(&w) {
                return Err(WindowNotFound(w));
            }

            if !self.close_requests.iter().any(|r| r.windows.contains(&w)) {
                group.insert(w);
            }
        }

        if group.is_empty() {
            return Ok(response_done_var(CloseWindowResult::Cancel));
        }

        let (responder, response) = response_var();
        self.close_requests.push(CloseWindowRequest { responder, windows: group });
        let _ = self.update_sender.send_ext_update();

        Ok(response)
    }

    /// Requests close of all open windows together, the operation can be canceled by listeners of
    /// the [`WindowCloseRequestedEvent`]. If canceled none of the windows are closed.
    ///
    /// Returns a response var that will update once with the result of the operation, Returns
    /// [`Cancel`] if no window is open or if close was already requested to all of the windows.
    ///
    /// [`Cancel`]: CloseWindowResult::Cancel
    pub fn close_all(&mut self) -> ResponseVar<CloseWindowResult> {
        let set: Vec<_> = self.windows_info.keys().copied().collect();
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
        self.windows_info.get(&window_id).map(|w| w.mode).ok_or(WindowNotFound(window_id))
    }

    /// Reference the metadata about the window's widgets.
    ///
    /// Returns [`WindowNotFound`] if the `window_id` is not one of the open windows or is only an open request.
    pub fn widget_tree(&self, window_id: impl Into<WindowId>) -> Result<&WidgetInfoTree, WindowNotFound> {
        let window_id = window_id.into();
        self.windows_info
            .get(&window_id)
            .map(|w| &w.widget_tree)
            .ok_or(WindowNotFound(window_id))
    }

    /// Reference the current window's subscriptions.
    ///
    /// Returns [`WindowNotFound`] if the `window_id` is not one of the open windows or is only an open request.
    pub fn subscriptions(&self, window_id: impl Into<WindowId>) -> Result<&WidgetSubscriptions, WindowNotFound> {
        let window_id = window_id.into();
        self.windows_info
            .get(&window_id)
            .map(|w| &w.subscriptions)
            .ok_or(WindowNotFound(window_id))
    }

    /// Generate an image from the current rendered frame of the window.
    ///
    /// The image is not loaded at the moment of return, it will update when it is loaded.
    ///
    /// If the window is not found the error is reported in the image error.
    pub fn frame_image(&mut self, window_id: impl Into<WindowId>) -> ImageVar {
        self.frame_image_impl(window_id.into(), |vr| vr.frame_image())
    }

    /// Generate an image from a selection of the current rendered frame of the window.
    ///
    /// The image is not loaded at the moment of return, it will update when it is loaded.
    ///
    /// If the window is not found the error is reported in the image error.
    pub fn frame_image_rect(&mut self, window_id: impl Into<WindowId>, rect: PxRect) -> ImageVar {
        self.frame_image_impl(window_id.into(), |vr| vr.frame_image_rect(rect))
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
                        img.into_read_only()
                    }
                    Err(_) => var(Image::dummy(Some(format!("{}", WindowNotFound(window_id))))).into_read_only(),
                }
            } else {
                var(Image::dummy(Some(format!("window `{window_id}` is headless without renderer")))).into_read_only()
            }
        } else {
            var(Image::dummy(Some(format!("{}", WindowNotFound(window_id))))).into_read_only()
        }
    }

    /// Reference the [`WindowVars`] for the window.
    ///
    /// Returns [`WindowNotFound`] if the `window_id` is not one of the open windows or is only an open request.
    pub fn vars(&self, window_id: impl Into<WindowId>) -> Result<&WindowVars, WindowNotFound> {
        let window_id = window_id.into();
        self.windows_info.get(&window_id).map(|w| &w.vars).ok_or(WindowNotFound(window_id))
    }

    /// Gets if the window is focused in the OS.
    ///
    /// Returns [`WindowNotFound`] if the `window_id` is not one of the open windows, returns `false` if the `window_id` is
    /// one of the open requests.
    pub fn is_focused(&self, window_id: impl Into<WindowId>) -> Result<bool, WindowNotFound> {
        let window_id = window_id.into();
        if let Some(w) = self.windows_info.get(&window_id) {
            Ok(w.is_focused)
        } else if self.open_requests.iter().any(|r| r.id == window_id) {
            Ok(false)
        } else {
            Err(WindowNotFound(window_id))
        }
    }

    /// Iterate over the widget trees of each open window.
    pub fn widget_trees(&self) -> impl Iterator<Item = &WidgetInfoTree> {
        self.windows_info.values().map(|w| &w.widget_tree)
    }

    /// Gets the id of the window that is focused in the OS.
    pub fn focused_window_id(&self) -> Option<WindowId> {
        self.windows_info.values().find(|w| w.is_focused).map(|w| w.id)
    }

    /// Gets the latest frame for the focused window.
    pub fn focused_info(&self) -> Option<&WidgetInfoTree> {
        self.windows_info.values().find(|w| w.is_focused).map(|w| &w.widget_tree)
    }

    /// Returns `true` if the window is open.
    pub fn is_open(&self, window_id: impl Into<WindowId>) -> bool {
        self.windows_info.contains_key(&window_id.into())
    }

    /// Returns `true` if a pending window open request is associated with the ID.
    ///
    /// Window open requests are processed after each update.
    pub fn is_open_request(&self, window_id: impl Into<WindowId>) -> bool {
        let window_id = window_id.into();
        self.open_requests.iter().any(|r| r.id == window_id)
    }

    /// Returns `true` if the window is open and loaded.
    pub fn is_loaded(&self, window_id: impl Into<WindowId>) -> bool {
        let window_id = window_id.into();
        self.windows_info.get(&window_id).map(|i| i.is_loaded).unwrap_or(false)
    }

    /// Requests that the window be made the foreground keyboard focused window.
    ///
    /// Prefer using the [`Focus`] service and advanced [`FocusRequest`] configs instead of using this method directly.
    ///
    /// This operation can steal keyboard focus from other apps disrupting the user, be careful with it.
    ///
    /// If the `window_id` is only associated with an open request it is modified to focus the window on open.
    ///
    /// [`Focus`]: crate::focus::Focus
    /// [`FocusRequest`]: crate::focus::FocusRequest
    pub fn focus(&mut self, window_id: impl Into<WindowId>) -> Result<(), WindowNotFound> {
        let window_id = window_id.into();
        if !self.is_focused(window_id)? {
            self.focus_request = Some(window_id);
            let _ = self.update_sender.send_ext_update();
        }
        Ok(())
    }

    /// Focus a window if it is open or opening, otherwise opens it focused.
    pub fn focus_or_open(
        &mut self,
        window_id: impl Into<WindowId>,
        open: impl FnOnce(&mut WindowContext) -> Window + 'static,
    ) -> Option<ResponseVar<WindowOpenArgs>> {
        let window_id = window_id.into();
        if self.focus(window_id).is_ok() {
            None
        } else {
            let r = self.open_id(window_id, open);
            self.focus_request = Some(window_id);
            Some(r)
        }
    }

    fn take_requests(&mut self) -> (Vec<OpenWindowRequest>, Vec<CloseWindowRequest>, Option<WindowId>) {
        (
            mem::take(&mut self.open_requests),
            mem::take(&mut self.close_requests),
            self.focus_request.take(),
        )
    }

    /// Update the reference to the renderer associated with the window, we need
    /// the render to enable the hit-test function.
    pub(super) fn set_renderer(&mut self, id: WindowId, renderer: ViewRenderer) {
        if let Some(info) = self.windows_info.get_mut(&id) {
            info.renderer = Some(renderer);
        }
    }

    /// Update widget info tree associated with the window.
    pub(super) fn set_widget_tree(&mut self, events: &mut Events, info_tree: WidgetInfoTree, pending_layout: bool, pending_render: bool) {
        if let Some(info) = self.windows_info.get_mut(&info_tree.window_id()) {
            let prev_tree = info.widget_tree.clone();
            info.widget_tree = info_tree.clone();

            let args = WidgetInfoChangedArgs::now(info_tree.window_id(), prev_tree, info_tree, pending_layout, pending_render);
            WidgetInfoChangedEvent.notify(events, args);
        }
    }

    /// Update window info subscriptions copy.
    pub(super) fn set_subscriptions(&mut self, window_id: WindowId, subscriptions: WidgetSubscriptions) {
        if let Some(info) = self.windows_info.get_mut(&window_id) {
            info.subscriptions = subscriptions;
        }
    }

    /// Change window state to loaded if there are no load handles active.
    ///
    /// Returns `true` if loaded.
    pub(super) fn try_load(&mut self, window_id: WindowId, vars: &Vars) -> bool {
        if let Some(info) = self.windows_info.get_mut(&window_id) {
            if info.loading_handle.is_loading() {
                false
            } else {
                info.is_loaded = true;
                info.vars.0.is_loaded.set_ne(vars, true);
                true
            }
        } else {
            unreachable!()
        }
    }

    pub(super) fn on_pre_event<EV: EventUpdateArgs>(ctx: &mut AppContext, args: &EV) {
        if let Some(args) = RawWindowFocusEvent.update(args) {
            let wns = Windows::req(ctx.services);

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
                        window.vars.focus_indicator().set_ne(ctx.vars, None);
                        new = Some(new_focus);
                    }
                }
            }

            if prev.is_some() || new.is_some() {
                let args = WindowFocusChangedArgs::new(args.timestamp, args.propagation().clone(), prev, new, false);
                WindowFocusChangedEvent.notify(ctx.events, args);
            }
        } else if let Some(args) = RawWindowCloseRequestedEvent.update(args) {
            let _ = Windows::req(ctx.services).close(args.window_id);
        } else if let Some(args) = RawWindowCloseEvent.update(args) {
            if Windows::req(ctx.services).windows.contains_key(&args.window_id) {
                tracing::error!("view-process closed window without request");
                let mut windows = LinearSet::with_capacity(1);
                windows.insert(args.window_id);
                let args = WindowCloseArgs::new(args.timestamp, args.propagation().clone(), windows);
                WindowCloseEvent.notify(ctx, args);
            }
        } else if ViewProcessInitedEvent.update(args).is_some() {
            // we skipped request fulfillment until this event.
            ctx.updates.update_ext();
        }

        Self::with_detached_windows(ctx, |ctx, windows| {
            for (_, window) in windows {
                window.pre_event(ctx, args);
            }
        })
    }

    pub(super) fn on_ui_event<EV: EventUpdateArgs>(ctx: &mut AppContext, args: &EV) {
        Self::with_detached_windows(ctx, |ctx, windows| {
            for (_, window) in windows {
                window.ui_event(ctx, args);
            }
        });
    }

    pub(super) fn on_event<EV: EventUpdateArgs>(ctx: &mut AppContext, args: &EV) {
        if let Some(args) = WindowCloseRequestedEvent.update(args) {
            let key = args.windows.iter().next().unwrap();
            if let Some(rsp) = Windows::req(ctx.services).close_responders.remove(key) {
                if !args.propagation().is_stopped() {
                    // close requested by us and not canceled.
                    WindowCloseEvent.notify(ctx.events, WindowCloseArgs::now(args.windows.clone()));
                    rsp.respond(ctx, CloseWindowResult::Closed);
                } else {
                    rsp.respond(ctx, CloseWindowResult::Cancel);
                }
            }
        } else if let Some(args) = WindowCloseEvent.update(args) {
            // finish close, this notifies  `UiNode::deinit` and drops the window
            // causing the ViewWindow to drop and close.

            for w in &args.windows {
                if let Some(w) = Windows::req(ctx.services).windows.remove(w) {
                    let id = w.id;
                    w.close(ctx);

                    let wns = Windows::req(ctx.services);
                    let info = wns.windows_info.remove(&id).unwrap();

                    info.vars.0.is_open.set(ctx.vars, false);

                    if info.is_focused {
                        let args = WindowFocusChangedArgs::now(Some(info.id), None, true);
                        WindowFocusChangedEvent.notify(ctx.events, args)
                    }
                }
            }

            let is_headless_app = app::App::window_mode(ctx.services).is_headless();
            let wns = Windows::req(ctx.services);

            // if set to exit on last headed window close in a headed app,
            // AND there is no more open headed window OR request for opening a headed window.
            if wns.exit_on_last_close
                && !is_headless_app
                && !wns.windows.values().any(|w| matches!(w.mode, WindowMode::Headed))
                && !wns
                    .open_requests
                    .iter()
                    .any(|w| matches!(w.force_headless, None | Some(WindowMode::Headed)))
            {
                // fulfill `exit_on_last_close`
                AppProcess::req(ctx.services).exit();
            }
        } else if let Some(args) = ExitRequestedEvent.update(args) {
            if !args.propagation().is_stopped() {
                let windows = Windows::req(ctx.services);
                if windows.windows_info.values().any(|w| w.mode == WindowMode::Headed) {
                    args.propagation().stop();
                    windows.exit_on_last_close = true;
                    windows.close_all();
                }
            }
        }
    }

    pub(super) fn on_ui_update(ctx: &mut AppContext) {
        Self::fullfill_requests(ctx);

        Self::with_detached_windows(ctx, |ctx, windows| {
            for (_, window) in windows {
                window.update(ctx);
            }
        });
    }

    pub(super) fn on_update(ctx: &mut AppContext) {
        Self::fullfill_requests(ctx);
    }

    fn fullfill_requests(ctx: &mut AppContext) {
        if let Some(vp) = ctx.services.get::<ViewProcess>() {
            if !vp.online() {
                // wait ViewProcessInitedEvent
                return;
            }
        }

        let (open, close, focus) = {
            let wns = Windows::req(ctx.services);
            wns.take_requests()
        };

        let window_mode = app::App::window_mode(ctx.services);

        // fulfill open requests.
        for r in open {
            let window_mode = match (window_mode, r.force_headless) {
                (WindowMode::Headed | WindowMode::HeadlessWithRenderer, Some(mode)) => {
                    debug_assert!(!matches!(mode, WindowMode::Headed));
                    mode
                }
                (mode, _) => mode,
            };

            let (window, info) = AppWindow::new(ctx, r.id, window_mode, r.new, r.loading_handle);

            let args = WindowOpenArgs::now(window.id);
            {
                let wns = Windows::req(ctx.services);
                if wns.windows.insert(window.id, window).is_some() {
                    // id conflict resolved on request.
                    unreachable!();
                }
                wns.windows_info.insert(info.id, info);
            }

            r.responder.respond(ctx, args.clone());
            WindowOpenEvent.notify(ctx, args);
        }

        let wns = Windows::req(ctx.services);

        // notify close requests, the request is fulfilled or canceled
        // in the `event` handler.
        for r in close {
            wns.close_responders.insert(*r.windows.iter().next().unwrap(), r.responder);

            let args = WindowCloseRequestedArgs::now(r.windows);
            WindowCloseRequestedEvent.notify(ctx.events, args);
        }

        // fulfill focus request
        if let Some(w_id) = focus {
            Self::with_detached_windows(ctx, |ctx, windows| {
                if let Some(w) = windows.get_mut(&w_id) {
                    w.focus(ctx);
                }
            });
        }
    }

    pub(super) fn on_layout(ctx: &mut AppContext) {
        Self::with_detached_windows(ctx, |ctx, windows| {
            for (_, window) in windows {
                window.layout(ctx);
            }
        });
    }

    pub(super) fn on_render(ctx: &mut AppContext) {
        Self::with_detached_windows(ctx, |ctx, windows| {
            for (_, window) in windows {
                window.render(ctx);
            }
        });
    }

    /// Takes ownership of [`Windows::windows`] for the duration of the call to `f`.
    ///
    /// The windows map is empty for the duration of `f` and should not be used, this is for
    /// mutating the window content while still allowing it to query the `Windows::windows_info`.
    fn with_detached_windows(ctx: &mut AppContext, f: impl FnOnce(&mut AppContext, &mut LinearMap<WindowId, AppWindow>)) {
        let mut windows = mem::take(&mut Windows::req(ctx.services).windows);
        f(ctx, &mut windows);
        let mut wns = Windows::req(ctx.services);
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
    subscriptions: WidgetSubscriptions,
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
            widget_tree: WidgetInfoTree::blank(id, root_id),
            subscriptions: WidgetSubscriptions::new(),
            is_focused: false,
            loading_handle,
            is_loaded: false,
        }
    }
}
struct OpenWindowRequest {
    id: WindowId,
    new: Box<dyn FnOnce(&mut WindowContext) -> Window>,
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
    ctrl: WindowCtrl,

    id: WindowId,
    pub(super) mode: WindowMode,
    state: OwnedStateMap<state_map::Window>,
}
impl AppWindow {
    pub fn new(
        ctx: &mut AppContext,
        id: WindowId,
        mode: WindowMode,
        new: Box<dyn FnOnce(&mut WindowContext) -> Window>,
        loading: WindowLoading,
    ) -> (Self, AppWindowInfo) {
        let primary_scale_factor = Monitors::req(ctx.services)
            .primary_monitor(ctx.vars)
            .map(|m| m.scale_factor().copy(ctx.vars))
            .unwrap_or_else(|| 1.fct());

        let vars = WindowVars::new(Windows::req(ctx.services).default_render_mode, primary_scale_factor);
        let mut state = OwnedStateMap::new();
        state.borrow_mut().set(WindowVarsKey, vars.clone());
        let (window, _) = ctx.window_context(id, mode, &mut state, new);

        if window.kiosk {
            vars.chrome().set_ne(ctx, WindowChrome::None);
            vars.visible().set_ne(ctx, true);
            if !vars.state().get(ctx).is_fullscreen() {
                vars.state().set(ctx, WindowState::Exclusive);
            }
        }

        if mode.is_headless() {
            vars.0.scale_factor.set_ne(ctx, window.headless_monitor.scale_factor);
        }

        let commands = WindowCommands::new(id, ctx.events);

        let root_id = window.id;
        let ctrl = WindowCtrl::new(id, &vars, commands, mode, window);

        let window = Self { ctrl, id, mode, state };
        let info = AppWindowInfo::new(id, root_id, mode, vars, loading);

        (window, info)
    }

    fn ctrl_in_ctx(&mut self, ctx: &mut AppContext, action: impl FnOnce(&mut WindowContext, &mut WindowCtrl)) {
        let (_, updates) = ctx.window_context(self.id, self.mode, &mut self.state, |ctx| action(ctx, &mut self.ctrl));
        if updates.is_any() {
            let (_, updates) = ctx.window_context(self.id, self.mode, &mut self.state, |ctx| self.ctrl.window_updates(ctx, updates));
            debug_assert!(updates.is_none());
        }
    }

    pub fn pre_event<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        self.ctrl_in_ctx(ctx, |ctx, ctrl| ctrl.pre_event(ctx, args))
    }

    pub fn ui_event<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        self.ctrl_in_ctx(ctx, |ctx, ctrl| ctrl.ui_event(ctx, args))
    }

    pub fn update(&mut self, ctx: &mut AppContext) {
        self.ctrl_in_ctx(ctx, |ctx, ctrl| ctrl.update(ctx));
    }

    pub fn layout(&mut self, ctx: &mut AppContext) {
        self.ctrl_in_ctx(ctx, |ctx, ctrl| ctrl.layout(ctx));
    }

    pub fn render(&mut self, ctx: &mut AppContext) {
        self.ctrl_in_ctx(ctx, |ctx, ctrl| ctrl.render(ctx));
    }

    pub fn focus(&mut self, ctx: &mut AppContext) {
        self.ctrl_in_ctx(ctx, |ctx, ctrl| ctrl.focus(ctx));
    }

    pub fn close(mut self, ctx: &mut AppContext) {
        let _ = ctx.window_context(self.id, self.mode, &mut self.state, |ctx| self.ctrl.close(ctx));
    }
}

struct WindowLoadingData {
    update: AppEventSender,
}
impl Drop for WindowLoadingData {
    fn drop(&mut self) {
        let _ = self.update.send_ext_update();
    }
}

struct WindowLoading {
    handle: std::sync::Weak<WindowLoadingData>,
    deadline: Deadline,
}
impl WindowLoading {
    pub fn new() -> Self {
        WindowLoading {
            handle: std::sync::Weak::new(),
            deadline: Deadline::timeout(24.hours()),
        }
    }

    pub fn is_loading(&self) -> bool {
        self.handle.strong_count() > 0 && !self.deadline.has_elapsed()
    }

    pub fn handle(&mut self, update: &AppEventSender, deadline: Deadline) -> WindowLoadingHandle {
        match self.handle.upgrade() {
            Some(h) => {
                self.deadline = self.deadline.min(deadline);
                WindowLoadingHandle(h)
            },
            None => {
                let h = Arc::new(WindowLoadingData { update: update.clone() });
                self.handle = Arc::downgrade(&h);
                self.deadline = deadline;
                WindowLoadingHandle(h)
            }
        }
    }
}

/// Represents a handle that stops a window from opening while it exists.
///
/// A handle can be retrieved using [`Windows::loading_handle`], the window does not
/// open until all handles are dropped.
#[derive(Clone)]
pub struct WindowLoadingHandle(Arc<WindowLoadingData>);
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
