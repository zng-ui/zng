//! App window and monitors manager.

use std::{mem, thread, time::Instant};

use linear_map::LinearMap;
use zero_ui_view_api::{webrender_api::HitTestResult, FrameUpdateRequest, FrameWaitId, IpcBytes};

pub use crate::app::view_process::{CursorIcon, EventCause, MonitorInfo, RenderMode, VideoMode, WindowState, WindowTheme};

use crate::{
    app::{
        self,
        raw_events::*,
        view_process::{self, Respawned, ViewHeadless, ViewProcess, ViewProcessGen, ViewProcessRespawnedEvent, ViewRenderer, ViewWindow},
        AppEventSender, AppExtended, AppExtension, AppProcessExt, ControlFlow,
    },
    color::RenderColor,
    context::{AppContext, WidgetContext, WindowContext, WindowRenderUpdate, WindowUpdates},
    event::{event, EventUpdateArgs},
    image::{Image, ImageVar, ImagesExt},
    render::{
        BuiltFrame, BuiltFrameUpdate, FrameBuilder, FrameHitInfo, FrameId, FrameUpdate, UsedFrameBuilder, UsedFrameUpdate,
        WidgetTransformKey,
    },
    service::Service,
    state::OwnedStateMap,
    units::*,
    var::Vars,
    var::{response_var, var, RcVar, ResponderVar, ResponseVar, Var},
    widget_info::{
        BoundsRect, UsedWidgetInfoBuilder, WidgetInfoBuilder, WidgetInfoTree, WidgetOffset, WidgetRendered, WidgetSubscriptions,
    },
    BoxedUiNode, UiNode, WidgetId,
};

mod types;
pub use types::*;

mod monitor;
pub use monitor::*;

mod vars;
pub use vars::*;

/// Extension trait, adds [`run_window`](AppRunWindowExt::run_window) to [`AppExtended`].
pub trait AppRunWindowExt {
    /// Runs the application event loop and requests a new window.
    ///
    /// The `new_window` argument is the [`WindowContext`] of the new window.
    ///
    /// This method only returns when the app has shutdown.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use zero_ui_core::app::App;
    /// # use zero_ui_core::window::AppRunWindowExt;
    /// # macro_rules! window { ($($tt:tt)*) => { todo!() } }
    /// App::default().run_window(|ctx| {
    ///     println!("starting app with window {:?}", ctx.window_id);
    ///     window! {
    ///         title = "Window 1";
    ///         content = text("Window 1");
    ///     }
    /// })   
    /// ```
    ///
    /// Which is a shortcut for:
    /// ```no_run
    /// # use zero_ui_core::app::App;
    /// # use zero_ui_core::window::WindowsExt;
    /// # macro_rules! window { ($($tt:tt)*) => { todo!() } }
    /// App::default().run(|ctx| {
    ///     ctx.services.windows().open(|ctx| {
    ///         println!("starting app with window {:?}", ctx.window_id);
    ///         window! {
    ///             title = "Window 1";
    ///             content = text("Window 1");
    ///         }
    ///     });
    /// })   
    /// ```
    fn run_window(self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static);
}
impl<E: AppExtension> AppRunWindowExt for AppExtended<E> {
    fn run_window(self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) {
        self.run(|ctx| {
            ctx.services.windows().open(new_window);
        })
    }
}

/// Extension trait, adds [`open_window`](HeadlessAppWindowExt::open_window) to [`HeadlessApp`](app::HeadlessApp).
pub trait HeadlessAppWindowExt {
    /// Open a new headless window and returns the new window ID.
    ///
    /// The `new_window` argument is the [`WindowContext`] of the new window.
    ///
    /// Returns the [`WindowId`] of the new window.
    fn open_window(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) -> WindowId;

    /// Cause the headless window to think it is focused in the screen.
    fn focus_window(&mut self, window_id: WindowId);
    /// Cause the headless window to think focus moved away from it.
    fn blur_window(&mut self, window_id: WindowId);

    /// Copy the current frame pixels of the window.
    ///
    /// The var will update until it is loaded or error.
    fn window_frame_image(&mut self, window_id: WindowId) -> ImageVar;

    /// Sends a close request, returns if the window was found and closed.
    fn close_window(&mut self, window_id: WindowId) -> bool;

    /// Open a new headless window and update the app until the window closes.
    fn run_window(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static);
}
impl HeadlessAppWindowExt for app::HeadlessApp {
    fn open_window(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) -> WindowId {
        let response = self.ctx().services.windows().open(new_window);
        let mut window_id = None;
        let cf = self.update_observe(
            |ctx| {
                if let Some(opened) = response.rsp_new(ctx) {
                    window_id = Some(opened.window_id);
                }
            },
            true,
        );

        window_id.unwrap_or_else(|| panic!("window did not open, ControlFlow: {:?}", cf))
    }

    fn focus_window(&mut self, window_id: WindowId) {
        let args = RawWindowFocusArgs::now(window_id, true);
        RawWindowFocusEvent.notify(self.ctx().events, args);
        let _ = self.update(false);
    }

    fn blur_window(&mut self, window_id: WindowId) {
        let args = RawWindowFocusArgs::now(window_id, false);
        RawWindowFocusEvent.notify(self.ctx().events, args);
        let _ = self.update(false);
    }

    fn window_frame_image(&mut self, window_id: WindowId) -> ImageVar {
        self.ctx().services.windows().frame_image(window_id)
    }

    fn close_window(&mut self, window_id: WindowId) -> bool {
        use app::raw_events::*;

        let args = RawWindowCloseRequestedArgs::now(window_id);
        RawWindowCloseRequestedEvent.notify(self.ctx().events, args);

        let mut requested = false;
        let mut closed = false;

        let _ = self.update_observe_event(
            |_, args| {
                if let Some(args) = WindowCloseRequestedEvent.update(args) {
                    requested |= args.window_id == window_id;
                } else if let Some(args) = WindowCloseEvent.update(args) {
                    closed |= args.window_id == window_id;
                }
            },
            false,
        );

        assert_eq!(requested, closed);

        closed
    }

    fn run_window(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) {
        let window_id = self.open_window(new_window);
        while self.ctx().services.windows().windows.contains_key(&window_id) {
            if let ControlFlow::Exit = self.update(true) {
                return;
            }
        }
    }
}

/// Application extension that manages windows.
///
/// # Events
///
/// Events this extension provides:
///
/// * [WindowOpenEvent]
/// * [WindowChangedEvent]
/// * [WindowFocusChangedEvent]
/// * [WindowScaleChangedEvent]
/// * [WindowCloseRequestedEvent]
/// * [WindowCloseEvent]
/// * [MonitorsChangedEvent]
/// * [WidgetInfoChangedEvent]
///
/// # Services
///
/// Services this extension provides:
///
/// * [Windows]
/// * [Monitors]
pub struct WindowManager {
    pending_closes: LinearMap<CloseGroupId, PendingClose>,
}
struct PendingClose {
    windows: LinearMap<WindowId, Option<bool>>,
    responder: ResponderVar<CloseWindowResult>,
}
impl Default for WindowManager {
    fn default() -> Self {
        Self {
            pending_closes: LinearMap::new(),
        }
    }
}
impl AppExtension for WindowManager {
    fn init(&mut self, ctx: &mut AppContext) {
        let monitors = Monitors::new(ctx.services.get::<ViewProcess>());
        ctx.services.register(monitors);
        ctx.services.register(Windows::new(ctx.updates.sender()));
    }

