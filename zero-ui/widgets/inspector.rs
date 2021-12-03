//! Inspector widgets.

use crate::core::{
    widget_info::*,
    window::{WidgetInfoChangedEvent, WindowsExt},
};
use crate::prelude::new_property::*;

/// Draws a debug dot in every widget [center point] in the window.
///
/// # Window Only
///
/// This property only works if set in a window, if set in another widget it will log an error and don't render any debug dot.
///
/// [center point]: crate::core::widget_info::WidgetInfo::center
#[property(context)]
pub fn show_center_points(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    render_widget_tree(
        child,
        |tree, frame| {
            for wgt in tree.all_widgets() {
                frame.push_debug_dot(wgt.center(), colors::GREEN)
            }
        },
        enabled,
    )
}

/// Draws a border for every widget outer and inner bounds in the window.
///
/// # Window Only
///
/// This property only works if set in a window, if set in another widget it will log an error and don't render any debug dot.
#[property(context)]
pub fn show_bounds(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    render_widget_tree(
        child,
        |tree, frame| {
            let p = Dip::new(1).to_px(frame.scale_factor().0);

            for wgt in tree.all_widgets() {
                if wgt.outer_bounds() != wgt.inner_bounds() {
                    frame.push_border(
                        wgt.outer_bounds(),
                        PxSideOffsets::new_all_same(p),
                        BorderSides::dotted(colors::GREEN),
                        PxCornerRadius::zero(),
                    );
                }

                frame.push_border(
                    wgt.inner_bounds(),
                    PxSideOffsets::new_all_same(p),
                    BorderSides::solid(colors::GREEN),
                    PxCornerRadius::zero(),
                );
            }
        },
        enabled,
    )
}

/// Calls the `render` closure once every frame after rendering the window contents.
///
/// # Window Only
///
/// This property only works if set in a window, if set in another widget it will log an error and never call `render`.
#[property(context, allowed_in_when = false)]
pub fn render_widget_tree(
    child: impl UiNode,
    render: impl Fn(&WidgetInfoTree, &mut FrameBuilder) + 'static,
    enabled: impl IntoVar<bool>,
) -> impl UiNode {
    struct RenderWidgetTreeNode<C, R, E> {
        child: C,
        render: R,
        enabled: E,

        tree: Option<WidgetInfoTree>,
        valid: bool,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, R: Fn(&WidgetInfoTree, &mut FrameBuilder) + 'static, E: Var<bool>> UiNode for RenderWidgetTreeNode<C, R, E> {
        fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
            self.child.info(ctx, widget);
            widget.subscriptions().var(ctx, &self.enabled).event(WidgetInfoChangedEvent);
        }

        fn init(&mut self, ctx: &mut WidgetContext) {
            self.valid = ctx.path.is_root();
            if !self.valid {
                tracing::error!("properties that render widget info are only valid in a window");
            }

            if self.valid && self.enabled.copy(ctx) {
                self.tree = ctx.services.windows().widget_tree(ctx.path.window_id()).ok().cloned();
            }
            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.tree = None;
            self.child.deinit(ctx);
        }

        fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            if let Some(args) = WidgetInfoChangedEvent.update(args) {
                if args.window_id == ctx.path.window_id() {
                    self.tree = Some(args.tree.clone());
                    ctx.updates.render();
                }
                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.valid {
                if let Some(enabled) = self.enabled.copy_new(ctx) {
                    if enabled {
                        self.tree = Some(ctx.services.windows().widget_tree(ctx.path.window_id()).unwrap().clone());
                    } else {
                        self.tree = None;
                    }

                    ctx.updates.render();
                }
            }
            self.child.update(ctx);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.child.render(ctx, frame);

            if self.valid {
                if let Some(tree) = &self.tree {
                    (self.render)(tree, frame);
                }
            }
        }
    }
    RenderWidgetTreeNode {
        child,
        render,
        enabled: enabled.into_var(),

        tree: None,
        valid: false,
    }
}
