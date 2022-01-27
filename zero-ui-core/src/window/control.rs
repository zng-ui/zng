//! This module implements Management of window content and synchronization of WindowVars and View-Process.

use std::{mem, time::Instant};

use crate::{
    app::{
        raw_events::{RawFrameRenderedEvent, RawWindowChangedEvent, RawWindowFocusArgs, RawWindowFocusEvent},
        view_process::*,
    },
    color::RenderColor,
    context::{LayoutContext, WindowContext, WindowRenderUpdate, WindowUpdates},
    event::EventUpdateArgs,
    image::{Image, ImageVar, ImagesExt},
    render::{FrameBuilder, FrameId, FrameUpdate, UsedFrameBuilder, UsedFrameUpdate, WidgetTransformKey},
    state::OwnedStateMap,
    units::*,
    var::*,
    widget_info::{BoundsInfo, UsedWidgetInfoBuilder, WidgetInfoBuilder, WidgetLayout, WidgetRendered, WidgetSubscriptions},
    window::AutoSize,
    BoxedUiNode, UiNode, WidgetId,
};

use super::{
    FrameCaptureMode, FrameImageReadyArgs, FrameImageReadyEvent, HeadlessMonitor, MonitorInfo, MonitorsChangedEvent, MonitorsExt,
    StartPosition, Window, WindowChangedArgs, WindowChangedEvent, WindowChrome, WindowIcon, WindowMode, WindowVars, WindowsExt,
};

/// Implementer of `App <-> View` sync in a headed window.
struct HeadedCtrl {
    window: Option<ViewWindow>,
    vars: WindowVars,
    respawned: bool,

    content: ContentCtrl,

    // init config.
    start_position: StartPosition,
    kiosk: Option<WindowState>, // Some(enforced_fullscreen)
    transparent: bool,
    render_mode: Option<RenderMode>,

    // current state.
    state: Option<WindowStateAll>, // None if not inited.
    monitor: Option<MonitorInfo>,
    resize_wait_id: Option<FrameWaitId>,
    icon: Option<ImageVar>,
    actual_state: Option<WindowState>, // for WindowChangedEvent
}
impl HeadedCtrl {
    pub fn new(vars: &WindowVars, content: Window) -> Self {
        Self {
            window: None,

            start_position: content.start_position,
            kiosk: if content.kiosk { Some(WindowState::Fullscreen) } else { None },
            transparent: content.transparent,
            render_mode: content.render_mode,

            content: ContentCtrl::new(vars.clone(), content),
            vars: vars.clone(),
            respawned: false,

            state: None,
            monitor: None,
            resize_wait_id: None,
            icon: None,

            actual_state: None,
        }
    }

