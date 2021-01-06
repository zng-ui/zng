//! Widget events, [`on_init`](fn@on_init), [`on_update`](fn@on_update), [`on_render`](fn@on_render) and more.
//!
//! These events map very close to the [`UiNode`] methods. The event handler have non-standard signatures
//! and the event does not respects widget [`enabled`](crate::core::widget_base::IsEnabled) status.

use crate::core::render::FrameBuilder;
use crate::core::units::*;
use crate::core::*;
use crate::core::{
    context::{LayoutContext, WidgetContext},
    render::FrameUpdate,
};

macro_rules! widget_context_handler_events {
    ($($Ident:ident),+) => {$(paste::paste!{
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

        #[doc = "Event fired during the widget [`" $Ident:snake "`](UiNode::" $Ident:snake ")."]
        ///
        #[doc = "The `handler` is called after the [preview event](on_pre_" $Ident:snake ") and after the widget children."]
        ///
        /// The `handler` is called even when the widget is [disabled](IsEnabled).
        #[property(event)]
        pub fn [<on_ $Ident:snake>](child: impl UiNode, handler: impl FnMut(&mut WidgetContext) + 'static) -> impl UiNode {
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
        #[property(event)]
        pub fn [<on_pre_ $Ident:snake>](child: impl UiNode, handler: impl FnMut(&mut WidgetContext) + 'static) -> impl UiNode {
            [<OnPreview $Ident Node>] {
                child,
                handler
            }
        }
    })+}
}
widget_context_handler_events! {
    Init, Deinit, Update, UpdateHp
}

/* Measure */

struct OnMeasureNode<C, F> {
    child: C,
    handler: F,
}
#[impl_ui_node(child)]
impl<C: UiNode, F: FnMut(LayoutSize, &mut LayoutContext, LayoutSize) + 'static> UiNode for OnMeasureNode<C, F> {
    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        let desired_size = self.child.measure(available_size, ctx);
        (self.handler)(available_size, ctx, desired_size);
        desired_size
    }
}

struct OnPreviewMeasureNode<C, F> {
    child: C,
    handler: F,
}
#[impl_ui_node(child)]
impl<C: UiNode, F: FnMut(LayoutSize, &mut LayoutContext) + 'static> UiNode for OnPreviewMeasureNode<C, F> {
    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        (self.handler)(available_size, ctx);
        self.child.measure(available_size, ctx)
    }
}

/// Event fired during the widget [`measure`](UiNode::measure) layout.
///
/// The `handler` is called after the [preview event](on_pre_arrange) and after the widget children.
/// The inputs are the available size, layout context and the desired size calculated by the widget.
///
/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event)]
pub fn on_measure(child: impl UiNode, handler: impl FnMut(LayoutSize, &mut LayoutContext, LayoutSize) + 'static) -> impl UiNode {
    OnMeasureNode { child, handler }
}

/// Preview [`on_measure`] event.
///
/// The `handler` is called before the main event and before the widget children.
///
/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event)]
pub fn on_pre_measure(child: impl UiNode, handler: impl FnMut(LayoutSize, &mut LayoutContext) + 'static) -> impl UiNode {
    OnPreviewMeasureNode { child, handler }
}

/* Arrange */

struct OnArrangeNode<C, F> {
    child: C,
    handler: F,
}
#[impl_ui_node(child)]
impl<C: UiNode, F: FnMut(LayoutSize, &mut LayoutContext) + 'static> UiNode for OnArrangeNode<C, F> {
    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.child.arrange(final_size, ctx);
        (self.handler)(final_size, ctx);
    }
}
struct OnPreviewArrangeNode<C, F> {
    child: C,
    handler: F,
}
#[impl_ui_node(child)]
impl<C: UiNode, F: FnMut(LayoutSize, &mut LayoutContext) + 'static> UiNode for OnPreviewArrangeNode<C, F> {
    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        (self.handler)(final_size, ctx);
        self.child.arrange(final_size, ctx);
    }
}

