//! This module implements Management of window content and synchronization of WindowVars and View-Process.

use std::mem;

use crate::{
    app::view_process::*,
    context::{WindowContext, WindowRenderUpdate, WindowUpdates},
    event::EventUpdateArgs,
    render::{FrameBuilder, FrameUpdate},
    state::OwnedStateMap,
    units::*,
    widget_info::{BoundsRect, UsedWidgetInfoBuilder, WidgetInfoBuilder, WidgetRendered, WidgetSubscriptions},
    BoxedUiNode, UiNode, WidgetId,
};

use super::{HeadlessMonitor, StartPosition, Window, WindowMode, WindowVars};

/// Implementer of `App <-> View` sync in a headed window.
struct HeadedCtrl {
    window: Option<ViewWindow>,
    vars: WindowVars,

    content: ContentCtrl,

    // init config.
    start_position: StartPosition,
    kiosk: bool,
    transparent: bool,
    render_mode: Option<RenderMode>,

    // current state.
    state: WindowStateAll,
}
impl HeadedCtrl {
    pub fn new(ctx: &mut WindowContext, vars: &WindowVars, content: Window) -> Self {
        Self {
            window: None,

            start_position: content.start_position,
            kiosk: content.kiosk,
            transparent: content.transparent,
            render_mode: content.render_mode,

            content: ContentCtrl::new(content),
            vars: vars.clone(),

            state: WindowStateAll {
                state: vars.state().copy(ctx),
                restore_rect: vars.0.restore_rect.copy(ctx),
                restore_state: vars.0.restore_state.copy(ctx),
                min_size: DipSize::zero(),
                max_size: DipSize::zero(),
                chrome_visible: vars.chrome().get(ctx).is_default(),
            },
        }
    }

    pub fn update(&mut self, ctx: &mut WindowContext) {
        self.content.update(ctx);

        if self.is_pre_init() {
            ctx.updates.layout();
        }
    }

    pub fn window_updates(&mut self, ctx: &mut WindowContext, updates: WindowUpdates) {
        self.content.window_updates(ctx, updates);
    }

    pub fn event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        self.content.event(ctx, args);
    }

    pub fn layout(&mut self, ctx: &mut WindowContext) {
        self.content.layout(ctx);
    }

    pub fn render(&mut self, ctx: &mut WindowContext) {
        self.content.render(ctx);
    }

    pub fn respawn(&mut self, ctx: &mut WindowContext) {
        self.content.respawn(ctx);
    }

    fn is_pre_init(&self) -> bool {
        self.window.is_none()
    }

    fn open(&mut self, ctx: &mut WindowContext) {
        let view = ctx.services.view_process();
        todo!()
    }
}

/// Implementer of `App <-> View` sync in a headless window.
struct HeadlessWithRendererCtrl {
    surface: Option<ViewHeadless>,
    vars: WindowVars,
    content: ContentCtrl,

    // init config.
    transparent: bool,
    render_mode: Option<RenderMode>,
    headless_monitor: HeadlessMonitor,

    // current state.
    size: DipSize,
}
impl HeadlessWithRendererCtrl {
    pub fn new(ctx: &mut WindowContext, vars: &WindowVars, content: Window) -> Self {
        Self {
            surface: None,
            vars: vars.clone(),

            transparent: content.transparent,
            render_mode: content.render_mode,
            headless_monitor: content.headless_monitor,

            content: ContentCtrl::new(content),

            size: DipSize::zero(),
        }
    }

    pub fn update(&mut self, ctx: &mut WindowContext) {
        self.content.update(ctx);
    }

    pub fn window_updates(&mut self, ctx: &mut WindowContext, updates: WindowUpdates) {
        self.content.window_updates(ctx, updates);
    }

    pub fn event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        self.content.event(ctx, args);
    }

    pub fn layout(&mut self, ctx: &mut WindowContext) {
        self.content.layout(ctx);
    }

    pub fn render(&mut self, ctx: &mut WindowContext) {
        self.content.render(ctx);
    }

    pub fn respawn(&mut self, ctx: &mut WindowContext) {
        self.content.respawn(ctx);
    }
}

/// implementer of `App` only content management.
struct HeadlessCtrl {
    vars: WindowVars,
    content: ContentCtrl,

    headless_monitor: HeadlessMonitor,
}
impl HeadlessCtrl {
    pub fn new(ctx: &mut WindowContext, vars: &WindowVars, content: Window) -> Self {
        Self {
            vars: vars.clone(),
            headless_monitor: content.headless_monitor,
            content: ContentCtrl::new(content),
        }
    }

    pub fn update(&mut self, ctx: &mut WindowContext) {
        self.content.update(ctx);
    }

    pub fn window_updates(&mut self, ctx: &mut WindowContext, updates: WindowUpdates) {
        self.content.window_updates(ctx, updates);
    }

    pub fn event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        self.content.event(ctx, args);
    }

    pub fn layout(&mut self, ctx: &mut WindowContext) {
        self.content.layout(ctx);
    }

    pub fn render(&mut self, ctx: &mut WindowContext) {
        self.content.render(ctx);
    }

    pub fn respawn(&mut self, ctx: &mut WindowContext) {
        self.content.respawn(ctx);
    }
}

