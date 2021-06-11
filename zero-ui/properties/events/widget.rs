//! Widget events, [`on_init`](fn@on_init), [`on_update`](fn@on_update), [`on_render`](fn@on_render) and more.
//!
//! These events map very close to the [`UiNode`] methods. The event handler have non-standard signatures
//! and the event does not respects widget [`enabled`](crate::core::widget_base::IsEnabled) status.

use std::future::Future;

use retain_mut::RetainMut;

use zero_ui_core::context::RenderContext;
use zero_ui_core::context::WidgetContextMut;
use zero_ui_core::task::WidgetTask;

use crate::core::render::FrameBuilder;
use crate::core::units::*;
use crate::core::*;
use crate::core::{
    context::{LayoutContext, WidgetContext},
    render::FrameUpdate,
};
macro_rules! widget_context_handler_events {
    ($($Ident:ident),+) => {$(paste::paste! {
        #[doc = "Arguments for the [`on_"$Ident:snake"`](fn@on_"$Ident:snake") event."]
        #[derive(Clone, Debug, Copy)]
        pub struct [<On $Ident Args>] {
            /// Number of time the handler was called.
            ///
            /// The number is `1` for the first call.
            pub count: usize,
        }

        #[doc = "Event fired during the widget [`" $Ident:snake "`](UiNode::" $Ident:snake ")."]
        ///
        #[doc = "The `handler` is called after the [preview event](on_pre_" $Ident:snake ") and after the widget children."]
        ///
        /// The `handler` is called even when the widget is [disabled](IsEnabled).
        #[property(event, default(|_, _|{}))]
        pub fn [<on_ $Ident:snake>](child: impl UiNode, handler: impl FnMut(&mut WidgetContext, [<On $Ident Args>]) + 'static) -> impl UiNode {
            struct [<On $Ident Node>]<C, F> {
                child: C,
                handler: F,
                count: usize,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode, F: FnMut(&mut WidgetContext, [<On $Ident Args>]) + 'static> UiNode for [<On $Ident Node>]<C, F> {
                fn [<$Ident:snake>](&mut self, ctx: &mut WidgetContext) {
                    self.child.[<$Ident:snake>](ctx);
                    self.count = self.count.wrapping_add(1);
                    let args = [<On $Ident Args>] { count: self.count };
                    (self.handler)(ctx, args);
                }
            }
            [<On $Ident Node>] {
                child,
                handler,
                count: 0
            }
        }

        #[doc = "Preview [`on_" $Ident:snake "`] event."]
        ///
        /// The `handler` is called before the main event and before the widget children.
        ///
        /// The `handler` is called even when the widget is [disabled](IsEnabled).
        #[property(event, default(|_, _|{}))]
        pub fn [<on_pre_ $Ident:snake>](child: impl UiNode, handler: impl FnMut(&mut WidgetContext, [<On $Ident Args>]) + 'static) -> impl UiNode {
            struct [<OnPreview $Ident Node>]<C, F> {
                child: C,
                handler: F,
                count: usize,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode, F: FnMut(&mut WidgetContext, [<On $Ident Args>]) + 'static> UiNode for [<OnPreview $Ident Node>]<C, F> {
                fn [<$Ident:snake>](&mut self, ctx: &mut WidgetContext) {
                    self.count = self.count.wrapping_add(1);
                    let args = [<On $Ident Args>] { count: self.count };
                    (self.handler)(ctx, args);
                    self.child.[<$Ident:snake>](ctx);
                }
            }
            [<OnPreview $Ident Node>] {
                child,
                handler,
                count: 0,
            }
        }
    })+}
}
widget_context_handler_events! {
    Init, Deinit, Update
}

/// Async [`on_init`] event.
///
/// Note that widgets can be deinited an reinited, so its possible for multiple init tasks to be running.
///
/// # Async Handlers
///
/// Async event handlers run in the UI thread only, the code before the first `await` runs immediately, subsequent code
/// runs during updates of the widget they are bound to, if the widget does not update the task does not advance and
/// if the widget is dropped the task is canceled (dropped).
///
/// The handler tasks are asynchronous but not parallel, when they are doing work they block the UI thread, you can use `Tasks`
/// to run CPU intensive work in parallel and await for the result in the handler.
///
/// See [`on_event_async`](zero_ui::core::event::on_event_async) for more details.
#[property(event, default(|_, _| async {}))]
pub fn on_init_async<C, F, H>(child: C, handler: H) -> impl UiNode
where
    C: UiNode,
    F: Future<Output = ()> + 'static,
    H: FnMut(WidgetContextMut, OnInitArgs) -> F + 'static,
{
    struct OnInitAsyncNode<C, H> {
        child: C,
        handler: H,
        count: usize,
        tasks: Vec<WidgetTask<()>>,
    }
    #[impl_ui_node(child)]
    impl<C, F, H> UiNode for OnInitAsyncNode<C, H>
    where
        C: UiNode,
        F: Future<Output = ()> + 'static,
        H: FnMut(WidgetContextMut, OnInitArgs) -> F + 'static,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);

            self.count = self.count.wrapping_add(1);

            let mut task = ctx.async_task(|ctx| (self.handler)(ctx, OnInitArgs { count: self.count }));
            if task.update(ctx).is_none() {
                self.tasks.push(task);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            self.tasks.retain_mut(|t| t.update(ctx).is_none());
        }
    }
    OnInitAsyncNode {
        child,
        handler,
        count: 0,
        tasks: Vec::with_capacity(1),
    }
}

