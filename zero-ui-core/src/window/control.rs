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
    context::{LayoutMetrics, WidgetCtx, WidgetUpdates, LAYOUT, UPDATES, WIDGET, WINDOW},
    crate_util::{IdEntry, IdMap},
    event::{AnyEventArgs, EventUpdate},
    image::{Image, ImageVar, IMAGES},
    render::{FrameBuilder, FrameId, FrameUpdate, UsedFrameBuilder, UsedFrameUpdate},
    text::FONTS,
    timer::TIMERS,
    units::*,
    var::*,
    widget_info::{LayoutPassId, UsedWidgetInfoBuilder, WidgetInfoBuilder, WidgetInfoTree, WidgetLayout},
    widget_instance::{BoxedUiNode, UiNode, WidgetId},
    window::AutoSize,
};

use super::{
    commands::{WindowCommands, MINIMIZE_CMD, RESTORE_CMD},
    FrameCaptureMode, FrameImageReadyArgs, HeadlessMonitor, MonitorInfo, StartPosition, TransformChangedArgs, Window, WindowChangedArgs,
    WindowChrome, WindowIcon, WindowId, WindowMode, WindowVars, FRAME_IMAGE_READY_EVENT, MONITORS_CHANGED_EVENT, TRANSFORM_CHANGED_EVENT,
    WINDOWS, WINDOW_CHANGED_EVENT,
};

/// Implementer of `App <-> View` sync in a headed window.
struct HeadedCtrl {
    window: Option<ViewWindow>,
    waiting_view: bool,
    delayed_view_updates: Vec<Box<dyn FnOnce(&ViewWindow) + Send>>,
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
    parent_color_scheme: Option<ReadOnlyArcVar<ColorScheme>>,
    actual_parent: Option<WindowId>,
    root_font_size: Dip,
}
impl HeadedCtrl {
    pub fn new(vars: &WindowVars, commands: WindowCommands, content: Window) -> Self {
        Self {
            window: None,
            waiting_view: false,
            delayed_view_updates: vec![],

            start_position: content.start_position,
            start_focused: content.start_focused,
            kiosk: if content.kiosk { Some(WindowState::Fullscreen) } else { None },
            transparent: content.transparent,
            render_mode: content.render_mode,

            content: ContentCtrl::new(vars.clone(), commands, content),
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
            root_font_size: Dip::from_px(Length::pt_to_px(11.0, 1.fct()), 1.0),
        }
    }

