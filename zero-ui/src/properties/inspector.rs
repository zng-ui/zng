//! Debug inspection properties.

use std::{cell::RefCell, rc::Rc};

use zero_ui_core::window::WindowVars;

use crate::core::{
    focus::*,
    mouse::{MOUSE_HOVERED_EVENT, MOUSE_MOVE_EVENT},
    widget_info::*,
};
use crate::prelude::new_property::*;

/// Draws a debug dot in every widget [center point] in the window.
///
/// # Window Only
///
/// This property only works if set in a window, if set in another widget it will log an error and don't render anything.
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
/// This property only works if set in a window, if set in another widget it will log an error and don't render anything.
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

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.enabled.is_new(ctx) {
                ctx.updates.render();
            }
            self.child.update(ctx, updates);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.child.render(ctx, frame);

            if self.valid && self.enabled.get() {
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

/// Draws the inner bounds that where tested for the mouse point.
///
/// # Window Only
///
/// This property only works if set in a window, if set in another widget it will log an error and don't render anything.
#[property(context)]
pub fn show_hit_test(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    struct ShowHitTestNode<C, E> {
        child: C,
        enabled: E,

        valid: bool,

        fails: Vec<PxRect>,
        hits: Vec<PxRect>,

        event_handles: Option<[EventWidgetHandle; 2]>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, E: Var<bool>> UiNode for ShowHitTestNode<C, E> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.valid = ctx.path.is_root();
            if !self.valid {
                tracing::error!("properties that render widget info are only valid in a window");
            }

            if self.enabled.get() {
                self.event_handles = Some([
                    MOUSE_MOVE_EVENT.subscribe(ctx.path.widget_id()),
                    MOUSE_HOVERED_EVENT.subscribe(ctx.path.widget_id()),
                ]);
            }

            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.event_handles = None;
            self.child.deinit(ctx);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.enabled);
            self.child.subscriptions(ctx, subs);
        }

        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            if let Some(args) = MOUSE_MOVE_EVENT.on(update) {
                if self.valid && self.enabled.get() {
                    let factor = WindowVars::req(ctx).scale_factor().get();
                    let pt = args.position.to_px(factor.0);

                    let fails = Rc::new(RefCell::new(vec![]));
                    let hits = Rc::new(RefCell::new(vec![]));

                    let _ = ctx
                        .info_tree
                        .root()
                        .spatial_iter(clone_move!(fails, hits, |w| {
                            let bounds = w.inner_bounds();
                            let hit = bounds.contains(pt);
                            if hit {
                                hits.borrow_mut().push(bounds);
                            } else {
                                fails.borrow_mut().push(bounds);
                            }
                            hit
                        }))
                        .count();

                    let fails = Rc::try_unwrap(fails).unwrap().into_inner();
                    let hits = Rc::try_unwrap(hits).unwrap().into_inner();

                    if self.fails != fails || self.hits != hits {
                        self.fails = fails;
                        self.hits = hits;

                        ctx.updates.render();
                    }
                }
            } else if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
                if args.target.is_none() && !self.fails.is_empty() && !self.hits.is_empty() {
                    self.fails.clear();
                    self.hits.clear();

                    ctx.updates.render();
                }
            }
            self.child.event(ctx, update);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if let Some(enabled) = self.enabled.copy_new(ctx) {
                if enabled {
                    self.event_handles = Some([
                        MOUSE_MOVE_EVENT.subscribe(ctx.path.widget_id()),
                        MOUSE_HOVERED_EVENT.subscribe(ctx.path.widget_id()),
                    ]);
                } else {
                    self.event_handles = None;
                }
                ctx.updates.render();
            }
            self.child.update(ctx, updates);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.child.render(ctx, frame);

            if self.valid && self.enabled.get() {
                let widths = PxSideOffsets::new_all_same(Px(1));
                let fail_sides = BorderSides::solid(colors::RED);
                let hits_sides = BorderSides::solid(colors::LIME_GREEN);

                frame.with_hit_tests_disabled(|frame| {
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
    ShowHitTestNode {
        child,
        enabled: enabled.into_var(),
        event_handles: None,
        valid: false,
        fails: vec![],
        hits: vec![],
    }
}

/// Draw the directional query for closest sibling of the hovered focusable widget.
///
/// # Window Only
///
/// This property only works if set in a window, if set in another widget it will log an error and don't render anything.
#[property(context)]
pub fn show_directional_query(child: impl UiNode, orientation: impl IntoVar<Option<Orientation2D>>) -> impl UiNode {
    struct ShowDirectionalQueryNode<C, E> {
        child: C,
        orientation: E,

        valid: bool,

        search_quads: Vec<PxRect>,

        mouse_hovered_handle: Option<EventWidgetHandle>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, E: Var<Option<Orientation2D>>> UiNode for ShowDirectionalQueryNode<C, E> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.valid = ctx.path.is_root();
            if !self.valid {
                tracing::error!("properties that render widget info are only valid in a window");
            }

            if self.orientation.get().is_some() {
                self.mouse_hovered_handle = Some(MOUSE_HOVERED_EVENT.subscribe(ctx.path.widget_id()));
            }

            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.mouse_hovered_handle = None;
            self.child.deinit(ctx);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.orientation);

            self.child.subscriptions(ctx, subs);
        }

        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
                if let Some(orientation) = self.orientation.get() {
                    let mut none = true;
                    if let Some(target) = &args.target {
                        for w_id in target.widgets_path().iter().rev() {
                            if let Some(w) = ctx.info_tree.get(*w_id) {
                                if let Some(w) = w.as_focusable(true, true) {
                                    let search_quads: Vec<_> = orientation
                                        .search_bounds(w.info.center(), Px::MAX, ctx.info_tree.spatial_bounds().to_box2d())
                                        .map(|q| q.to_rect())
                                        .collect();

                                    if self.search_quads != search_quads {
                                        self.search_quads = search_quads;
                                        ctx.updates.render();
                                    }

                                    none = false;
                                    break;
                                }
                            }
                        }
                    }

                    if none && !self.search_quads.is_empty() {
                        self.search_quads.clear();
                        ctx.updates.render();
                    }
                }
            }
            self.child.event(ctx, update);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if let Some(ori) = self.orientation.copy_new(ctx) {
                self.search_quads.clear();

                if ori.is_some() {
                    self.mouse_hovered_handle = Some(MOUSE_HOVERED_EVENT.subscribe(ctx.path.widget_id()));
                } else {
                    self.mouse_hovered_handle = None;
                }

                ctx.updates.render();
            }
            self.child.update(ctx, updates);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.child.render(ctx, frame);

            if self.valid && self.orientation.get().is_some() {
                let widths = PxSideOffsets::new_all_same(Px(1));
                let quad_sides = BorderSides::solid(colors::YELLOW);

                frame.with_hit_tests_disabled(|frame| {
                    for quad in &self.search_quads {
                        frame.push_border(*quad, widths, quad_sides, PxCornerRadius::zero());
                    }
                });
            }
        }
    }

    ShowDirectionalQueryNode {
        child,
        orientation: orientation.into_var(),
        valid: false,
        search_quads: vec![],
        mouse_hovered_handle: None,
    }
}
