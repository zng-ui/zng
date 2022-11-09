//! Widget events, [`on_init`](fn@on_init), [`on_update`](fn@on_update), and more.
//!
//! These events map very close to the [`UiNode`] methods. The event handler have non-standard signatures
//! and the event does not consider the widget's [`interactivity`](crate::core::widget_info::WidgetInfo::interactivity).

use std::mem;

use crate::core::{context::*, handler::*, widget_instance::*, *};

/// Arguments for the [`on_init`](fn@on_init) event.
#[derive(Clone, Debug, Copy)]
pub struct OnInitArgs {
    /// Number of time the handler was called.
    ///
    /// The number is `1` for the first call.
    pub count: usize,
}

/// Widget [`init`](UiNode::init) event.
///
/// This property calls `handler` when the widget and its content initializes. Note that widgets
/// can be [deinitialized](fn@on_deinit) and reinitialized, so the `handler` can be called more then once,
/// you can use one of the *once* handlers to only be called once or use the arguments [`count`](OnInitArgs::count).
/// to determinate if you are in the first init.
///
/// Note that the widget is not in the [`WidgetInfoTree`] when this event happens, you can use [`on_info_init`] for initialization
/// that depends on the widget info.
///
/// # Handlers
///
/// This property accepts any [`WidgetHandler`], including the async handlers. Use one of the handler macros, [`hn!`],
/// [`hn_once!`], [`async_hn!`] or [`async_hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers spawn a task that is associated with the widget, it will only update when the widget updates,
/// so the task *pauses* when the widget is deinited, and is *canceled* when the widget is dropped.
///
/// [`on_info_init`]: fn@on_info_init
/// [`WidgetInfoTree`]: crate::core::widget_info::WidgetInfoTree
#[property(EVENT)]
pub fn on_init(child: impl UiNode, handler: impl WidgetHandler<OnInitArgs>) -> impl UiNode {
    #[ui_node(struct OnInitNode {
        child: impl UiNode,
        handler: impl WidgetHandler<OnInitArgs>,
        count: usize,
    })]
    impl UiNode for OnInitNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);

            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnInitArgs { count: self.count });
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);
            self.handler.update(ctx);
        }
    }
    OnInitNode { child, handler, count: 0 }
}

/// Preview [`on_init`] event.
///
/// This property calls `handler` when the widget initializes, before the widget content initializes. This means
/// that the `handler` is raised before any [`on_init`] handler.
///
/// # Handlers
///
/// This property accepts any [`WidgetHandler`], including the async handlers. Use one of the handler macros, [`hn!`],
/// [`hn_once!`], [`async_hn!`] or [`async_hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers spawn a task that is associated with the widget, it will only update when the widget updates,
/// so the task *pauses* when the widget is deinited, and is *canceled* when the widget is dropped.
///
/// [`on_init`]: fn@on_init
#[property(EVENT)]
pub fn on_pre_init(child: impl UiNode, handler: impl WidgetHandler<OnInitArgs>) -> impl UiNode {
    #[ui_node(struct OnPreviewInitNode {
        child: impl UiNode,
        handler: impl WidgetHandler<OnInitArgs>,
        count: usize,
    })]
    impl UiNode for OnPreviewInitNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnInitArgs { count: self.count });

            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.handler.update(ctx);
            self.child.update(ctx, updates);
        }
    }
    OnPreviewInitNode { child, handler, count: 0 }
}

/// Widget inited and info collected event.
///
/// This event fires after the first [`UiNode::info`] construction after [`UiNode::init`]. This event can be used when
/// some widget initialization needs to happen, but the widget must be in the [`WidgetInfoTree`] for it to work.
///
/// # Handlers
///
/// This property accepts any [`WidgetHandler`], including the async handlers. Use one of the handler macros, [`hn!`],
/// [`hn_once!`], [`async_hn!`] or [`async_hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers spawn a task that is associated with the widget, it will only update when the widget updates,
/// so the task *pauses* when the widget is deinited, and is *canceled* when the widget is dropped.
///
/// [`WidgetInfoTree`]: crate::core::widget_info::WidgetInfoTree
#[property(EVENT)]
pub fn on_info_init(child: impl UiNode, handler: impl WidgetHandler<OnInitArgs>) -> impl UiNode {
    #[ui_node(struct OnInfoInitNode {
        child: impl UiNode,
        handler: impl  WidgetHandler<OnInitArgs>,
        count: usize,
        pending: bool,
    })]
    impl UiNode for OnInfoInitNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);

            self.pending = true;
            ctx.updates.update(ctx.path.widget_id());
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);

            if mem::take(&mut self.pending) {
                self.count = self.count.wrapping_add(1);
                self.handler.event(ctx, &OnInitArgs { count: self.count });
            }

            self.handler.update(ctx);
        }
    }
    OnInfoInitNode {
        child,
        handler,
        count: 0,
        pending: true,
    }
}

/// Arguments for the [`on_update`](fn@on_update) event.
#[derive(Clone, Debug, Copy)]
pub struct OnUpdateArgs {
    /// Number of time the handler was called.
    ///
    /// The number is `1` for the first call.
    pub count: usize,
}

