//! This module implements Management of window content and synchronization of WindowVars and View-Process.

use std::mem;

use crate::{
    app::{
        raw_events::{
            RawWindowFocusArgs, RAW_COLOR_SCHEME_CHANGED_EVENT, RAW_FRAME_RENDERED_EVENT, RAW_HEADLESS_OPEN_EVENT,
            RAW_WINDOW_CHANGED_EVENT, RAW_WINDOW_FOCUS_EVENT, RAW_WINDOW_OPEN_EVENT, RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT,
        },
        view_process::*,
    },
    color::{ColorScheme, RenderColor},
    context::{state_map, InfoLayoutRenderUpdates, LayoutContext, OwnedStateMap, WidgetUpdates, WindowContext, WindowRenderUpdate},
    event::{AnyEventArgs, EventHandles, EventUpdate},
    image::{Image, ImageVar, Images},
    render::{FrameBuilder, FrameId, FrameUpdate, UsedFrameBuilder, UsedFrameUpdate},
    text::Fonts,
    units::*,
    var::*,
    widget_info::{LayoutPassId, UsedWidgetInfoBuilder, WidgetContextInfo, WidgetInfoBuilder, WidgetInfoTree, WidgetLayout},
    widget_instance::{BoxedUiNode, UiNode, WidgetId},
    window::AutoSize,
};

use super::{
    commands::{WindowCommands, MINIMIZE_CMD, RESTORE_CMD},
    FrameCaptureMode, FrameImageReadyArgs, HeadlessMonitor, MonitorInfo, Monitors, StartPosition, Window, WindowChangedArgs, WindowChrome,
    WindowIcon, WindowId, WindowMode, WindowVars, Windows, FRAME_IMAGE_READY_EVENT, MONITORS_CHANGED_EVENT, WINDOW_CHANGED_EVENT,
};

/// Implementer of `App <-> View` sync in a headed window.
struct HeadedCtrl {
    window_id: WindowId,
    window: Option<ViewWindow>,
    waiting_view: bool,
    delayed_view_updates: Vec<Box<dyn FnOnce(&ViewWindow)>>,
    vars: WindowVars,
    respawned: bool,

    content: ContentCtrl,

    // init config.
    start_position: StartPosition,
    start_focused: bool,
    kiosk: Option<WindowState>, // Some(enforced_fullscreen)
    transparent: bool,
    render_mode: Option<RenderMode>,

    // current state.
    state: Option<WindowStateAll>, // None if not inited.
    monitor: Option<MonitorInfo>,
    resize_wait_id: Option<FrameWaitId>,
    icon: Option<ImageVar>,
    icon_binding: VarHandle,
    icon_deadline: Deadline,
    actual_state: Option<WindowState>, // for WindowChangedEvent
    system_color_scheme: Option<ColorScheme>,
    parent_color_scheme: Option<ReadOnlyRcVar<ColorScheme>>,
    actual_parent: Option<WindowId>,
}
impl HeadedCtrl {
    pub fn new(window_id: WindowId, vars: &WindowVars, commands: WindowCommands, content: Window) -> Self {
        Self {
            window_id,
            window: None,
            waiting_view: false,
            delayed_view_updates: vec![],

            start_position: content.start_position,
            start_focused: content.start_focused,
            kiosk: if content.kiosk { Some(WindowState::Fullscreen) } else { None },
            transparent: content.transparent,
            render_mode: content.render_mode,

            content: ContentCtrl::new(window_id, vars.clone(), commands, content),
            vars: vars.clone(),
            respawned: false,

            state: None,
            monitor: None,
            resize_wait_id: None,
            icon: None,
            icon_binding: VarHandle::dummy(),
            icon_deadline: Deadline::timeout(1.secs()),
            system_color_scheme: None,
            parent_color_scheme: None,
            actual_parent: None,
            actual_state: None,
        }
    }

    fn update_view(&mut self, update: impl FnOnce(&ViewWindow) + 'static) {
        if let Some(view) = &self.window {
            // view is online, just update.
            update(view);
        } else if self.waiting_view {
            // update after view requested, but still not ready. Will apply when the view is received
            // or be discarded if the view-process respawns.
            self.delayed_view_updates.push(Box::new(update));
        } else {
            // respawning or view-process not inited, will recreate entire window.
        }
    }

