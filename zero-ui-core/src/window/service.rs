use std::mem;

use linear_map::LinearMap;

use super::*;
use crate::app::{
    view_process::{self, ViewRenderer},
    AppEventSender,
};
use crate::event::EventUpdateArgs;
use crate::image::{Image, ImageVar};
use crate::render::FrameHitInfo;
use crate::service::Service;
use crate::state::OwnedStateMap;
use crate::var::*;
use crate::widget_info::WidgetInfoTree;
use crate::{units::*, WidgetId};

/// Windows service.
///
/// # Provider
///
/// This service is provided by the [`WindowManager`].
#[derive(Service)]
pub struct Windows {
    /// If shutdown is requested when a window closes and there are no more windows open, `true` by default.
    ///
    /// This setting is ignored in headless apps, in headed apps the shutdown happens when all headed windows
    /// are closed, headless windows are ignored.
    pub shutdown_on_last_close: bool,

    /// Default render mode of windows opened by this service, the initial value is [`RenderMode::default`].
    ///
    /// Note that this setting only affects windows opened after it is changed, also the view-process may select
    /// a different render mode if it cannot support the requested mode.
    pub default_render_mode: RenderMode,

    windows: LinearMap<WindowId, AppWindow>,
    windows_info: LinearMap<WindowId, AppWindowInfo>,

    open_requests: Vec<OpenWindowRequest>,
    update_sender: AppEventSender,

    close_group_id: CloseGroupId,
    close_requests: LinearMap<WindowId, CloseWindowRequest>,

    frame_images: Vec<RcVar<Image>>,
}
impl Windows {
    pub(super) fn new(update_sender: AppEventSender) -> Self {
        Windows {
            shutdown_on_last_close: true,
            default_render_mode: RenderMode::default(),
            windows: LinearMap::with_capacity(1),
            windows_info: LinearMap::with_capacity(1),
            open_requests: Vec::with_capacity(1),
            update_sender,

            close_group_id: 1,
            close_requests: LinearMap::new(),

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
        self.open_impl(new_window, None)
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
            new_window,
            Some(if with_renderer {
                WindowMode::HeadlessWithRenderer
            } else {
                WindowMode::Headless
            }),
        )
    }

    fn open_impl(
        &mut self,
        new_window: impl FnOnce(&mut WindowContext) -> Window + 'static,
        force_headless: Option<WindowMode>,
    ) -> ResponseVar<WindowOpenArgs> {
        let (responder, response) = response_var();
        let request = OpenWindowRequest {
            new: Box::new(new_window),
            force_headless,
            responder,
        };
        self.open_requests.push(request);
        let _ = self.update_sender.send_ext_update();

        response
    }

