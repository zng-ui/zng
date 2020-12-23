//! Widget events, [`on_init`], [`on_update`], [`on_render`] and more.
//!
//! These events map very close to the [`UiNode`] methods. The event handler have non-standard signatures
//! and the event does not respects widget [`enabled`](crate::properties::IsEnabled) status.

use crate::core::context::{LayoutContext, WidgetContext};
use crate::core::render::FrameBuilder;
use crate::core::units::*;
use crate::core::*;

macro_rules! on_ctx_mtd {
    ($( $(#[$outer:meta])* struct $OnCtxMtd:ident { fn $mtd:ident } fn $on_mtd:ident;)+) => {$(
        struct $OnCtxMtd<C: UiNode, F: FnMut(&mut WidgetContext)> {
            child: C,
            handler: F
        }

        #[impl_ui_node(child)]
        impl<C: UiNode, F: FnMut(&mut WidgetContext) + 'static> UiNode for $OnCtxMtd<C, F> {
            fn $mtd(&mut self, ctx: &mut WidgetContext) {
                self.child.$mtd(ctx);
                (self.handler)(ctx);
            }
        }

        $(#[$outer])*
        ///
        /// The `handler` is called even when the widget is [disabled](IsEnabled).
        #[property(event)]
        pub fn $on_mtd(child: impl UiNode, handler: impl FnMut(&mut WidgetContext) + 'static) -> impl UiNode {
            $OnCtxMtd {
                child,
                handler
            }
        }
    )+};
}

on_ctx_mtd! {
    /// Called when the widget is initialized.
    struct OnInitNode { fn init } fn on_init;
    struct OnDeinitNode { fn deinit } fn on_denit;
    struct OnUpdateNode { fn update } fn on_update;
    struct OnUpdateHpNode { fn update_hp } fn on_update_hp;
}

struct OnRenderNode<C: UiNode, F: Fn(&mut FrameBuilder)> {
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

/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event)]
pub fn on_pre_render(child: impl UiNode, handler: impl Fn(&mut FrameBuilder) + 'static) -> impl UiNode {
    OnPreviewRenderNode { child, handler }
}

#[derive(Debug)]
pub struct OnArrangeArgs<'c> {
    pub final_size: LayoutSize,
    pub ctx: &'c mut LayoutContext,
}

struct OnArrangeNode<C: UiNode, F: FnMut(OnArrangeArgs)> {
    child: C,
    handler: F,
}
#[impl_ui_node(child)]
impl<C: UiNode, F: FnMut(OnArrangeArgs) + 'static> UiNode for OnArrangeNode<C, F> {
    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.child.arrange(final_size, ctx);
        (self.handler)(OnArrangeArgs { final_size, ctx });
    }
}

/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event)]
pub fn on_arrange(child: impl UiNode, handler: impl FnMut(OnArrangeArgs) + 'static) -> impl UiNode {
    OnArrangeNode { child, handler }
}

#[derive(Debug)]
pub struct OnMeasureArgs<'c> {
    pub available_size: LayoutSize,
    pub desired_size: LayoutSize,
    pub ctx: &'c mut LayoutContext,
}

struct OnMeasureNode<C: UiNode, F: FnMut(OnMeasureArgs) -> LayoutSize> {
    child: C,
    handler: F,
}
#[impl_ui_node(child)]
impl<C: UiNode, F: FnMut(OnMeasureArgs) -> LayoutSize + 'static> UiNode for OnMeasureNode<C, F> {
    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        let desired_size = self.child.measure(available_size, ctx);

        (self.handler)(OnMeasureArgs {
            available_size,
            desired_size,
            ctx,
        })
    }
}

/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event)]
pub fn on_measure(child: impl UiNode, handler: impl FnMut(OnMeasureArgs) -> LayoutSize + 'static) -> impl UiNode {
    OnMeasureNode { child, handler }
}
