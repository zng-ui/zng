//! This module implements Management of window content and synchronization of WindowVars and View-Process.

use std::mem;

use crate::{
    app::{
        raw_events::{RawFrameRenderedEvent, RawMonitorsChangedEvent, RawScaleFactorChangedEvent, RawWindowChangedEvent},
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
    widget_info::{BoundsRect, UsedWidgetInfoBuilder, WidgetInfoBuilder, WidgetOffset, WidgetRendered, WidgetSubscriptions},
    window::AutoSize,
    BoxedUiNode, UiNode, WidgetId,
};

use super::{
    FrameCaptureMode, FrameImageReadyArgs, FrameImageReadyEvent, HeadlessMonitor, MonitorsExt, StartPosition, Window, WindowIcon,
    WindowMode, WindowScaleChangedArgs, WindowScaleChangedEvent, WindowVars, WindowsExt,
};

/// Implementer of `App <-> View` sync in a headed window.
struct HeadedCtrl {
    window: Option<ViewWindow>,
    vars: WindowVars,

    content: ContentCtrl,

    // init config.
    start_position: StartPosition,
    kiosk: Option<WindowState>, // Some(enforced_fullscreen)
    transparent: bool,
    render_mode: Option<RenderMode>,

    // current state.
    state: Option<WindowStateAll>, // None means it must be recomputed and send.
    scale_factor: Factor,
    resize_wait_id: Option<FrameWaitId>,
    icon: Option<ImageVar>,
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

            state: None,
            scale_factor: 1.fct(),
            resize_wait_id: None,
            icon: None,
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
            } else {
                if let Some(query) = self.vars.monitor().get_new(ctx.vars) {
                    if let Some(new) = query.select(ctx.services.monitors()).map(|m| m.id) {
                        let current = self.vars.0.actual_monitor.copy(ctx.vars);
                        if Some(new) != current {
                            // TODO, see vars.monitor() docs
                            match self.vars.state().copy(ctx) {
                                WindowState::Normal => todo!(),
                                WindowState::Minimized => todo!(),
                                WindowState::Maximized => todo!(),
                                WindowState::Fullscreen => todo!(),
                                WindowState::Exclusive => todo!(),
                            }
                        }
                    }
                }

                if self.vars.size().is_new(ctx)
                    || self.vars.min_size().is_new(ctx)
                    || self.vars.max_size().is_new(ctx)
                    || self.vars.auto_size().is_new(ctx)
                {
                    todo!()
                }

                if self.vars.position().is_new(ctx) {
                    todo!() // related to monitor too
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

                if let Some(visible) = self.vars.visible().copy_new(ctx) {
                    let _: Ignore = view.set_visible(visible);
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

                if let Some(movable) = self.vars.movable().copy_new(ctx) {
                    let _: Ignore = view.set_movable(movable);
                }

                if let Some(resizable) = self.vars.resizable().copy_new(ctx) {
                    let _: Ignore = view.set_resizable(resizable);
                }

                if let Some(mode) = self.vars.frame_capture_mode().copy_new(ctx.vars) {
                    let _: Ignore = view.set_capture_mode(matches!(mode, FrameCaptureMode::All));
                }

                if let Some(allow) = self.vars.allow_alt_f4().copy_new(ctx) {
                    let _: Ignore = view.set_allow_alt_f4(allow);
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
            todo!("")
        } else if let Some(args) = RawScaleFactorChangedEvent.update(args) {
            if args.windows.contains(ctx.window_id) {
                let args = WindowScaleChangedArgs::new(args.timestamp, *ctx.window_id, args.scale_factor);
                WindowScaleChangedEvent.notify(ctx.events, args);

                self.content.layout_requested = true;
                self.content.render_requested = WindowRenderUpdate::Render;

                ctx.updates.layout_and_render();
            }
        } else if let Some(args) = ViewProcessRespawnedEvent.update(args) {
            if let Some(view) = &self.window {
                if view.renderer().generation() == Ok(args.generation) {
                    self.content.layout_requested = true;
                    self.content.render_requested = WindowRenderUpdate::Render;

                    ctx.updates.layout_and_render();

                    self.window = None;
                }
            }
        } else if let Some(args) = RawMonitorsChangedEvent.update(args) {
            todo!("revalidate window monitor for {args:?}");
        }

        self.content.pre_event(ctx, args);
    }

    pub fn ui_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        self.content.ui_event(ctx, args);
    }

    pub fn layout(&mut self, ctx: &mut WindowContext) {
        let state = self.vars.state().copy(ctx);
        if matches!(state, WindowState::Minimized) && self.is_inited() {
            return;
        }

        let skip_auto_size = matches!(state, WindowState::Normal);

        let mut screen_ppi = 96.0;
        let mut screen_size = PxSize::new(Px(800), Px(600));
        self.scale_factor = 1.fct();
        if let Some(id) = self.vars.0.actual_monitor.copy(ctx) {
            if let Some(info) = ctx.services.monitors().monitor(id) {
                screen_ppi = info.ppi.copy(ctx.vars);
                screen_size = info.info.size;
                self.scale_factor = info.info.scale_factor.fct();
            }
        }

        let (min_size, max_size, size) = self.content.outer_layout(ctx, self.scale_factor, screen_ppi, screen_size, |ctx| {
            let available_size = AvailableSize::finite(screen_size);

            let default_min = DipSize::new(Dip::new(192), Dip::new(48)).to_px(self.scale_factor.0);
            let min_size = self
                .vars
                .min_size()
                .get(ctx.vars)
                .to_layout(ctx.metrics, available_size, default_min);
            let max_size = self
                .vars
                .max_size()
                .get(ctx.vars)
                .to_layout(ctx.metrics, available_size, screen_size);
            let default_size = DipSize::new(Dip::new(800), Dip::new(600)).to_px(self.scale_factor.0);
            let size = self.vars.size().get(ctx.vars).to_layout(ctx.metrics, available_size, default_size);

            (min_size, max_size, size.min(min_size).max(max_size))
        });

        let size = self
            .content
            .layout(ctx, self.scale_factor, screen_ppi, min_size, max_size, size, skip_auto_size);

        if let Some(size) = size {}
    }

    pub fn render(&mut self, ctx: &mut WindowContext) {
        if let Some(view) = &self.window {
            self.content
                .render(ctx, Some(view.renderer()), self.scale_factor, self.resize_wait_id.take());
        }
    }

    pub fn close(&mut self, ctx: &mut WindowContext) {
        self.content.close(ctx);
        self.window = None;
    }

    fn is_inited(&self) -> bool {
        self.window.is_none()
    }

    fn open(&mut self, ctx: &mut WindowContext) {
        let view = ctx.services.view_process();
        todo!()
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
                    self.content.layout_requested = true;
                    self.content.render_requested = WindowRenderUpdate::Render;

                    ctx.updates.layout_and_render();

                    self.surface = None;
                }
            }
        }

        self.content.pre_event(ctx, args);
    }

    pub fn ui_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        self.content.ui_event(ctx, args);
    }

    pub fn layout(&mut self, ctx: &mut WindowContext) {
        let scale_factor = self.headless_monitor.scale_factor;
        let screen_ppi = self.headless_monitor.ppi;
        let screen_size = self.headless_monitor.size.to_px(scale_factor.0);

        let (min_size, max_size, size) = self.content.outer_layout(ctx, scale_factor, screen_ppi, screen_size, |ctx| {
            let available_size = AvailableSize::finite(screen_size);

            let default_min = DipSize::new(Dip::new(192), Dip::new(48)).to_px(scale_factor.0);
            let min_size = self
                .vars
                .min_size()
                .get(ctx.vars)
                .to_layout(ctx.metrics, available_size, default_min);
            let max_size = self
                .vars
                .max_size()
                .get(ctx.vars)
                .to_layout(ctx.metrics, available_size, screen_size);
            let default_size = DipSize::new(Dip::new(800), Dip::new(600)).to_px(scale_factor.0);
            let size = self.vars.size().get(ctx.vars).to_layout(ctx.metrics, available_size, default_size);

            (min_size, max_size, size.min(min_size).max(max_size))
        });

        let size = self.content.layout(ctx, scale_factor, screen_ppi, min_size, max_size, size, false);
        if let Some(size) = size {
            // did layout:

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
                }
                // else `Respawn`, handled in `pre_event`.
            }
        }
    }

    pub fn render(&mut self, ctx: &mut WindowContext) {
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
}
impl HeadlessCtrl {
    pub fn new(vars: &WindowVars, content: Window) -> Self {
        Self {
            vars: vars.clone(),
            headless_monitor: content.headless_monitor,
            content: ContentCtrl::new(vars.clone(), content),
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

        self.content.update(ctx);
    }

    pub fn window_updates(&mut self, ctx: &mut WindowContext, updates: WindowUpdates) {
        self.content.window_updates(ctx, updates);
    }

    pub fn pre_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        self.content.pre_event(ctx, args);
    }