    pub fn update(&mut self, ctx: &mut WindowContext, updates: &mut WidgetUpdates) {
        if self.window.is_none() && !self.waiting_view {
            // we request a view on the first layout.
            ctx.updates.layout();

            if let Some(enforced_fullscreen) = self.kiosk {
                // enforce kiosk in pre-init.

                if !self.vars.state().get().is_fullscreen() {
                    self.vars.state().set(ctx, enforced_fullscreen);
                }
            }
        }

        if let Some(enforced_fullscreen) = &mut self.kiosk {
            // always fullscreen, but can be windowed or exclusive.

            if let Some(state) = self.vars.state().get_new(ctx) {
                if !state.is_fullscreen() {
                    tracing::error!("window in `kiosk` mode can only be fullscreen");

                    self.vars.state().set(ctx, *enforced_fullscreen);
                } else {
                    *enforced_fullscreen = state;
                }
            }

            if let Some(false) = self.vars.visible().get_new(ctx) {
                tracing::error!("window in `kiosk` mode can not be hidden");

                self.vars.visible().set(ctx, true);
            }

            if let Some(mode) = self.vars.chrome().get_new(ctx) {
                if !mode.is_none() {
                    tracing::error!("window in `kiosk` mode can not show chrome");
                    self.vars.chrome().set(ctx, WindowChrome::None);
                }
            }
        } else {
            // not kiosk mode.

            if let Some(prev_state) = self.state.clone() {
                debug_assert!(self.window.is_some() || self.waiting_view || self.respawned);

                let mut new_state = prev_state.clone();

                if let Some(query) = self.vars.monitor().get_new(ctx.vars) {
                    let monitors = Monitors::req(ctx.services);

                    if self.monitor.is_none() {
                        let monitor = query.select_fallback(monitors);
                        let scale_factor = monitor.scale_factor().get();
                        self.vars.0.scale_factor.set_ne(ctx, scale_factor);
                        self.monitor = Some(monitor);
                    } else if let Some(new) = query.select(monitors) {
                        let current = self.vars.0.actual_monitor.get();
                        if Some(new.id()) != current {
                            let scale_factor = new.scale_factor().get();
                            self.vars.0.scale_factor.set_ne(ctx.vars, scale_factor);
                            self.vars.0.actual_monitor.set_ne(ctx.vars, new.id());
                            self.monitor = Some(new.clone());
                        }
                    }
                }

                if let Some(chrome) = self.vars.chrome().get_new(ctx.vars) {
                    new_state.chrome_visible = chrome.is_default();
                }

                if let Some(req_state) = self.vars.state().get_new(ctx) {
                    new_state.set_state(req_state);
                    self.vars.0.restore_state.set_ne(ctx, new_state.restore_state);
                }

                if self.vars.min_size().is_new(ctx) || self.vars.max_size().is_new(ctx) {
                    if let Some(m) = &self.monitor {
                        let scale_factor = m.scale_factor().get();
                        let screen_ppi = m.ppi().get();
                        let screen_size = m.size().get();
                        let (min_size, max_size) = self.content.outer_layout(ctx, scale_factor, screen_ppi, screen_size, |ctx| {
                            let min_size = self.vars.min_size().get().layout(ctx, |_| default_min_size(scale_factor));

                            let max_size = self.vars.max_size().get().layout(ctx, |_| screen_size);

                            (min_size.to_dip(scale_factor.0), max_size.to_dip(scale_factor.0))
                        });

                        let size = new_state.restore_rect.size;

                        new_state.restore_rect.size = size.min(max_size).max(min_size);
                        new_state.min_size = min_size;
                        new_state.max_size = max_size;
                    }
                }

                if let Some(auto) = self.vars.auto_size().get_new(ctx) {
                    if auto != AutoSize::DISABLED {
                        self.content.layout_requested = true;
                        ctx.updates.layout();
                    }
                }

                if self.vars.size().is_new(ctx) {
                    let auto_size = self.vars.auto_size().get();

                    if auto_size != AutoSize::CONTENT {
                        if let Some(m) = &self.monitor {
                            let scale_factor = m.scale_factor().get();
                            let screen_ppi = m.ppi().get();
                            let screen_size = m.size().get();
                            let size = self.content.outer_layout(ctx, scale_factor, screen_ppi, screen_size, |ctx| {
                                self.vars
                                    .size()
                                    .get()
                                    .layout(ctx, |_| default_size(scale_factor))
                                    .to_dip(scale_factor.0)
                            });

                            let size = size.min(new_state.max_size).max(new_state.min_size);

                            if !auto_size.contains(AutoSize::CONTENT_WIDTH) {
                                new_state.restore_rect.size.width = size.width;
                            }
                            if !auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                                new_state.restore_rect.size.height = size.height;
                            }
                        }
                    }
                }

                if let Some(pos) = self.vars.position().get_new(ctx.vars) {
                    if let Some(m) = &self.monitor {
                        let scale_factor = m.scale_factor().get();
                        let screen_ppi = m.ppi().get();
                        let screen_size = m.size().get();
                        let pos = self.content.outer_layout(ctx, scale_factor, screen_ppi, screen_size, |ctx| {
                            pos.layout(ctx, |_| PxPoint::new(Px(50), Px(50)))
                        });
                        new_state.restore_rect.origin = pos.to_dip(scale_factor.0);
                    }
                }

                if let Some(visible) = self.vars.visible().get_new(ctx) {
                    self.update_view(move |view| {
                        let _: Ignore = view.set_visible(visible);
                    });
                }

                if let Some(movable) = self.vars.movable().get_new(ctx) {
                    self.update_view(move |view| {
                        let _: Ignore = view.set_movable(movable);
                    });
                }

                if let Some(resizable) = self.vars.resizable().get_new(ctx) {
                    self.update_view(move |view| {
                        let _: Ignore = view.set_resizable(resizable);
                    });
                }

                if prev_state != new_state {
                    self.update_view(move |view| {
                        let _: Ignore = view.set_state(new_state);
                    })
                }
            }

            // icon:
            let mut send_icon = false;
            if let Some(ico) = self.vars.icon().get_new(ctx.vars) {
                use crate::image::ImageSource;

                self.icon = match ico {
                    WindowIcon::Default => None,
                    WindowIcon::Image(ImageSource::Render(ico, _)) => Some(Images::req(ctx.services).cache(ImageSource::Render(
                        ico.clone(),
                        Some(crate::image::ImageRenderArgs {
                            parent: Some(*ctx.window_id),
                        }),
                    ))),
                    WindowIcon::Image(source) => Some(Images::req(ctx.services).cache(source)),
                };

                if let Some(ico) = &self.icon {
                    self.icon_binding = ico.bind_map(&self.vars.0.actual_icon, |img| Some(img.clone()));
                } else {
                    self.vars.0.actual_icon.set_ne(ctx.vars, None);
                    self.icon_binding = VarHandle::dummy();
                }

                send_icon = true;
            } else if self.icon.as_ref().map(|ico| ico.is_new(ctx)).unwrap_or(false) {
                send_icon = true;
            }
            if send_icon {
                let icon = self.icon.as_ref().and_then(|ico| ico.get().view().cloned());
                self.update_view(move |view| {
                    let _: Ignore = view.set_icon(icon.as_ref());
                });
            }

            if let Some(title) = self.vars.title().get_new(ctx) {
                self.update_view(move |view| {
                    let _: Ignore = view.set_title(title.into_owned());
                });
            }

            if let Some(mode) = self.vars.video_mode().get_new(ctx) {
                self.update_view(move |view| {
                    let _: Ignore = view.set_video_mode(mode);
                });
            }

            if let Some(cursor) = self.vars.cursor().get_new(ctx) {
                self.update_view(move |view| {
                    let _: Ignore = view.set_cursor(cursor);
                });
            }

            if let Some(visible) = self.vars.taskbar_visible().get_new(ctx) {
                self.update_view(move |view| {
                    let _: Ignore = view.set_taskbar_visible(visible);
                });
            }

            if let Some(top) = self.vars.always_on_top().get_new(ctx) {
                self.update_view(move |view| {
                    let _: Ignore = view.set_always_on_top(top);
                });
            }

            if let Some(mode) = self.vars.frame_capture_mode().get_new(ctx) {
                self.update_view(move |view| {
                    let _: Ignore = view.set_capture_mode(matches!(mode, FrameCaptureMode::All));
                });
            }

            if let Some(m) = &self.monitor {
                if let Some(fct) = m.scale_factor().get_new(ctx) {
                    self.vars.0.scale_factor.set_ne(ctx, fct);
                }
                if m.scale_factor().is_new(ctx) || m.size().is_new(ctx) || m.ppi().is_new(ctx) {
                    self.content.layout_requested = true;
                    ctx.updates.layout();
                }
            }

            if let Some(indicator) = self.vars.focus_indicator().get_new(ctx) {
                if Windows::req(ctx.services).is_focused(*ctx.window_id).unwrap_or(false) {
                    self.vars.focus_indicator().set_ne(ctx, None);
                } else if let Some(view) = &self.window {
                    let _ = view.set_focus_indicator(indicator);
                    // will be set to `None` once the window is focused.
                }
                // else indicator is send with init.
            }

            let mut update_color_scheme = false;

            if update_parent(ctx, &mut self.actual_parent, &self.vars) {
                self.parent_color_scheme = self
                    .actual_parent
                    .and_then(|id| Windows::req(ctx.services).vars(id).ok().map(|v| v.actual_color_scheme()));
                update_color_scheme = true;
            }

            if update_color_scheme
                || self.vars.color_scheme().is_new(ctx)
                || self.parent_color_scheme.as_ref().map(|t| t.is_new(ctx)).unwrap_or(false)
            {
                let scheme = self
                    .vars
                    .color_scheme()
                    .get()
                    .or_else(|| self.parent_color_scheme.as_ref().map(|t| t.get()))
                    .or(self.system_color_scheme)
                    .unwrap_or_default();
                self.vars.0.actual_color_scheme.set_ne(ctx, scheme);
            }
        }