    fn update_gen(&mut self, update: impl FnOnce(&ViewWindow) + Send + 'static) {
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

    pub fn update(&mut self, updates: &WidgetUpdates) {
        if self.window.is_none() && !self.waiting_view {
            // we request a view on the first layout.
            UPDATES.layout();

            if let Some(enforced_fullscreen) = self.kiosk {
                // enforce kiosk in pre-init.

                if !self.vars.state().get().is_fullscreen() {
                    self.vars.state().set(enforced_fullscreen);
                }
            }
        }

        if let Some(enforced_fullscreen) = &mut self.kiosk {
            // always fullscreen, but can be windowed or exclusive.

            if let Some(state) = self.vars.state().get_new() {
                if !state.is_fullscreen() {
                    tracing::error!("window in `kiosk` mode can only be fullscreen");

                    self.vars.state().set(*enforced_fullscreen);
                } else {
                    *enforced_fullscreen = state;
                }
            }

            if let Some(false) = self.vars.visible().get_new() {
                tracing::error!("window in `kiosk` mode can not be hidden");

                self.vars.visible().set(true);
            }

            if let Some(mode) = self.vars.chrome().get_new() {
                if !mode.is_none() {
                    tracing::error!("window in `kiosk` mode can not show chrome");
                    self.vars.chrome().set(WindowChrome::None);
                }
            }
        } else {
            // not kiosk mode.

            if let Some(prev_state) = self.state.clone() {
                debug_assert!(self.window.is_some() || self.waiting_view || self.respawned);

                let mut new_state = prev_state.clone();

                if let Some(query) = self.vars.monitor().get_new() {
                    if self.monitor.is_none() {
                        let monitor = query.select_fallback();
                        let scale_factor = monitor.scale_factor().get();
                        self.vars.0.scale_factor.set_ne(scale_factor);
                        self.monitor = Some(monitor);
                    } else if let Some(new) = query.select() {
                        let current = self.vars.0.actual_monitor.get();
                        if Some(new.id()) != current {
                            let scale_factor = new.scale_factor().get();
                            self.vars.0.scale_factor.set_ne(scale_factor);
                            self.vars.0.actual_monitor.set_ne(new.id());
                            self.monitor = Some(new);
                        }
                    }
                }

                if let Some(chrome) = self.vars.chrome().get_new() {
                    new_state.chrome_visible = chrome.is_default();
                }

                if let Some(req_state) = self.vars.state().get_new() {
                    new_state.set_state(req_state);
                    self.vars.0.restore_state.set_ne(new_state.restore_state);
                }

                if self.vars.min_size().is_new() || self.vars.max_size().is_new() {
                    if let Some(m) = &self.monitor {
                        let scale_factor = m.scale_factor().get();
                        let screen_ppi = m.ppi().get();
                        let screen_size = m.size().get();
                        let (min_size, max_size) = self.content.outer_layout(scale_factor, screen_ppi, screen_size, || {
                            let min_size = self.vars.min_size().layout_dft(default_min_size(scale_factor));
                            let max_size = self.vars.max_size().layout_dft(screen_size);

                            (min_size.to_dip(scale_factor.0), max_size.to_dip(scale_factor.0))
                        });

                        let size = new_state.restore_rect.size;

                        new_state.restore_rect.size = size.min(max_size).max(min_size);
                        new_state.min_size = min_size;
                        new_state.max_size = max_size;
                    }
                }

                if let Some(auto) = self.vars.auto_size().get_new() {
                    if auto != AutoSize::DISABLED {
                        self.content.layout_requested = true;
                        UPDATES.layout();
                    }
                }

                if self.vars.size().is_new() {
                    let auto_size = self.vars.auto_size().get();

                    if auto_size != AutoSize::CONTENT {
                        if let Some(m) = &self.monitor {
                            let scale_factor = m.scale_factor().get();
                            let screen_ppi = m.ppi().get();
                            let screen_size = m.size().get();
                            let size = self.content.outer_layout(scale_factor, screen_ppi, screen_size, || {
                                self.vars.size().layout_dft(default_size(scale_factor)).to_dip(scale_factor.0)
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

                if let Some(font_size) = self.vars.font_size().get_new() {
                    if let Some(m) = &self.monitor {
                        let scale_factor = m.scale_factor().get();
                        let screen_ppi = m.ppi().get();
                        let screen_size = m.size().get();
                        let mut font_size_px = self.content.outer_layout(scale_factor, screen_ppi, screen_size, || {
                            font_size.layout_dft_x(Length::pt_to_px(11.0, scale_factor))
                        });
                        if font_size_px < Px(0) {
                            tracing::error!("invalid font size {font_size:?} => {font_size_px:?}");
                            font_size_px = Length::pt_to_px(11.0, scale_factor);
                        }
                        let font_size_dip = font_size_px.to_dip(scale_factor.0);

                        if font_size_dip != self.root_font_size {
                            self.root_font_size = font_size_dip;
                            self.content.layout_requested = true;
                            UPDATES.layout();
                        }
                    }
                }

                if let Some(pos) = self.vars.position().get_new() {
                    if let Some(m) = &self.monitor {
                        let scale_factor = m.scale_factor().get();
                        let screen_ppi = m.ppi().get();
                        let screen_size = m.size().get();
                        let pos = self.content.outer_layout(scale_factor, screen_ppi, screen_size, || {
                            pos.layout_dft(PxPoint::new(Px(50), Px(50)))
                        });
                        new_state.restore_rect.origin = pos.to_dip(scale_factor.0);
                    }
                }

                if let Some(visible) = self.vars.visible().get_new() {
                    self.update_gen(move |view| {
                        let _: Ignore = view.set_visible(visible);
                    });
                }

                if let Some(movable) = self.vars.movable().get_new() {
                    self.update_gen(move |view| {
                        let _: Ignore = view.set_movable(movable);
                    });
                }

                if let Some(resizable) = self.vars.resizable().get_new() {
                    self.update_gen(move |view| {
                        let _: Ignore = view.set_resizable(resizable);
                    });
                }

                if prev_state != new_state {
                    self.update_gen(move |view| {
                        let _: Ignore = view.set_state(new_state);
                    })
                }
            }

            // icon:
            let mut send_icon = false;
            if let Some(ico) = self.vars.icon().get_new() {
                use crate::image::ImageSource;

                self.icon = match ico {
                    WindowIcon::Default => None,
                    WindowIcon::Image(ImageSource::Render(ico, _)) => Some(IMAGES.cache(ImageSource::Render(
                        ico.clone(),
                        Some(crate::image::ImageRenderArgs { parent: Some(WINDOW.id()) }),
                    ))),
                    WindowIcon::Image(source) => Some(IMAGES.cache(source)),
                };

                if let Some(ico) = &self.icon {
                    self.icon_binding = ico.bind_map(&self.vars.0.actual_icon, |img| Some(img.clone()));

                    if ico.get().is_loading() && self.window.is_none() && !self.waiting_view {
                        if self.icon_deadline.has_elapsed() {
                            UPDATES.layout();
                        } else {
                            TIMERS
                                .on_deadline(
                                    self.icon_deadline,
                                    app_hn_once!(ico, |_| {
                                        if ico.get().is_loading() {
                                            UPDATES.layout();
                                        }
                                    }),
                                )
                                .perm();
                        }
                    }
                } else {
                    self.vars.0.actual_icon.set_ne(None);
                    self.icon_binding = VarHandle::dummy();
                }

                send_icon = true;
            } else if self.icon.as_ref().map(|ico| ico.is_new()).unwrap_or(false) {
                send_icon = true;
            }
            if send_icon {
                let icon = self.icon.as_ref().and_then(|ico| ico.get().view().cloned());
                self.update_gen(move |view| {
                    let _: Ignore = view.set_icon(icon.as_ref());
                });
            }

            if let Some(title) = self.vars.title().get_new() {
                self.update_gen(move |view| {
                    let _: Ignore = view.set_title(title.into_owned());
                });
            }

            if let Some(mode) = self.vars.video_mode().get_new() {
                self.update_gen(move |view| {
                    let _: Ignore = view.set_video_mode(mode);
                });
            }

            if let Some(cursor) = self.vars.cursor().get_new() {
                self.update_gen(move |view| {
                    let _: Ignore = view.set_cursor(cursor);
                });
            }

            if let Some(visible) = self.vars.taskbar_visible().get_new() {
                self.update_gen(move |view| {
                    let _: Ignore = view.set_taskbar_visible(visible);
                });
            }

            if let Some(top) = self.vars.always_on_top().get_new() {
                self.update_gen(move |view| {
                    let _: Ignore = view.set_always_on_top(top);
                });
            }

            if let Some(mode) = self.vars.frame_capture_mode().get_new() {
                self.update_gen(move |view| {
                    let _: Ignore = view.set_capture_mode(matches!(mode, FrameCaptureMode::All));
                });
            }

            if let Some(m) = &self.monitor {
                if let Some(fct) = m.scale_factor().get_new() {
                    self.vars.0.scale_factor.set_ne(fct);
                }
                if m.scale_factor().is_new() || m.size().is_new() || m.ppi().is_new() {
                    self.content.layout_requested = true;
                    UPDATES.layout();
                }
            }

            if let Some(indicator) = self.vars.focus_indicator().get_new() {
                if WINDOWS.is_focused(WINDOW.id()).unwrap_or(false) {
                    self.vars.focus_indicator().set_ne(None);
                } else if let Some(view) = &self.window {
                    let _ = view.set_focus_indicator(indicator);
                    // will be set to `None` once the window is focused.
                }
                // else indicator is send with init.
            }

            let mut update_color_scheme = false;

            if update_parent(&mut self.actual_parent, &self.vars) {
                self.parent_color_scheme = self
                    .actual_parent
                    .and_then(|id| WINDOWS.vars(id).ok().map(|v| v.actual_color_scheme()));
                update_color_scheme = true;
            }

            if update_color_scheme
                || self.vars.color_scheme().is_new()
                || self.parent_color_scheme.as_ref().map(|t| t.is_new()).unwrap_or(false)
            {
                let scheme = self
                    .vars
                    .color_scheme()
                    .get()
                    .or_else(|| self.parent_color_scheme.as_ref().map(|t| t.get()))
                    .or(self.system_color_scheme)
                    .unwrap_or_default();
                self.vars.0.actual_color_scheme.set_ne(scheme);
            }

            if let Some(dbg) = self.vars.renderer_debug().get_new() {
                if let Some(view) = &self.window {
                    let _ = view.renderer().set_debug(dbg);
                }
            }
        }

        self.content.update(updates);
    }

    #[must_use]
    pub fn window_updates(&mut self) -> Option<WidgetInfoTree> {
        self.content.window_updates()
    }

    pub fn pre_event(&mut self, update: &EventUpdate) {
        if let Some(args) = RAW_WINDOW_CHANGED_EVENT.on(update) {
            if args.window_id == WINDOW.id() {
                let mut state_change = None;
                let mut pos_change = None;
                let mut size_change = None;

                if let Some((monitor, _)) = args.monitor {
                    if self.vars.0.actual_monitor.get().map(|m| m != monitor).unwrap_or(true) {
                        self.vars.0.actual_monitor.set_ne(Some(monitor));
                        self.monitor = None;
                        self.content.layout_requested = true;
                        UPDATES.layout();
                    }
                }

                if let Some(state) = args.state.clone() {
                    self.vars.state().set_ne(state.state);
                    self.vars.0.restore_rect.set_ne(state.restore_rect);
                    self.vars.0.restore_state.set_ne(state.restore_state);

                    let new_state = state.state;
                    if self.actual_state != Some(new_state) {
                        let prev_state = self.actual_state.unwrap_or(WindowState::Normal);
                        state_change = Some((prev_state, new_state));
                        self.actual_state = Some(new_state);

                        match (prev_state, new_state) {
                            (_, WindowState::Minimized) => {
                                // minimized, minimize children.
                                self.vars.0.children.with(|c| {
                                    for &c in c.iter() {
                                        MINIMIZE_CMD.scoped(c).notify();
                                    }
                                });
                            }
                            (WindowState::Minimized, _) => {
                                // restored, restore children.
                                self.vars.0.children.with(|c| {
                                    for &c in c.iter() {
                                        RESTORE_CMD.scoped(c).notify();
                                    }
                                });

                                // we skip layout & render when minimized.
                                if self.content.layout_requested {
                                    UPDATES.layout();
                                }
                                if !matches!(self.content.render_requested, RenderUpdate::None) {
                                    UPDATES.render();
                                }
                            }
                            _ => {}
                        }
                    }

                    self.state = Some(state);
                }

                if let Some(pos) = args.position {
                    if self.vars.0.actual_position.get() != pos {
                        self.vars.0.actual_position.set_ne(pos);
                        pos_change = Some(pos);
                    }
                }

                if let Some(size) = args.size {
                    if self.vars.0.actual_size.get() != size {
                        self.vars.0.actual_size.set_ne(size);
                        size_change = Some(size);

                        self.content.layout_requested = true;
                        UPDATES.layout();

                        if args.cause == EventCause::System {
                            // resize by system (user)
                            self.vars.auto_size().set_ne(AutoSize::DISABLED);
                        }
                    }
                }

                if let Some(id) = args.frame_wait_id {
                    self.resize_wait_id = Some(id);

                    if !matches!(self.content.pending_render, RenderUpdate::Render) {
                        self.content.pending_render = RenderUpdate::RenderUpdate;
                    }
                    self.content.render_requested = mem::replace(&mut self.content.pending_render, RenderUpdate::None);
                    UPDATES.render();
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
                    WINDOW_CHANGED_EVENT.notify(args);
                }
            } else if self.actual_state.unwrap_or(WindowState::Normal) == WindowState::Minimized
                && args.state.as_ref().map(|s| s.state != WindowState::Minimized).unwrap_or(false)
                && self.vars.0.children.with(|c| c.contains(&args.window_id))
            {
                // child restored.
                RESTORE_CMD.scoped(WINDOW.id()).notify();
            }
        } else if let Some(args) = RAW_WINDOW_FOCUS_EVENT.on(update) {
            if args.new_focus == Some(WINDOW.id()) {
                self.vars.0.children.with(|c| {
                    for &c in c.iter() {
                        let _ = WINDOWS.bring_to_top(c);
                    }
                });
            } else if let Some(new_focus) = args.new_focus {
                self.vars.0.children.with(|c| {
                    if c.contains(&new_focus) {
                        let _ = WINDOWS.bring_to_top(WINDOW.id());

                        for c in c.iter() {
                            if *c != new_focus {
                                let _ = WINDOWS.bring_to_top(WINDOW.id());
                            }
                        }

                        let _ = WINDOWS.bring_to_top(new_focus);
                    }
                });
            }
        } else if let Some(args) = MONITORS_CHANGED_EVENT.on(update) {
            if let Some(m) = &self.monitor {
                if args.removed.contains(&m.id()) {
                    self.monitor = None;
                    self.vars.0.actual_monitor.set_ne(None);
                }
            }
            self.vars.monitor().touch();
        } else if let Some(args) = RAW_WINDOW_OPEN_EVENT.on(update) {
            if args.window_id == WINDOW.id() {
                self.waiting_view = false;

                WINDOWS.set_renderer(WINDOW.id(), args.window.renderer());

                self.window = Some(args.window.clone());
                self.vars.0.render_mode.set_ne(args.data.render_mode);
                self.vars.state().set_ne(args.data.state.state);
                self.actual_state = Some(args.data.state.state);
                self.vars.0.restore_state.set_ne(args.data.state.restore_state);
                self.vars.0.restore_rect.set_ne(args.data.state.restore_rect);
                self.vars.0.actual_position.set_ne(args.data.position);
                self.vars.0.actual_size.set_ne(args.data.size);
                self.vars.0.actual_monitor.set_ne(args.data.monitor);
                self.vars.0.scale_factor.set_ne(args.data.scale_factor);

                self.state = Some(args.data.state.clone());
                self.system_color_scheme = Some(args.data.color_scheme);

                let scheme = self
                    .vars
                    .color_scheme()
                    .get()
                    .or_else(|| self.parent_color_scheme.as_ref().map(|t| t.get()))
                    .or(self.system_color_scheme)
                    .unwrap_or_default();
                self.vars.0.actual_color_scheme.set_ne(scheme);

                UPDATES.layout().render();

                for update in mem::take(&mut self.delayed_view_updates) {
                    update(&args.window);
                }
            }
        } else if let Some(args) = RAW_COLOR_SCHEME_CHANGED_EVENT.on(update) {
            if args.window_id == WINDOW.id() {
                self.system_color_scheme = Some(args.color_scheme);

                let scheme = self
                    .vars
                    .color_scheme()
                    .get()
                    .or_else(|| self.parent_color_scheme.as_ref().map(|t| t.get()))
                    .or(self.system_color_scheme)
                    .unwrap_or_default();
                self.vars.0.actual_color_scheme.set_ne(scheme);
            }
        } else if let Some(args) = RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT.on(update) {
            if args.window_id == WINDOW.id() && self.window.is_none() && self.waiting_view {
                tracing::error!("view-process failed to open a window, {}", args.error);

                // was waiting view and failed, treat like a respawn.

                self.waiting_view = false;
                self.delayed_view_updates = vec![];
                self.respawned = true;

                self.content.layout_requested = true;
                self.content.render_requested = RenderUpdate::Render;
                self.content.is_rendering = false;

                UPDATES.layout().render();
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
                    self.content.render_requested = RenderUpdate::Render;
                    self.content.is_rendering = false;

                    UPDATES.layout().render();
                }
            }
        }

        self.content.pre_event(update);
    }

    pub fn ui_event(&mut self, update: &EventUpdate) {
        self.content.ui_event(update);
    }

    pub fn layout(&mut self) {
        if !self.content.layout_requested {
            return;
        }

        if self.window.is_some() {
            if matches!(self.state.as_ref().map(|s| s.state), Some(WindowState::Minimized)) {
                return;
            }
            self.layout_update();
        } else if self.respawned && !self.waiting_view {
            self.layout_respawn();
        } else if !self.waiting_view {
            self.layout_init();
        }
    }

    /// First layout, opens the window.
    fn layout_init(&mut self) {
        self.monitor = Some(self.vars.monitor().get().select_fallback());

        // await icon load for up to 1s.
        if let Some(icon) = &self.icon {
            if !self.icon_deadline.has_elapsed() && icon.get().is_loading() {
                // block on icon loading.
                return;
            }
        }
        // update window "load" state, `is_loaded` and the `WindowLoadEvent` happen here.
        if !WINDOWS.try_load(WINDOW.id()) {
            // block on loading handles.
            return;
        }

        let m = self.monitor.as_ref().unwrap();
        let scale_factor = m.scale_factor().get();
        let screen_ppi = m.ppi().get();
        let screen_rect = m.px_rect();

        // Layout min, max and size in the monitor space.
        let (min_size, max_size, mut size, root_font_size) = self.content.outer_layout(scale_factor, screen_ppi, screen_rect.size, || {
            let min_size = self.vars.min_size().layout_dft(default_min_size(scale_factor));
            let max_size = self.vars.max_size().layout_dft(screen_rect.size);
            let size = self.vars.size().layout_dft(default_size(scale_factor));

            let font_size = self.vars.font_size().get();
            let mut root_font_size = font_size.layout_dft_x(Length::pt_to_px(11.0, scale_factor));
            if root_font_size < Px(0) {
                tracing::error!("invalid font size {font_size:?} => {root_font_size:?}");
                root_font_size = Length::pt_to_px(11.0, scale_factor);
            }

            (min_size, max_size, size.min(max_size).max(min_size), root_font_size)
        });

        self.root_font_size = root_font_size.to_dip(scale_factor.0);

        let state = self.vars.state().get();
        if state == WindowState::Normal && self.vars.auto_size().get() != AutoSize::DISABLED {
            // layout content to get auto-size size.
            size = self
                .content
                .layout(scale_factor, screen_ppi, min_size, max_size, size, root_font_size, false);
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
                    self.content.outer_layout(scale_factor, screen_ppi, screen_rect.size, || {
                        pos.layout() + screen_rect.origin.to_vector()
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
                    if let Ok(w) = WINDOWS.vars(parent) {
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
            id: WINDOW.id().get(),
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
            render_mode: self.render_mode.unwrap_or_else(|| WINDOWS.default_render_mode().get()),
            renderer_debug: self.vars.renderer_debug().get(),

            focus: self.start_focused,
            focus_indicator: self.vars.focus_indicator().get(),
        };

        match VIEW_PROCESS.open_window(request) {
            Ok(()) => {
                self.state = Some(state);
                self.waiting_view = true;
            }
            Err(ViewProcessOffline) => {} //respawn
        };
    }

    /// Layout for already open window.
    fn layout_update(&mut self) {
        let m = self.monitor.as_ref().unwrap();
        let scale_factor = m.scale_factor().get();
        let screen_ppi = m.ppi().get();

        let mut state = self.state.clone().unwrap();

        let current_size = self.vars.0.actual_size.get().to_px(scale_factor.0);
        let mut size = current_size;
        let min_size = state.min_size.to_px(scale_factor.0);
        let max_size = state.max_size.to_px(scale_factor.0);
        let root_font_size = self.root_font_size.to_px(scale_factor.0);

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
            .layout(scale_factor, screen_ppi, min_size, max_size, size, root_font_size, skip_auto_size);

        if size != current_size {
            assert!(!skip_auto_size);

            let auto_size_origin = self.vars.auto_size_origin().get();
            let auto_size_origin = |size| {
                let metrics = LayoutMetrics::new(scale_factor, size, root_font_size).with_screen_ppi(screen_ppi);
                LAYOUT.with_context(metrics, || auto_size_origin.layout().to_dip(scale_factor.0))
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
    fn layout_respawn(&mut self) {
        if self.monitor.is_none() {
            self.monitor = Some(self.vars.monitor().get().select_fallback());
        }

        self.layout_update();

        let request = WindowRequest {
            id: WINDOW.id().get(),
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
            render_mode: self.render_mode.unwrap_or_else(|| WINDOWS.default_render_mode().get()),
            renderer_debug: self.vars.renderer_debug().get(),

            focus: WINDOWS.is_focused(WINDOW.id()).unwrap_or(false),
            focus_indicator: self.vars.focus_indicator().get(),
        };

        match VIEW_PROCESS.open_window(request) {
            Ok(()) => self.waiting_view = true,
            Err(ViewProcessOffline) => {} // respawn.
        }
    }

    pub fn render(&mut self) {
        if matches!(self.content.render_requested, RenderUpdate::None) {
            return;
        }

        if let Some(view) = &self.window {
            if matches!(self.state.as_ref().map(|s| s.state), Some(WindowState::Minimized)) {
                return;
            }

            let scale_factor = self.monitor.as_ref().unwrap().scale_factor().get();
            self.content.render(Some(view.renderer()), scale_factor, self.resize_wait_id.take());
        }
    }

    pub fn focus(&mut self) {
        self.update_gen(|view| {
            let _ = view.focus();
        });
    }

    pub fn bring_to_top(&mut self) {
        self.update_gen(|view| {
            let _ = view.bring_to_top();
        });
    }

    pub fn close(&mut self) {
        self.content.close();
        self.window = None;
    }
}

/// Respond to `parent_var` updates, returns `true` if the `parent` value has changed.
fn update_parent(parent: &mut Option<WindowId>, vars: &WindowVars) -> bool {
    let parent_var = vars.parent();
    if let Some(parent_id) = parent_var.get_new() {
        if parent_id == *parent {
            return false;
        }

        match parent_id {
            Some(mut parent_id) => {
                if parent_id == WINDOW.id() {
                    tracing::error!("cannot set `{:?}` as it's own parent", parent_id);
                    parent_var.set(*parent);
                    return false;
                }
                if !vars.0.children.with(|c| c.is_empty()) {
                    tracing::error!("cannot set parent for `{:?}` because it already has children", WINDOW.id());
                    parent_var.set(*parent);
                    return false;
                }

                if let Ok(parent_vars) = WINDOWS.vars(parent_id) {
                    // redirect to parent's parent.
                    if let Some(grand) = parent_vars.parent().get() {
                        tracing::debug!("using `{grand:?}` as parent, because it is the parent of requested `{parent_id:?}`");
                        parent_var.set(grand);

                        parent_id = grand;
                        if Some(parent_id) == *parent {
                            return false;
                        }
                    }

                    // remove previous
                    if let Some(parent_id) = parent.take() {
                        if let Ok(parent_vars) = WINDOWS.vars(parent_id) {
                            let id = WINDOW.id();
                            parent_vars.0.children.modify(move |c| {
                                c.to_mut().remove(&id);
                            });
                        }
                    }

                    // insert new
                    *parent = Some(parent_id);
                    let id = WINDOW.id();
                    parent_vars.0.children.modify(move |c| {
                        c.to_mut().insert(id);
                    });

                    true
                } else {
                    tracing::error!("cannot use `{:?}` as a parent because it does not exist", parent_id);
                    parent_var.set(*parent);
                    false
                }
            }
            None => {
                if let Some(parent_id) = parent.take() {
                    if let Ok(parent_vars) = WINDOWS.vars(parent_id) {
                        let id = WINDOW.id();
                        parent_vars.0.children.modify(move |c| {
                            c.to_mut().remove(&id);
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
    surface: Option<ViewHeadless>,
    waiting_view: bool,
    delayed_view_updates: Vec<Box<dyn FnOnce(&ViewHeadless) + Send>>,
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
    pub fn new(vars: &WindowVars, commands: WindowCommands, content: Window) -> Self {
        Self {
            surface: None,
            waiting_view: false,
            delayed_view_updates: vec![],
            vars: vars.clone(),

            render_mode: content.render_mode,
            headless_monitor: content.headless_monitor,
            headless_simulator: HeadlessSimulator::new(),

            content: ContentCtrl::new(vars.clone(), commands, content),

            actual_parent: None,
            size: DipSize::zero(),
            var_bindings: VarHandles::dummy(),
        }
    }

    pub fn update(&mut self, updates: &WidgetUpdates) {
        if self.surface.is_some() {
            if self.vars.size().is_new()
                || self.vars.min_size().is_new()
                || self.vars.max_size().is_new()
                || self.vars.auto_size().is_new()
                || self.vars.font_size().is_new()
            {
                self.content.layout_requested = true;
                UPDATES.layout();
            }
        } else {
            // we init on the first layout.
            UPDATES.layout();
        }

        if update_parent(&mut self.actual_parent, &self.vars) || self.var_bindings.is_dummy() {
            self.var_bindings = update_headless_vars(self.headless_monitor.scale_factor, &self.vars);
        }
        if let Some(dbg) = self.vars.renderer_debug().get_new() {
            if let Some(view) = &self.surface {
                let _ = view.renderer().set_debug(dbg);
            }
        }

        self.content.update(updates);
    }

    #[must_use]
    pub fn window_updates(&mut self) -> Option<WidgetInfoTree> {
        self.content.window_updates()
    }

    pub fn pre_event(&mut self, update: &EventUpdate) {
        if let Some(args) = RAW_HEADLESS_OPEN_EVENT.on(update) {
            if args.window_id == WINDOW.id() {
                self.waiting_view = false;

                WINDOWS.set_renderer(args.window_id, args.surface.renderer());

                self.surface = Some(args.surface.clone());
                self.vars.0.render_mode.set_ne(args.data.render_mode);

                UPDATES.render();

                for update in mem::take(&mut self.delayed_view_updates) {
                    update(&args.surface);
                }
            }
        } else if let Some(args) = RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT.on(update) {
            if args.window_id == WINDOW.id() && self.surface.is_none() && self.waiting_view {
                tracing::error!("view-process failed to open a headless surface, {}", args.error);

                // was waiting view and failed, treat like a respawn.

                self.waiting_view = false;
                self.delayed_view_updates = vec![];

                self.content.layout_requested = true;
                self.content.render_requested = RenderUpdate::Render;

                UPDATES.layout().render();
            }
        } else if let Some(args) = VIEW_PROCESS_INITED_EVENT.on(update) {
            if let Some(view) = &self.surface {
                if view.renderer().generation() != Ok(args.generation) {
                    debug_assert!(args.is_respawn);

                    self.surface = None;

                    self.content.is_rendering = false;
                    self.content.layout_requested = true;
                    self.content.render_requested = RenderUpdate::Render;

                    UPDATES.layout().render();
                }
            }
        }

        self.content.pre_event(update);

        self.headless_simulator.pre_event(update);
    }

    pub fn ui_event(&mut self, update: &EventUpdate) {
        self.content.ui_event(update);
    }

    pub fn layout(&mut self) {
        if !self.content.layout_requested {
            return;
        }

        let scale_factor = self.vars.0.scale_factor.get();
        let screen_ppi = self.headless_monitor.ppi;
        let screen_size = self.headless_monitor.size.to_px(scale_factor.0);

        let (min_size, max_size, size, root_font_size) = self.content.outer_layout(scale_factor, screen_ppi, screen_size, || {
            let min_size = self.vars.min_size().layout_dft(default_min_size(scale_factor));
            let max_size = self.vars.max_size().layout_dft(screen_size);
            let size = self.vars.size().layout_dft(default_size(scale_factor));
            let root_font_size = self.vars.font_size().layout_dft_x(Length::pt_to_px(11.0, scale_factor));

            (min_size, max_size, size.min(max_size).max(min_size), root_font_size)
        });

        let size = self
            .content
            .layout(scale_factor, screen_ppi, min_size, max_size, size, root_font_size, false);
        let size = size.to_dip(scale_factor.0);

        if let Some(view) = &self.surface {
            // already has surface, maybe resize:
            if self.size != size {
                self.size = size;
                let _: Ignore = view.set_size(size, scale_factor);
            }
        } else if !self.waiting_view {
            // (re)spawn the view surface:

            if !WINDOWS.try_load(WINDOW.id()) {
                return;
            }

            let render_mode = self.render_mode.unwrap_or_else(|| WINDOWS.default_render_mode().get());

            let r = VIEW_PROCESS.open_headless(HeadlessRequest {
                id: WINDOW.id().get(),
                scale_factor: scale_factor.0,
                size,
                render_mode,
                renderer_debug: self.vars.renderer_debug().get(),
            });

            match r {
                Ok(()) => self.waiting_view = true,
                Err(ViewProcessOffline) => {} // respawn
            }
        }

        self.headless_simulator.layout();
    }

    pub fn render(&mut self) {
        if matches!(self.content.render_requested, RenderUpdate::None) {
            return;
        }

        if let Some(view) = &self.surface {
            let fct = self.vars.0.scale_factor.get();
            self.content.render(Some(view.renderer()), fct, None);
        }
    }

    pub fn focus(&mut self) {
        self.headless_simulator.focus();
    }

    pub fn bring_to_top(&mut self) {
        self.headless_simulator.bring_to_top();
    }

    pub fn close(&mut self) {
        self.content.close();
        self.surface = None;
    }
}

fn update_headless_vars(mfactor: Option<Factor>, hvars: &WindowVars) -> VarHandles {
    let mut handles = VarHandles::dummy();

    if let Some(parent_vars) = hvars.parent().get().and_then(|id| WINDOWS.vars(id).ok()) {
        // bind parent factor
        if mfactor.is_none() {
            let h = hvars.0.scale_factor.bind(&parent_vars.0.scale_factor);
            handles.push(h);
        }

        // merge bind color scheme.
        let user = hvars.color_scheme();
        let parent = &parent_vars.0.actual_color_scheme;
        let actual = &hvars.0.actual_color_scheme;

        let h = user.hook(Box::new(clmv!(parent, actual, |value| {
            let value = *value.as_any().downcast_ref::<Option<ColorScheme>>().unwrap();
            let scheme = value.unwrap_or_else(|| parent.get());
            actual.set_ne(scheme);
            true
        })));
        handles.push(h);

        let h = parent.hook(Box::new(clmv!(user, actual, |value| {
            let scheme = user.get().unwrap_or_else(|| *value.as_any().downcast_ref::<ColorScheme>().unwrap());
            actual.set_ne(scheme);
            true
        })));
        handles.push(h);

        hvars.0.actual_color_scheme.set_ne(user.get().unwrap_or_else(|| parent.get()));
    } else {
        // bind color scheme
        let h = hvars
            .color_scheme()
            .bind_map(&hvars.0.actual_color_scheme, |&s| s.unwrap_or_default());
        handles.push(h);

        hvars.0.actual_color_scheme.set_ne(hvars.color_scheme().get().unwrap_or_default());
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
    pub fn new(vars: &WindowVars, commands: WindowCommands, content: Window) -> Self {
        Self {
            vars: vars.clone(),
            headless_monitor: content.headless_monitor,
            content: ContentCtrl::new(vars.clone(), commands, content),
            headless_simulator: HeadlessSimulator::new(),
            actual_parent: None,
            var_bindings: VarHandles::dummy(),
        }
    }

    pub fn update(&mut self, updates: &WidgetUpdates) {
        if self.vars.size().is_new() || self.vars.min_size().is_new() || self.vars.max_size().is_new() || self.vars.auto_size().is_new() {
            self.content.layout_requested = true;
            UPDATES.layout();
        }

        if matches!(self.content.init_state, InitState::Init) {
            self.content.layout_requested = true;
            self.content.pending_render = RenderUpdate::Render;

            UPDATES.layout();
            UPDATES.render();
        }

        if update_parent(&mut self.actual_parent, &self.vars) || self.var_bindings.is_dummy() {
            self.var_bindings = update_headless_vars(self.headless_monitor.scale_factor, &self.vars);
        }

        self.content.update(updates);
    }

    #[must_use]
    pub fn window_updates(&mut self) -> Option<WidgetInfoTree> {
        self.content.window_updates()
    }

    pub fn pre_event(&mut self, update: &EventUpdate) {
        self.content.pre_event(update);
        self.headless_simulator.pre_event(update);
    }

    pub fn ui_event(&mut self, update: &EventUpdate) {
        self.content.ui_event(update);
    }

    pub fn layout(&mut self) {
        if !self.content.layout_requested {
            return;
        }

        if !WINDOWS.try_load(WINDOW.id()) {
            return;
        }

        let scale_factor = self.vars.0.scale_factor.get();
        let screen_ppi = self.headless_monitor.ppi;
        let screen_size = self.headless_monitor.size.to_px(scale_factor.0);

        let (min_size, max_size, size, root_font_size) = self.content.outer_layout(scale_factor, screen_ppi, screen_size, || {
            let min_size = self.vars.min_size().layout_dft(default_min_size(scale_factor));
            let max_size = self.vars.max_size().layout_dft(screen_size);
            let size = self.vars.size().layout_dft(default_size(scale_factor));
            let root_font_size = self.vars.font_size().layout_dft_x(Length::pt_to_px(11.0, scale_factor));

            (min_size, max_size, size.min(max_size).max(min_size), root_font_size)
        });

        let _surface_size = self
            .content
            .layout(scale_factor, screen_ppi, min_size, max_size, size, root_font_size, false);

        self.headless_simulator.layout();
    }

    pub fn render(&mut self) {
        if matches!(self.content.render_requested, RenderUpdate::None) {
            return;
        }

        // layout and render cannot happen yet
        if !WINDOWS.try_load(WINDOW.id()) {
            return;
        }

        let fct = self.vars.0.scale_factor.get();
        self.content.render(None, fct, None);
    }

    pub fn focus(&mut self) {
        self.headless_simulator.focus();
    }

    pub fn bring_to_top(&mut self) {
        self.headless_simulator.bring_to_top();
    }

    pub fn close(&mut self) {
        self.content.close();
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

    fn enabled(&mut self) -> bool {
        *self.is_enabled.get_or_insert_with(|| crate::app::App::window_mode().is_headless())
    }

    pub fn pre_event(&mut self, update: &EventUpdate) {
        if self.enabled() && self.is_open && VIEW_PROCESS_INITED_EVENT.on(update).map(|a| a.is_respawn).unwrap_or(false) {
            self.is_open = false;
        }
    }

    pub fn layout(&mut self) {
        if self.enabled() && !self.is_open {
            self.is_open = true;
            self.focus();
        }
    }

    pub fn focus(&mut self) {
        let mut prev = None;
        if let Some(id) = WINDOWS.focused_window_id() {
            prev = Some(id);
        }
        let args = RawWindowFocusArgs::now(prev, Some(WINDOW.id()));
        RAW_WINDOW_FOCUS_EVENT.notify(args);
    }

    pub fn bring_to_top(&mut self) {
        // we don't have "bring-to-top" event.
    }
}

#[derive(Clone, Copy)]
enum InitState {
    /// We let one update cycle happen before init
    /// to let the constructor closure setup vars
    /// that are read on init.
    SkipOne,
    Init,
    Inited,
}

#[derive(Clone, Copy, Debug)]
enum RenderUpdate {
    None,
    Render,
    RenderUpdate,
}

/// Implementer of window UI node tree initialization and management.
struct ContentCtrl {
    vars: WindowVars,
    commands: WindowCommands,

    root_ctx: WidgetCtx,
    root: BoxedUiNode,
    // info
    used_info_builder: Option<UsedWidgetInfoBuilder>,
    layout_pass: LayoutPassId,

    used_frame_builder: Option<UsedFrameBuilder>,
    used_frame_update: Option<UsedFrameUpdate>,

    init_state: InitState,
    frame_id: FrameId,
    clear_color: RenderColor,

    is_rendering: bool,
    pending_render: RenderUpdate,

    layout_requested: bool,
    render_requested: RenderUpdate,

    previous_transforms: IdMap<WidgetId, PxTransform>,
}
impl ContentCtrl {
    pub fn new(vars: WindowVars, commands: WindowCommands, window: Window) -> Self {
        Self {
            vars,
            commands,

            root_ctx: WidgetCtx::new(window.id),
            root: window.child,

            used_info_builder: None,
            layout_pass: 0,

            used_frame_builder: None,
            used_frame_update: None,

            init_state: InitState::SkipOne,
            frame_id: FrameId::INVALID,
            clear_color: RenderColor::BLACK,

            is_rendering: false,
            pending_render: RenderUpdate::None,

            layout_requested: false,
            render_requested: RenderUpdate::None,

            previous_transforms: IdMap::default(),
        }
    }

    pub fn update(&mut self, updates: &WidgetUpdates) {
        match self.init_state {
            InitState::Inited => {
                self.commands.update(&self.vars);

                updates.with_window(|| {
                    WIDGET.with_context(&self.root_ctx, || {
                        updates.with_widget(|| {
                            self.root.update(updates);
                        });
                    });
                });
            }

            InitState::SkipOne => {
                UPDATES.update_ext();
                self.init_state = InitState::Init;
            }
            InitState::Init => {
                self.commands.init(&self.vars);
                WIDGET.with_context(&self.root_ctx, || {
                    self.root.init();
                    // requests info, layout and render just in case `root` is a blank.
                    WIDGET.update_info().layout().render();
                });
                self.init_state = InitState::Inited;
            }
        }
    }

    #[must_use]
    pub fn window_updates(&mut self) -> Option<WidgetInfoTree> {
        if self.root_ctx.take_layout() {
            self.layout_requested = true;
            UPDATES.layout();
        }
        if self.root_ctx.is_pending_render() {
            let _ = self.root_ctx.take_render();
            self.render_requested = RenderUpdate::Render;
            UPDATES.render();
        } else if self.root_ctx.is_pending_render_update() {
            let _ = self.root_ctx.take_render_update();
            if !matches!(&self.render_requested, RenderUpdate::Render) {
                self.render_requested = RenderUpdate::RenderUpdate;
            }
            UPDATES.render();
        }

        if self.root_ctx.take_info() {
            let mut info = WidgetInfoBuilder::new(
                WINDOW.id(),
                self.root_ctx.id(),
                self.root_ctx.bounds(),
                self.root_ctx.border(),
                self.vars.0.scale_factor.get(),
                self.used_info_builder.take(),
            );

            WIDGET.with_context(&self.root_ctx, || {
                self.root.info(&mut info);
            });

            let (info, used) = info.finalize(WINDOW.widget_tree().stats().generation.wrapping_add(1));
            self.used_info_builder = Some(used);

            WINDOWS.set_widget_tree(
                info.clone(),
                self.layout_requested,
                !matches!(self.render_requested, RenderUpdate::None),
            );

            Some(info)
        } else {
            None
        }
    }

    pub fn pre_event(&mut self, update: &EventUpdate) {
        if let Some(args) = RAW_FRAME_RENDERED_EVENT.on(update) {
            if args.window_id == WINDOW.id() {
                self.is_rendering = false;
                match mem::replace(&mut self.pending_render, RenderUpdate::None) {
                    RenderUpdate::None => {}
                    RenderUpdate::Render => {
                        self.render_requested = RenderUpdate::Render;
                        UPDATES.render();
                    }
                    RenderUpdate::RenderUpdate => {
                        if !matches!(self.render_requested, RenderUpdate::Render) {
                            self.render_requested = RenderUpdate::RenderUpdate;
                        }
                        UPDATES.render();
                    }
                }

                let image = args.frame_image.as_ref().cloned().map(Image::new);

                let args = FrameImageReadyArgs::new(args.timestamp, args.propagation().clone(), args.window_id, args.frame_id, image);
                FRAME_IMAGE_READY_EVENT.notify(args);
            }
        } else {
            self.commands.event(&self.vars, update);
        }
    }

    pub fn ui_event(&mut self, update: &EventUpdate) {
        debug_assert!(matches!(self.init_state, InitState::Inited));

        update.with_window(|| {
            WIDGET.with_context(&self.root_ctx, || {
                update.with_widget(|| {
                    self.root.event(update);
                })
            });
        });
    }

    pub fn close(&mut self) {
        WIDGET.with_context(&self.root_ctx, || {
            self.root.deinit();
        });

        self.vars.0.is_open.set(false);
        self.root_ctx.deinit();
    }

    /// Run an `action` in the context of a monitor screen that is parent of this content.
    pub fn outer_layout<R>(&mut self, scale_factor: Factor, screen_ppi: f32, screen_size: PxSize, action: impl FnOnce() -> R) -> R {
        let metrics = LayoutMetrics::new(scale_factor, screen_size, Length::pt_to_px(11.0, scale_factor)).with_screen_ppi(screen_ppi);
        LAYOUT.with_context(metrics, action)
    }

    /// Layout content if there was a pending request, returns `Some(final_size)`.
    #[allow(clippy::too_many_arguments)]
    pub fn layout(
        &mut self,
        scale_factor: Factor,
        screen_ppi: f32,
        min_size: PxSize,
        max_size: PxSize,
        size: PxSize,
        root_font_size: Px,
        skip_auto_size: bool,
    ) -> PxSize {
        debug_assert!(matches!(self.init_state, InitState::Inited));
        debug_assert!(self.layout_requested);

        let _s = tracing::trace_span!("window.on_layout", window = %WINDOW.id().sequential()).entered();

        self.layout_requested = false;

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

        WIDGET.with_context(&self.root_ctx, || {
            let metrics = LayoutMetrics::new(scale_factor, viewport_size, root_font_size).with_screen_ppi(screen_ppi);
            LAYOUT.with_context(metrics, || {
                let desired_size = LAYOUT.with_constrains(
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
                    || WidgetLayout::with_root_widget(self.layout_pass, |wl| self.root.layout(wl)),
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
            })
        })
    }

    pub fn render(&mut self, renderer: Option<ViewRenderer>, scale_factor: Factor, wait_id: Option<FrameWaitId>) {
        match mem::replace(&mut self.render_requested, RenderUpdate::None) {
            // RENDER FULL FRAME
            RenderUpdate::Render => {
                if self.is_rendering {
                    self.pending_render = RenderUpdate::Render;
                    return;
                }
                let _s = tracing::trace_span!("window.on_render", window = %WINDOW.id().sequential()).entered();

                self.frame_id = self.frame_id.next();

                let default_text_aa = FONTS.system_font_aa().get();

                let mut frame = FrameBuilder::new(
                    self.frame_id,
                    self.root_ctx.id(),
                    &self.root_ctx.bounds(),
                    &WINDOW.widget_tree(),
                    renderer.clone(),
                    scale_factor,
                    default_text_aa,
                    self.used_frame_builder.take(),
                );

                let (frame, used) = WIDGET.with_context(&self.root_ctx, || {
                    self.root.render(&mut frame);
                    frame.finalize(&WINDOW.widget_tree())
                });

                self.notify_transform_changes();

                self.used_frame_builder = Some(used);

                self.clear_color = frame.clear_color;

                let capture_image = self.take_capture_image();

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
                } else {
                    // simulate frame in headless
                    FRAME_IMAGE_READY_EVENT.notify(FrameImageReadyArgs::now(WINDOW.id(), self.frame_id, None));
                }
            }

            // RENDER UPDATE
            RenderUpdate::RenderUpdate => {
                if self.is_rendering {
                    if !matches!(self.pending_render, RenderUpdate::Render) {
                        self.pending_render = RenderUpdate::RenderUpdate;
                    }
                    return;
                }

                let _s = tracing::trace_span!("window.on_render_update", window = %WINDOW.id().sequential()).entered();

                self.frame_id = self.frame_id.next_update();

                let mut update = FrameUpdate::new(
                    self.frame_id,
                    self.root_ctx.id(),
                    self.root_ctx.bounds(),
                    renderer.as_ref(),
                    self.clear_color,
                    self.used_frame_update.take(),
                );

                let (update, used) = WIDGET.with_context(&self.root_ctx, || {
                    self.root.render_update(&mut update);
                    update.finalize(&WINDOW.widget_tree())
                });

                self.notify_transform_changes();

                self.used_frame_update = Some(used);

                if let Some(c) = update.clear_color {
                    self.clear_color = c;
                }

                let capture_image = self.take_capture_image();

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
                } else {
                    // simulate frame in headless
                    FRAME_IMAGE_READY_EVENT.notify(FrameImageReadyArgs::now(WINDOW.id(), self.frame_id, None));
                }
            }
            RenderUpdate::None => {
                debug_assert!(false, "self.render_requested != RenderUpdate::None")
            }
        }
    }
    fn take_capture_image(&self) -> bool {
        match self.vars.frame_capture_mode().get() {
            FrameCaptureMode::Sporadic => false,
            FrameCaptureMode::Next => {
                self.vars.frame_capture_mode().set(FrameCaptureMode::Sporadic);
                true
            }
            FrameCaptureMode::All => true,
        }
    }

    fn notify_transform_changes(&mut self) {
        let mut changes_count = 0;

        TRANSFORM_CHANGED_EVENT.visit_subscribers(|wid| {
            let tree = WINDOW.widget_tree();
            if let Some(wgt) = tree.get(wid) {
                let transform = wgt.bounds_info().inner_transform();

                match self.previous_transforms.entry(wid) {
                    IdEntry::Occupied(mut e) => {
                        let prev = e.insert(transform);
                        if prev != transform {
                            TRANSFORM_CHANGED_EVENT.notify(TransformChangedArgs::now(wgt.path(), prev, transform));
                            changes_count += 1;
                        }
                    }
                    IdEntry::Vacant(e) => {
                        e.insert(transform);
                    }
                }
            }
        });

        if (self.previous_transforms.len() - changes_count) > 500 {
            self.previous_transforms.retain(|k, _| TRANSFORM_CHANGED_EVENT.is_subscriber(*k));
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
    pub fn new(vars: &WindowVars, commands: WindowCommands, mode: WindowMode, content: Window) -> Self {
        WindowCtrl(match mode {
            WindowMode::Headed => WindowCtrlMode::Headed(HeadedCtrl::new(vars, commands, content)),
            WindowMode::Headless => WindowCtrlMode::Headless(HeadlessCtrl::new(vars, commands, content)),
            WindowMode::HeadlessWithRenderer => {
                WindowCtrlMode::HeadlessWithRenderer(HeadlessWithRendererCtrl::new(vars, commands, content))
            }
        })
    }

    pub fn update(&mut self, updates: &WidgetUpdates) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.update(updates),
            WindowCtrlMode::Headless(c) => c.update(updates),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.update(updates),
        }
    }

    #[must_use]
    pub fn window_updates(&mut self) -> Option<WidgetInfoTree> {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.window_updates(),
            WindowCtrlMode::Headless(c) => c.window_updates(),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.window_updates(),
        }
    }

    pub fn pre_event(&mut self, update: &EventUpdate) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.pre_event(update),
            WindowCtrlMode::Headless(c) => c.pre_event(update),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.pre_event(update),
        }
    }

    pub fn ui_event(&mut self, update: &EventUpdate) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.ui_event(update),
            WindowCtrlMode::Headless(c) => c.ui_event(update),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.ui_event(update),
        }
    }

    pub fn layout(&mut self) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.layout(),
            WindowCtrlMode::Headless(c) => c.layout(),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.layout(),
        }
    }

    pub fn render(&mut self) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.render(),
            WindowCtrlMode::Headless(c) => c.render(),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.render(),
        }
    }

    pub fn focus(&mut self) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.focus(),
            WindowCtrlMode::Headless(c) => c.focus(),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.focus(),
        }
    }

    pub fn bring_to_top(&mut self) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.bring_to_top(),
            WindowCtrlMode::Headless(c) => c.bring_to_top(),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.bring_to_top(),
        }
    }

    pub fn close(&mut self) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.close(),
            WindowCtrlMode::Headless(c) => c.close(),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.close(),
        }
    }
}

fn default_min_size(scale_factor: Factor) -> PxSize {
    DipSize::new(Dip::new(192), Dip::new(48)).to_px(scale_factor.0)
}

fn default_size(scale_factor: Factor) -> PxSize {
    DipSize::new(Dip::new(800), Dip::new(600)).to_px(scale_factor.0)
}

/// Respawned error is ok here, because we recreate the window/surface on respawn.
type Ignore = Result<(), ViewProcessOffline>;