/// Implementer of window UI node tree initialization and management.
struct ContentCtrl {
    root_id: WidgetId,
    root_state: OwnedStateMap,
    root: BoxedUiNode,
    // info
    root_bounds: BoundsRect,
    root_rendered: WidgetRendered,
    used_info_builder: Option<UsedWidgetInfoBuilder>,

    inited: bool,
    subscriptions: WidgetSubscriptions,

    layout_requested: bool,
    render_requested: WindowRenderUpdate,
}
impl ContentCtrl {
    pub fn new(window: Window) -> Self {
        Self {
            root_id: window.id,
            root_state: OwnedStateMap::new(),
            root: window.child,
            root_bounds: BoundsRect::new(),
            root_rendered: WidgetRendered::new(),
            used_info_builder: None,

            inited: false,
            subscriptions: WidgetSubscriptions::new(),

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

    pub fn event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        debug_assert!(self.inited);

        if self.subscriptions.event_contains(args) {
            ctx.widget_context(self.root_id, &mut self.root_state, |ctx| {
                self.root.event(ctx, args);
            });
        }
    }

    pub fn layout(&mut self, ctx: &mut WindowContext) {
        debug_assert!(self.inited);

        if mem::take(&mut self.layout_requested) {
            let root_font_size = 13.pt().to_layout(ctx, available_size, default_value);

            ctx.layout_context(
                font_size,
                scale_factor,
                screen_ppi,
                viewport_size,
                metrics_diff,
                self.root_id,
                &mut self.root_state,
                |ctx| todo!(),
            );
        }
    }

    pub fn render(&mut self, ctx: &mut WindowContext) {
        match mem::take(&mut self.render_requested) {
            WindowRenderUpdate::Render => {
                let mut frame = FrameBuilder::new(frame_id, window_id, renderer, root_id, root_transform_key, scale_factor, used_data);

                ctx.render_context(self.root_id, &mut self.root_state, |ctx| {
                    self.root.render(ctx, &mut frame);
                });

                todo!()
            }
            WindowRenderUpdate::RenderUpdate => {
                let mut update = FrameUpdate::new(window_id, root_id, root_transform_key, frame_id, clear_color, used_data);

                ctx.render_context(self.root_id, &mut self.root_state, |ctx| {
                    self.root.render_update(ctx, &mut update);
                });

                todo!()
            }
            WindowRenderUpdate::None => {}
        }
    }

    pub fn respawn(&mut self, ctx: &mut WindowContext) {
        todo!()
    }
}

/// Management of window content and synchronization of WindowVars and View-Process.
pub(super) enum WindowCtrl {
    Headed(HeadedCtrl),
    Headless(HeadlessCtrl),
    HeadlessWithRenderer(HeadlessWithRendererCtrl),
}
impl WindowCtrl {
    pub fn new(ctx: &mut WindowContext, vars: &WindowVars, mode: WindowMode, content: Window) -> Self {
        match mode {
            WindowMode::Headed => WindowCtrl::Headed(HeadedCtrl::new(ctx, vars, content)),
            WindowMode::Headless => WindowCtrl::Headless(HeadlessCtrl::new(ctx, vars, content)),
            WindowMode::HeadlessWithRenderer => WindowCtrl::HeadlessWithRenderer(HeadlessWithRendererCtrl::new(ctx, vars, content)),
        }
    }

    pub fn update(&mut self, ctx: &mut WindowContext) {
        match self {
            WindowCtrl::Headed(c) => c.update(ctx),
            WindowCtrl::Headless(c) => c.update(ctx),
            WindowCtrl::HeadlessWithRenderer(c) => c.update(ctx),
        }
    }

    pub fn window_updates(&mut self, ctx: &mut WindowContext, updates: WindowUpdates) {
        match self {
            WindowCtrl::Headed(c) => c.window_updates(ctx, updates),
            WindowCtrl::Headless(c) => c.window_updates(ctx, updates),
            WindowCtrl::HeadlessWithRenderer(c) => c.window_updates(ctx, updates),
        }
    }

    pub fn event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        match self {
            WindowCtrl::Headed(c) => c.event(ctx, args),
            WindowCtrl::Headless(c) => c.event(ctx, args),
            WindowCtrl::HeadlessWithRenderer(c) => c.event(ctx, args),
        }
    }

    pub fn layout(&mut self, ctx: &mut WindowContext) {
        match self {
            WindowCtrl::Headed(c) => c.layout(ctx),
            WindowCtrl::Headless(c) => c.layout(ctx),
            WindowCtrl::HeadlessWithRenderer(c) => c.layout(ctx),
        }
    }

    pub fn render(&mut self, ctx: &mut WindowContext) {
        match self {
            WindowCtrl::Headed(c) => c.render(ctx),
            WindowCtrl::Headless(c) => c.render(ctx),
            WindowCtrl::HeadlessWithRenderer(c) => c.render(ctx),
        }
    }

    pub fn respawn(&mut self, ctx: &mut WindowContext) {
        match self {
            WindowCtrl::Headed(c) => c.respawn(ctx),
            WindowCtrl::Headless(c) => c.respawn(ctx),
            WindowCtrl::HeadlessWithRenderer(c) => c.respawn(ctx),
        }
    }
}