        self.content.update(ctx, updates);
    }

    pub fn window_updates(&mut self, ctx: &mut WindowContext, updates: InfoLayoutRenderUpdates) {
        self.content.window_updates(ctx, updates);
    }

    pub fn pre_event(&mut self, ctx: &mut WindowContext, update: &mut EventUpdate) {
        if let Some(args) = RAW_WINDOW_CHANGED_EVENT.on(update) {
            if args.window_id == *ctx.window_id {
                let mut state_change = None;
                let mut pos_change = None;
                let mut size_change = None;

                if let Some((monitor, _)) = args.monitor {
                    if self.vars.0.actual_monitor.get().map(|m| m != monitor).unwrap_or(true) {
                        self.vars.0.actual_monitor.set_ne(ctx, Some(monitor));
                        self.monitor = None;
                        self.content.layout_requested = true;
                        ctx.updates.layout();
                    }
                }

                if let Some(state) = args.state.clone() {
                    self.vars.state().set_ne(ctx, state.state);
                    self.vars.0.restore_rect.set_ne(ctx, state.restore_rect);
                    self.vars.0.restore_state.set_ne(ctx, state.restore_state);

                    let new_state = state.state;
                    if self.actual_state != Some(new_state) {
                        let prev_state = self.actual_state.unwrap_or(WindowState::Normal);
                        state_change = Some((prev_state, new_state));
                        self.actual_state = Some(new_state);

                        match (prev_state, new_state) {
                            (_, WindowState::Minimized) => {
                                // minimized, minimize children.
                                self.vars.0.children.with(|c| {
                                    for &c in c {
                                        MINIMIZE_CMD.scoped(c).notify(ctx.events);
                                    }
                                });
                            }
                            (WindowState::Minimized, _) => {
                                // restored, restore children.
                                self.vars.0.children.with(|c| {
                                    for &c in c {
                                        RESTORE_CMD.scoped(c).notify(ctx.events);
                                    }
                                });

                                // we skip layout & render when minimized.
                                if self.content.layout_requested {
                                    ctx.updates.layout();
                                }
                                if !self.content.render_requested.is_none() {
                                    ctx.updates.render();
                                }
                            }
                            _ => {}
                        }
                    }

                    self.state = Some(state);
                }

                if let Some(pos) = args.position {
                    if self.vars.0.actual_position.get() != pos {
                        self.vars.0.actual_position.set_ne(ctx, pos);
                        pos_change = Some(pos);
                    }
                }

                if let Some(size) = args.size {
                    if self.vars.0.actual_size.get() != size {
                        self.vars.0.actual_size.set_ne(ctx, size);
                        size_change = Some(size);

                        self.content.layout_requested = true;
                        ctx.updates.layout();

                        if args.cause == EventCause::System {
                            // resize by system (user)
                            self.vars.auto_size().set_ne(ctx, AutoSize::DISABLED);
                        }
                    }
                }

                if let Some(id) = args.frame_wait_id {
                    self.resize_wait_id = Some(id);

                    self.content.pending_render |= WindowRenderUpdate::RenderUpdate;
                    self.content.render_requested = self.content.pending_render.take();
                    ctx.updates.render_update();
                }

                if state_change.is_some() || pos_change.is_some() || size_change.is_some() {
                    let args = WindowChangedArgs::new(
                        args.timestamp,
                        args.propagation().clone(),
                        args.window_id,
                        state_change,
                        pos_change,
                        size_change,
                        args.cause,
                    );
                    WINDOW_CHANGED_EVENT.notify(ctx.events, args);
                }
            }
        } else if let Some(args) = RAW_WINDOW_FOCUS_EVENT.on(update) {
            if args.new_focus == Some(self.window_id) {
                self.vars.0.children.with(|c| {
                    let w = Windows::req(ctx.services);
                    for &c in c {
                        let _ = w.bring_to_top(c);
                    }
                });
            }
        } else if let Some(args) = MONITORS_CHANGED_EVENT.on(update) {
            if let Some(m) = &self.monitor {
                if args.removed.contains(&m.id()) {
                    self.monitor = None;
                    self.vars.0.actual_monitor.set_ne(ctx, None);
                }
            }
            self.vars.monitor().touch(ctx);
        } else if let Some(args) = RAW_WINDOW_OPEN_EVENT.on(update) {
            if args.window_id == self.window_id {
                self.waiting_view = false;

                Windows::req(ctx.services).set_renderer(*ctx.window_id, args.window.renderer());

                self.window = Some(args.window.clone());
                self.vars.0.render_mode.set_ne(ctx, args.data.render_mode);
                self.vars.state().set_ne(ctx, args.data.state.state);
                self.actual_state = Some(args.data.state.state);
                self.vars.0.restore_state.set_ne(ctx, args.data.state.restore_state);
                self.vars.0.restore_rect.set_ne(ctx, args.data.state.restore_rect);
                self.vars.0.actual_position.set_ne(ctx, args.data.position);
                self.vars.0.actual_size.set_ne(ctx, args.data.size);
                self.vars.0.actual_monitor.set_ne(ctx, args.data.monitor);
                self.vars.0.scale_factor.set_ne(ctx, args.data.scale_factor);

                self.state = Some(args.data.state.clone());
                self.system_color_scheme = Some(args.data.color_scheme);

                let scheme = self
                    .vars
                    .color_scheme()
                    .get()
                    .or_else(|| self.parent_color_scheme.as_ref().map(|t| t.get()))
                    .or(self.system_color_scheme)
                    .unwrap_or_default();
                self.vars.0.actual_color_scheme.set_ne(ctx, scheme);

                ctx.updates.layout_and_render();

                for update in mem::take(&mut self.delayed_view_updates) {
                    update(&args.window);
                }
            }
        } else if let Some(args) = RAW_COLOR_SCHEME_CHANGED_EVENT.on(update) {
            if args.window_id == self.window_id {
                self.system_color_scheme = Some(args.color_scheme);

                let scheme = self
                    .vars
                    .color_scheme()
                    .get()
                    .or_else(|| self.parent_color_scheme.as_ref().map(|t| t.get()))
                    .or(self.system_color_scheme)
                    .unwrap_or_default();
                self.vars.0.actual_color_scheme.set_ne(ctx, scheme);
            }
        } else if let Some(args) = RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT.on(update) {
            if args.window_id == self.window_id && self.window.is_none() && self.waiting_view {
                tracing::error!("view-process failed to open a window, {}", args.error);

                // was waiting view and failed, treat like a respawn.

                self.waiting_view = false;
                self.delayed_view_updates = vec![];
                self.respawned = true;

                self.content.layout_requested = true;
                self.content.render_requested = WindowRenderUpdate::Render;
                self.content.is_rendering = false;

                ctx.updates.layout_and_render();
            }
        } else if let Some(args) = VIEW_PROCESS_INITED_EVENT.on(update) {
            if let Some(view) = &self.window {
                if view.renderer().generation() != Ok(args.generation) {
                    debug_assert!(args.is_respawn);

                    self.window = None;
                    self.waiting_view = false;
                    self.delayed_view_updates = vec![];
                    self.respawned = true;

                    self.content.layout_requested = true;
                    self.content.render_requested = WindowRenderUpdate::Render;
                    self.content.is_rendering = false;

                    ctx.updates.layout_and_render();
                }
            }
        }

        self.content.pre_event(ctx, update);
    }

    pub fn ui_event(&mut self, ctx: &mut WindowContext, update: &mut EventUpdate) {
        self.content.ui_event(ctx, update);
    }

    pub fn layout(&mut self, ctx: &mut WindowContext) {
        if !self.content.layout_requested {
            return;
        }

        if self.window.is_some() {
            if matches!(self.state.as_ref().map(|s| s.state), Some(WindowState::Minimized)) {
                return;
            }
            self.layout_update(ctx);
        } else if self.respawned && !self.waiting_view {
            self.layout_respawn(ctx);
        } else if !self.waiting_view {
            self.layout_init(ctx);
        }
    }

    /// First layout, opens the window.
    fn layout_init(&mut self, ctx: &mut WindowContext) {
        self.monitor = Some(self.vars.monitor().get().select_fallback(Monitors::req(ctx.services)));

        let m = self.monitor.as_ref().unwrap();
        let scale_factor = m.scale_factor().get();
        let screen_ppi = m.ppi().get();
        let screen_rect = m.px_rect();

        // Layout min, max and size in the monitor space.
        let (min_size, max_size, mut size) = self.content.outer_layout(ctx, scale_factor, screen_ppi, screen_rect.size, |ctx| {
            let min_size = self.vars.min_size().get().layout(ctx.metrics, |_| default_min_size(scale_factor));

            let max_size = self.vars.max_size().get().layout(ctx.metrics, |_| screen_rect.size);

            let size = self.vars.size().get().layout(ctx.metrics, |_| default_size(scale_factor));

            (min_size, max_size, size.min(max_size).max(min_size))
        });

        let state = self.vars.state().get();

        if state == WindowState::Normal && self.vars.auto_size().get() != AutoSize::DISABLED {
            // layout content to get auto-size size.
            size = self.content.layout(ctx, scale_factor, screen_ppi, min_size, max_size, size, false);
        }

        // await icon load for up to 1s.
        if let Some(icon) = &self.icon {
            if !self.icon_deadline.has_elapsed() && icon.get().is_loading() {
                // block on icon loading.
                return;
            }
        }

        // update window "load" state, `is_loaded` and the `WindowLoadEvent` happen here.
        if !Windows::req(ctx.services).try_load(ctx.vars, ctx.events, ctx.timers, self.window_id) {
            // block on loading handles.
            return;
        }

        // Layout initial position in the monitor space.
        let mut system_pos = false;
        let position = match self.start_position {
            StartPosition::Default => {
                let pos = self.vars.position().get();
                if pos.x.is_default() || pos.y.is_default() {
                    system_pos = true;
                    PxPoint::zero()
                } else {
                    self.content.outer_layout(ctx, scale_factor, screen_ppi, screen_rect.size, |ctx| {
                        pos.layout(ctx.metrics, |_| PxPoint::zero()) + screen_rect.origin.to_vector()
                    })
                }
            }
            StartPosition::CenterMonitor => {
                PxPoint::new(
                    (screen_rect.size.width - size.width) / Px(2),
                    (screen_rect.size.height - size.height) / Px(2),
                ) + screen_rect.origin.to_vector()
            }
            StartPosition::CenterParent => {
                // center monitor if no parent
                let mut parent_rect = screen_rect;

                if let Some(parent) = self.vars.parent().get() {
                    if let Ok(w) = Windows::req(ctx.services).vars(parent) {
                        let factor = w.scale_factor().get();
                        let pos = w.actual_position().get().to_px(factor.0);
                        let size = w.actual_size().get().to_px(factor.0);

                        parent_rect = PxRect::new(pos, size);
                    }
                }

                PxPoint::new(
                    (parent_rect.size.width - size.width) / Px(2),
                    (parent_rect.size.height - size.height) / Px(2),
                ) + parent_rect.origin.to_vector()
            }
        };

        // send view window request:

        let position = position.to_dip(scale_factor.0);
        let size = size.to_dip(scale_factor.0);

        let state = WindowStateAll {
            state,
            restore_rect: DipRect::new(position, size),
            restore_state: WindowState::Normal,
            min_size: min_size.to_dip(scale_factor.0),
            max_size: max_size.to_dip(scale_factor.0),
            chrome_visible: self.vars.chrome().get().is_default(),
        };

        let request = WindowRequest {
            id: ctx.window_id.get(),
            title: self.vars.title().get().to_string(),
            state: state.clone(),
            kiosk: self.kiosk.is_some(),
            default_position: system_pos,
            video_mode: self.vars.video_mode().get(),
            visible: self.vars.visible().get(),
            taskbar_visible: self.vars.taskbar_visible().get(),
            always_on_top: self.vars.always_on_top().get(),
            movable: self.vars.movable().get(),
            resizable: self.vars.resizable().get(),
            icon: self.icon.as_ref().and_then(|ico| ico.get().view().map(|ico| ico.id())),
            cursor: self.vars.cursor().get(),
            transparent: self.transparent,
            capture_mode: matches!(self.vars.frame_capture_mode().get(), FrameCaptureMode::All),
            render_mode: self.render_mode.unwrap_or_else(|| Windows::req(ctx.services).default_render_mode),

            focus: self.start_focused,
            focus_indicator: self.vars.focus_indicator().get(),
        };

        match ViewProcess::req(ctx.services).open_window(request) {
            Ok(()) => {
                self.state = Some(state);
                self.waiting_view = true;
            }
            Err(ViewProcessOffline) => {} //respawn
        };
    }

    /// Layout for already open window.
    fn layout_update(&mut self, ctx: &mut WindowContext) {
        let m = self.monitor.as_ref().unwrap();
        let scale_factor = m.scale_factor().get();
        let screen_ppi = m.ppi().get();

        let mut state = self.state.clone().unwrap();

        let current_size = self.vars.0.actual_size.get().to_px(scale_factor.0);
        let mut size = current_size;
        let min_size = state.min_size.to_px(scale_factor.0);
        let max_size = state.max_size.to_px(scale_factor.0);

        let skip_auto_size = !matches!(state.state, WindowState::Normal);

        if !skip_auto_size {
            let auto_size = self.vars.auto_size().get();

            if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                size.width = max_size.width;
            }
            if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                size.height = max_size.height;
            }
        }

        let size = self
            .content
            .layout(ctx, scale_factor, screen_ppi, min_size, max_size, size, skip_auto_size);

        if size != current_size {
            assert!(!skip_auto_size);

            let auto_size_origin = self.vars.auto_size_origin().get();
            let base_font_size = base_font_size(scale_factor);
            let mut auto_size_origin = |size| {
                ctx.layout_context(
                    base_font_size,
                    scale_factor,
                    screen_ppi,
                    size,
                    &self.content.info_tree,
                    &self.content.root_info,
                    &mut self.content.root_state,
                    |ctx| auto_size_origin.layout(ctx, |_| PxPoint::zero()).to_dip(scale_factor.0),
                )
            };
            let prev_origin = auto_size_origin(current_size);
            let new_origin = auto_size_origin(size);

            let size = size.to_dip(scale_factor.0);

            state.restore_rect.size = size;
            state.restore_rect.origin += prev_origin - new_origin;

            if let Some(view) = &self.window {
                let _: Ignore = view.set_state(state);
            } else {
                debug_assert!(self.respawned);
                self.state = Some(state);
            }
        }
    }

    /// First layout after respawn, opens the window but used previous sizes.
    fn layout_respawn(&mut self, ctx: &mut WindowContext) {
        if self.monitor.is_none() {
            self.monitor = Some(self.vars.monitor().get().select_fallback(Monitors::req(ctx.services)));
        }

        self.layout_update(ctx);

        let request = WindowRequest {
            id: ctx.window_id.get(),
            title: self.vars.title().get_string(),
            state: self.state.clone().unwrap(),
            kiosk: self.kiosk.is_some(),
            default_position: false,
            video_mode: self.vars.video_mode().get(),
            visible: self.vars.visible().get(),
            taskbar_visible: self.vars.taskbar_visible().get(),
            always_on_top: self.vars.always_on_top().get(),
            movable: self.vars.movable().get(),
            resizable: self.vars.resizable().get(),
            icon: self.icon.as_ref().and_then(|ico| ico.get().view().map(|ico| ico.id())),
            cursor: self.vars.cursor().get(),
            transparent: self.transparent,
            capture_mode: matches!(self.vars.frame_capture_mode().get(), FrameCaptureMode::All),
            render_mode: self.render_mode.unwrap_or_else(|| Windows::req(ctx.services).default_render_mode),

            focus: Windows::req(ctx.services).is_focused(self.window_id).unwrap_or(false),
            focus_indicator: self.vars.focus_indicator().get(),
        };

        match ViewProcess::req(ctx.services).open_window(request) {
            Ok(()) => self.waiting_view = true,
            Err(ViewProcessOffline) => {} // respawn.
        }
    }

    pub fn render(&mut self, ctx: &mut WindowContext) {
        if self.content.render_requested.is_none() {
            return;
        }

        if let Some(view) = &self.window {
            if matches!(self.state.as_ref().map(|s| s.state), Some(WindowState::Minimized)) {
                return;
            }

            let scale_factor = self.monitor.as_ref().unwrap().scale_factor().get();
            self.content
                .render(ctx, Some(view.renderer()), scale_factor, self.resize_wait_id.take());
        }
    }

    pub fn focus(&mut self, _: &mut WindowContext) {
        self.update_view(|view| {
            let _ = view.focus();
        });
    }

    pub fn bring_to_top(&mut self, _: &mut WindowContext) {
        self.update_view(|view| {
            let _ = view.bring_to_top();
        });
    }

    pub fn close(&mut self, ctx: &mut WindowContext) {
        self.content.close(ctx);
        self.window = None;
    }
}

