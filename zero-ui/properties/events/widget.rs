//! Widget events, [`on_init`](fn@on_init), [`on_update`](fn@on_update), [`on_render`](fn@on_render) and more.
//!
//! These events map very close to the [`UiNode`] methods. The event handler have non-standard signatures
//! and the event does not respects widget [`enabled`](crate::core::widget_base::IsEnabled) status.

use crate::core::handler::*;
use crate::core::render::FrameBuilder;
use crate::core::units::*;
use crate::core::*;
use crate::core::{
    context::{LayoutContext, WidgetContext},
    render::FrameUpdate,
};
use zero_ui_core::context::RenderContext;

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
/// # Handlers
///
/// This property accepts any [`WidgetHandler`], including the async handlers. Use one of the handler macros, [`hn!`],
/// [`hn_once!`], [`async_hn!`] or [`async_hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers spawn a task that is associated with the widget, it will only update when the widget updates,
/// so the task *pauses* when the widget is deinited, and is *canceled* when the widget is dropped.
#[property(event,  default( hn!(|_, _|{}) ))]
pub fn on_init(child: impl UiNode, handler: impl WidgetHandler<OnInitArgs>) -> impl UiNode {
    struct OnInitNode<C, H> {
        child: C,
        handler: H,
        count: usize,
    }

    #[impl_ui_node(child)]
    impl<C: UiNode, H: WidgetHandler<OnInitArgs>> UiNode for OnInitNode<C, H> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);

            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnInitArgs { count: self.count });
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);
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
#[property(event,  default( hn!(|_, _|{}) ))]
pub fn on_pre_init(child: impl UiNode, handler: impl WidgetHandler<OnInitArgs>) -> impl UiNode {
    struct OnPreviewInitNode<C, H> {
        child: C,
        handler: H,
        count: usize,
    }

    #[impl_ui_node(child)]
    impl<C: UiNode, H: WidgetHandler<OnInitArgs>> UiNode for OnPreviewInitNode<C, H> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnInitArgs { count: self.count });

            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.handler.update(ctx);
            self.child.update(ctx);
        }
    }

    OnPreviewInitNode { child, handler, count: 0 }
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
/// This property accepts the [`WidgetHandler`] that are not async. Use one of the handler macros, [`hn!`] or
/// [`hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers are not permitted here because of the high volume of calls and because async tasks cause an
/// UI update every time they awake, so it is very easy to lock the app in a constant sequence of updates.
#[property(event,  default( hn!(|_, _|{}) ))]
pub fn on_update(child: impl UiNode, handler: impl WidgetHandler<OnUpdateArgs> + marker::NotAsyncHn) -> impl UiNode {
    struct OnUpdateNode<C, H> {
        child: C,
        handler: H,
        count: usize,
    }

    #[impl_ui_node(child)]
    impl<C: UiNode, H: WidgetHandler<OnUpdateArgs>> UiNode for OnUpdateNode<C, H> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnUpdateArgs { count: self.count });
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
/// This property accepts the [`WidgetHandler`] that are not async. Use one of the handler macros, [`hn!`] or
/// [`hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers are not permitted here because of the high volume of calls and because async tasks cause an
/// UI update every time they awake, so it is very easy to lock the app in a constant sequence of updates.
#[property(event,  default( hn!(|_, _|{}) ))]
pub fn on_pre_update(child: impl UiNode, handler: impl WidgetHandler<OnUpdateArgs> + marker::NotAsyncHn) -> impl UiNode {
    struct OnPreviewUpdateNode<C, H> {
        child: C,
        handler: H,
        count: usize,
    }

    #[impl_ui_node(child)]
    impl<C: UiNode, H: WidgetHandler<OnUpdateArgs>> UiNode for OnPreviewUpdateNode<C, H> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnUpdateArgs { count: self.count });

            self.child.update(ctx);
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
/// The async handlers are not permitted here because widget bound async tasks only advance past the first `.await`
/// during widget updates, but we are deiniting the widget, probably about to drop it. You can start an UI bound
/// async task in the app context using [`Updates::run`] or you can use [`task::spawn`] to start a parallel async task
/// in a worker thread.
#[property(event,  default( hn!(|_, _|{}) ))]
pub fn on_deinit(child: impl UiNode, handler: impl WidgetHandler<OnDeinitArgs> + marker::NotAsyncHn) -> impl UiNode {
    struct OnDeinitNode<C, H> {
        child: C,
        handler: H,
        count: usize,
    }

    #[impl_ui_node(child)]
    impl<C: UiNode, H: WidgetHandler<OnDeinitArgs>> UiNode for OnDeinitNode<C, H> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnDeinitArgs { count: self.count });
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
/// The async handlers are not permitted here because widget bound async tasks only advance past the first `.await`
/// during widget updates, but we are deiniting the widget, probably about to drop it. You can start an UI bound
/// async task in the app context using [`Updates::run`] or you can use [`task::spawn`] to start a parallel async task
/// in a worker thread.
#[property(event,  default( hn!(|_, _|{}) ))]
pub fn on_pre_deinit(child: impl UiNode, handler: impl WidgetHandler<OnDeinitArgs> + marker::NotAsyncHn) -> impl UiNode {
    struct OnPreviewDeinitNode<C, H> {
        child: C,
        handler: H,
        count: usize,
    }

    #[impl_ui_node(child)]
    impl<C: UiNode, H: WidgetHandler<OnDeinitArgs>> UiNode for OnPreviewDeinitNode<C, H> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnDeinitArgs { count: self.count });

            self.child.update(ctx);
        }
    }

    OnPreviewDeinitNode { child, handler, count: 0 }
}

