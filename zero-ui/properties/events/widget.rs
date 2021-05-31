//! Widget events, [`on_init`](fn@on_init), [`on_update`](fn@on_update), [`on_render`](fn@on_render) and more.
//!
//! These events map very close to the [`UiNode`] methods. The event handler have non-standard signatures
//! and the event does not respects widget [`enabled`](crate::core::widget_base::IsEnabled) status.

use zero_ui_core::context::RenderContext;

use crate::core::render::FrameBuilder;
use crate::core::units::*;
use crate::core::*;
use crate::core::{
    context::{LayoutContext, WidgetContext},
    render::FrameUpdate,
};

macro_rules! widget_context_handler_events {
    ($($Ident:ident),+) => {$(paste::paste!{
        #[doc = "Event fired during the widget [`" $Ident:snake "`](UiNode::" $Ident:snake ")."]
        ///
        #[doc = "The `handler` is called after the [preview event](on_pre_" $Ident:snake ") and after the widget children."]
        ///
        /// The `handler` is called even when the widget is [disabled](IsEnabled).
        #[property(event, default(|_|{}))]
        pub fn [<on_ $Ident:snake>](child: impl UiNode, handler: impl FnMut(&mut WidgetContext) + 'static) -> impl UiNode {
            struct [<On $Ident Node>]<C, F> {
                child: C,
                handler: F,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode, F: FnMut(&mut WidgetContext) + 'static> UiNode for [<On $Ident Node>]<C, F> {
                fn [<$Ident:snake>](&mut self, ctx: &mut WidgetContext) {
                    self.child.[<$Ident:snake>](ctx);
                    (self.handler)(ctx);
                }
            }
            [<On $Ident Node>] {
                child,
                handler
            }
        }

        #[doc = "Preview [`on_" $Ident:snake "`] event."]
        ///
        /// The `handler` is called before the main event and before the widget children.
        ///
        /// The `handler` is called even when the widget is [disabled](IsEnabled).
        #[property(event, default(|_|{}))]
        pub fn [<on_pre_ $Ident:snake>](child: impl UiNode, handler: impl FnMut(&mut WidgetContext) + 'static) -> impl UiNode {
            struct [<OnPreview $Ident Node>]<C, F> {
                child: C,
                handler: F,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode, F: FnMut(&mut WidgetContext) + 'static> UiNode for [<OnPreview $Ident Node>]<C, F> {
                fn [<$Ident:snake>](&mut self, ctx: &mut WidgetContext) {
                    (self.handler)(ctx);
                    self.child.[<$Ident:snake>](ctx);
                }
            }
            [<OnPreview $Ident Node>] {
                child,
                handler
            }
        }
    })+}
}
widget_context_handler_events! {
    Init, Deinit, Update
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