/// Respond to `parent_var` updates, returns `true` if the `parent` value has changed.
fn update_parent(ctx: &mut WindowContext, parent: &mut Option<WindowId>, vars: &WindowVars) -> bool {
    let parent_var = vars.parent();
    if let Some(parent_id) = parent_var.get_new(ctx) {
        if parent_id == *parent {
            return false;
        }

        match parent_id {
            Some(parent_id) => {
                if parent_id == *ctx.window_id {
                    tracing::error!("cannot set `{:?}` as it's own parent", parent_id);
                    parent_var.set(ctx.vars, *parent);
                    return false;
                }
                if !vars.0.children.with(|c| c.is_empty()) {
                    tracing::error!("cannot set parent for `{:?}` because it already has children", *ctx.window_id);
                    parent_var.set(ctx.vars, *parent);
                    return false;
                }

                let windows = Windows::req(ctx.services);
                if let Ok(parent_vars) = windows.vars(parent_id) {
                    if parent_vars.parent().with(Option::is_some) {
                        tracing::error!("cannot use `{:?}` as a parent because it already has a parent", parent_id);
                        parent_var.set(ctx.vars, *parent);
                        return false;
                    }

                    // remove previous
                    if let Some(parent_id) = parent.take() {
                        if let Ok(parent_vars) = windows.vars(parent_id) {
                            let id = *ctx.window_id;
                            parent_vars.0.children.modify(ctx.vars, move |c| {
                                c.get_mut().remove(&id);
                            });
                        }
                    }

                    // insert new
                    *parent = Some(parent_id);
                    let id = *ctx.window_id;
                    parent_vars.0.children.modify(ctx.vars, move |c| {
                        c.get_mut().insert(id);
                    });

                    true
                } else {
                    tracing::error!("cannot use `{:?}` as a parent because it does not exist", parent_id);
                    parent_var.set(ctx.vars, *parent);
                    false
                }
            }
            None => {
                if let Some(parent_id) = parent.take() {
                    if let Ok(parent_vars) = Windows::req(ctx.services).vars(parent_id) {
                        let id = *ctx.window_id;
                        parent_vars.0.children.modify(ctx.vars, move |c| {
                            c.get_mut().remove(&id);
                        });
                    }
                    true
                } else {
                    false
                }
            }
        }
    } else {
        false
    }
}