    fn event_preview<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        if let Some(args) = RawFrameRenderedEvent.update(args) {
            let wns = ctx.services.windows();
            if let Some(window) = wns.windows.get_mut(&args.window_id) {
                if let Some(pending) = window.pending_render.take() {
                    match pending {
                        WindowRenderUpdate::None => {}
                        WindowRenderUpdate::Render => {
                            window.context.update.render = WindowRenderUpdate::Render;
                            ctx.updates.render();
                        }
                        WindowRenderUpdate::RenderUpdate => {
                            window.context.update.render |= WindowRenderUpdate::RenderUpdate;
                            ctx.updates.render_update();
                        }
                    }
                }

                let image = args.frame_image.as_ref().cloned().map(Image::new);
                let args = FrameImageReadyArgs::new(args.timestamp, args.window_id, args.frame_id, image);
                FrameImageReadyEvent.notify(ctx.events, args);
            }
        } else if let Some(args) = RawWindowChangedEvent.update(args) {
            let windows = ctx.services.windows();
            if let Some(mut window) = windows.windows.get_mut(&args.window_id) {
                let mut state_change = None;
                let mut pos_change = None;
                let mut size_change = None;

                // STATE CHANGED
                if let Some(new_state) = args.state {
                    if window.notified_state != new_state {
                        let prev_state = mem::replace(&mut window.notified_state, new_state);

                        if let EventCause::System = args.cause {
                            window.vars.state().set(ctx.vars, new_state);
                        }

                        state_change = Some((prev_state, new_state));

                        if let WindowState::Minimized = prev_state {
                            // we skip layout&render when minimized, but leave the flags set.

                            if window.context.update.layout {
                                ctx.updates.layout();
                            }
                            match window.context.update.render {
                                WindowRenderUpdate::None => {}
                                WindowRenderUpdate::Render => ctx.updates.render(),
                                WindowRenderUpdate::RenderUpdate => ctx.updates.render_update(),
                            }
                        }

                        let restore_state = if let WindowState::Minimized = new_state {
                            prev_state
                        } else {
                            WindowState::Normal
                        };
                        window.vars.0.restore_state.set_ne(ctx.vars, restore_state);
                    }
                }

                let window_state = args.state.unwrap_or_else(|| window.vars.state().copy(ctx.vars));

                let mut restore_rect = window.vars.0.restore_rect.copy(ctx.vars);

                // MOVED
                if let Some(new_pos) = args.position {
                    if window.vars.0.actual_position.set_ne(ctx.vars, new_pos) {
                        window.position = Some(new_pos);

                        pos_change = Some(new_pos);

                        if let WindowState::Normal = window_state {
                            restore_rect.origin = new_pos;
                        }
                    }
                }

                // MONITOR CHANGED
                if let Some((new_monitor, scale_factor)) = args.monitor {
                    if let Some(info) = windows.windows_info.get_mut(&args.window_id) {
                        if info.scale_factor != scale_factor {
                            info.scale_factor = scale_factor;

                            let args = WindowScaleChangedArgs::new(args.timestamp, args.window_id, scale_factor);
                            WindowScaleChangedEvent.notify(ctx.events, args);

                            window.context.update.layout = true;
                            window.context.update.render = WindowRenderUpdate::Render;
                            ctx.updates.layout_and_render();
                        }

                        window.vars.0.actual_monitor.set_ne(ctx.vars, Some(new_monitor));
                        window.monitor_info = None;
                    }
                }

                // RESIZED
                if let Some(new_size) = args.size {
                    if window.vars.0.actual_size.set_ne(ctx.vars, new_size) {
                        if args.cause == EventCause::System {
                            window.vars.0.auto_size.set_ne(ctx.vars, AutoSize::DISABLED);
                        }
                        window.size = new_size;

                        size_change = Some(new_size);

                        if let WindowState::Normal = window_state {
                            restore_rect.size = new_size;
                        }
                    }
                }

                window.vars.0.restore_rect.set_ne(ctx.vars, restore_rect);

                if args.frame_wait_id.is_some() {
                    // the view process is waiting a new frame or update, this will send one.
                    window.context.update.layout = true;
                    window.context.update.render = WindowRenderUpdate::Render;
                    window.pending_render = None;
                    window.resized_frame_wait_id = args.frame_wait_id;
                    ctx.updates.layout_and_render();
                }

                if state_change.is_some() || pos_change.is_some() || size_change.is_some() {
                    let args = WindowChangedArgs::new(args.timestamp, args.window_id, state_change, pos_change, size_change, args.cause);
                    WindowChangedEvent.notify(ctx.events, args);
                }
            }
        } else if let Some(args) = RawWindowFocusEvent.update(args) {
            let wns = ctx.services.windows();
            if let Some(window) = wns.windows_info.get_mut(&args.window_id) {
                if window.is_focused == args.focused {
                    return;
                }

                window.is_focused = args.focused;

                let args = WindowFocusArgs::new(args.timestamp, args.window_id, window.is_focused, false);
                WindowFocusChangedEvent.notify(ctx.events, args);
            }
        } else if let Some(args) = RawWindowCloseRequestedEvent.update(args) {
            let _ = ctx.services.windows().close(args.window_id);
        } else if let Some(args) = RawScaleFactorChangedEvent.update(args) {
            // Update Monitors:
            if let Some(m) = ctx.services.monitors().monitor_mut(args.monitor_id) {
                m.info.scale_factor = args.scale_factor.0;
            }

            // Update Windows:
            let windows = ctx.services.windows();
            for &window_id in &args.windows {
                if let Some(info) = windows.windows_info.get_mut(&window_id) {
                    if info.scale_factor != args.scale_factor {
                        info.scale_factor = args.scale_factor;

                        let args = WindowScaleChangedArgs::new(args.timestamp, window_id, args.scale_factor);
                        WindowScaleChangedEvent.notify(ctx.events, args);

                        let window = windows.windows.get_mut(&window_id).unwrap();
                        window.context.update.layout = true;
                        window.context.update.render = WindowRenderUpdate::Render;
                        ctx.updates.layout_and_render();
                        windows.windows.get_mut(&window_id).unwrap().monitor_info = None;
                    }
                }
            }
        } else if let Some(args) = RawWindowCloseEvent.update(args) {
            if ctx.services.windows().windows.contains_key(&args.window_id) {
                tracing::error!("view-process closed window without request");
                let args = WindowCloseArgs::new(args.timestamp, args.window_id);
                WindowCloseEvent.notify(ctx, args);
            }
        } else if let Some(args) = RawMonitorsChangedEvent.update(args) {
            ctx.services.monitors().on_monitors_changed(ctx.events, args);
        }
    }

    fn event_ui<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        with_detached_windows(ctx, |ctx, windows| {
            for (_, w) in windows.iter_mut() {
                w.on_event(ctx, args);
            }
        })
    }

    fn event<EV: event::EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        if let Some(args) = WindowCloseRequestedEvent.update(args) {
            // If we caused this event, fulfill the close request.
            match self.pending_closes.entry(args.close_group) {
                linear_map::Entry::Occupied(mut e) => {
                    let caused_by_us = if let Some(canceled) = e.get_mut().windows.get_mut(&args.window_id) {
                        // caused by us, update the status for the window.
                        *canceled = Some(args.cancel_requested());
                        true
                    } else {
                        // not us, window not in group
                        false
                    };

                    if caused_by_us {
                        // check if this is the last window in the group
                        let mut all_some = true;
                        // and if any cancelled we cancel all, otherwise close all.
                        let mut cancel = false;

                        for cancel_flag in e.get().windows.values() {
                            if let Some(c) = cancel_flag {
                                cancel |= c;
                            } else {
                                all_some = false;
                                break;
                            }
                        }

                        if all_some {
                            // if the last window in the group, no longer pending
                            let e = e.remove();

                            if cancel {
                                // respond to all windows in the group.
                                e.responder.respond(ctx, CloseWindowResult::Cancel);
                            } else {
                                e.responder.respond(ctx, CloseWindowResult::Closed);

                                // notify close, but does not remove then yet, this
                                // lets the window content handle the close event,
                                // we deinit the window when we handle our own close event.
                                let windows = ctx.services.windows();
                                for (w, _) in e.windows {
                                    if windows.windows.contains_key(&w) {
                                        WindowCloseEvent.notify(ctx.events, WindowCloseArgs::now(w));
                                    }
                                }
                            }
                        }
                    }
                }
                linear_map::Entry::Vacant(_) => {
                    // Not us, no pending entry.
                }
            }
        } else if let Some(args) = WindowCloseEvent.update(args) {
            // finish close, this notifies  `UiNode::deinit` and drops the window
            // causing the ViewWindow to drop and close.

            if let Some(w) = ctx.services.windows().windows.remove(&args.window_id) {
                w.deinit(ctx);

                let is_headless_app = ctx.services.get::<ViewProcess>().map(|vp| vp.headless()).unwrap_or(true);

                let wns = ctx.services.windows();
                let info = wns.windows_info.remove(&args.window_id).unwrap();

                info.vars.0.is_open.set(ctx.vars, false);

                // if set to shutdown on last headed window close in a headed app,
                // AND there is no more open headed window OR request for opening a headed window.
                if wns.shutdown_on_last_close
                    && !is_headless_app
                    && !wns.windows.values().any(|w| matches!(w.window_mode, WindowMode::Headed))
                    && !wns
                        .open_requests
                        .iter()
                        .any(|w| matches!(w.force_headless, None | Some(WindowMode::Headed)))
                {
                    // fulfill `shutdown_on_last_close`
                    ctx.services.app_process().shutdown();
                }

                if info.is_focused {
                    let args = WindowFocusArgs::now(info.id, false, true);
                    WindowFocusChangedEvent.notify(ctx.events, args)
                }
            }
        } else if let Some(args) = ViewProcessRespawnedEvent.update(args) {
            // `respawn` will force a `render` only and the `RenderContext` does not
            // give access to `services` so this is fine.
            let mut windows = mem::take(&mut ctx.services.windows().windows);

            for (_, w) in windows.iter_mut() {
                w.respawn(ctx, args.generation);
            }

            ctx.services.windows().windows = windows;
        }
    }

    fn update_ui(&mut self, ctx: &mut AppContext) {
        let (wm, (open, close)) = {
            let wns = ctx.services.windows();
            (wns.default_render_mode, wns.take_requests())
        };

        // fulfill open requests.
        for r in open {
            let (w, info) = AppWindow::new(ctx, r.new, r.force_headless, wm);
            let args = WindowOpenArgs::now(w.id);
            {
                let wns = ctx.services.windows();
                wns.windows.insert(w.id, w);
                wns.windows_info.insert(info.id, info);
            }

            r.responder.respond(ctx, args.clone());
            WindowOpenEvent.notify(ctx, args);
        }

        // notify close requests, the request is fulfilled or canceled
        // in the `event` handler.
        for (w_id, r) in close {
            let args = WindowCloseRequestedArgs::now(w_id, r.group);
            WindowCloseRequestedEvent.notify(ctx.events, args);

            self.pending_closes
                .entry(r.group)
                .or_insert_with(|| PendingClose {
                    responder: r.responder,
                    windows: LinearMap::with_capacity(1),
                })
                .windows
                .insert(w_id, None);
        }

        // notify content
        with_detached_windows(ctx, |ctx, windows| {
            for (_, w) in windows.iter_mut() {
                w.on_update(ctx);
            }
        });
    }

    fn layout(&mut self, ctx: &mut AppContext) {
        with_detached_windows(ctx, |ctx, windows| {
            for (_, w) in windows.iter_mut() {
                w.on_layout(ctx);
            }
        });
    }

    fn render(&mut self, ctx: &mut AppContext) {
        with_detached_windows(ctx, |ctx, windows| {
            for (_, w) in windows.iter_mut() {
                w.on_render(ctx);
                w.on_render_update(ctx);
            }
        });
    }
}