/// Widget [`update`](UiNode::update) event.
///
/// This property calls `handler` every UI update, after the widget content updates. Updates happen in
/// high volume in between idle moments, so the handler code should be considered a *hot-path*.
///
/// # Handlers
///
/// You can use one of the handler macros, [`hn!`] or [`hn_once!`], to declare a handler closure. You must avoid using the async
/// handlers as they cause an update every time the UI task advances from an await point causing another task to spawn.
#[property(EVENT)]
pub fn on_update(child: impl UiNode, handler: impl WidgetHandler<OnUpdateArgs>) -> impl UiNode {
    #[ui_node(struct OnUpdateNode {
        child: impl UiNode,
        handler: impl WidgetHandler<OnUpdateArgs>,
        count: usize,
    })]
    impl UiNode for OnUpdateNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);

            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnUpdateArgs { count: self.count });
            self.handler.update(ctx);
        }
    }
    OnUpdateNode { child, handler, count: 0 }
}

/// Preview [`on_update`] event.
///
/// This property calls `handler` every time the UI updates, before the widget content updates. This means
/// that the `handler` is raised before any [`on_init`] handler.
///
/// # Handlers
///
/// You can use one of the handler macros, [`hn!`] or [`hn_once!`], to declare a handler closure. You must avoid using the async
/// handlers as they cause an update every time the UI task advances from an await point causing another task to spawn.
///
/// [`on_update`]: fn@on_update
/// [`on_init`]: fn@on_init
#[property(EVENT)]
pub fn on_pre_update(child: impl UiNode, handler: impl WidgetHandler<OnUpdateArgs>) -> impl UiNode {
    #[ui_node(struct OnPreviewUpdateNode {
        child: impl UiNode,
        handler: impl WidgetHandler<OnUpdateArgs>,
        count: usize,
    })]
    impl UiNode for OnPreviewUpdateNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.count = self.count.wrapping_add(1);
            self.handler.update(ctx);
            self.handler.event(ctx, &OnUpdateArgs { count: self.count });

            self.child.update(ctx, updates);
        }
    }
    OnPreviewUpdateNode { child, handler, count: 0 }
}

/// Arguments for the [`on_deinit`](fn@on_deinit) event.
#[derive(Clone, Debug, Copy)]
pub struct OnDeinitArgs {
    /// Number of time the handler was called.
    ///
    /// The number is `1` for the first call.
    pub count: usize,
}

/// Widget [`deinit`](UiNode::deinit) event.
///
/// This property calls `handler` when the widget deinits, after the widget content deinits. Note that
/// widgets can be [reinitialized](fn@on_init) so the `handler` can be called more then once,
/// you can use one of the *once* handlers to only be called once or use the arguments [`count`](OnDeinitArgs::count)
/// to determinate if you are in the first deinit.
///
/// # Handlers
///
/// This property accepts the [`WidgetHandler`] that are not async. Use one of the handler macros, [`hn!`] or
/// [`hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers do not work here because widget bound async tasks only advance past the first `.await`
/// during widget updates, but we are deiniting the widget, probably about to drop it. You can start an UI bound
/// async task in the app context using [`WidgetContext::async_task`] or you can use [`task::spawn`] to start a parallel async task
/// in a worker thread.
#[property(EVENT)]
pub fn on_deinit(child: impl UiNode, handler: impl WidgetHandler<OnDeinitArgs>) -> impl UiNode {
    #[ui_node(struct OnDeinitNode {
        child: impl UiNode,
        handler: impl WidgetHandler<OnDeinitArgs>,
        count: usize,
    })]
    impl UiNode for OnDeinitNode {
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);

            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnDeinitArgs { count: self.count });
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);
            self.handler.update(ctx);
        }
    }
    OnDeinitNode { child, handler, count: 0 }
}

/// Preview [`on_update`] event.
///
/// This property calls `handler` every time the UI updates, before the widget content updates. This means
/// that the `handler` is raised before any [`on_init`] handler.
///
/// # Handlers
///
/// This property accepts the [`WidgetHandler`] that are not async. Use one of the handler macros, [`hn!`] or
/// [`hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers do not work here because widget bound async tasks only advance past the first `.await`
/// during widget updates, but we are deiniting the widget, probably about to drop it. You can start an UI bound
/// async task in the app context using [`WidgetContext::async_task`] or you can use [`task::spawn`] to start a parallel async task
/// in a worker thread.
///
/// [`on_update`]: fn@on_update
/// [`on_init`]: fn@on_init
#[property(EVENT)]
pub fn on_pre_deinit(child: impl UiNode, handler: impl WidgetHandler<OnDeinitArgs>) -> impl UiNode {
    #[ui_node(struct OnPreviewDeinitNode {
        child: impl UiNode,
        handler: impl WidgetHandler<OnDeinitArgs>,
        count: usize,
    })]
    impl UiNode for OnPreviewDeinitNode {
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnDeinitArgs { count: self.count });

            self.child.deinit(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.handler.update(ctx);
            self.child.update(ctx, updates);
        }
    }
    OnPreviewDeinitNode { child, handler, count: 0 }
}