/// Implementer of `App <-> View` sync in a headless window.
struct HeadlessWithRendererCtrl {
    window_id: WindowId,
    surface: Option<ViewHeadless>,
    waiting_view: bool,
    delayed_view_updates: Vec<Box<dyn FnOnce(&ViewHeadless)>>,
    vars: WindowVars,
    content: ContentCtrl,

    // init config.
    render_mode: Option<RenderMode>,
    headless_monitor: HeadlessMonitor,
    headless_simulator: HeadlessSimulator,

    // current state.
    size: DipSize,

    actual_parent: Option<WindowId>,
    /// actual_color_scheme and scale_factor binding.
    var_bindings: VarHandles,
}
impl HeadlessWithRendererCtrl {
    pub fn new(window_id: WindowId, vars: &WindowVars, commands: WindowCommands, content: Window) -> Self {
        Self {
            window_id,
            surface: None,
            waiting_view: false,
            delayed_view_updates: vec![],
            vars: vars.clone(),

            render_mode: content.render_mode,
            headless_monitor: content.headless_monitor,
            headless_simulator: HeadlessSimulator::new(),

            content: ContentCtrl::new(window_id, vars.clone(), commands, content),

            actual_parent: None,
            size: DipSize::zero(),
            var_bindings: VarHandles::dummy(),
        }
    }

    pub fn update(&mut self, ctx: &mut WindowContext, updates: &mut WidgetUpdates) {
        if self.surface.is_some() {
            if self.vars.size().is_new(ctx)
                || self.vars.min_size().is_new(ctx)
                || self.vars.max_size().is_new(ctx)
                || self.vars.auto_size().is_new(ctx)
            {
                self.content.layout_requested = true;
                ctx.updates.layout();
            }
        } else {
            // we init on the first layout.
            ctx.updates.layout();
        }

        if update_parent(ctx, &mut self.actual_parent, &self.vars) || self.var_bindings.is_dummy() {
            self.var_bindings = update_headless_vars(ctx.vars, Windows::req(ctx.services), self.headless_monitor.scale_factor, &self.vars);
        }

        self.content.update(ctx, updates);
    }

    pub fn window_updates(&mut self, ctx: &mut WindowContext, updates: InfoLayoutRenderUpdates) {
        self.content.window_updates(ctx, updates);
    }

    pub fn pre_event(&mut self, ctx: &mut WindowContext, update: &mut EventUpdate) {
        if let Some(args) = RAW_HEADLESS_OPEN_EVENT.on(update) {
            if args.window_id == *ctx.window_id {
                self.waiting_view = false;

                Windows::req(ctx.services).set_renderer(args.window_id, args.surface.renderer());

                self.surface = Some(args.surface.clone());
                self.vars.0.render_mode.set_ne(ctx.vars, args.data.render_mode);

                ctx.updates.render();

                for update in mem::take(&mut self.delayed_view_updates) {
                    update(&args.surface);
                }
            }
        } else if let Some(args) = RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT.on(update) {
            if args.window_id == self.window_id && self.surface.is_none() && self.waiting_view {
                tracing::error!("view-process failed to open a headless surface, {}", args.error);

                // was waiting view and failed, treat like a respawn.

                self.waiting_view = false;
                self.delayed_view_updates = vec![];

                self.content.layout_requested = true;
                self.content.render_requested = WindowRenderUpdate::Render;

                ctx.updates.layout_and_render();
            }
        } else if let Some(args) = VIEW_PROCESS_INITED_EVENT.on(update) {
            if let Some(view) = &self.surface {
                if view.renderer().generation() != Ok(args.generation) {
                    debug_assert!(args.is_respawn);

                    self.surface = None;

                    self.content.is_rendering = false;
                    self.content.layout_requested = true;
                    self.content.render_requested = WindowRenderUpdate::Render;

                    ctx.updates.layout_and_render();
                }
            }
        }

        self.content.pre_event(ctx, update);

        self.headless_simulator.pre_event(ctx, update);
    }

    pub fn ui_event(&mut self, ctx: &mut WindowContext, update: &mut EventUpdate) {
        self.content.ui_event(ctx, update);
    }

    pub fn layout(&mut self, ctx: &mut WindowContext) {
        if !self.content.layout_requested {
            return;
        }

        let scale_factor = self.vars.0.scale_factor.get();
        let screen_ppi = self.headless_monitor.ppi;
        let screen_size = self.headless_monitor.size.to_px(scale_factor.0);

        let (min_size, max_size, size) = self.content.outer_layout(ctx, scale_factor, screen_ppi, screen_size, |ctx| {
            let min_size = self.vars.min_size().get().layout(ctx.metrics, |_| default_min_size(scale_factor));

            let max_size = self.vars.max_size().get().layout(ctx.metrics, |_| screen_size);

            let size = self.vars.size().get().layout(ctx.metrics, |_| default_size(scale_factor));

            (min_size, max_size, size.min(max_size).max(min_size))
        });

        let size = self.content.layout(ctx, scale_factor, screen_ppi, min_size, max_size, size, false);
        let size = size.to_dip(scale_factor.0);

        if let Some(view) = &self.surface {
            // already has surface, maybe resize:
            if self.size != size {
                self.size = size;
                let _: Ignore = view.set_size(size, scale_factor);
            }
        } else if !self.waiting_view {
            // (re)spawn the view surface:

            if !Windows::req(ctx.services).try_load(ctx.vars, ctx.events, ctx.timers, self.window_id) {
                return;
            }

            let window_id = *ctx.window_id;
            let render_mode = self.render_mode.unwrap_or_else(|| Windows::req(ctx.services).default_render_mode);

            let r = ViewProcess::req(ctx.services).open_headless(HeadlessRequest {
                id: window_id.get(),
                scale_factor: scale_factor.0,
                size,
                render_mode,
            });

            match r {
                Ok(()) => self.waiting_view = true,
                Err(ViewProcessOffline) => {} // respawn
            }
        }

        self.headless_simulator.layout(ctx);
    }