/// Takes ownership of [`Windows::windows`] for the duration of the call to `f`.
///
/// The windows map is empty for the duration of `f` and should not be used, this is for
/// mutating the window content while still allowing it to query the [`Windows::windows_info`].
fn with_detached_windows(ctx: &mut AppContext, f: impl FnOnce(&mut AppContext, &mut LinearMap<WindowId, AppWindow>)) {
    let mut windows = mem::take(&mut ctx.services.windows().windows);
    f(ctx, &mut windows);
    let mut wns = ctx.services.windows();
    debug_assert!(wns.windows.is_empty());
    wns.windows = windows;
}

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
    fn new(update_sender: AppEventSender) -> Self {
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
    /// Returns a listener that will update once when the window is opened, note that while the `window_id` is
    /// available in the `new_window` argument already, the window is only available in this service after
    /// the returned listener updates.
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

    /// Gets the current window scale factor.
    pub fn scale_factor(&self, window_id: WindowId) -> Result<Factor, WindowNotFound> {
        self.windows_info
            .get(&window_id)
            .map(|w| w.scale_factor)
            .ok_or(WindowNotFound(window_id))
    }

    /// Gets the id of the window that is focused in the OS.
    pub fn focused_window_id(&self) -> Option<WindowId> {
        self.windows_info.values().find(|w| w.is_focused).map(|w| w.id)
    }

    /// Gets the latest frame for the focused window.
    pub fn focused_info(&self) -> Option<&WidgetInfoTree> {
        self.windows_info.values().find(|w| w.is_focused).map(|w| &w.widget_tree)
    }

    fn take_requests(&mut self) -> (Vec<OpenWindowRequest>, LinearMap<WindowId, CloseWindowRequest>) {
        (mem::take(&mut self.open_requests), mem::take(&mut self.close_requests))
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

/// [`AppWindow`] data, detached so we can make the window visible in [`Windows`]
/// from inside the window content.
struct AppWindowInfo {
    id: WindowId,
    mode: WindowMode,
    renderer: Option<ViewRenderer>,
    vars: WindowVars,
    scale_factor: Factor,

    widget_tree: WidgetInfoTree,
    // focus tracked by the raw focus events.
    is_focused: bool,
}
impl AppWindowInfo {
    fn hit_test(&self, point: DipPoint) -> FrameHitInfo {
        let _scope = tracing::trace_span!("hit_test", window = %self.id.sequential(), ?point).entered();

        if let Some(r) = &self.renderer {
            let px_pt = point.to_px(self.scale_factor.0);
            match r.hit_test(point) {
                Ok((frame_id, hit_test)) => {
                    return FrameHitInfo::new(self.id, frame_id, px_pt, &hit_test);
                }
                Err(Respawned) => tracing::debug!("respawned calling `hit_test`, will return `no_hits`"),
            }
        }

        FrameHitInfo::no_hits(self.id)
    }
}

/// An open window.
struct AppWindow {
    // Is `Some` if the window is headed and the first frame was generated.
    headed: Option<ViewWindow>,

    // Is `Some` if the window is headless, a fake screen for size calculations.
    headless_monitor: Option<HeadlessMonitor>,
    // Is `Some` if the window is headless with renderer and the first frame was generated.
    headless_surface: Option<ViewHeadless>,

    // Is `Some` if the window is headed or headless with renderer.
    renderer: Option<ViewRenderer>,

    // Window context.
    context: OwnedWindowContext,

    // copy of some `context` values.
    window_mode: WindowMode,
    id: WindowId,
    root_id: WidgetId,
    kiosk: bool,

    vars: WindowVars,
    notified_state: WindowState,
    icon_img: Option<ImageVar>,

    first_update: bool,
    first_layout: bool,

    // latest frame.
    frame_id: FrameId,
    // request for after the current frame finishes rendering in the view-process.
    pending_render: Option<WindowRenderUpdate>,

    resized_frame_wait_id: Option<FrameWaitId>,

    // latest computed monitor info, use self.monitor_info() to get.
    monitor_info: Option<WindowMonitorInfo>,

    position: Option<DipPoint>,
    size: DipSize,
    min_size: DipSize,
    max_size: DipSize,

    clear_color: RenderColor,

    deinited: bool,
}
impl AppWindow {
    fn new(
        ctx: &mut AppContext,
        new_window: Box<dyn FnOnce(&mut WindowContext) -> Window>,
        force_headless: Option<WindowMode>,
        default_render_mode: RenderMode,
    ) -> (Self, AppWindowInfo) {
        // get mode.
        let window_mode = match (ctx.window_mode(), force_headless) {
            (WindowMode::Headed | WindowMode::HeadlessWithRenderer, Some(mode)) => {
                debug_assert!(!matches!(mode, WindowMode::Headed));
                mode
            }
            (mode, _) => mode,
        };

        // init vars.
        let vars = WindowVars::new(default_render_mode);
        let mut wn_state = OwnedStateMap::default();
        wn_state.set(WindowVarsKey, vars.clone());

        // init root.
        let id = WindowId::new_unique();
        let root = ctx.window_context(id, window_mode, &mut wn_state, new_window).0;
        let render_mode = root.render_mode.unwrap_or(default_render_mode);
        vars.0.render_mode.set_ne(ctx, render_mode);
        let root_id = root.id;

        let headless_monitor = if window_mode.is_headless() {
            Some(root.headless_monitor.clone())
        } else {
            None
        };

        let kiosk = root.kiosk;

        // init context.
        let context = OwnedWindowContext {
            window_id: id,
            window_mode,
            root_transform_key: WidgetTransformKey::new_unique(),
            state: wn_state,
            root,
            root_bounds: BoundsRect::new(),
            root_rendered: WidgetRendered::new(),
            update: WindowUpdates::all(),
            subscriptions: WidgetSubscriptions::new(),
            prev_metrics: None,
            used_frame_info_builder: None,
            used_frame_builder: None,
            used_frame_update: None,
        };

        // we want the window content to init, update, layout & render to get
        // all the values needed to actually spawn a real window, this is so we
        // have a frame ready to show when the window is visible.
        ctx.updates.update_ext();
        ctx.updates.layout_and_render();

        let frame_info = WidgetInfoTree::blank(id, root_id);

        let win = AppWindow {
            headed: None, // headed & renderer will initialize on first render.
            renderer: None,
            headless_monitor,
            headless_surface: None,
            context,
            window_mode,
            id,
            root_id,
            kiosk,
            vars: vars.clone(),
            notified_state: WindowState::Normal,
            icon_img: None,

            first_update: true,
            first_layout: true,

            monitor_info: None,

            frame_id: FrameId::INVALID,
            pending_render: None,
            position: None,
            size: DipSize::zero(),
            min_size: DipSize::zero(),
            max_size: DipSize::zero(),
            clear_color: RenderColor::TRANSPARENT,

            resized_frame_wait_id: None,

            deinited: false,
        };
        let info = AppWindowInfo {
            id,
            mode: window_mode,
            renderer: None, // will be set on the first render
            vars,
            scale_factor: 1.0.fct(), // will be set on the first layout
            widget_tree: frame_info,
            is_focused: false, // will be set by listening to RawWindowFocusEvent
        };

        (win, info)
    }

    fn on_info(&mut self, ctx: &mut AppContext) {
        if self.context.update.info {
            let _s = tracing::trace_span!("window.info", window = %self.id.sequential()).entered();
            let tree = self.context.info(ctx);
            ctx.services.windows().windows_info.get_mut(&self.id).unwrap().widget_tree = tree.clone();
            WidgetInfoChangedEvent.notify(
                ctx,
                WidgetInfoChangedArgs::now(self.id, tree, ctx.updates.layout_requested(), ctx.updates.render_requested()),
            );
        }
    }

    fn on_subscriptions(&mut self, ctx: &mut AppContext) {
        if self.context.update.subscriptions {
            let _s = tracing::trace_span!("window.subscriptions", window = %self.id.sequential()).entered();
            self.context.subscriptions(ctx);
        }
    }

    fn on_event<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        self.context.event(ctx, args);
        self.on_info(ctx);
        self.on_subscriptions(ctx);
    }

    fn on_update(&mut self, ctx: &mut AppContext) {
        if self.first_update {
            let _s = tracing::trace_span!("window.on_update#first", window = %self.id.sequential()).entered();

            self.context.init(ctx);

            self.context.update.info = true;
            self.context.update.subscriptions = true;
            self.on_info(ctx);
            self.on_subscriptions(ctx);

            self.first_update = false;
        } else {
            let _s = tracing::trace_span!("window.on_update", window = %self.id.sequential()).entered();

            self.context.update(ctx);

            self.on_info(ctx);
            self.on_subscriptions(ctx);

            if self.vars.size().is_new(ctx)
                || self.vars.auto_size().is_new(ctx)
                || self.vars.min_size().is_new(ctx)
                || self.vars.max_size().is_new(ctx)
            {
                self.on_size_vars_update(ctx);
            }

            /// Respawned error is ok here, because we recreate the window on respawn.
            type Ignore = Result<(), Respawned>;

            if self.vars.position().is_new(ctx) && !self.first_layout {
                self.position = self.layout_position(ctx);

                if let Some(pos) = self.position {
                    let restore_only = !matches!(self.vars.state().copy(ctx), WindowState::Normal);

                    if let Some(w) = &self.headed {
                        let _: Ignore = w.set_position(pos);
                    } else if !restore_only {
                        RawWindowChangedEvent.notify(
                            ctx.events,
                            RawWindowChangedArgs::now(self.id, None, Some(pos), None, None, EventCause::App, None),
                        );
                    }

                    if restore_only {
                        // we don't get a window move in this case.
                        self.vars.0.restore_rect.modify(ctx, move |r| {
                            if r.origin != pos {
                                r.origin = pos;
                            }
                        });
                    }
                }
            }

            if let Some(w) = &self.headed {
                if let Some(monitor) = self.vars.monitor().get_new(ctx.vars) {
                    let monitor_info = monitor.select(ctx.services.monitors());

                    if let Some(pos) = self.vars.position().get_new(ctx.vars) {
                        todo!("use pos, else center {:?}", pos)
                    }

                    if let Some(size) = self.vars.size().get_new(ctx.vars) {
                        todo!("use new size, else actual_size {:?}", size)
                    }

                    todo!("refresh monitor {:?}", monitor_info);
                }

                if let Some(mode) = self.vars.video_mode().copy_new(ctx.vars) {
                    let _: Ignore = w.set_video_mode(mode);
                }

                if let Some(title) = self.vars.title().get_new(ctx) {
                    let _: Ignore = w.set_title(title.to_string());
                }
                if let Some(chrome) = self.vars.chrome().get_new(ctx) {
                    match chrome {
                        WindowChrome::Default => {
                            let _: Ignore = w.set_chrome_visible(true);
                        }
                        WindowChrome::None => {
                            let _: Ignore = w.set_chrome_visible(false);
                        }
                        WindowChrome::Custom => {
                            let _: Ignore = w.set_chrome_visible(false);
                            todo!();
                        }
                    }
                }
                if let Some(ico) = self.vars.icon().get_new(ctx.vars) {
                    match ico {
                        WindowIcon::Default => {
                            let _: Ignore = w.set_icon(None);
                            self.icon_img = None;
                        }
                        WindowIcon::Image(r) => {
                            let ico = ctx.services.images().cache(r.clone());
                            let _: Ignore = w.set_icon(ico.get(ctx).view());
                            self.icon_img = Some(ico);
                        }
                        WindowIcon::Render(_) => {
                            self.icon_img = None;
                            todo!()
                        }
                    }
                } else if let Some(ico) = &self.icon_img {
                    let _: Ignore = w.set_icon(ico.get(ctx).view());
                }
                if let Some(cur) = self.vars.cursor().copy_new(ctx.vars) {
                    let _: Ignore = w.set_cursor(cur);
                }
                if let Some(state) = self.vars.state().copy_new(ctx) {
                    if self.kiosk && !state.is_fullscreen() {
                        tracing::warn!("kiosk mode blocked state `{:?}`, will remain fullscreen", state);
                    } else {
                        let _: Ignore = w.set_state(state);
                    }
                }
                if let Some(resizable) = self.vars.resizable().copy_new(ctx) {
                    let _: Ignore = w.set_resizable(resizable);
                }
                if let Some(movable) = self.vars.movable().copy_new(ctx) {
                    let _: Ignore = w.set_movable(movable);
                }
                if let Some(always_on_top) = self.vars.always_on_top().copy_new(ctx) {
                    let _: Ignore = w.set_always_on_top(always_on_top);
                }
                if let Some(visible) = self.vars.visible().copy_new(ctx) {
                    let _: Ignore = w.set_visible(visible);
                }
                if let Some(visible) = self.vars.taskbar_visible().copy_new(ctx) {
                    let _: Ignore = w.set_taskbar_visible(visible);
                }
                if self.vars.parent().is_new(ctx) || self.vars.modal().is_new(ctx) {
                    let _: Ignore = w.set_parent(self.vars.parent().copy(ctx), self.vars.modal().copy(ctx));
                }
                if let Some(allow) = self.vars.allow_alt_f4().copy_new(ctx) {
                    let _: Ignore = w.set_allow_alt_f4(allow);
                }
                if let Some(mode) = self.vars.frame_capture_mode().copy_new(ctx) {
                    let _: Ignore = w.set_capture_mode(matches!(mode, FrameCaptureMode::All));
                }
            }

            if let Some(r) = &self.renderer {
                if let Some(text_aa) = self.vars.text_aa().copy_new(ctx) {
                    let _: Ignore = r.set_text_aa(text_aa);
                }
            }
        }
    }

    fn init_monitor_info(&self, ctx: &mut AppContext) -> WindowMonitorInfo {
        if let WindowMode::Headed = self.window_mode {
            // try `actual_monitor`
            let monitor = self.vars.actual_monitor().copy(ctx);
            if let Some(id) = monitor {
                if let Some(m) = ctx.services.monitors().monitor(id) {
                    return WindowMonitorInfo {
                        // id: Some(id),
                        // position: m.info.position,
                        size: m.info.dip_size(),
                        scale_factor: m.info.scale_factor.fct(),
                        ppi: m.ppi.copy(ctx.vars),
                    };
                }
            }

            // no `actual_monitor`, run `monitor` query.
            let query = self.vars.monitor().get(ctx.vars);
            if let Some(m) = query.select(ctx.services.monitors()) {
                self.vars.0.actual_monitor.set_ne(ctx.vars, Some(m.id));

                return WindowMonitorInfo {
                    // id: Some(m.id),
                    // position: m.info.position,
                    size: m.info.dip_size(),
                    scale_factor: m.info.scale_factor.fct(),
                    ppi: m.ppi.copy(ctx.vars),
                };
            }

            tracing::warn!("monitor query did not find a match, fallback to primary monitor");

            // fallback to primary monitor.
            if let Some(p) = ctx.services.monitors().primary_monitor() {
                self.vars.0.actual_monitor.set_ne(ctx.vars, Some(p.id));

                return WindowMonitorInfo {
                    // id: Some(p.id),
                    // position: p.info.position,
                    size: p.info.dip_size(),
                    scale_factor: p.info.scale_factor.fct(),
                    ppi: p.ppi.copy(ctx.vars),
                };
            }

            tracing::error!("no primary monitor found, fallback to `headless_monitor` values");

            // fallback to headless defaults.
            let h = self.headless_monitor.clone().unwrap_or_default();
            WindowMonitorInfo {
                // id: None,
                // position: PxPoint::zero(),
                size: h.size,
                scale_factor: h.scale_factor,
                ppi: h.ppi,
            }
        } else {
            let h = self.headless_monitor.as_ref().unwrap();
            WindowMonitorInfo {
                // id: None,
                // position: PxPoint::zero(),
                size: h.size,
                scale_factor: h.scale_factor,
                ppi: h.ppi,
            }
        }
    }

    /// Gets or init the current monitor info.
    fn monitor_info(&mut self, ctx: &mut AppContext) -> WindowMonitorInfo {
        if let Some(info) = self.monitor_info {
            info
        } else {
            let info = self.init_monitor_info(ctx);
            self.monitor_info = Some(info);
            info
        }
    }

    /// On any of the variables involved in sizing updated.
    ///
    /// Do measure/arrange, and if sizes actually changed send resizes.
    fn on_size_vars_update(&mut self, ctx: &mut AppContext) {
        if self.first_layout {
            // values will be used in first-layout.
            return;
        }

        if self.kiosk {
            // only fullscreen size allowed.
            return;
        }

        // `size` var is only used on init or once after update AND if auto_size did not override it.
        let use_system_size = !self.vars.size().is_new(ctx.vars);
        let (size, min_size, max_size) = self.layout_size(ctx, use_system_size);

        if self.size != size {
            let _s = tracing::trace_span!("resize/render-vars").entered();

            // resize view
            self.size = size;
            if let Some(w) = &self.headed {
                let _ = w.set_size(size);
            } else if let Some(s) = &self.headless_surface {
                let _ = s.set_size(size, self.headless_monitor.as_ref().map(|m| m.scale_factor).unwrap_or(Factor(1.0)));
            } else {
                // headless "resize"
                RawWindowChangedEvent.notify(
                    ctx.events,
                    RawWindowChangedArgs::now(self.id, None, None, None, Some(self.size), EventCause::App, None),
                );
            }

            // the `restore_size` is set from the resize event, unless we are not `Normal`, then it is only recorded
            // by the view-process, so we need to update here as well.
            if !matches!(self.vars.state().copy(ctx), WindowState::Normal) {
                self.vars.0.restore_rect.modify(ctx, move |r| {
                    if r.size != size {
                        r.size = size;
                    }
                });
            }

            // the `actual_size` is set from the resize event only.
        }

        // after potential resize, so we don't cause a resize from system
        // because winit coerces sizes.
        if self.min_size != min_size {
            self.min_size = min_size;
            if let Some(w) = &self.headed {
                let _ = w.set_min_size(min_size);
            }
        }
        if self.max_size != max_size {
            self.max_size = max_size;
            if let Some(w) = &self.headed {
                let _ = w.set_max_size(max_size);
            }
        }
    }

    /// On layout request can go two ways, if auto-size is enabled we will end-up resizing the window (probably)
    /// in this case we also render to send the frame together with the resize request, otherwise we just do layout
    /// and then wait for the normal render request.
    fn on_layout(&mut self, ctx: &mut AppContext) {
        if !self.context.update.layout {
            return;
        }

        if let WindowState::Minimized = self.vars.state().copy(ctx) {
            return;
        }

        if self.first_layout {
            self.on_init_layout(ctx);
            return;
        }

        let _s = tracing::trace_span!("window.on_layout", window = %self.id.sequential()).entered();

        // layout using the "system" size, it can still be overwritten by auto_size.
        let (size, _, _) = self.layout_size(ctx, true);

        if self.size != size {
            let _s = tracing::trace_span!("resize/layout").entered();
            self.size = size;
            if let Some(w) = &self.headed {
                // check normal because can change size while maximized when the system DPI changes.
                if let WindowState::Normal = self.vars.state().copy(ctx) {
                    let _ = w.set_size(size);
                }
            } else if let Some(s) = &self.headless_surface {
                let _ = s.set_size(size, self.headless_monitor.as_ref().map(|m| m.scale_factor).unwrap_or(Factor(1.0)));
            } else {
                // headless "resize"
                RawWindowChangedEvent.notify(
                    ctx.events,
                    RawWindowChangedArgs::now(self.id, None, None, None, self.size, EventCause::App, None),
                );
            }
            // the `actual_size` is set from the resize event only.
        }
    }

    /// `on_layout` requested before the first frame render.
    fn on_init_layout(&mut self, ctx: &mut AppContext) {
        debug_assert!(self.first_layout);

        let _s = tracing::trace_span!("window.on_init_layout", window = %self.id.sequential()).entered();

        self.first_layout = false;

        let mut state = self.vars.state().copy(ctx);
        if self.kiosk && !state.is_fullscreen() {
            tracing::warn!("kiosk mode but not fullscreen, will force to fullscreen");
            state = WindowState::Fullscreen;
        }

        if let WindowState::Normal | WindowState::Minimized = state {
            // we already have the size, and need to calculate the start-position.

            let (final_size, min_size, max_size) = self.layout_size(ctx, false);

            self.size = final_size;
            self.min_size = min_size;
            self.max_size = max_size;

            // compute start position.
            match self.context.root.start_position {
                StartPosition::Default => {
                    // `layout_position` can return `None` or a computed position.
                    // We use `None` to signal the view-process to let the OS define the start position.
                    self.position = self.layout_position(ctx);
                }
                StartPosition::CenterMonitor => {
                    let scr_size = self.monitor_info(ctx).size;
                    self.position = Some(DipPoint::new(
                        (scr_size.width - self.size.width) / Dip::new(2),
                        (scr_size.height - self.size.height) / Dip::new(2),
                    ));
                }
                StartPosition::CenterParent => todo!(),
            }

            // `on_render` will complete first_render.
            self.context.update.render = WindowRenderUpdate::Render;
            ctx.updates.render();
        } else if state.is_fullscreen() {
            // we already have the size, it is the monitor size (for Exclusive we are okay with a blink if the resolution does not match).

            let size = self.monitor_info(ctx).size;

            if self.size != size {
                self.size = size;
                RawWindowChangedEvent.notify(
                    ctx,
                    RawWindowChangedArgs::now(self.id, None, None, None, size, EventCause::App, None),
                );
            }

            let (size, min_size, max_size) = self.layout_size(ctx, true);
            debug_assert_eq!(size, self.size);
            self.min_size = min_size;
            self.max_size = max_size;

            self.context.update.render = WindowRenderUpdate::Render;
            ctx.updates.render();
        } else {
            // we don't have the size, the maximized size needs to exclude window-chrome and non-client area.
            // but we do calculate the size as the "restore" size.
            let (size, min_size, max_size) = self.layout_size(ctx, false);
            self.size = size;
            self.min_size = min_size;
            self.max_size = max_size;

            // and then will layout again once the window opens.
            self.context.update.layout = true;
            self.context.update.render = WindowRenderUpdate::Render;
            ctx.updates.layout();
        }

        // open the view window, it will remain invisible until the first frame is rendered
        // but we need it now to get the frame size.
        let vp = ctx.services.get::<ViewProcess>();
        match self.window_mode {
            WindowMode::Headed => {
                // send window request to the view-process, in the view-process the window will start but
                // still not visible, when the renderer has a frame ready to draw then the window becomes
                // visible. All layout values are ready here too.
                let config = view_process::WindowRequest {
                    id: self.id.get(),
                    title: self.vars.title().get(ctx.vars).to_string(),
                    pos: self.position,
                    size: self.size,
                    min_size: self.min_size,
                    max_size: self.max_size,
                    state,
                    video_mode: self.vars.video_mode().copy(ctx.vars),
                    visible: self.vars.visible().copy(ctx.vars),
                    taskbar_visible: self.vars.taskbar_visible().copy(ctx.vars),
                    chrome_visible: self.vars.chrome().get(ctx.vars).is_default(),
                    allow_alt_f4: self.vars.allow_alt_f4().copy(ctx.vars),
                    text_aa: self.vars.text_aa().copy(ctx.vars),
                    always_on_top: self.vars.always_on_top().copy(ctx.vars),
                    movable: self.vars.movable().copy(ctx.vars),
                    resizable: self.vars.resizable().copy(ctx.vars),
                    icon: match self.vars.icon().get(ctx.vars) {
                        WindowIcon::Default => None,
                        WindowIcon::Image(_) => {
                            let vars = ctx.vars;
                            self.icon_img.as_ref().and_then(|i| i.get(vars).view()).map(|i| i.id())
                        }
                        WindowIcon::Render(_) => todo!(),
                    },
                    cursor: self.vars.cursor().copy(ctx.vars),
                    transparent: self.context.root.transparent,
                    capture_mode: matches!(self.vars.frame_capture_mode().copy(ctx.vars), FrameCaptureMode::All),
                    render_mode: self.vars.0.render_mode.copy(ctx.vars),
                };

                // keep the ViewWindow connection and already create the weak-ref ViewRenderer too.
                let (headed, data) = match vp.unwrap().open_window(config) {
                    Ok(h) => h,
                    // we re-render and re-open the window on respawn event.
                    Err(Respawned) => return,
                };

                self.renderer = Some(headed.renderer());
                self.headed = Some(headed);
                ctx.services.windows().windows_info.get_mut(&self.id).unwrap().renderer = self.renderer.clone();

                let mut syn_args = RawWindowChangedArgs::now(self.id, None, None, None, None, EventCause::App, None);

                if self.size != data.size || self.context.update.layout {
                    self.size = data.size;
                    syn_args.size = Some(self.size);

                    self.context.update.render = WindowRenderUpdate::Render;
                    ctx.updates.render();

                    let (size, min_size, max_size) = self.layout_size(ctx, true);

                    if size != self.size {
                        tracing::error!(
                            "content size does not match window size, expected `{:?}`, but was `{:?}`",
                            self.size,
                            size
                        );
                    }
                    self.min_size = min_size;
                    self.max_size = max_size;
                }

                if self.position != Some(data.position) {
                    self.position = Some(data.position);

                    syn_args.position = self.position;
                }

                self.vars.0.render_mode.set_ne(ctx.vars, data.render_mode);

                //syn_args.monitor = Some((data.monitor, data.scale_factor)); // TODO, set scale-factor via event?
                ctx.services.windows().windows_info.get_mut(&self.id).unwrap().scale_factor = data.scale_factor.fct();

                RawWindowChangedEvent.notify(ctx, syn_args);
            }
            WindowMode::HeadlessWithRenderer => {
                let scale_factor = self.headless_monitor.as_ref().unwrap().scale_factor;
                let config = view_process::HeadlessRequest {
                    id: self.id.get(),
                    size: self.size,
                    scale_factor: scale_factor.0,
                    text_aa: self.vars.text_aa().copy(ctx.vars),
                    render_mode: self.vars.0.render_mode.copy(ctx.vars),
                };

                let (surface, data) = match vp.unwrap().open_headless(config) {
                    Ok(h) => h,
                    // we re-render and re-open the window on respawn event.
                    Err(Respawned) => return,
                };
                self.renderer = Some(surface.renderer());
                self.headless_surface = Some(surface);
                ctx.services.windows().windows_info.get_mut(&self.id).unwrap().renderer = self.renderer.clone();

                self.vars.0.render_mode.set_ne(ctx.vars, data.render_mode);

                ctx.services.windows().windows_info.get_mut(&self.id).unwrap().scale_factor = scale_factor;
            }
            WindowMode::Headless => {
                // headless without renderer only provides the `FrameInfo` (notified in `render_frame`),
                // but if we are in a full headless app we can simulate the behavior of headed windows that
                // become visible and focused when they present the first frame and "resized" and "moved" with
                // initial values.

                let timestamp = Instant::now();
                if vp.is_none() {
                    // if we are in a headless app too, we simulate focus.
                    drop(vp);
                    if let Some((prev_focus_id, _)) = ctx.services.windows().windows_info.iter_mut().find(|(_, w)| w.is_focused) {
                        let args = RawWindowFocusArgs::new(timestamp, *prev_focus_id, false);
                        RawWindowFocusEvent.notify(ctx.events, args)
                    }
                    let args = RawWindowFocusArgs::new(timestamp, self.id, true);
                    RawWindowFocusEvent.notify(ctx.events, args)
                }

                let syn_args = RawWindowChangedArgs::new(
                    timestamp,
                    self.id,
                    None,
                    self.position,
                    None,
                    Some(self.size),
                    EventCause::App,
                    None,
                );
                RawWindowChangedEvent.notify(ctx.events, syn_args);

                let scale_factor = self.headless_monitor.as_ref().unwrap().scale_factor;

                ctx.services.windows().windows_info.get_mut(&self.id).unwrap().scale_factor = scale_factor;
            }
        }
    }

    /// Calculate the position var in the current monitor.
    fn layout_position(&mut self, ctx: &mut AppContext) -> Option<DipPoint> {
        let monitor_info = self.monitor_info(ctx);

        let pos = self.vars.position().get(ctx.vars);

        if pos.x.is_default() || pos.y.is_default() {
            None
        } else {
            let pos = ctx.outer_layout_context(
                monitor_info.size.to_px(monitor_info.scale_factor.0),
                monitor_info.scale_factor,
                monitor_info.ppi,
                LayoutMask::all(),
                self.id,
                self.root_id,
                |ctx| pos.to_layout(ctx, AvailableSize::finite(ctx.viewport_size), PxPoint::zero()),
            );
            Some(pos.to_dip(monitor_info.scale_factor.0))
        }
    }

    /// Measure and arrange the content, returns the final, min and max sizes.
    ///
    /// If `use_system_size` is `true` the `size` variable is ignored.
    fn layout_size(&mut self, ctx: &mut AppContext, use_system_size: bool) -> (DipSize, DipSize, DipSize) {
        let monitor_info = self.monitor_info(ctx);

        let (available_size, min_size, max_size, auto_size) = ctx.outer_layout_context(
            monitor_info.size.to_px(monitor_info.scale_factor.0),
            monitor_info.scale_factor,
            monitor_info.ppi,
            LayoutMask::all(),
            self.id,
            self.root_id,
            |ctx| {
                let scr_size = AvailableSize::finite(ctx.viewport_size);

                let default_size = Size::new(800, 600).to_layout(ctx, scr_size, PxSize::zero());
                let default_min_size = Size::new(192, 48).to_layout(ctx, scr_size, PxSize::zero());
                let default_max_size = ctx.viewport_size; // (100%, 100%)

                let mut size = if use_system_size || self.kiosk {
                    self.size.to_px(ctx.scale_factor.0)
                } else {
                    self.vars.size().get(ctx.vars).to_layout(ctx, scr_size, default_size)
                };
                let min_size = self.vars.min_size().get(ctx.vars).to_layout(ctx, scr_size, default_min_size);
                let max_size = self.vars.max_size().get(ctx.vars).to_layout(ctx, scr_size, default_max_size);

                let auto_size = self.vars.auto_size().copy(ctx);
                if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                    size.width = max_size.width;
                } else {
                    size.width = size.width.max(min_size.width).min(max_size.width);
                }
                if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                    size.height = max_size.height;
                } else {
                    size.height = size.height.max(min_size.height).min(max_size.height);
                }

                (size, min_size, max_size, auto_size)
            },
        );

        let root_font_size = Length::pt_to_px(14.0, monitor_info.scale_factor);

        let final_size = self.context.layout(
            ctx,
            root_font_size,
            monitor_info.scale_factor,
            monitor_info.ppi,
            available_size,
            |desired_size| {
                let mut final_size = available_size;
                if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                    final_size.width = desired_size.width.max(min_size.width).min(available_size.width);
                }
                if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                    final_size.height = desired_size.height.max(min_size.height).min(available_size.height);
                }
                final_size
            },
        );

        self.context.root_bounds.set_size(final_size.to_px(monitor_info.scale_factor.0));

        (
            final_size,
            min_size.to_dip(monitor_info.scale_factor.0),
            max_size.to_dip(monitor_info.scale_factor.0),
        )
    }

    /// Render frame for sending.
    ///
    /// The `frame_id` and `frame_info` are updated.
    #[must_use = "must send the frame"]
    fn render_frame(&mut self, ctx: &mut AppContext) -> Option<view_process::FrameRequest> {
        let scale_factor = self.monitor_info(ctx).scale_factor;
        let next_frame_id = self.frame_id.next();

        // `UiNode::render`
        let frame = self.context.render(
            ctx,
            next_frame_id,
            scale_factor,
            self.renderer.clone(),
        );

        self.clear_color = frame.clear_color;
        self.frame_id = frame.id;

        let (payload, descriptor) = frame.display_list;

        let capture_image = self.take_capture_image(ctx.vars);
        // will need to send frame if there is a renderer
        if let Some(r) = &self.renderer {
            Some(view_process::FrameRequest {
                id: self.frame_id,
                pipeline_id: frame.pipeline_id,
                document_id: r.document_id().unwrap_or(zero_ui_view_api::webrender_api::DocumentId::INVALID),
                clear_color: frame.clear_color,
                display_list: (
                    IpcBytes::from_vec(payload.items_data),
                    IpcBytes::from_vec(payload.cache_data),
                    IpcBytes::from_vec(payload.spatial_tree),
                    descriptor,
                ),
                capture_image,
                wait_id: self.resized_frame_wait_id.take(),
            })
        } else {
            RawFrameRenderedEvent.notify(
                ctx,
                RawFrameRenderedArgs::now(self.id, self.frame_id, None, HitTestResult::default()),
            );
            None
        }
    }

    fn take_capture_image(&self, vars: &Vars) -> bool {
        match self.vars.frame_capture_mode().copy(vars) {
            FrameCaptureMode::Sporadic => false,
            FrameCaptureMode::Next => {
                self.vars.frame_capture_mode().set(vars, FrameCaptureMode::Sporadic);
                true
            }
            FrameCaptureMode::All => true,
        }
    }

    /// On render request.
    ///
    /// If there is a pending request we generate the frame and send.
    fn on_render(&mut self, ctx: &mut AppContext) {
        if !self.context.update.render.is_render() {
            return;
        }

        if let WindowState::Minimized = self.vars.state().copy(ctx) {
            return;
        }

        if let Some(pending) = &mut self.pending_render {
            *pending = WindowRenderUpdate::Render;
            self.context.update.render = WindowRenderUpdate::None;
            return;
        }

        let _s = tracing::trace_span!("window.on_render", window = %self.id.sequential()).entered();

        let frame = self.render_frame(ctx);

        if let Some(renderer) = &mut self.renderer {
            // we re-render and re-open the window on respawn event.
            let _: Result<(), Respawned> = renderer.render(frame.unwrap());
            self.pending_render = Some(WindowRenderUpdate::None);
        }
    }

    /// On render update request.
    ///
    /// If there is a pending request we collect updates and send.
    fn on_render_update(&mut self, ctx: &mut AppContext) {
        if !self.context.update.render.is_render_update() {
            return;
        }

        if let WindowState::Minimized = self.vars.state().copy(ctx) {
            return;
        }

        if let Some(pending) = &mut self.pending_render {
            *pending |= WindowRenderUpdate::RenderUpdate;
            self.context.update.render = WindowRenderUpdate::None;
            return;
        }

        let _s = tracing::trace_span!("window.on_render_update", window = %self.id.sequential()).entered();

        let capture_image = self.take_capture_image(ctx.vars);

        let next_frame_id = self.frame_id.next_update();

        let updates = self.context.render_update(ctx, next_frame_id, self.clear_color);

        if let Some(c) = updates.clear_color {
            self.clear_color = c;
        }

        let request = FrameUpdateRequest {
            id: next_frame_id,
            updates: updates.bindings,
            scroll_updates: updates.scrolls,
            clear_color: updates.clear_color,
            capture_image,
            wait_id: self.resized_frame_wait_id.take(),
        };
        if request.is_empty() {
            return;
        }

        self.frame_id = next_frame_id;

        if let Some(renderer) = &self.renderer {
            // send update if we have a renderer, ignore Respawned because we handle this using the respawned event.
            let _: Result<(), Respawned> = renderer.render_update(request);
            self.pending_render = Some(WindowRenderUpdate::None);
        }
    }

    fn respawn(&mut self, ctx: &mut AppContext, gen: ViewProcessGen) {
        if let Some(r) = &self.renderer {
            if r.generation() == Ok(gen) {
                // already recovered, this can happen in case of two respawns
                // happening very fast.
                return;
            }
        }

        self.pending_render = None;
        self.first_layout = true;
        self.headed = None;
        self.renderer = None;
        ctx.services.windows().windows_info.get_mut(&self.id).unwrap().renderer = None;

        self.context.update = WindowUpdates::all();
        self.on_layout(ctx);
        self.on_render(ctx);
    }

    fn deinit(mut self, ctx: &mut AppContext) {
        assert!(!self.deinited);
        self.deinited = true;
        self.context.deinit(ctx);
    }
}
impl Drop for AppWindow {
    fn drop(&mut self) {
        if !self.deinited && !thread::panicking() {
            tracing::error!("`AppWindow` dropped without calling `deinit`, no memory is leaked but shared state may be incorrect now");
        }
    }
}
#[derive(Clone, Copy)]
struct WindowMonitorInfo {
    // id: Option<MonitorId>,
    // position: PxPoint,
    size: DipSize,
    scale_factor: Factor,
    ppi: f32,
}