    pub fn update(&mut self, ctx: &mut WindowContext) {
        if let Some(view) = &self.window {
            // is inited:

            if let Some(enforced_fullscreen) = &mut self.kiosk {
                // always fullscreen, but can be windowed or exclusive.

                if let Some(state) = self.vars.state().copy_new(ctx) {
                    if !state.is_fullscreen() {
                        tracing::error!("window in `kiosk` mode can only be fullscreen");

                        self.vars.state().set(ctx, *enforced_fullscreen);
                    } else {
                        *enforced_fullscreen = state;
                    }
                }

                if let Some(false) = self.vars.visible().copy_new(ctx) {
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
                let prev_state = self.state.as_ref().unwrap();
                let mut new_state = prev_state.clone();

                if let Some(query) = self.vars.monitor().get_new(ctx.vars) {
                    let monitors = ctx.services.monitors();

                    if self.monitor.is_none() {
                        self.monitor = Some(query.select_fallback(ctx.vars, monitors));
                    } else if let Some(new) = query.select(ctx.vars, monitors) {
                        let current = self.vars.0.actual_monitor.copy(ctx.vars);
                        if Some(new.id()) != current {
                            todo!()
                        }
                    }
                }

                if let Some(chrome) = self.vars.chrome().get_new(ctx.vars) {
                    new_state.chrome_visible = chrome.is_default();
                }

                if let Some(req_state) = self.vars.state().copy_new(ctx) {
                    new_state.set_state(req_state);
                    self.vars.0.restore_state.set_ne(ctx, new_state.restore_state);
                }

                if self.vars.min_size().is_new(ctx) || self.vars.max_size().is_new(ctx) {
                    if let Some(m) = &self.monitor {
                        let scale_factor = m.scale_factor().copy(ctx);
                        let screen_ppi = m.ppi().copy(ctx);
                        let screen_size = m.size().copy(ctx);
                        let (min_size, max_size) = self.content.outer_layout(ctx, scale_factor, screen_ppi, screen_size, |ctx| {
                            let available_size = AvailableSize::finite(screen_size);

                            let min_size =
                                self.vars
                                    .min_size()
                                    .get(ctx.vars)
                                    .to_layout(ctx, available_size, default_min_size(scale_factor));

                            let max_size = self.vars.max_size().get(ctx.vars).to_layout(ctx, available_size, screen_size);

                            (min_size.to_dip(scale_factor.0), max_size.to_dip(scale_factor.0))
                        });

                        let size = new_state.restore_rect.size;

                        new_state.restore_rect.size = size.min(max_size).max(min_size);
                        new_state.min_size = min_size;
                        new_state.max_size = max_size;
                    }
                }

                if let Some(auto) = self.vars.auto_size().copy_new(ctx) {
                    if auto != AutoSize::DISABLED {
                        self.content.layout_requested = true;
                        ctx.updates.layout();
                    }
                }

                if self.vars.size().is_new(ctx) {
                    let auto_size = self.vars.auto_size().copy(ctx);

                    if auto_size != AutoSize::CONTENT {
                        if let Some(m) = &self.monitor {
                            let scale_factor = m.scale_factor().copy(ctx);
                            let screen_ppi = m.ppi().copy(ctx);
                            let screen_size = m.size().copy(ctx);
                            let size = self.content.outer_layout(ctx, scale_factor, screen_ppi, screen_size, |ctx| {
                                let available_size = AvailableSize::finite(screen_size);
                                self.vars
                                    .size()
                                    .get(ctx.vars)
                                    .to_layout(ctx, available_size, default_size(scale_factor))
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
                        let scale_factor = m.scale_factor().copy(ctx);
                        let screen_ppi = m.ppi().copy(ctx);
                        let screen_size = m.size().copy(ctx);
                        let pos = self.content.outer_layout(ctx, scale_factor, screen_ppi, screen_size, |ctx| {
                            pos.to_layout(ctx, AvailableSize::finite(screen_size), PxPoint::new(Px(50), Px(50)))
                        });
                        new_state.restore_rect.origin = pos.to_dip(scale_factor.0);
                    }
                }

                if let Some(visible) = self.vars.visible().copy_new(ctx) {
                    let _: Ignore = view.set_visible(visible);
                }

                if let Some(movable) = self.vars.movable().copy_new(ctx) {
                    let _: Ignore = view.set_movable(movable);
                }

                if let Some(resizable) = self.vars.resizable().copy_new(ctx) {
                    let _: Ignore = view.set_resizable(resizable);
                }

                if prev_state != &new_state {
                    let _: Ignore = view.set_state(new_state);
                }
            }

            // icon:
            let mut send_icon = false;
            if self.vars.icon().is_new(ctx) {
                Self::init_icon(&mut self.icon, &self.vars, ctx);
                send_icon = true;
            } else if self.icon.as_ref().map(|ico| ico.is_new(ctx)).unwrap_or(false) {
                send_icon = true;
            }
            if send_icon {
                let icon = self.icon.as_ref().and_then(|ico| ico.get(ctx).view());
                let _: Ignore = view.set_icon(icon);
            }

            if let Some(title) = self.vars.title().get_new(ctx) {
                let _: Ignore = view.set_title(title.to_string());
            }

            if let Some(mode) = self.vars.video_mode().copy_new(ctx) {
                let _: Ignore = view.set_video_mode(mode);
            }

            if let Some(cursor) = self.vars.cursor().copy_new(ctx) {
                let _: Ignore = view.set_cursor(cursor);
            }

            if let Some(visible) = self.vars.taskbar_visible().copy_new(ctx) {
                let _: Ignore = view.set_taskbar_visible(visible);
            }

            if let Some(aa) = self.vars.text_aa().copy_new(ctx) {
                let _: Ignore = view.renderer().set_text_aa(aa);
            }

            if let Some(top) = self.vars.always_on_top().copy_new(ctx) {
                let _: Ignore = view.set_always_on_top(top);
            }

            if let Some(mode) = self.vars.frame_capture_mode().copy_new(ctx.vars) {
                let _: Ignore = view.set_capture_mode(matches!(mode, FrameCaptureMode::All));
            }

            if let Some(allow) = self.vars.allow_alt_f4().copy_new(ctx) {
                let _: Ignore = view.set_allow_alt_f4(allow);
            }

            if let Some(m) = &self.monitor {
                if m.scale_factor().is_new(ctx) || m.size().is_new(ctx) || m.ppi().is_new(ctx) {
                    self.content.layout_requested = true;
                    ctx.updates.layout();
                }
            }
        } else {
            // is not inited:

            if let Some(enforced_fullscreen) = self.kiosk {
                if !self.vars.state().get(ctx).is_fullscreen() {
                    self.vars.state().set(ctx, enforced_fullscreen);
                }
            }

            // we init on the first layout.
            ctx.updates.layout();
        }

        self.content.update(ctx);
    }

    pub fn window_updates(&mut self, ctx: &mut WindowContext, updates: WindowUpdates) {
        self.content.window_updates(ctx, updates);
    }

    pub fn pre_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        if let Some(args) = RawWindowChangedEvent.update(args) {
            if args.window_id == *ctx.window_id {
                let mut state_change = None;
                let mut pos_change = None;
                let mut size_change = None;

                if let Some((monitor, _)) = args.monitor {
                    if self.vars.0.actual_monitor.set_ne(ctx, Some(monitor)) {
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
                    }

                    self.state = Some(state);
                }

                if let Some(pos) = args.position {
                    if self.vars.0.actual_position.set_ne(ctx, pos) {
                        pos_change = Some(pos);
                    }
                }

                if let Some(size) = args.size {
                    if self.vars.0.actual_size.set_ne(ctx, size) {
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
                    let args = WindowChangedArgs::new(args.timestamp, args.window_id, state_change, pos_change, size_change, args.cause);
                    WindowChangedEvent.notify(ctx.events, args);
                }
            }
        } else if let Some(args) = MonitorsChangedEvent.update(args) {
            if let Some(m) = &self.monitor {
                if args.removed.contains(&m.id()) {
                    self.monitor = None;
                    self.vars.0.actual_monitor.set_ne(ctx, None);
                }
            }
            self.vars.monitor().touch(ctx);
        } else if let Some(args) = ViewProcessRespawnedEvent.update(args) {
            if let Some(view) = &self.window {
                if view.renderer().generation() != Ok(args.generation) {
                    self.window = None;
                    self.respawned = true;

                    self.content.layout_requested = true;
                    self.content.render_requested = WindowRenderUpdate::Render;
                    self.content.is_rendering = false;

                    ctx.updates.layout_and_render();
                }
            }
        }

        self.content.pre_event(ctx, args);
    }

    pub fn ui_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        self.content.ui_event(ctx, args);
    }

    pub fn layout(&mut self, ctx: &mut WindowContext) {
        if !self.content.layout_requested {
            return;
        }

        if self.is_inited() {
            if matches!(self.state.as_ref().map(|s| s.state), Some(WindowState::Minimized)) {
                return;
            }
            self.layout_update(ctx);
        } else if self.respawned {
            self.layout_respawn(ctx);
        } else {
            self.layout_init(ctx);
        }
    }

    /// First layout, opens the window.
    fn layout_init(&mut self, ctx: &mut WindowContext) {
        self.monitor = Some(self.vars.monitor().get(ctx.vars).select_fallback(ctx.vars, ctx.services.monitors()));

        let m = self.monitor.as_ref().unwrap();
        let scale_factor = m.scale_factor().copy(ctx);
        let screen_ppi = m.ppi().copy(ctx);
        let screen_size = m.size().copy(ctx);

        let (min_size, max_size, mut size) = self.content.outer_layout(ctx, scale_factor, screen_ppi, screen_size, |ctx| {
            let available_size = AvailableSize::finite(screen_size);

            let min_size = self
                .vars
                .min_size()
                .get(ctx.vars)
                .to_layout(ctx.metrics, available_size, default_min_size(scale_factor));
            let max_size = self
                .vars
                .max_size()
                .get(ctx.vars)
                .to_layout(ctx.metrics, available_size, screen_size);

            let size = self
                .vars
                .size()
                .get(ctx.vars)
                .to_layout(ctx.metrics, available_size, default_size(scale_factor));

            (min_size, max_size, size.min(max_size).max(min_size))
        });

        let state = self.vars.state().copy(ctx);

        if state == WindowState::Normal && self.vars.auto_size().copy(ctx) != AutoSize::DISABLED {
            size = self.content.layout(ctx, scale_factor, screen_ppi, min_size, max_size, size, false);
        }

        let mut system_pos = false;
        let position = match self.start_position {
            StartPosition::Default => {
                let pos = self.vars.position().get(ctx.vars);
                if pos.x.is_default() || pos.y.is_default() {
                    system_pos = true;
                    PxPoint::zero()
                } else {
                    self.content.outer_layout(ctx, scale_factor, screen_ppi, screen_size, |ctx| {
                        let available_size = AvailableSize::finite(screen_size);
                        pos.to_layout(ctx.metrics, available_size, PxPoint::zero())
                    })
                }
            }
            StartPosition::CenterMonitor => {
                PxPoint::new((screen_size.width - size.width) / Px(2), (screen_size.height - size.height) / Px(2))
            }
            StartPosition::CenterParent => {
                // center monitor if no parent
                todo!()
            }
        };

        let position = position.to_dip(scale_factor.0);
        let size = size.to_dip(scale_factor.0);

        let state = WindowStateAll {
            state,
            restore_rect: DipRect::new(position, size),
            restore_state: WindowState::Normal,
            min_size: min_size.to_dip(scale_factor.0),
            max_size: max_size.to_dip(scale_factor.0),
            chrome_visible: self.vars.chrome().get(ctx).is_default(),
        };

        Self::init_icon(&mut self.icon, &self.vars, ctx);

        let request = WindowRequest {
            id: ctx.window_id.get(),
            title: self.vars.title().get(ctx).to_string(),
            state,
            kiosk: self.kiosk.is_some(),
            default_position: system_pos,
            video_mode: self.vars.video_mode().copy(ctx),
            visible: self.vars.visible().copy(ctx),
            taskbar_visible: self.vars.taskbar_visible().copy(ctx),
            allow_alt_f4: self.vars.allow_alt_f4().copy(ctx),
            always_on_top: self.vars.always_on_top().copy(ctx),
            movable: self.vars.movable().copy(ctx),
            resizable: self.vars.resizable().copy(ctx),
            icon: self.icon.as_ref().and_then(|ico| ico.get(ctx).view()).map(|ico| ico.id()),
            cursor: self.vars.cursor().copy(ctx),
            transparent: self.transparent,
            text_aa: self.vars.text_aa().copy(ctx),
            capture_mode: matches!(self.vars.frame_capture_mode().get(ctx), FrameCaptureMode::All),
            render_mode: self.render_mode.unwrap_or_else(|| ctx.services.windows().default_render_mode),
        };
        let r = ctx.services.view_process().open_window(request);

        if let Ok((window, data)) = r {
            ctx.services.windows().set_renderer(*ctx.window_id, window.renderer());

            self.window = Some(window);
            self.vars.0.render_mode.set_ne(ctx, data.render_mode);
            self.vars.state().set_ne(ctx, data.state.state);
            self.vars.0.restore_state.set_ne(ctx, data.state.restore_state);
            self.vars.0.restore_rect.set_ne(ctx, data.state.restore_rect);
            self.vars.0.actual_position.set_ne(ctx, data.position);
            self.vars.0.actual_size.set_ne(ctx, data.size);
            self.vars.0.actual_monitor.set_ne(ctx, data.monitor);

            self.state = Some(data.state);

            // if we did not layout yet or the view-process used a different size.
            self.content.layout_requested |= size != data.size;
            if self.content.layout_requested {
                self.content.layout(
                    ctx,
                    scale_factor,
                    screen_ppi,
                    min_size,
                    max_size,
                    data.size.to_px(scale_factor.0),
                    true,
                );
            }

            ctx.updates.render();
        }
        // else `Respawn`
    }

    /// Layout for already open window.
    fn layout_update(&mut self, ctx: &mut WindowContext) {
        let m = self.monitor.as_ref().unwrap();
        let scale_factor = m.scale_factor().copy(ctx);
        let screen_ppi = m.ppi().copy(ctx);

        let mut state = self.state.clone().unwrap();

        let current_size = self.vars.0.actual_size.copy(ctx).to_px(scale_factor.0);
        let mut size = current_size;
        let min_size = state.min_size.to_px(scale_factor.0);
        let max_size = state.max_size.to_px(scale_factor.0);

        let skip_auto_size = !matches!(state.state, WindowState::Normal);

        if !skip_auto_size {
            let auto_size = self.vars.auto_size().copy(ctx);

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

            let auto_size_origin = self.vars.auto_size_origin().get(ctx.vars);
            let base_font_size = base_font_size(scale_factor);
            let mut auto_size_origin = |size| {
                ctx.layout_context(
                    base_font_size,
                    scale_factor,
                    screen_ppi,
                    size,
                    LayoutMask::LAYOUT_METRICS,
                    self.content.root_id,
                    &mut self.content.root_state,
                    |ctx| {
                        auto_size_origin
                            .to_layout(ctx, AvailableSize::finite(size), PxPoint::zero())
                            .to_dip(scale_factor.0)
                    },
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
            self.monitor = Some(self.vars.monitor().get(ctx.vars).select_fallback(ctx.vars, ctx.services.monitors()));
        }

        self.layout_update(ctx);

        Self::init_icon(&mut self.icon, &self.vars, ctx);

        let request = WindowRequest {
            id: ctx.window_id.get(),
            title: self.vars.title().get(ctx).to_string(),
            state: self.state.clone().unwrap(),
            kiosk: self.kiosk.is_some(),
            default_position: false,
            video_mode: self.vars.video_mode().copy(ctx),
            visible: self.vars.visible().copy(ctx),
            taskbar_visible: self.vars.taskbar_visible().copy(ctx),
            allow_alt_f4: self.vars.allow_alt_f4().copy(ctx),
            always_on_top: self.vars.always_on_top().copy(ctx),
            movable: self.vars.movable().copy(ctx),
            resizable: self.vars.resizable().copy(ctx),
            icon: self.icon.as_ref().and_then(|ico| ico.get(ctx).view()).map(|ico| ico.id()),
            cursor: self.vars.cursor().copy(ctx),
            transparent: self.transparent,
            text_aa: self.vars.text_aa().copy(ctx),
            capture_mode: matches!(self.vars.frame_capture_mode().get(ctx), FrameCaptureMode::All),
            render_mode: self.render_mode.unwrap_or_else(|| ctx.services.windows().default_render_mode),
        };
        let r = ctx.services.view_process().open_window(request);

        if let Ok((window, data)) = r {
            ctx.services.windows().set_renderer(*ctx.window_id, window.renderer());

            self.window = Some(window);
            self.vars.0.render_mode.set_ne(ctx, data.render_mode);
            self.vars.state().set_ne(ctx, data.state.state);
            self.vars.0.restore_state.set_ne(ctx, data.state.restore_state);
            self.vars.0.restore_rect.set_ne(ctx, data.state.restore_rect);
            self.vars.0.actual_position.set_ne(ctx, data.position);
            self.vars.0.actual_size.set_ne(ctx, data.size);
            self.vars.0.actual_monitor.set_ne(ctx, data.monitor);

            self.state = Some(data.state);

            ctx.updates.render();
        }
        // else `Respawn`
    }

    pub fn render(&mut self, ctx: &mut WindowContext) {
        if self.content.render_requested.is_none() {
            return;
        }

        if let Some(view) = &self.window {
            let scale_factor = self.monitor.as_ref().unwrap().scale_factor().copy(ctx);
            self.content
                .render(ctx, Some(view.renderer()), scale_factor, self.resize_wait_id.take());
        }
    }

    pub fn close(&mut self, ctx: &mut WindowContext) {
        self.content.close(ctx);
        self.window = None;
    }

    fn is_inited(&self) -> bool {
        self.window.is_some()
    }

    fn init_icon(icon: &mut Option<ImageVar>, vars: &WindowVars, ctx: &mut WindowContext) {
        *icon = match vars.icon().get(ctx.vars) {
            WindowIcon::Default => None,
            WindowIcon::Image(source) => Some(ctx.services.images().cache(source.clone())),
            WindowIcon::Render(_) => todo!(),
        };
    }
}

/// Implementer of `App <-> View` sync in a headless window.
struct HeadlessWithRendererCtrl {
    surface: Option<ViewHeadless>,
    vars: WindowVars,
    content: ContentCtrl,

    // init config.
    render_mode: Option<RenderMode>,
    headless_monitor: HeadlessMonitor,
    headless_simulator: HeadlessSimulator,

    // current state.
    size: DipSize,
}
impl HeadlessWithRendererCtrl {
    pub fn new(vars: &WindowVars, content: Window) -> Self {
        Self {
            surface: None,
            vars: vars.clone(),

            render_mode: content.render_mode,
            headless_monitor: content.headless_monitor,
            headless_simulator: HeadlessSimulator::new(),

            content: ContentCtrl::new(vars.clone(), content),

            size: DipSize::zero(),
        }
    }

    pub fn update(&mut self, ctx: &mut WindowContext) {
        if self.is_inited() {
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

        self.content.update(ctx);
    }

    pub fn window_updates(&mut self, ctx: &mut WindowContext, updates: WindowUpdates) {
        self.content.window_updates(ctx, updates);
    }

    pub fn pre_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        if let Some(args) = ViewProcessRespawnedEvent.update(args) {
            if let Some(view) = &self.surface {
                if view.renderer().generation() == Ok(args.generation) {
                    self.surface = None;

                    self.content.is_rendering = false;
                    self.content.layout_requested = true;
                    self.content.render_requested = WindowRenderUpdate::Render;

                    ctx.updates.layout_and_render();
                }
            }
        }

        self.content.pre_event(ctx, args);

        self.headless_simulator.pre_event(ctx, args);
    }

    pub fn ui_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        self.content.ui_event(ctx, args);
    }

    pub fn layout(&mut self, ctx: &mut WindowContext) {
        if !self.content.layout_requested {
            return;
        }

        let scale_factor = self.headless_monitor.scale_factor;
        let screen_ppi = self.headless_monitor.ppi;
        let screen_size = self.headless_monitor.size.to_px(scale_factor.0);

        let (min_size, max_size, size) = self.content.outer_layout(ctx, scale_factor, screen_ppi, screen_size, |ctx| {
            let available_size = AvailableSize::finite(screen_size);

            let min_size = self
                .vars
                .min_size()
                .get(ctx.vars)
                .to_layout(ctx.metrics, available_size, default_min_size(scale_factor));

            let max_size = self
                .vars
                .max_size()
                .get(ctx.vars)
                .to_layout(ctx.metrics, available_size, screen_size);

            let size = self
                .vars
                .size()
                .get(ctx.vars)
                .to_layout(ctx.metrics, available_size, default_size(scale_factor));

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
        } else {
            // (re)spawn the view surface:

            let window_id = *ctx.window_id;
            let render_mode = self.render_mode.unwrap_or_else(|| ctx.services.windows().default_render_mode);

            let r = ctx.services.view_process().open_headless(HeadlessRequest {
                id: window_id.get(),
                scale_factor: scale_factor.0,
                size,
                text_aa: self.vars.text_aa().copy(ctx.vars),
                render_mode,
            });

            if let Ok((surface, data)) = r {
                ctx.services.windows().set_renderer(window_id, surface.renderer());

                self.surface = Some(surface);
                self.vars.0.render_mode.set_ne(ctx.vars, data.render_mode);

                ctx.updates.render();
            }
            // else `Respawn`, handled in `pre_event`.
        }

        self.headless_simulator.layout(ctx);
    }

    pub fn render(&mut self, ctx: &mut WindowContext) {
        if self.content.render_requested.is_none() {
            return;
        }

        if let Some(view) = &self.surface {
            self.content
                .render(ctx, Some(view.renderer()), self.headless_monitor.scale_factor, None);
        }
    }
    pub fn close(&mut self, ctx: &mut WindowContext) {
        self.content.close(ctx);
        self.surface = None;
    }

    fn is_inited(&self) -> bool {
        self.surface.is_some()
    }
}

/// implementer of `App` only content management.
struct HeadlessCtrl {
    vars: WindowVars,
    content: ContentCtrl,

    headless_monitor: HeadlessMonitor,
    headless_simulator: HeadlessSimulator,
}
impl HeadlessCtrl {
    pub fn new(vars: &WindowVars, content: Window) -> Self {
        Self {
            vars: vars.clone(),
            headless_monitor: content.headless_monitor,
            content: ContentCtrl::new(vars.clone(), content),
            headless_simulator: HeadlessSimulator::new(),
        }
    }

    pub fn update(&mut self, ctx: &mut WindowContext) {
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

        self.content.update(ctx);
    }

    pub fn window_updates(&mut self, ctx: &mut WindowContext, updates: WindowUpdates) {
        self.content.window_updates(ctx, updates);
    }

    pub fn pre_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        self.content.pre_event(ctx, args);
        self.headless_simulator.pre_event(ctx, args);
    }

    pub fn ui_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        self.content.ui_event(ctx, args);
    }

    pub fn layout(&mut self, ctx: &mut WindowContext) {
        if !self.content.layout_requested {
            return;
        }

        let scale_factor = self.headless_monitor.scale_factor;
        let screen_ppi = self.headless_monitor.ppi;
        let screen_size = self.headless_monitor.size.to_px(scale_factor.0);

        let (min_size, max_size, size) = self.content.outer_layout(ctx, scale_factor, screen_ppi, screen_size, |ctx| {
            let available_size = AvailableSize::finite(screen_size);

            let min_size = self
                .vars
                .min_size()
                .get(ctx.vars)
                .to_layout(ctx.metrics, available_size, default_min_size(scale_factor));

            let max_size = self
                .vars
                .max_size()
                .get(ctx.vars)
                .to_layout(ctx.metrics, available_size, screen_size);

            let size = self
                .vars
                .size()
                .get(ctx.vars)
                .to_layout(ctx.metrics, available_size, default_size(scale_factor));

            (min_size, max_size, size.min(max_size).max(min_size))
        });

        let _surface_size = self.content.layout(ctx, scale_factor, screen_ppi, min_size, max_size, size, false);

        self.headless_simulator.layout(ctx);
    }

    pub fn render(&mut self, ctx: &mut WindowContext) {
        if self.content.render_requested.is_none() {
            return;
        }
        self.content.render(ctx, None, self.headless_monitor.scale_factor, None);
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

    fn is_enabled(&mut self, ctx: &mut WindowContext) -> bool {
        *self
            .is_enabled
            .get_or_insert_with(|| crate::app::App::window_mode(ctx.services).is_headless())
    }

    pub fn pre_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        if self.is_enabled(ctx) && self.is_open && ViewProcessRespawnedEvent.update(args).is_some() {
            self.is_open = false;
        }
    }

    pub fn layout(&mut self, ctx: &mut WindowContext) {
        if self.is_enabled(ctx) && !self.is_open {
            self.is_open = true;

            let timestamp = Instant::now();

            // simulate focus:
            if let Some(prev_focus_id) = ctx.services.windows().focused_window_id() {
                let args = RawWindowFocusArgs::new(timestamp, prev_focus_id, false);
                RawWindowFocusEvent.notify(ctx.events, args)
            }
            let args = RawWindowFocusArgs::new(timestamp, *ctx.window_id, true);
            RawWindowFocusEvent.notify(ctx.events, args)
        }
    }
}

/// Implementer of window UI node tree initialization and management.
struct ContentCtrl {
    vars: WindowVars,

    root_id: WidgetId,
    root_state: OwnedStateMap,
    root_transform_key: WidgetTransformKey,
    root: BoxedUiNode,
    // info
    root_outer_bounds: BoundsInfo,
    root_inner_bounds: BoundsInfo,
    root_rendered: WidgetRendered,
    used_info_builder: Option<UsedWidgetInfoBuilder>,

    prev_metrics: Option<(Px, Factor, f32, PxSize)>,
    used_frame_builder: Option<UsedFrameBuilder>,
    used_frame_update: Option<UsedFrameUpdate>,

    inited: bool,
    subscriptions: WidgetSubscriptions,
    frame_id: FrameId,
    clear_color: RenderColor,

    is_rendering: bool,
    pending_render: WindowRenderUpdate,

    layout_requested: bool,
    render_requested: WindowRenderUpdate,
}
impl ContentCtrl {
    pub fn new(vars: WindowVars, window: Window) -> Self {
        Self {
            vars,

            root_id: window.id,
            root_state: OwnedStateMap::new(),
            root_transform_key: WidgetTransformKey::new_unique(),
            root: window.child,

            root_outer_bounds: BoundsInfo::new(),
            root_inner_bounds: BoundsInfo::new(),
            root_rendered: WidgetRendered::new(),
            used_info_builder: None,

            prev_metrics: None,
            used_frame_builder: None,
            used_frame_update: None,

            inited: false,
            subscriptions: WidgetSubscriptions::new(),
            frame_id: FrameId::INVALID,
            clear_color: RenderColor::BLACK,

            is_rendering: false,
            pending_render: WindowRenderUpdate::None,

            layout_requested: false,
            render_requested: WindowRenderUpdate::None,
        }
    }

    pub fn update(&mut self, ctx: &mut WindowContext) {
        if !self.inited {
            ctx.widget_context(self.root_id, &mut self.root_state, |ctx| {
                self.root.init(ctx);

                ctx.updates.info();
                ctx.updates.subscriptions();
            });
            self.inited = true;
        } else {
            ctx.widget_context(self.root_id, &mut self.root_state, |ctx| {
                self.root.update(ctx);
            })
        }
    }

    pub fn window_updates(&mut self, ctx: &mut WindowContext, updates: WindowUpdates) {
        self.layout_requested |= updates.layout;
        self.render_requested |= updates.render;

        if updates.info {
            let mut info = WidgetInfoBuilder::new(
                *ctx.window_id,
                self.root_id,
                self.root_outer_bounds.clone(),
                self.root_rendered.clone(),
                self.used_info_builder.take(),
            );

            ctx.info_context(self.root_id, &self.root_state, |ctx| {
                self.root.info(ctx, &mut info);
            });

            let (info, used) = info.finalize();
            self.used_info_builder = Some(used);

            ctx.services
                .windows()
                .set_widget_tree(ctx.events, info, self.layout_requested, !self.render_requested.is_none());
        }

        if updates.subscriptions {
            self.subscriptions = ctx.info_context(self.root_id, &self.root_state, |ctx| {
                let mut subscriptions = WidgetSubscriptions::new();
                self.root.subscriptions(ctx, &mut subscriptions);
                subscriptions
            });
        }
    }

    pub fn pre_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        if let Some(args) = RawFrameRenderedEvent.update(args) {
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
                let args = FrameImageReadyArgs::new(args.timestamp, args.window_id, args.frame_id, image);
                FrameImageReadyEvent.notify(ctx.events, args);
            }
        }
    }

    pub fn ui_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        debug_assert!(self.inited);

        if self.subscriptions.event_contains(args) {
            ctx.widget_context(self.root_id, &mut self.root_state, |ctx| {
                self.root.event(ctx, args);
            });
        } else {
        }
    }