/// Async [`on_pre_init`] event.
///
/// # Async Handlers
///
/// Async event handlers run in the UI thread only, the code before the first `await` runs immediately, subsequent code
/// runs during updates of the widget they are bound to, if the widget does not update the task does not advance and
/// if the widget is dropped the task is canceled (dropped).
///
/// The handler tasks are asynchronous but not parallel, when they are doing work they block the UI thread, you can use `Tasks`
/// to run CPU intensive work in parallel and await for the result in the handler.
///
/// See [`on_event_async`](zero_ui::core::event::on_event_async) for more details.
///
/// ## Async Preview
///
/// Only the code before the first `await` runs immediately so only that code is *preview*.
#[property(event, default(|_, _| async {}))]
pub fn on_pre_init_async<C, F, H>(child: C, handler: H) -> impl UiNode
where
    C: UiNode,
    F: Future<Output = ()> + 'static,
    H: FnMut(WidgetContextMut, OnInitArgs) -> F + 'static,
{
    struct OnPreInitAsyncNode<C, H> {
        child: C,
        handler: H,
        count: usize,
        tasks: Vec<WidgetTask<()>>,
    }
    #[impl_ui_node(child)]
    impl<C, F, H> UiNode for OnPreInitAsyncNode<C, H>
    where
        C: UiNode,
        F: Future<Output = ()> + 'static,
        H: FnMut(WidgetContextMut, OnInitArgs) -> F + 'static,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.count = self.count.wrapping_add(1);

            let mut task = ctx.async_task(|ctx| (self.handler)(ctx, OnInitArgs { count: self.count }));
            if task.update(ctx).is_none() {
                self.tasks.push(task);
            }

            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.tasks.retain_mut(|t| t.update(ctx).is_none());

            self.child.update(ctx);
        }
    }
    OnPreInitAsyncNode {
        child,
        handler,
        count: 0,
        tasks: Vec::with_capacity(1),
    }
}

/// Arguments of the [`on_measure`](fn@on_measure) event.
#[derive(Debug, Clone, Copy)]
pub struct OnMeasureArgs {
    /// The maximum size available to the widget.
    pub available_size: LayoutSize,

    /// The [outer](crate::core::property#outer) size calculated by the widget.
    pub desired_size: LayoutSize,
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
        fn measure(&mut self, ctx: &mut LayoutContext, available_size: LayoutSize) -> LayoutSize {
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
pub fn on_pre_measure(child: impl UiNode, handler: impl FnMut(&mut LayoutContext, LayoutSize) + 'static) -> impl UiNode {
    struct OnPreviewMeasureNode<C, F> {
        child: C,
        handler: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: FnMut(&mut LayoutContext, LayoutSize) + 'static> UiNode for OnPreviewMeasureNode<C, F> {
        fn measure(&mut self, ctx: &mut LayoutContext, available_size: LayoutSize) -> LayoutSize {
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
pub fn on_arrange(child: impl UiNode, handler: impl FnMut(&mut LayoutContext, LayoutSize) + 'static) -> impl UiNode {
    struct OnArrangeNode<C, F> {
        child: C,
        handler: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: FnMut(&mut LayoutContext, LayoutSize) + 'static> UiNode for OnArrangeNode<C, F> {
        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: LayoutSize) {
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
pub fn on_pre_arrange(child: impl UiNode, handler: impl FnMut(&mut LayoutContext, LayoutSize) + 'static) -> impl UiNode {
    struct OnPreviewArrangeNode<C, F> {
        child: C,
        handler: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: FnMut(&mut LayoutContext, LayoutSize) + 'static> UiNode for OnPreviewArrangeNode<C, F> {
        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: LayoutSize) {
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