    pub fn render(&mut self, ctx: &mut WindowContext) {
        if self.content.render_requested.is_none() {
            return;
        }

        if let Some(view) = &self.surface {
            let fct = self.vars.0.scale_factor.get();
            self.content.render(ctx, Some(view.renderer()), fct, None);
        }
    }

    pub fn focus(&mut self, ctx: &mut WindowContext) {
        self.headless_simulator.focus(ctx);
    }

    pub fn bring_to_top(&mut self, ctx: &mut WindowContext) {
        self.headless_simulator.bring_to_top(ctx);
    }

    pub fn close(&mut self, ctx: &mut WindowContext) {
        self.content.close(ctx);
        self.surface = None;
    }
}

fn update_headless_vars(vars: &Vars, windows: &mut Windows, mfactor: Option<Factor>, hvars: &WindowVars) -> VarHandles {
    let mut handles = VarHandles::dummy();

    if let Some(parent_vars) = hvars.parent().get().and_then(|id| windows.vars(id).ok()) {
        // bind parent factor
        if mfactor.is_none() {
            let h = hvars.0.scale_factor.bind(&parent_vars.0.scale_factor);
            handles.push(h);
        }

        // merge bind color scheme.
        let user = hvars.color_scheme();
        let parent = &parent_vars.0.actual_color_scheme;
        let actual = &hvars.0.actual_color_scheme;

        let h = user.hook(Box::new(clone_move!(parent, actual, |vars, _, value| {
            let value = *value.as_any().downcast_ref::<Option<ColorScheme>>().unwrap();
            let scheme = value.unwrap_or_else(|| parent.get());
            actual.set_ne(vars, scheme);
            true
        })));
        handles.push(h);

        let h = parent.hook(Box::new(clone_move!(user, actual, |vars, _, value| {
            let scheme = user.get().unwrap_or_else(|| *value.as_any().downcast_ref::<ColorScheme>().unwrap());
            actual.set_ne(vars, scheme);
            true
        })));
        handles.push(h);

        hvars.0.actual_color_scheme.set_ne(vars, user.get().unwrap_or_else(|| parent.get()));
    } else {
        // bind color scheme
        let h = hvars
            .color_scheme()
            .bind_map(&hvars.0.actual_color_scheme, |&s| s.unwrap_or_default());
        handles.push(h);

        hvars
            .0
            .actual_color_scheme
            .set_ne(vars, hvars.color_scheme().get().unwrap_or_default());
    }

    handles
}

/// implementer of `App` only content management.
struct HeadlessCtrl {
    vars: WindowVars,
    content: ContentCtrl,

    headless_monitor: HeadlessMonitor,
    headless_simulator: HeadlessSimulator,

    actual_parent: Option<WindowId>,
    /// actual_color_scheme and scale_factor binding.
    var_bindings: VarHandles,
}
impl HeadlessCtrl {
    pub fn new(window_id: WindowId, vars: &WindowVars, commands: WindowCommands, content: Window) -> Self {
        Self {
            vars: vars.clone(),
            headless_monitor: content.headless_monitor,
            content: ContentCtrl::new(window_id, vars.clone(), commands, content),
            headless_simulator: HeadlessSimulator::new(),
            actual_parent: None,
            var_bindings: VarHandles::dummy(),
        }
    }

    pub fn update(&mut self, ctx: &mut WindowContext, updates: &mut WidgetUpdates) {
        if self.vars.size().is_new(ctx)
            || self.vars.min_size().is_new(ctx)
            || self.vars.max_size().is_new(ctx)
            || self.vars.auto_size().is_new(ctx)
        {
            self.content.layout_requested = true;
            ctx.updates.layout();
        }

        if !self.content.inited {
            self.content.layout_requested = true;
            self.content.pending_render = WindowRenderUpdate::Render;

            ctx.updates.layout();
            ctx.updates.render();
        }

        if update_parent(ctx, &mut self.actual_parent, &self.vars) || self.var_bindings.is_dummy() {
            self.var_bindings = update_headless_vars(ctx.vars, Windows::req(ctx.services), self.headless_monitor.scale_factor, &self.vars);
        }

        self.content.update(ctx, updates);
    }

    pub fn window_updates(&mut self, ctx: &mut WindowContext, updates: InfoLayoutRenderUpdates) {
        self.content.window_updates(ctx, updates);
    }

    pub fn pre_event(&mut self, ctx: &mut WindowContext, update: &mut EventUpdate) {
        self.content.pre_event(ctx, update);
        self.headless_simulator.pre_event(ctx, update);
    }

    pub fn ui_event(&mut self, ctx: &mut WindowContext, update: &mut EventUpdate) {
        self.content.ui_event(ctx, update);
    }

    pub fn layout(&mut self, ctx: &mut WindowContext) {
        if !self.content.layout_requested {
            return;
        }

        if !Windows::req(ctx.services).try_load(ctx.vars, ctx.events, ctx.timers, *ctx.window_id) {
            return;
        }

        let scale_factor = self.vars.0.scale_factor.get();
        let screen_ppi = self.headless_monitor.ppi;
        let screen_size = self.headless_monitor.size.to_px(scale_factor.0);

        let (min_size, max_size, size) = self.content.outer_layout(ctx, scale_factor, screen_ppi, screen_size, |ctx| {
            let min_size = self.vars.min_size().get().layout(ctx.metrics, |_| default_min_size(scale_factor));

            let max_size = self.vars.max_size().get().layout(ctx.metrics, |_| screen_size);

            let size = self.vars.size().get().layout(ctx.metrics, |_| default_size(scale_factor));

            (min_size, max_size, size.min(max_size).max(min_size))
        });

        let _surface_size = self.content.layout(ctx, scale_factor, screen_ppi, min_size, max_size, size, false);

        self.headless_simulator.layout(ctx);
    }

    pub fn render(&mut self, ctx: &mut WindowContext) {
        if self.content.render_requested.is_none() {
            return;
        }
        let fct = self.vars.0.scale_factor.get();
        self.content.render(ctx, None, fct, None);
    }

    pub fn focus(&mut self, ctx: &mut WindowContext) {
        self.headless_simulator.focus(ctx);
    }

    pub fn bring_to_top(&mut self, ctx: &mut WindowContext) {
        self.headless_simulator.bring_to_top(ctx);
    }

    pub fn close(&mut self, ctx: &mut WindowContext) {
        self.content.close(ctx);
    }
}

/// Implementer of headless apps simulation of headed events for tests.
struct HeadlessSimulator {
    is_enabled: Option<bool>,
    is_open: bool,
}
impl HeadlessSimulator {
    fn new() -> Self {
        HeadlessSimulator {
            is_enabled: None,
            is_open: false,
        }
    }

    fn enabled(&mut self, ctx: &mut WindowContext) -> bool {
        *self
            .is_enabled
            .get_or_insert_with(|| crate::app::App::window_mode(ctx.services).is_headless())
    }

    pub fn pre_event(&mut self, ctx: &mut WindowContext, update: &mut EventUpdate) {
        if self.enabled(ctx) && self.is_open && VIEW_PROCESS_INITED_EVENT.on(update).map(|a| a.is_respawn).unwrap_or(false) {
            self.is_open = false;
        }
    }

    pub fn layout(&mut self, ctx: &mut WindowContext) {
        if self.enabled(ctx) && !self.is_open {
            self.is_open = true;
            self.focus(ctx);
        }
    }

