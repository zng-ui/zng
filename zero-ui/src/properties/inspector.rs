//! Debug inspection properties.

use crate::core::{mouse::MouseMoveEvent, widget_info::*};
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
    show_widget_tree(
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
    show_widget_tree(
        child,
        |tree, frame| {
            let p = Dip::new(1).to_px(frame.scale_factor().0);

            for wgt in tree.all_widgets() {
                if wgt.outer_bounds() != wgt.inner_bounds() {
                    frame.push_border(
                        wgt.outer_bounds(),
                        PxSideOffsets::new_all_same(p),
                        BorderSides::dotted(colors::PINK),
                        PxCornerRadius::zero(),
                    );
                }

                frame.push_border(
                    wgt.inner_bounds(),
                    PxSideOffsets::new_all_same(p),
                    BorderSides::solid(colors::ROYAL_BLUE),
                    PxCornerRadius::zero(),
                );
            }
        },
        enabled,
    )
}

/// Draws the widget inner bounds quad-tree.
///
/// # Window Only
///
/// This property only works if set in a window, if set in another widget it will log an error and don't render any debug dot.
#[property(context)]
pub fn show_quad_tree(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    show_widget_tree(
        child,
        |tree, frame| {
            let widths = PxSideOffsets::new_all_same(Px(1));
            let sides = BorderSides::solid(colors::GRAY);

            // TODO: render the up-to-date quads.
            tree.visit_quads(
                |quad| {
                    frame.push_border(quad.to_rect(), widths, sides, PxCornerRadius::zero());
                    true
                },
                |_| std::ops::ControlFlow::<(), ()>::Continue(()),
            );
        },
        enabled,
    )
}

fn show_widget_tree(
    child: impl UiNode,
    render: impl Fn(&WidgetInfoTree, &mut FrameBuilder) + 'static,
    enabled: impl IntoVar<bool>,
) -> impl UiNode {
    struct RenderWidgetTreeNode<C, R, E> {
        child: C,
        render: R,
        enabled: E,

        valid: bool,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, R: Fn(&WidgetInfoTree, &mut FrameBuilder) + 'static, E: Var<bool>> UiNode for RenderWidgetTreeNode<C, R, E> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.valid = ctx.path.is_root();
            if !self.valid {
                tracing::error!("properties that render widget info are only valid in a window");
            }

            self.child.init(ctx);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.enabled);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.enabled.is_new(ctx) {
                ctx.updates.render();
            }
            self.child.update(ctx);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.child.render(ctx, frame);

            if self.valid && self.enabled.copy(ctx) {
                frame.with_hit_tests_disabled(|frame| {
                    (self.render)(ctx.info_tree, frame);
                });
            }
        }
    }
    RenderWidgetTreeNode {
        child,
        render,
        enabled: enabled.into_var(),

        valid: false,
    }
}

/// Draws the inner bounds that where tested for the mouse point and the quadrants visited.
///
/// # Window Only
///
/// This property only works if set in a window, if set in another widget it will log an error and don't render any debug dot.
#[property(context)]
pub fn show_quad_tree_hits(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    struct ShowQuadTreeHitsNode<C, E> {
        child: C,
        enabled: E,

        valid: bool,

        quads: Vec<PxRect>,
        fails: Vec<PxRect>,
        hits: Vec<PxRect>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, E: Var<bool>> UiNode for ShowQuadTreeHitsNode<C, E> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.valid = ctx.path.is_root();
            if !self.valid {
                tracing::error!("properties that render widget info are only valid in a window");
            }

            self.child.init(ctx);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.enabled);
            if self.enabled.copy(ctx.vars) {
                subs.event(MouseMoveEvent);
            }
            self.child.subscriptions(ctx, subs);
        }

        fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            if let Some(args) = MouseMoveEvent.update(args) {
                if self.valid && self.enabled.copy(ctx) {
                    let mut quads = Vec::with_capacity(self.quads.len());
                    let mut fails = Vec::with_capacity(self.fails.len());
                    let mut hits = Vec::with_capacity(self.hits.len());

                    let factor = ctx
                        .window_state
                        .req(zero_ui::core::window::WindowVarsKey)
                        .scale_factor()
                        .copy(ctx.vars);

                    let pt = args.position.to_px(factor.0);
                    ctx.info_tree.visit_quads(
                        |quad| {
                            let include = quad.contains(pt);
                            if include {
                                quads.push(quad.to_rect());
                            }
                            include
                        },
                        |wgt| {
                            let bounds = wgt.inner_bounds();
                            if bounds.contains(pt) {
                                hits.push(bounds);
                            } else {
                                fails.push(bounds);
                            }

                            std::ops::ControlFlow::<(), ()>::Continue(())
                        },
                    );
                    // debug point algorithm.
                    #[cfg(debug_assertions)]
                    {
                        let q_fails = fails.clone();
                        let q_hits = hits.clone();
                        fails.clear();
                        hits.clear();

                        ctx.info_tree.visit_point(pt, |wgt| {
                            let bounds = wgt.inner_bounds();
                            if bounds.contains(pt) {
                                hits.push(bounds);
                            } else {
                                fails.push(bounds);
                            }

                            std::ops::ControlFlow::<(), ()>::Continue(())
                        });

                        if q_fails != fails || q_hits != hits {
                            tracing::error!("quad and points hits did not match");
                        }
                    }

                    if self.quads != quads || self.fails != fails || self.hits != hits {
                        self.quads = quads;
                        self.fails = fails;
                        self.hits = hits;

                        ctx.updates.render();
                    }
                }

                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.enabled.is_new(ctx) {
                ctx.updates.subscriptions();
                ctx.updates.render();
            }
            self.child.update(ctx);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.child.render(ctx, frame);

            if self.valid && self.enabled.copy(ctx) {
                let widths = PxSideOffsets::new_all_same(Px(1));
                let quad_sides = BorderSides::solid(colors::YELLOW);
                let fail_sides = BorderSides::solid(colors::RED);
                let hits_sides = BorderSides::solid(colors::LIME_GREEN);

                frame.with_hit_tests_disabled(|frame| {
                    for quad in &self.quads {
                        frame.push_border(*quad, widths, quad_sides, PxCornerRadius::zero());
                    }

                    for fail in &self.fails {
                        frame.push_border(*fail, widths, fail_sides, PxCornerRadius::zero());
                    }

                    for hit in &self.hits {
                        frame.push_border(*hit, widths, hits_sides, PxCornerRadius::zero());
                    }
                });
            }
        }
    }
    ShowQuadTreeHitsNode {
        child,
        enabled: enabled.into_var(),
        valid: false,
        quads: vec![],
        fails: vec![],
        hits: vec![],
    }
}