    pub fn close(&mut self, ctx: &mut WindowContext) {
        ctx.widget_context(self.root_id, &mut self.root_state, |ctx| {
            self.root.deinit(ctx);
        });

        self.vars.0.is_open.set(ctx, false);
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
            LayoutMask::LAYOUT_METRICS,
            self.root_id,
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

        let auto_size = self.vars.auto_size().copy(ctx);

        let mut viewport_size = size;
        if !skip_auto_size {
            if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                viewport_size.width = max_size.width;
            }
            if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                viewport_size.height = max_size.height;
            }
        }

        let mut changes = LayoutMask::NONE;
        if let Some((prev_font_size, prev_scale_factor, prev_screen_ppi, prev_viewport_size)) = self.prev_metrics {
            if prev_font_size != base_font_size {
                changes |= LayoutMask::FONT_SIZE;
            }
            if prev_scale_factor != scale_factor {
                changes |= LayoutMask::SCALE_FACTOR;
            }
            if !about_eq(prev_screen_ppi, screen_ppi, 0.001) {
                changes |= LayoutMask::SCREEN_PPI;
            }
            if prev_viewport_size != viewport_size {
                changes |= LayoutMask::VIEWPORT_SIZE;
            }
        } else {
            changes = LayoutMask::FONT_SIZE | LayoutMask::SCALE_FACTOR | LayoutMask::SCREEN_PPI;
        }
        self.prev_metrics = Some((base_font_size, scale_factor, screen_ppi, viewport_size));

        ctx.layout_context(
            base_font_size,
            scale_factor,
            screen_ppi,
            viewport_size,
            changes,
            self.root_id,
            &mut self.root_state,
            |ctx| {
                let mut available_size = AvailableSize::finite(viewport_size);
                if !skip_auto_size {
                    if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                        available_size.width = AvailablePx::Infinite;
                    }
                    if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                        available_size.height = AvailablePx::Infinite;
                    }
                }

                let desired_size = self.root.measure(ctx, available_size);

                let mut final_size = viewport_size;
                if !skip_auto_size {
                    if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                        final_size.width = desired_size.width.max(min_size.width).min(max_size.width);
                    }
                    if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                        final_size.height = desired_size.height.max(min_size.height).min(max_size.height);
                    }
                }

                WidgetLayout::with_root_widget(&self.root_outer_bounds, &self.root_inner_bounds, final_size, |wl| {
                    self.root.arrange(ctx, wl, final_size);
                });

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

                let mut frame = FrameBuilder::new(
                    self.frame_id,
                    *ctx.window_id,
                    renderer.clone(),
                    self.root_id,
                    self.root_transform_key,
                    scale_factor,
                    self.used_frame_builder.take(),
                );

                let (frame, used) = ctx.render_context(self.root_id, &self.root_state, |ctx| {
                    self.root.render(ctx, &mut frame);
                    frame.finalize(&self.root_rendered)
                });

                self.used_frame_builder = Some(used);

                self.clear_color = frame.clear_color;

                let capture_image = self.take_capture_image(ctx.vars);

                if let Some(renderer) = renderer {
                    let _: Ignore = renderer.render(FrameRequest {
                        id: self.frame_id,
                        pipeline_id: frame.pipeline_id,
                        document_id: renderer
                            .document_id()
                            .unwrap_or(zero_ui_view_api::webrender_api::DocumentId::INVALID),
                        clear_color: self.clear_color,
                        display_list: {
                            let (payload, descriptor) = frame.display_list;
                            (
                                IpcBytes::from_vec(payload.items_data),
                                IpcBytes::from_vec(payload.cache_data),
                                IpcBytes::from_vec(payload.spatial_tree),
                                descriptor,
                            )
                        },
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
                    *ctx.window_id,
                    self.root_id,
                    self.root_transform_key,
                    self.frame_id,
                    self.clear_color,
                    self.used_frame_update.take(),
                );

                ctx.render_context(self.root_id, &self.root_state, |ctx| {
                    self.root.render_update(ctx, &mut update);
                });

                let (update, used) = update.finalize();
                self.used_frame_update = Some(used);

                if let Some(c) = update.clear_color {
                    self.clear_color = c;
                }

                let capture_image = self.take_capture_image(ctx.vars);

                if let Some(renderer) = renderer {
                    let _: Ignore = renderer.render_update(FrameUpdateRequest {
                        id: self.frame_id,
                        updates: update.bindings,
                        scroll_updates: update.scrolls,
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
        match self.vars.frame_capture_mode().copy(vars) {
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
    pub fn new(vars: &WindowVars, mode: WindowMode, content: Window) -> Self {
        WindowCtrl(match mode {
            WindowMode::Headed => WindowCtrlMode::Headed(HeadedCtrl::new(vars, content)),
            WindowMode::Headless => WindowCtrlMode::Headless(HeadlessCtrl::new(vars, content)),
            WindowMode::HeadlessWithRenderer => WindowCtrlMode::HeadlessWithRenderer(HeadlessWithRendererCtrl::new(vars, content)),
        })
    }

    pub fn update(&mut self, ctx: &mut WindowContext) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.update(ctx),
            WindowCtrlMode::Headless(c) => c.update(ctx),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.update(ctx),
        }
    }

    pub fn window_updates(&mut self, ctx: &mut WindowContext, updates: WindowUpdates) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.window_updates(ctx, updates),
            WindowCtrlMode::Headless(c) => c.window_updates(ctx, updates),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.window_updates(ctx, updates),
        }
    }

    pub fn pre_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.pre_event(ctx, args),
            WindowCtrlMode::Headless(c) => c.pre_event(ctx, args),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.pre_event(ctx, args),
        }
    }

    pub fn ui_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        match &mut self.0 {
            WindowCtrlMode::Headed(c) => c.ui_event(ctx, args),
            WindowCtrlMode::Headless(c) => c.ui_event(ctx, args),
            WindowCtrlMode::HeadlessWithRenderer(c) => c.ui_event(ctx, args),
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
type Ignore = Result<(), Respawned>;