/// Event fired during the widget [`arrange`](UiNode::arrange) layout.
///
/// The `handler` is called after the [preview event](on_pre_arrange) and after the widget children.
///
/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event)]
pub fn on_arrange(child: impl UiNode, handler: impl FnMut(LayoutSize, &mut LayoutContext) + 'static) -> impl UiNode {
    OnArrangeNode { child, handler }
}

/// Preview [`on_arrange`] event.
///
/// The `handler` is called before the main event and before the widget children.
///
/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event)]
pub fn on_pre_arrange(child: impl UiNode, handler: impl FnMut(LayoutSize, &mut LayoutContext) + 'static) -> impl UiNode {
    OnArrangeNode { child, handler }
}

/* Render */

struct OnRenderNode<C, F> {
    child: C,
    handler: F,
}
#[impl_ui_node(child)]
impl<C: UiNode, F: Fn(&mut FrameBuilder) + 'static> UiNode for OnRenderNode<C, F> {
    fn render(&self, frame: &mut FrameBuilder) {
        self.child.render(frame);
        (self.handler)(frame);
    }
}

/// Event fired during the widget [`render`](UiNode::render).
///
/// The `handler` is called after the [preview event](on_pre_render) and after the widget children. That means that
/// display items added by the `handler` are rendered on top of the widget visual.
///
/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event)]
pub fn on_render(child: impl UiNode, handler: impl Fn(&mut FrameBuilder) + 'static) -> impl UiNode {
    OnRenderNode { child, handler }
}

struct OnPreviewRenderNode<C: UiNode, F: Fn(&mut FrameBuilder)> {
    child: C,
    handler: F,
}
#[impl_ui_node(child)]
impl<C: UiNode, F: Fn(&mut FrameBuilder) + 'static> UiNode for OnPreviewRenderNode<C, F> {
    fn render(&self, frame: &mut FrameBuilder) {
        (self.handler)(frame);
        self.child.render(frame);
    }
}

/// Preview [`on_render`] event.
///
/// The `handler` is called before the main event and before the widget children. That means that
/// display items added by the `handler` are rendered as background of the widget visual.
///
/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event)]
pub fn on_pre_render(child: impl UiNode, handler: impl Fn(&mut FrameBuilder) + 'static) -> impl UiNode {
    OnPreviewRenderNode { child, handler }
}

/* Render Update */

struct OnRenderUpdateNode<C, F> {
    child: C,
    handler: F,
}
#[impl_ui_node(child)]
impl<C: UiNode, F: Fn(&mut FrameUpdate) + 'static> UiNode for OnRenderUpdateNode<C, F> {
    fn render_update(&self, update: &mut FrameUpdate) {
        self.child.render_update(update);
        (self.handler)(update);
    }
}

struct OnPreviewRenderUpdateNode<C, F> {
    child: C,
    handler: F,
}
#[impl_ui_node(child)]
impl<C: UiNode, F: Fn(&mut FrameUpdate) + 'static> UiNode for OnPreviewRenderUpdateNode<C, F> {
    fn render_update(&self, update: &mut FrameUpdate) {
        (self.handler)(update);
        self.child.render_update(update);
    }
}

/// Event fired during the widget [`render_update`](UiNode::render_update).
///
/// The `handler` is called after the [preview event](on_pre_render_update) and after the widget children.
///
/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event)]
pub fn on_render_update(child: impl UiNode, handler: impl Fn(&mut FrameUpdate) + 'static) -> impl UiNode {
    OnRenderUpdateNode { child, handler }
}

/// Preview [`on_render_update`] event.
///
/// The `handler` is called before the main event and before the widget children.
///
/// The `handler` is called event when widget is [disabled](IsEnabled).
#[property(event)]
pub fn on_pre_render_update(child: impl UiNode, handler: impl Fn(&mut FrameUpdate) + 'static) -> impl UiNode {
    OnPreviewRenderUpdateNode { child, handler }
}