    pub fn ui_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        self.content.ui_event(ctx, args);
    }

    pub fn layout(&mut self, ctx: &mut WindowContext) {
        let scale_factor = self.headless_monitor.scale_factor;
        let screen_ppi = self.headless_monitor.ppi;
        let screen_size = self.headless_monitor.size.to_px(scale_factor.0);

        let (min_size, max_size, size) = self.content.outer_layout(ctx, scale_factor, screen_ppi, screen_size, |ctx| {
            let available_size = AvailableSize::finite(screen_size);

            let default_min = DipSize::new(Dip::new(192), Dip::new(48)).to_px(scale_factor.0);
            let min_size = self
                .vars
                .min_size()
                .get(ctx.vars)
                .to_layout(ctx.metrics, available_size, default_min);
            let max_size = self
                .vars
                .max_size()
                .get(ctx.vars)
                .to_layout(ctx.metrics, available_size, screen_size);
            let default_size = DipSize::new(Dip::new(800), Dip::new(600)).to_px(scale_factor.0);
            let size = self.vars.size().get(ctx.vars).to_layout(ctx.metrics, available_size, default_size);

            (min_size, max_size, size.min(min_size).max(max_size))
        });

        let _surface_size = self.content.layout(ctx, scale_factor, screen_ppi, min_size, max_size, size, false);
    }

    pub fn render(&mut self, ctx: &mut WindowContext) {
        self.content.render(ctx, None, self.headless_monitor.scale_factor, None);
    }

    pub fn close(&mut self, ctx: &mut WindowContext) {
        self.content.close(ctx);
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
    root_bounds: BoundsRect,
    root_rendered: WidgetRendered,
    used_info_builder: Option<UsedWidgetInfoBuilder>,

    prev_metrics: Option<(Px, Factor, f32, PxSize)>,
    used_frame_builder: Option<UsedFrameBuilder>,
    used_frame_update: Option<UsedFrameUpdate>,

    inited: bool,
    deinited: bool,
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

            root_bounds: BoundsRect::new(),
            root_rendered: WidgetRendered::new(),
            used_info_builder: None,

            prev_metrics: None,
            used_frame_builder: None,
            used_frame_update: None,

            inited: false,
            deinited: false,
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
                ctx.updates.layout();
                ctx.updates.render();
            });
            self.inited = true;
        } else {
            ctx.widget_context(self.root_id, &mut self.root_state, |ctx| {
                self.root.update(ctx);
            })
        }
    }

    pub fn window_updates(&mut self, ctx: &mut WindowContext, updates: WindowUpdates) {
        if updates.info {
            let mut info = WidgetInfoBuilder::new(
                *ctx.window_id,
                self.root_id,
                self.root_bounds.clone(),
                self.root_rendered.clone(),
                self.used_info_builder.take(),
            );

            ctx.info_context(self.root_id, &mut self.root_state, |ctx| {
                self.root.info(ctx, &mut info);
            });

            let (info, used) = info.finalize();
            self.used_info_builder = Some(used);
        }

        if updates.subscriptions {
            self.subscriptions = ctx.info_context(self.root_id, &mut self.root_state, |ctx| {
                let mut subscriptions = WidgetSubscriptions::new();
                self.root.subscriptions(ctx, &mut subscriptions);
                subscriptions
            });
        }

        self.layout_requested |= updates.layout;
        self.render_requested |= updates.render;
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
    pub fn layout(
        &mut self,
        ctx: &mut WindowContext,
        scale_factor: Factor,
        screen_ppi: f32,
        min_size: PxSize,
        max_size: PxSize,
        size: PxSize,
        skip_auto_size: bool,
    ) -> Option<PxSize> {
        debug_assert!(self.inited);

        if mem::take(&mut self.layout_requested) {
            let _s = tracing::trace_span!("window.on_layout", window = %ctx.window_id.sequential()).entered();

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

            let final_size = ctx.layout_context(
                base_font_size,
                scale_factor,
                screen_ppi,
                viewport_size,
                changes,
                self.root_id,
                &mut self.root_state,
                |ctx| {
                    let desired_size = self.root.measure(ctx, AvailableSize::finite(viewport_size));

                    let mut final_size = viewport_size;
                    if !skip_auto_size {
                        if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                            final_size.width = desired_size.width.max(min_size.width).min(max_size.width);
                        }
                        if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                            final_size.height = desired_size.height.max(min_size.height).min(max_size.height);
                        }
                    }

                    self.root.arrange(ctx, &mut WidgetOffset::new(), final_size);

                    final_size
                },
            );

            Some(final_size)
        } else {
            None
        }
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

                let (frame, used) = ctx.render_context(self.root_id, &mut self.root_state, |ctx| {
                    self.root.render(ctx, &mut frame);
                    frame.finalize(ctx, &self.root_rendered)
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

                ctx.render_context(self.root_id, &mut self.root_state, |ctx| {
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
            WindowRenderUpdate::None => {}
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

/// Respawned error is ok here, because we recreate the window/surface on respawn.
type Ignore = Result<(), Respawned>;
