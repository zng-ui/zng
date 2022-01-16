//! This module implements Management of window content and synchronization of WindowVars and View-Process.

use crate::{
    app::view_process::*,
    context::{WindowContext, WindowRenderUpdate},
    event::EventUpdateArgs,
    state::OwnedStateMap,
    units::*,
    widget_info::{BoundsRect, UsedWidgetInfoBuilder, WidgetInfoBuilder, WidgetRendered},
    BoxedUiNode, UiNode, WidgetId,
};

use super::{Window, WindowMode, WindowVars, StartPosition, HeadlessMonitor};

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

    pub fn on_update(&mut self, ctx: &mut WindowContext) {
        if self.is_pre_init() {
            ctx.updates.layout();
            self.content.layout_request = true;
            return;
        }

        todo!()
    }

    pub fn on_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        todo!()
    }

    pub fn on_layout(&mut self, ctx: &mut WindowContext) {
        todo!()
    }

    pub fn on_render(&mut self, ctx: &mut WindowContext) {
        todo!()
    }

    pub fn on_respawn(&mut self, ctx: &mut WindowContext) {
        todo!()
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

    pub fn on_update(&mut self, ctx: &mut WindowContext) {
        todo!()
    }

    pub fn on_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        todo!()
    }

    pub fn on_layout(&mut self, ctx: &mut WindowContext) {
        todo!()
    }

    pub fn on_render(&mut self, ctx: &mut WindowContext) {
        todo!()
    }

    pub fn on_respawn(&mut self, ctx: &mut WindowContext) {
        todo!()
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

    pub fn on_update(&mut self, ctx: &mut WindowContext) {
        todo!()
    }

    pub fn on_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        todo!()
    }

    pub fn on_layout(&mut self, ctx: &mut WindowContext) {
        todo!()
    }

    pub fn on_render(&mut self, ctx: &mut WindowContext) {
        todo!()
    }

    pub fn on_respawn(&mut self, ctx: &mut WindowContext) {
        todo!()
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

    layout_request: bool,
    render_request: WindowRenderUpdate,
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

            layout_request: true,
            render_request: WindowRenderUpdate::Render,
        }
    }

    pub fn on_update(&mut self, ctx: &mut WindowContext) {
        if !self.inited {
            ctx.widget_context(self.root_id, &mut self.root_state, |ctx| {
                self.root.init(ctx);
            });
            self.inited = true;

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

            todo!()
        }

        todo!()
    }

    pub fn on_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        debug_assert!(self.inited);

        ctx.widget_context(self.root_id, &mut self.root_state, |ctx| {
            self.root.event(ctx, args);
        });
    }

    pub fn on_layout(&mut self, ctx: &mut WindowContext) {
        debug_assert!(self.inited);

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

    pub fn on_render(&mut self, ctx: &mut WindowContext) {
        todo!()
    }

    pub fn on_render_update(&mut self, ctx: &mut WindowContext) {
        todo!()
    }

    pub fn on_respawn(&mut self, ctx: &mut WindowContext) {
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

    pub fn on_update(&mut self, ctx: &mut WindowContext) {
        match self {
            WindowCtrl::Headed(c) => c.on_update(ctx),
            WindowCtrl::Headless(c) => c.on_update(ctx),
            WindowCtrl::HeadlessWithRenderer(c) => c.on_update(ctx),
        }
    }

    pub fn on_event<EV: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, args: &EV) {
        match self {
            WindowCtrl::Headed(c) => c.on_event(ctx, args),
            WindowCtrl::Headless(c) => c.on_event(ctx, args),
            WindowCtrl::HeadlessWithRenderer(c) => c.on_event(ctx, args),
        }
    }

    pub fn on_layout(&mut self, ctx: &mut WindowContext) {
        match self {
            WindowCtrl::Headed(c) => c.on_layout(ctx),
            WindowCtrl::Headless(c) => c.on_layout(ctx),
            WindowCtrl::HeadlessWithRenderer(c) => c.on_layout(ctx),
        }
    }

    pub fn on_render(&mut self, ctx: &mut WindowContext) {
        match self {
            WindowCtrl::Headed(c) => c.on_render(ctx),
            WindowCtrl::Headless(c) => c.on_render(ctx),
            WindowCtrl::HeadlessWithRenderer(c) => c.on_render(ctx),
        }
    }

    pub fn on_respawn(&mut self, ctx: &mut WindowContext) {
        match self {
            WindowCtrl::Headed(c) => c.on_respawn(ctx),
            WindowCtrl::Headless(c) => c.on_respawn(ctx),
            WindowCtrl::HeadlessWithRenderer(c) => c.on_respawn(ctx),
        }
    }
}