    /// Starts closing a window, the operation can be canceled by listeners of
    /// [`WindowCloseRequestedEvent`].
    ///
    /// Returns a response var that will update once with the result of the operation.
    pub fn close(&mut self, window_id: WindowId) -> Result<ResponseVar<CloseWindowResult>, WindowNotFound> {
        if self.windows_info.contains_key(&window_id) {
            let (responder, response) = response_var();

            let group = self.close_group_id.wrapping_add(1);
            self.close_group_id = group;

            self.close_requests.insert(window_id, CloseWindowRequest { responder, group });
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
    /// [`Cancel`] if `windows` is empty or contains a window that already requested close
    /// during this update.
    ///
    /// [`Cancel`]: CloseWindowResult::Cancel
    pub fn close_together(
        &mut self,
        windows: impl IntoIterator<Item = WindowId>,
    ) -> Result<ResponseVar<CloseWindowResult>, WindowNotFound> {
        let windows = windows.into_iter();
        let mut requests = LinearMap::with_capacity(windows.size_hint().0);

        let group = self.close_group_id.wrapping_add(1);
        self.close_group_id = group;

        let (responder, response) = response_var();

        for window in windows {
            if !self.windows_info.contains_key(&window) {
                return Err(WindowNotFound(window));
            }

            requests.insert(
                window,
                CloseWindowRequest {
                    responder: responder.clone(),
                    group,
                },
            );
        }

        self.close_requests.extend(requests);
        let _ = self.update_sender.send_ext_update();

        Ok(response)
    }

    /// Requests close of all open windows together, the operation can be canceled by listeners of
    /// the [`WindowCloseRequestedEvent`]. If canceled none of the windows are closed.
    ///
    /// Returns a response var that will update once with the result of the operation, Returns
    /// [`Cancel`] if no window is open or if close was already requested to one of the windows.
    ///
    /// [`Cancel`]: CloseWindowResult::Cancel
    pub fn close_all(&mut self) -> ResponseVar<CloseWindowResult> {
        let set: Vec<_> = self.windows.keys().copied().collect();
        self.close_together(set).unwrap()
    }

    /// Get the window [mode].
    ///
    /// This value indicates if the window is headless or not.
    ///
    /// [mode]: WindowMode
    pub fn mode(&self, window_id: WindowId) -> Result<WindowMode, WindowNotFound> {
        self.windows_info.get(&window_id).map(|w| w.mode).ok_or(WindowNotFound(window_id))
    }

    /// Reference the metadata about the window's widgets.
    pub fn widget_tree(&self, window_id: WindowId) -> Result<&WidgetInfoTree, WindowNotFound> {
        self.windows_info
            .get(&window_id)
            .map(|w| &w.widget_tree)
            .ok_or(WindowNotFound(window_id))
    }

    /// Generate an image from the current rendered frame of the window.
    ///
    /// The image is not loaded at the moment of return, it will update when it is loaded.
    ///
    /// If the window is not found the error is reported in the image error.
    pub fn frame_image(&mut self, window_id: WindowId) -> ImageVar {
        self.frame_image_impl(window_id, |vr| vr.frame_image())
    }

    /// Generate an image from a selection of the current rendered frame of the window.
    ///
    /// The image is not loaded at the moment of return, it will update when it is loaded.
    ///
    /// If the window is not found the error is reported in the image error.
    pub fn frame_image_rect(&mut self, window_id: WindowId, rect: PxRect) -> ImageVar {
        self.frame_image_impl(window_id, |vr| vr.frame_image_rect(rect))
    }

    fn frame_image_impl(
        &mut self,
        window_id: WindowId,
        action: impl FnOnce(&ViewRenderer) -> std::result::Result<view_process::ViewImage, view_process::Respawned>,
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
                var(Image::dummy(Some(format!("window `{}` is headless without renderer", window_id)))).into_read_only()
            }
        } else {
            var(Image::dummy(Some(format!("{}", WindowNotFound(window_id))))).into_read_only()
        }
    }

    /// Reference the [`WindowVars`] for the window.
    pub fn vars(&self, window_id: WindowId) -> Result<&WindowVars, WindowNotFound> {
        self.windows_info.get(&window_id).map(|w| &w.vars).ok_or(WindowNotFound(window_id))
    }

    /// Hit-test the latest window frame.
    pub fn hit_test(&self, window_id: WindowId, point: DipPoint) -> Result<FrameHitInfo, WindowNotFound> {
        self.windows_info
            .get(&window_id)
            .map(|w| w.hit_test(point))
            .ok_or(WindowNotFound(window_id))
    }