/// Arguments of the [`on_measure`](fn@on_measure) event.
#[derive(Debug, Clone, Copy)]
pub struct OnMeasureArgs {
    /// The maximum size available to the widget.
    pub available_size: AvailableSize,

    /// The [outer](crate::core::property#outer) size calculated by the widget.
    pub desired_size: PxSize,
}

/// Event fired during the widget [`measure`](UiNode::measure) layout.
///
/// The `handler` is called after the [preview event](on_pre_arrange) and after the widget children.
/// The inputs are layout context and the [`OnMeasureArgs`].
///
/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event, default(|_, _|{}))]
pub fn on_measure(child: impl UiNode, handler: impl FnMut(&mut LayoutContext, OnMeasureArgs) + 'static) -> impl UiNode {
    struct OnMeasureNode<C, F> {
        child: C,
        handler: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: FnMut(&mut LayoutContext, OnMeasureArgs) + 'static> UiNode for OnMeasureNode<C, F> {
        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let desired_size = self.child.measure(ctx, available_size);
            (self.handler)(
                ctx,
                OnMeasureArgs {
                    available_size,
                    desired_size,
                },
            );
            desired_size
        }
    }
    OnMeasureNode { child, handler }
}

/// Preview [`on_measure`] event.
///
/// The `handler` is called before the main event and before the widget children. The inputs are the layout context
/// and the available size.
///
/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event, default(|_, _|{}))]
pub fn on_pre_measure(child: impl UiNode, handler: impl FnMut(&mut LayoutContext, AvailableSize) + 'static) -> impl UiNode {
    struct OnPreviewMeasureNode<C, F> {
        child: C,
        handler: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: FnMut(&mut LayoutContext, AvailableSize) + 'static> UiNode for OnPreviewMeasureNode<C, F> {
        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            (self.handler)(ctx, available_size);
            self.child.measure(ctx, available_size)
        }
    }
    OnPreviewMeasureNode { child, handler }
}

/// Event fired during the widget [`arrange`](UiNode::arrange) layout.
///
/// The `handler` is called after the [preview event](on_pre_arrange) and after the widget children.
///
/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event, default(|_, _|{}))]
pub fn on_arrange(child: impl UiNode, handler: impl FnMut(&mut LayoutContext, PxSize) + 'static) -> impl UiNode {
    struct OnArrangeNode<C, F> {
        child: C,
        handler: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: FnMut(&mut LayoutContext, PxSize) + 'static> UiNode for OnArrangeNode<C, F> {
        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
            self.child.arrange(ctx, final_size);
            (self.handler)(ctx, final_size);
        }
    }
    OnArrangeNode { child, handler }
}