struct OwnedWindowContext {
    window_id: WindowId,
    window_mode: WindowMode,
    root_transform_key: WidgetTransformKey,
    state: OwnedStateMap,
    root: Window,
    root_bounds: BoundsRect,
    root_rendered: WidgetRendered,
    update: WindowUpdates,
    subscriptions: WidgetSubscriptions,

    prev_metrics: Option<(Px, Factor, f32, PxSize)>,
    used_frame_info_builder: Option<UsedWidgetInfoBuilder>,
    used_frame_builder: Option<UsedFrameBuilder>,
    used_frame_update: Option<UsedFrameUpdate>,
}
impl OwnedWindowContext {
    fn init(&mut self, ctx: &mut AppContext) {
        self.widget_ctx(ctx, |ctx, child| child.init(ctx));
    }

    fn event<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        if self.subscriptions.event_contains(args) {
            self.widget_ctx(ctx, |ctx, root| root.event(ctx, args));
        }
    }

    fn update(&mut self, ctx: &mut AppContext) {
        if self.subscriptions.update_intersects(ctx.updates) {
            self.widget_ctx(ctx, |ctx, child| child.update(ctx))
        }
    }

    #[must_use]
    fn info(&mut self, ctx: &mut AppContext) -> WidgetInfoTree {
        debug_assert!(self.update.info);
        self.update.info = false;

        let root = &self.root;
        let root_bounds = self.root_bounds.clone();
        let (builder, _) = ctx.window_context(self.window_id, self.window_mode, &mut self.state, |ctx| {
            let child = &root.child;
            let mut builder = WidgetInfoBuilder::new(
                *ctx.window_id,
                root.id,
                root_bounds,
                self.root_rendered.clone(),
                self.used_frame_info_builder.take(),
            );
            ctx.info_context(root.id, &root.state, |ctx| {
                child.info(ctx, &mut builder);
            });
            builder
        });

        let (info, used) = builder.finalize();
        self.used_frame_info_builder = Some(used);
        info
    }

    /// Update the root widget subscriptions.
    fn subscriptions(&mut self, ctx: &mut AppContext) {
        debug_assert!(self.update.subscriptions);
        self.update.subscriptions = false;

        let root = &self.root;
        let (subscriptions, _) = ctx.window_context(self.window_id, self.window_mode, &mut self.state, |ctx| {
            let child = &root.child;
            let mut subscriptions = WidgetSubscriptions::new();
            ctx.info_context(root.id, &root.state, |ctx| {
                child.subscriptions(ctx, &mut subscriptions);
            });
            subscriptions
        });

        self.subscriptions = subscriptions;
    }

    fn deinit(&mut self, ctx: &mut AppContext) {
        self.widget_ctx(ctx, |ctx, child| child.deinit(ctx))
    }

    fn widget_ctx(&mut self, ctx: &mut AppContext, f: impl FnOnce(&mut WidgetContext, &mut BoxedUiNode)) {
        let root = &mut self.root;
        let ((), update) = ctx.window_context(self.window_id, self.window_mode, &mut self.state, |ctx| {
            let child = &mut root.child;
            ctx.widget_context(root.id, &mut root.state, |ctx| f(ctx, child))
        });
        self.update |= update;
    }

    fn layout(
        &mut self,
        ctx: &mut AppContext,
        font_size: Px,
        scale_factor: Factor,
        screen_ppi: f32,
        available_size: PxSize,
        calc_final_size: impl FnOnce(PxSize) -> PxSize,
    ) -> DipSize {
        debug_assert!(self.update.layout);
        self.update.layout = false;

        let mut changes = LayoutMask::NONE;
        if let Some((prev_font_size, prev_scale_factor, prev_screen_ppi, prev_viewport_size)) = self.prev_metrics {
            if prev_font_size != font_size {
                changes |= LayoutMask::FONT_SIZE;
            }
            if prev_scale_factor != scale_factor {
                changes |= LayoutMask::SCALE_FACTOR;
            }
            if !about_eq(prev_screen_ppi, screen_ppi, 0.001) {
                changes |= LayoutMask::SCREEN_PPI;
            }
            if prev_viewport_size != available_size {
                changes |= LayoutMask::VIEWPORT_SIZE;
            }
        } else {
            changes = LayoutMask::FONT_SIZE | LayoutMask::SCALE_FACTOR | LayoutMask::SCREEN_PPI;
        }
        self.prev_metrics = Some((font_size, scale_factor, screen_ppi, available_size));

        let root = &mut self.root;
        let (final_size, update) = ctx.window_context(self.window_id, self.window_mode, &mut self.state, |ctx| {
            let child = &mut root.child;
            ctx.layout_context(
                font_size,
                scale_factor,
                screen_ppi,
                available_size,
                changes,
                root.id,
                &mut root.state,
                |ctx| {
                    let desired_size = child.measure(ctx, AvailableSize::finite(available_size));
                    let final_size = calc_final_size(desired_size);
                    child.arrange(ctx, &mut WidgetOffset::new(), final_size);
                    final_size
                },
            )
        });
        self.update |= update;
        final_size.to_dip(scale_factor.0)
    }

    fn render(
        &mut self,
        ctx: &mut AppContext,
        frame_id: FrameId,
        scale_factor: Factor,
        renderer: Option<ViewRenderer>,
    ) -> BuiltFrame {
        debug_assert!(self.update.render.is_render());
        self.update.render = WindowRenderUpdate::None;

        let root = &mut self.root;
        let root_transform_key = self.root_transform_key;

        let (builder, _) = ctx.window_context(self.window_id, self.window_mode, &mut self.state, |ctx| {
            let child = &root.child;
            let mut builder = FrameBuilder::new(
                frame_id,
                *ctx.window_id,
                renderer,
                root.id,
                root_transform_key,
                scale_factor,
                self.used_frame_builder.take(),
            );
            ctx.render_context(root.id, &root.state, |ctx| {
                child.render(ctx, &mut builder);
            });

            builder
        });

        let (frame, used) = builder.finalize(&self.root_rendered);
        self.used_frame_builder = Some(used);
        frame
    }

    fn render_update(&mut self, ctx: &mut AppContext, frame_id: FrameId, clear_color: RenderColor) -> BuiltFrameUpdate {
        debug_assert!(self.update.render.is_render_update());

        self.update.render = WindowRenderUpdate::None;

        let root = &self.root;
        let root_transform_key = self.root_transform_key;

        let (updates, _) = ctx.window_context(self.window_id, self.window_mode, &mut self.state, |ctx| {
            let window_id = *ctx.window_id;
            ctx.render_context(root.id, &root.state, |ctx| {
                let mut update = FrameUpdate::new(
                    window_id,
                    root.id,
                    root_transform_key,
                    frame_id,
                    clear_color,
                    self.used_frame_update.take(),
                );
                root.child.render_update(ctx, &mut update);
                update
            })
        });

        let (update, used) = updates.finalize();

        self.used_frame_update = Some(used);

        update
    }
}