    /// Gets if the window is focused in the OS.
    pub fn is_focused(&self, window_id: WindowId) -> Result<bool, WindowNotFound> {
        self.windows_info
            .get(&window_id)
            .map(|w| w.is_focused)
            .ok_or(WindowNotFound(window_id))
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

    /// Returns `true` if the window is found.
    pub fn is_open(&self, window_id: WindowId) -> bool {
        self.windows_info.contains_key(&window_id)
    }

    fn take_requests(&mut self) -> (Vec<OpenWindowRequest>, LinearMap<WindowId, CloseWindowRequest>) {
        (mem::take(&mut self.open_requests), mem::take(&mut self.close_requests))
    }

    /// Update the reference to the renderer associated with the window, we need
    /// the render to enable the hit-test function.
    pub(super) fn set_renderer(&mut self, id: WindowId, renderer: ViewRenderer) {
        if let Some(info) = self.windows_info.get_mut(&id) {
            info.renderer = Some(renderer);
        }
    }

    pub(super) fn on_pre_event<EV: EventUpdateArgs>(ctx: &mut AppContext, args: &EV) {
        todo!()
    }

    pub(super) fn on_ui_event<EV: EventUpdateArgs>(ctx: &mut AppContext, args: &EV) {
        Self::with_detached_windows(ctx, |ctx, windows| {
            for (_, window) in windows {
                window.event(ctx, args);
            }
        });
    }

    pub(super) fn on_event<EV: EventUpdateArgs>(ctx: &mut AppContext, args: &EV) {
        todo!()
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
        let (open, close) = {
            let wns = ctx.services.windows();
            wns.take_requests()
        };

        let window_mode = ctx.window_mode();

        // fulfill open requests.
        for r in open {
            let window_mode = match (window_mode, r.force_headless) {
                (WindowMode::Headed | WindowMode::HeadlessWithRenderer, Some(mode)) => {
                    debug_assert!(!matches!(mode, WindowMode::Headed));
                    mode
                }
                (mode, _) => mode,
            };

            let (window, info) = AppWindow::new(ctx, window_mode, r.new);

            let args = WindowOpenArgs::now(window.id);
            {
                let wns = ctx.services.windows();
                wns.windows.insert(window.id, window);
                wns.windows_info.insert(info.id, info);
            }

            r.responder.respond(ctx, args.clone());
            WindowOpenEvent.notify(ctx, args);
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
        let mut windows = mem::take(&mut ctx.services.windows().windows);
        f(ctx, &mut windows);
        let mut wns = ctx.services.windows();
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
}
impl AppWindowInfo {
    pub fn new(id: WindowId, root_id: WidgetId, mode: WindowMode, vars: WindowVars) -> Self {
        Self {
            id,
            mode,
            renderer: None,
            vars,
            widget_tree: WidgetInfoTree::blank(id, root_id),
            is_focused: false,
        }
    }

    fn hit_test(&self, point: DipPoint) -> FrameHitInfo {
        let _scope = tracing::trace_span!("hit_test", window = %self.id.sequential(), ?point).entered();

        if let Some(r) = &self.renderer {
            match r.hit_test(point) {
                Ok((frame_id, px_pt, hit_test)) => {
                    return FrameHitInfo::new(self.id, frame_id, px_pt, &hit_test);
                }
                Err(Respawned) => tracing::debug!("respawned calling `hit_test`, will return `no_hits`"),
            }
        }

        FrameHitInfo::no_hits(self.id)
    }
}
struct OpenWindowRequest {
    new: Box<dyn FnOnce(&mut WindowContext) -> Window>,
    force_headless: Option<WindowMode>,
    responder: ResponderVar<WindowOpenArgs>,
}

struct CloseWindowRequest {
    responder: ResponderVar<CloseWindowResult>,
    group: CloseGroupId,
}

/// Window context owner.
struct AppWindow {
    ctrl: WindowCtrl,

    id: WindowId,
    mode: WindowMode,
    state: OwnedStateMap,
}
impl AppWindow {
    pub fn new(ctx: &mut AppContext, mode: WindowMode, new: Box<dyn FnOnce(&mut WindowContext) -> Window>) -> (Self, AppWindowInfo) {
        let id = WindowId::new_unique();
        let mut state = OwnedStateMap::new();
        let (window, _) = ctx.window_context(id, mode, &mut state, new);
        let root_id = window.id;
        let vars = WindowVars::new(ctx.services.windows().default_render_mode);
        let (ctrl, _) = ctx.window_context(id, mode, &mut state, move |ctx| WindowCtrl::new(ctx, &vars, mode, window));

        let window = Self { ctrl, id, mode, state };
        let info = AppWindowInfo::new(id, root_id, mode, vars);

        (window, info)
    }

    fn ctrl_in_ctx(&mut self, ctx: &mut AppContext, action: impl FnOnce(&mut WindowContext, &mut WindowCtrl)) {
        let (_, updates) = ctx.window_context(self.id, self.mode, &mut self.state, |ctx| action(ctx, &mut self.ctrl));
        if updates.is_any() {
            let (_, updates) = ctx.window_context(self.id, self.mode, &mut self.state, |ctx| self.ctrl.window_updates(ctx, updates));
            debug_assert!(!updates.is_none());
        }
    }

    pub fn event<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        self.ctrl_in_ctx(ctx, |ctx, ctrl| ctrl.event(ctx, args))
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
}