    pub fn focus(&mut self, ctx: &mut WindowContext) {
        let mut prev = None;
        if let Some(id) = Windows::req(ctx.services).focused_window_id() {
            prev = Some(id);
        }
        let args = RawWindowFocusArgs::now(prev, Some(*ctx.window_id));
        RAW_WINDOW_FOCUS_EVENT.notify(ctx.events, args);
    }

    pub fn bring_to_top(&mut self, _: &mut WindowContext) {
        // we don't have "bring-to-top" event.
    }
}

/// Implementer of window UI node tree initialization and management.
struct ContentCtrl {
    vars: WindowVars,
    commands: WindowCommands,

    root_id: WidgetId,
    root_state: OwnedStateMap<state_map::Widget>,
    root: BoxedUiNode,
    // info
    info_tree: WidgetInfoTree,
    root_info: WidgetContextInfo,
    used_info_builder: Option<UsedWidgetInfoBuilder>,
    root_var_handles: VarHandles,
    root_event_handles: EventHandles,
    layout_pass: LayoutPassId,

    used_frame_builder: Option<UsedFrameBuilder>,
    used_frame_update: Option<UsedFrameUpdate>,

    inited: bool,
    frame_id: FrameId,
    clear_color: RenderColor,

    is_rendering: bool,
    pending_render: WindowRenderUpdate,

    layout_requested: bool,
    render_requested: WindowRenderUpdate,
}
impl ContentCtrl {
    pub fn new(window_id: WindowId, vars: WindowVars, commands: WindowCommands, window: Window) -> Self {
        Self {
            vars,
            commands,

            root_id: window.id,
            root_state: OwnedStateMap::new(),
            root: window.child,

            info_tree: WidgetInfoTree::blank(window_id, window.id),
            root_info: WidgetContextInfo::new(),
            used_info_builder: None,
            root_var_handles: VarHandles::default(),
            root_event_handles: EventHandles::default(),
            layout_pass: 0,

            used_frame_builder: None,
            used_frame_update: None,

            inited: false,
            frame_id: FrameId::INVALID,
            clear_color: RenderColor::BLACK,

            is_rendering: false,
            pending_render: WindowRenderUpdate::None,

            layout_requested: false,
            render_requested: WindowRenderUpdate::None,
        }
    }

    pub fn update(&mut self, ctx: &mut WindowContext, updates: &mut WidgetUpdates) {
        if !self.inited {
            self.commands.init(&self.vars);
            ctx.widget_context(
                &self.info_tree,
                &self.root_info,
                &mut self.root_state,
                &mut self.root_var_handles,
                &mut self.root_event_handles,
                |ctx| {
                    self.root.init(ctx);

                    ctx.updates.info();
                },
            );
            self.inited = true;
        } else {
            self.commands.update(ctx.vars, &self.vars);

            updates.with_window(ctx, |ctx, updates| {
                ctx.widget_context(
                    &self.info_tree,
                    &self.root_info,
                    &mut self.root_state,
                    &mut self.root_var_handles,
                    &mut self.root_event_handles,
                    |ctx| {
                        updates.with_widget(ctx, |ctx, updates| {
                            self.root.update(ctx, updates);
                        });
                    },
                );
            });
        }
    }

    pub fn window_updates(&mut self, ctx: &mut WindowContext, updates: InfoLayoutRenderUpdates) {
        self.layout_requested |= updates.layout;
        self.render_requested |= updates.render;

        if updates.info {
            let mut info = WidgetInfoBuilder::new(
                *ctx.window_id,
                self.root_id,
                self.root_info.bounds.clone(),
                self.root_info.border.clone(),
                self.vars.0.scale_factor.get(),
                self.used_info_builder.take(),
            );

            ctx.info_context(&self.info_tree, &self.root_info, &self.root_state, |ctx| {
                self.root.info(ctx, &mut info);
            });

            let (info, used) = info.finalize();
            self.info_tree = info.clone();
            self.used_info_builder = Some(used);

            Windows::req(ctx.services).set_widget_tree(ctx.events, info, self.layout_requested, !self.render_requested.is_none());
        }
    }

    pub fn pre_event(&mut self, ctx: &mut WindowContext, update: &mut EventUpdate) {
        if let Some(args) = RAW_FRAME_RENDERED_EVENT.on(update) {
            if args.window_id == *ctx.window_id {
                self.is_rendering = false;
                match self.pending_render.take() {
                    WindowRenderUpdate::None => {}
                    WindowRenderUpdate::Render => {
                        self.render_requested = WindowRenderUpdate::Render;
                        ctx.updates.render();
                    }
                    WindowRenderUpdate::RenderUpdate => {
                        self.render_requested |= WindowRenderUpdate::RenderUpdate;
                        ctx.updates.render_update();
                    }
                }

                let image = args.frame_image.as_ref().cloned().map(Image::new);

                let args = FrameImageReadyArgs::new(args.timestamp, args.propagation().clone(), args.window_id, args.frame_id, image);
                FRAME_IMAGE_READY_EVENT.notify(ctx.events, args);
            }
        } else {
            self.commands.event(ctx, &self.vars, update);
        }
    }

    pub fn ui_event(&mut self, ctx: &mut WindowContext, update: &mut EventUpdate) {
        debug_assert!(self.inited);

        update.with_window(ctx, |ctx, update| {
            ctx.widget_context(
                &self.info_tree,
                &self.root_info,
                &mut self.root_state,
                &mut self.root_var_handles,
                &mut self.root_event_handles,
                |ctx| {
                    update.with_widget(ctx, |ctx, update| {
                        self.root.event(ctx, update);
                    });
                },
            );
        });
    }

    pub fn close(&mut self, ctx: &mut WindowContext) {
        ctx.widget_context(
            &self.info_tree,
            &self.root_info,
            &mut self.root_state,
            &mut self.root_var_handles,
            &mut self.root_event_handles,
            |ctx| {
                self.root.deinit(ctx);
            },
        );

        self.vars.0.is_open.set(ctx, false);
        self.root_var_handles.clear();
        self.root_event_handles.clear();
    }

    /// Run an `action` in the context of a monitor screen that is parent of this content.
    pub fn outer_layout<R>(
        &mut self,
        ctx: &mut WindowContext,
        scale_factor: Factor,
        screen_ppi: f32,
        screen_size: PxSize,
        action: impl FnOnce(&mut LayoutContext) -> R,
    ) -> R {
        ctx.layout_context(
            base_font_size(scale_factor),
            scale_factor,
            screen_ppi,
            screen_size,
            &self.info_tree,
            &self.root_info,
            &mut self.root_state,
            action,
        )
    }