/// Preview [`on_arrange`] event.
///
/// The `handler` is called before the main event and before the widget children.
///
/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event, default(|_, _|{}))]
pub fn on_pre_arrange(child: impl UiNode, handler: impl FnMut(&mut LayoutContext, PxSize) + 'static) -> impl UiNode {
    struct OnPreviewArrangeNode<C, F> {
        child: C,
        handler: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: FnMut(&mut LayoutContext, PxSize) + 'static> UiNode for OnPreviewArrangeNode<C, F> {
        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
            (self.handler)(ctx, final_size);
            self.child.arrange(ctx, final_size);
        }
    }
    OnPreviewArrangeNode { child, handler }
}

/// Event fired during the widget [`render`](UiNode::render).
///
/// The `handler` is called after the [preview event](on_pre_render) and after the widget children. That means that
/// display items added by the `handler` are rendered on top of the widget visual.
///
/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event, default(|_, _|{}))]
pub fn on_render(child: impl UiNode, handler: impl Fn(&mut RenderContext, &mut FrameBuilder) + 'static) -> impl UiNode {
    struct OnRenderNode<C, F> {
        child: C,
        handler: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: Fn(&mut RenderContext, &mut FrameBuilder) + 'static> UiNode for OnRenderNode<C, F> {
        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.child.render(ctx, frame);
            (self.handler)(ctx, frame);
        }
    }
    OnRenderNode { child, handler }
}
/// Preview [`on_render`] event.
///
/// The `handler` is called before the main event and before the widget children. That means that
/// display items added by the `handler` are rendered as background of the widget visual.
///
/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event, default(|_, _|{}))]
pub fn on_pre_render(child: impl UiNode, handler: impl Fn(&mut RenderContext, &mut FrameBuilder) + 'static) -> impl UiNode {
    struct OnPreviewRenderNode<C: UiNode, F: Fn(&mut RenderContext, &mut FrameBuilder)> {
        child: C,
        handler: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: Fn(&mut RenderContext, &mut FrameBuilder) + 'static> UiNode for OnPreviewRenderNode<C, F> {
        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            (self.handler)(ctx, frame);
            self.child.render(ctx, frame);
        }
    }
    OnPreviewRenderNode { child, handler }
}

/// Event fired during the widget [`render_update`](UiNode::render_update).
///
/// The `handler` is called after the [preview event](on_pre_render_update) and after the widget children.
///
/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event, default(|_, _|{}))]
pub fn on_render_update(child: impl UiNode, handler: impl Fn(&mut RenderContext, &mut FrameUpdate) + 'static) -> impl UiNode {
    struct OnRenderUpdateNode<C, F> {
        child: C,
        handler: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: Fn(&mut RenderContext, &mut FrameUpdate) + 'static> UiNode for OnRenderUpdateNode<C, F> {
        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            self.child.render_update(ctx, update);
            (self.handler)(ctx, update);
        }
    }
    OnRenderUpdateNode { child, handler }
}
/// Preview [`on_render_update`] event.
///
/// The `handler` is called before the main event and before the widget children.
///
/// The `handler` is called event when widget is [disabled](IsEnabled).
#[property(event, default(|_, _|{}))]
pub fn on_pre_render_update(child: impl UiNode, handler: impl Fn(&mut RenderContext, &mut FrameUpdate) + 'static) -> impl UiNode {
    struct OnPreviewRenderUpdateNode<C, F> {
        child: C,
        handler: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: Fn(&mut RenderContext, &mut FrameUpdate) + 'static> UiNode for OnPreviewRenderUpdateNode<C, F> {
        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            (self.handler)(ctx, update);
            self.child.render_update(ctx, update);
        }
    }
    OnPreviewRenderUpdateNode { child, handler }
}