    /// Layout content if there was a pending request, returns `Some(final_size)`.
    #[allow(clippy::too_many_arguments)]
    pub fn layout(
        &mut self,
        ctx: &mut WindowContext,
        scale_factor: Factor,
        screen_ppi: f32,
        min_size: PxSize,
        max_size: PxSize,
        size: PxSize,
        skip_auto_size: bool,
    ) -> PxSize {
        debug_assert!(self.inited);
        debug_assert!(self.layout_requested);

        let _s = tracing::trace_span!("window.on_layout", window = %ctx.window_id.sequential()).entered();

        self.layout_requested = false;

        let base_font_size = base_font_size(scale_factor);

        let auto_size = self.vars.auto_size().get();

        let mut viewport_size = size;
        if !skip_auto_size {
            if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                viewport_size.width = max_size.width;
            }
            if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                viewport_size.height = max_size.height;
            }
        }

        self.layout_pass += 1;

        ctx.layout_context(
            base_font_size,
            scale_factor,
            screen_ppi,
            viewport_size,
            &self.info_tree,
            &self.root_info,
            &mut self.root_state,
            |ctx| {
                let desired_size = ctx.with_constrains(
                    |mut c| {
                        if !skip_auto_size {
                            if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                                c = c.with_unbounded_x();
                            }
                            if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                                c = c.with_unbounded_y();
                            }
                        }
                        c
                    },
                    |ctx| WidgetLayout::with_root_widget(ctx, self.layout_pass, |ctx, wl| self.root.layout(ctx, wl)),
                );

                let mut final_size = viewport_size;
                if !skip_auto_size {
                    if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                        final_size.width = desired_size.width.max(min_size.width).min(max_size.width);
                    }
                    if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                        final_size.height = desired_size.height.max(min_size.height).min(max_size.height);
                    }
                }

                final_size
            },
        )
    }

    pub fn render(&mut self, ctx: &mut WindowContext, renderer: Option<ViewRenderer>, scale_factor: Factor, wait_id: Option<FrameWaitId>) {
        match mem::take(&mut self.render_requested) {
            // RENDER FULL FRAME
            WindowRenderUpdate::Render => {
                if self.is_rendering {
                    self.pending_render = WindowRenderUpdate::Render;
                    return;
                }

                let _s = tracing::trace_span!("window.on_render", window = %ctx.window_id.sequential()).entered();

                self.frame_id = self.frame_id.next();

                let default_text_aa = ctx.services.get::<Fonts>().map(|f| f.system_font_aa().get()).unwrap_or_default();

                let mut frame = FrameBuilder::new(
                    self.frame_id,
                    self.root_id,
                    &self.root_info.bounds,
                    &self.info_tree,
                    renderer.clone(),
                    scale_factor,
                    default_text_aa,
                    self.used_frame_builder.take(),
                );

                let (frame, used) = ctx.render_context(self.root_id, &self.root_state, &self.info_tree, &self.root_info, |ctx| {
                    self.root.render(ctx, &mut frame);
                    frame.finalize(ctx.info_tree)
                });

                self.used_frame_builder = Some(used);

                self.clear_color = frame.clear_color;

                let capture_image = self.take_capture_image(ctx.vars);

                if let Some(renderer) = renderer {
                    let _: Ignore = renderer.render(FrameRequest {
                        id: self.frame_id,
                        pipeline_id: frame.display_list.pipeline_id(),
                        clear_color: self.clear_color,
                        display_list: frame.display_list,
                        capture_image,
                        wait_id,
                    });

                    self.is_rendering = true;
                }
            }

            // RENDER UPDATE
            WindowRenderUpdate::RenderUpdate => {
                if self.is_rendering {
                    self.pending_render |= WindowRenderUpdate::RenderUpdate;
                    return;
                }

                let _s = tracing::trace_span!("window.on_render_update", window = %ctx.window_id.sequential()).entered();

                self.frame_id = self.frame_id.next_update();

                let mut update = FrameUpdate::new(
                    self.frame_id,
                    self.root_id,
                    self.root_info.bounds.clone(),
                    renderer.as_ref(),
                    self.clear_color,
                    self.used_frame_update.take(),
                );

                let (update, used) = ctx.render_context(self.root_id, &self.root_state, &self.info_tree, &self.root_info, |ctx| {
                    self.root.render_update(ctx, &mut update);
                    update.finalize(ctx.info_tree)
                });

                self.used_frame_update = Some(used);

                if let Some(c) = update.clear_color {
                    self.clear_color = c;
                }

                let capture_image = self.take_capture_image(ctx.vars);

                if let Some(renderer) = renderer {
                    let _: Ignore = renderer.render_update(FrameUpdateRequest {
                        id: self.frame_id,
                        transforms: update.transforms,
                        floats: update.floats,
                        colors: update.colors,
                        clear_color: update.clear_color,
                        capture_image,
                        wait_id,
                    });

                    self.is_rendering = true;
                }
            }
            WindowRenderUpdate::None => {
                debug_assert!(false, "self.render_requested != WindowRenderUpdate::None")
            }
        }
    }
    fn take_capture_image(&self, vars: &Vars) -> bool {
        match self.vars.frame_capture_mode().get() {
            FrameCaptureMode::Sporadic => false,
            FrameCaptureMode::Next => {
                self.vars.frame_capture_mode().set(vars, FrameCaptureMode::Sporadic);
                true
            }
            FrameCaptureMode::All => true,
        }
    }
}

/// Management of window content and synchronization of WindowVars and View-Process.
pub(super) struct WindowCtrl(WindowCtrlMode);
enum WindowCtrlMode {
    Headed(HeadedCtrl),
    Headless(HeadlessCtrl),
    HeadlessWithRenderer(HeadlessWithRendererCtrl),
}
impl WindowCtrl {
    pub fn new(window_id: WindowId, vars: &WindowVars, commands: WindowCommands, mode: WindowMode, content: Window) -> Self {
        WindowCtrl(match mode {
            WindowMode::Headed => WindowCtrlMode::Headed(HeadedCtrl::new(window_id, vars, commands, content)),
            WindowMode::Headless => WindowCtrlMode::Headless(HeadlessCtrl::new(window_id, vars, commands, content)),
            WindowMode::HeadlessWithRenderer => {
                WindowCtrlMode::HeadlessWithRenderer(HeadlessWithRendererCtrl::new(window_id, vars, commands, content))
            }
        })
    }

    pub fn update(&mut self, ctx: &mut WindowContext, updates: &mut WidgetUpdates) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.update(ctx, updates),
            WindowCtrlMode::Headless(c) => c.update(ctx, updates),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.update(ctx, updates),
        }
    }

    pub fn window_updates(&mut self, ctx: &mut WindowContext, updates: InfoLayoutRenderUpdates) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.window_updates(ctx, updates),
            WindowCtrlMode::Headless(c) => c.window_updates(ctx, updates),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.window_updates(ctx, updates),
        }
    }

    pub fn pre_event(&mut self, ctx: &mut WindowContext, update: &mut EventUpdate) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.pre_event(ctx, update),
            WindowCtrlMode::Headless(c) => c.pre_event(ctx, update),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.pre_event(ctx, update),
        }
    }

    pub fn ui_event(&mut self, ctx: &mut WindowContext, update: &mut EventUpdate) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.ui_event(ctx, update),
            WindowCtrlMode::Headless(c) => c.ui_event(ctx, update),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.ui_event(ctx, update),
        }
    }

    pub fn layout(&mut self, ctx: &mut WindowContext) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.layout(ctx),
            WindowCtrlMode::Headless(c) => c.layout(ctx),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.layout(ctx),
        }
    }

    pub fn render(&mut self, ctx: &mut WindowContext) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.render(ctx),
            WindowCtrlMode::Headless(c) => c.render(ctx),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.render(ctx),
        }
    }

    pub fn focus(&mut self, ctx: &mut WindowContext) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.focus(ctx),
            WindowCtrlMode::Headless(c) => c.focus(ctx),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.focus(ctx),
        }
    }

    pub fn bring_to_top(&mut self, ctx: &mut WindowContext) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.bring_to_top(ctx),
            WindowCtrlMode::Headless(c) => c.bring_to_top(ctx),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.bring_to_top(ctx),
        }
    }

    pub fn close(&mut self, ctx: &mut WindowContext) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.close(ctx),
            WindowCtrlMode::Headless(c) => c.close(ctx),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.close(ctx),
        }
    }
}

fn base_font_size(scale_factor: Factor) -> Px {
    Length::pt_to_px(13.0, scale_factor)
}

fn default_min_size(scale_factor: Factor) -> PxSize {
    DipSize::new(Dip::new(192), Dip::new(48)).to_px(scale_factor.0)
}

fn default_size(scale_factor: Factor) -> PxSize {
    DipSize::new(Dip::new(800), Dip::new(600)).to_px(scale_factor.0)
}

/// Respawned error is ok here, because we recreate the window/surface on respawn.
type Ignore = Result<(), ViewProcessOffline>;
