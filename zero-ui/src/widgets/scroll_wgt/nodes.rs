//! UI nodes used for building the scroll widget.
//!
use std::cell::Cell;

use crate::prelude::new_widget::*;

use crate::core::{
    focus::FOCUS_CHANGED_EVENT,
    mouse::{MouseScrollDelta, MOUSE_WHEEL_EVENT},
};

use super::commands::*;
use super::parts::*;
use super::properties::*;
use super::types::*;

/// The actual content presenter.
pub fn viewport(child: impl UiNode, mode: impl IntoVar<ScrollMode>) -> impl UiNode {
    #[impl_ui_node(struct ViewportNode {
        child: impl UiNode,
        mode: impl Var<ScrollMode>,

        viewport_unit: PxSize,
        viewport_size: PxSize,
        content_size: PxSize,
        content_offset: PxVector,
        last_render_offset: Cell<PxVector>,

        spatial_id: SpatialFrameId,
        binding_key: FrameValueKey<PxTransform>,

        info: ScrollInfo,
    })]
    impl UiNode for ViewportNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            ctx.sub_var(&self.mode)
                .sub_var(&SCROLL_VERTICAL_OFFSET_VAR)
                .sub_var(&SCROLL_HORIZONTAL_OFFSET_VAR);
            self.child.init(ctx);
        }

        fn info(&self, ctx: &mut InfoContext, builder: &mut WidgetInfoBuilder) {
            builder.meta().set(&SCROLL_INFO_ID, self.info.clone());
            self.child.info(ctx, builder);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);

            if self.mode.is_new(ctx) || SCROLL_VERTICAL_OFFSET_VAR.is_new(ctx) || SCROLL_HORIZONTAL_OFFSET_VAR.is_new(ctx) {
                ctx.updates.layout();
            }
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let constrains = ctx.constrains();
            if constrains.is_fill_max().all() {
                return constrains.fill_size();
            }

            let mode = self.mode.get();

            let viewport_unit = constrains.fill_size();
            let define_vp_unit = DEFINE_VIEWPORT_UNIT_VAR.get() // requested
                && viewport_unit.width > Px(0) // and has fill-size
                && viewport_unit.height > Px(0)
                && constrains.max_size() == Some(viewport_unit); // that is not just min size.

            let ct_size = ctx.with_constrains(
                |mut c| {
                    c = c.with_min_size(viewport_unit);
                    if mode.contains(ScrollMode::VERTICAL) {
                        c = c.with_unbounded_y();
                    }
                    if mode.contains(ScrollMode::HORIZONTAL) {
                        c = c.with_unbounded_x();
                    }
                    c
                },
                |ctx| {
                    if define_vp_unit {
                        ctx.with_viewport(viewport_unit, |ctx| self.child.measure(ctx))
                    } else {
                        self.child.measure(ctx)
                    }
                },
            );

            constrains.fill_size_or(ct_size)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let mode = self.mode.get();

            let constrains = ctx.constrains();
            let viewport_unit = constrains.fill_size();
            let define_vp_unit = DEFINE_VIEWPORT_UNIT_VAR.get() // requested
                && viewport_unit.width > Px(0) // and has fill-size
                && viewport_unit.height > Px(0)
                && constrains.max_size() == Some(viewport_unit); // that is not just min size.

            let ct_size = ctx.with_constrains(
                |mut c| {
                    c = c.with_min_size(viewport_unit);
                    if mode.contains(ScrollMode::VERTICAL) {
                        c = c.with_unbounded_y();
                    }
                    if mode.contains(ScrollMode::HORIZONTAL) {
                        c = c.with_unbounded_x();
                    }
                    c
                },
                |ctx| {
                    if define_vp_unit {
                        ctx.with_viewport(viewport_unit, |ctx| {
                            self.viewport_unit = viewport_unit;
                            self.child.layout(ctx, wl)
                        })
                    } else {
                        self.child.layout(ctx, wl)
                    }
                },
            );

            let viewport_size = constrains.fill_size_or(ct_size);
            if self.viewport_size != viewport_size {
                self.viewport_size = viewport_size;
                SCROLL_VIEWPORT_SIZE_VAR.set(ctx, viewport_size).unwrap();
                ctx.updates.render();
            }

            self.info.set_viewport_size(viewport_size);

            self.content_size = ct_size;
            if !mode.contains(ScrollMode::VERTICAL) {
                self.content_size.height = viewport_size.height;
            }
            if !mode.contains(ScrollMode::HORIZONTAL) {
                self.content_size.width = viewport_size.width;
            }

            let mut content_offset = PxVector::zero();
            let v_offset = SCROLL_VERTICAL_OFFSET_VAR.get();
            content_offset.y = (self.viewport_size.height - self.content_size.height) * v_offset;
            let h_offset = SCROLL_HORIZONTAL_OFFSET_VAR.get();
            content_offset.x = (self.viewport_size.width - self.content_size.width) * h_offset;

            if content_offset != self.content_offset {
                self.content_offset = content_offset;

                // we use the viewport + 1vp of margin as the culling rect, so after some distance only updating we need to
                // render again to load more widgets in the view-process.
                let update_only_offset = (self.last_render_offset.get() - self.content_offset).abs();
                if update_only_offset.y <= self.viewport_size.height / Px(2) && update_only_offset.x <= self.viewport_size.width / Px(2) {
                    ctx.updates.render_update();
                } else {
                    ctx.updates.render();
                }
            }

            let v_ratio = self.viewport_size.height.0 as f32 / self.content_size.height.0 as f32;
            let h_ratio = self.viewport_size.width.0 as f32 / self.content_size.width.0 as f32;

            SCROLL_VERTICAL_RATIO_VAR.set_ne(ctx, v_ratio.fct()).unwrap();
            SCROLL_HORIZONTAL_RATIO_VAR.set_ne(ctx, h_ratio.fct()).unwrap();
            SCROLL_CONTENT_SIZE_VAR.set_ne(ctx, self.content_size).unwrap();

            self.viewport_size
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.info.set_viewport_transform(*frame.transform());
            self.last_render_offset.set(self.content_offset);

            let culling_rect = PxBox::from_size(self.viewport_size).inflate(self.viewport_size.width, self.viewport_size.height);
            let culling_rect = frame.transform().outer_transformed(culling_rect).unwrap_or(culling_rect).to_rect();

            frame.push_reference_frame(
                self.spatial_id,
                self.binding_key.bind(self.content_offset.into(), true),
                true,
                false,
                |frame| {
                    frame.with_auto_hide_rect(culling_rect, |frame| {
                        self.child.render(ctx, frame);
                    });
                },
            );
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            self.info.set_viewport_transform(*update.transform());

            update.with_transform(self.binding_key.update(self.content_offset.into(), true), false, |update| {
                self.child.render_update(ctx, update);
            });
        }
    }
    ViewportNode {
        child: child.cfg_boxed(),
        mode: mode.into_var(),
        viewport_size: PxSize::zero(),
        viewport_unit: PxSize::zero(),
        content_size: PxSize::zero(),
        content_offset: PxVector::zero(),
        last_render_offset: Cell::new(PxVector::zero()),
        info: ScrollInfo::default(),

        spatial_id: SpatialFrameId::new_unique(),
        binding_key: FrameValueKey::new_unique(),
    }
    .cfg_boxed()
}

/// Create a node that generates and presents the [vertical scrollbar].
///
/// [vertical scrollbar]: VERTICAL_SCROLLBAR_VIEW_VAR
pub fn v_scrollbar_presenter() -> impl UiNode {
    scrollbar_presenter(VERTICAL_SCROLLBAR_VIEW_VAR, scrollbar::Orientation::Vertical)
}

/// Create a node that generates and presents the [horizontal scrollbar].
///
/// [horizontal scrollbar]: HORIZONTAL_SCROLLBAR_VIEW_VAR
pub fn h_scrollbar_presenter() -> impl UiNode {
    scrollbar_presenter(HORIZONTAL_SCROLLBAR_VIEW_VAR, scrollbar::Orientation::Horizontal)
}

fn scrollbar_presenter(var: impl IntoVar<ViewGenerator<ScrollBarArgs>>, orientation: scrollbar::Orientation) -> impl UiNode {
    ViewGenerator::presenter(var, move |_, is_new| {
        if is_new {
            DataUpdate::Update(ScrollBarArgs::new(orientation))
        } else {
            DataUpdate::Same
        }
    })
}

/// Create a node that generates and presents the [scrollbar joiner].
///
/// [scrollbar joiner]: SCROLLBAR_JOINER_VIEW_VAR
pub fn scrollbar_joiner_presenter() -> impl UiNode {
    ViewGenerator::presenter_default(SCROLLBAR_JOINER_VIEW_VAR)
}

/// Create a node that implements [`SCROLL_UP_CMD`], [`SCROLL_DOWN_CMD`],
/// [`SCROLL_LEFT_CMD`] and [`SCROLL_RIGHT_CMD`] scoped on the widget.
pub fn scroll_commands_node(child: impl UiNode) -> impl UiNode {    
    #[impl_ui_node(struct ScrollCommandsNode {
        child: impl UiNode,

        up: CommandHandle,
        down: CommandHandle,
        left: CommandHandle,
        right: CommandHandle,

        layout_line: PxVector,
    })]
    impl UiNode for ScrollCommandsNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            ctx.sub_var(&VERTICAL_LINE_UNIT_VAR).sub_var(&HORIZONTAL_LINE_UNIT_VAR);

            let scope = ctx.path.widget_id();

            self.up = SCROLL_UP_CMD.scoped(scope).subscribe(ctx, ScrollContext::can_scroll_up());
            self.down = SCROLL_DOWN_CMD.scoped(scope).subscribe(ctx, ScrollContext::can_scroll_down());
            self.left = SCROLL_LEFT_CMD.scoped(scope).subscribe(ctx, ScrollContext::can_scroll_left());
            self.right = SCROLL_RIGHT_CMD.scoped(scope).subscribe(ctx, ScrollContext::can_scroll_right());

            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);

            self.up = CommandHandle::dummy();
            self.down = CommandHandle::dummy();
            self.left = CommandHandle::dummy();
            self.right = CommandHandle::dummy();
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);

            self.up.set_enabled(ScrollContext::can_scroll_up());
            self.down.set_enabled(ScrollContext::can_scroll_down());
            self.left.set_enabled(ScrollContext::can_scroll_left());
            self.right.set_enabled(ScrollContext::can_scroll_right());

            if VERTICAL_LINE_UNIT_VAR.is_new(ctx) || HORIZONTAL_LINE_UNIT_VAR.is_new(ctx) {
                ctx.updates.layout();
            }
        }

        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            self.child.event(ctx, update);

            let scope = ctx.path.widget_id();

            if let Some(args) = SCROLL_UP_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.up, |_| {
                    let mut offset = -self.layout_line.y;
                    if ScrollRequest::from_args(args).map(|f| f.alternate).unwrap_or(false) {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    ScrollContext::scroll_vertical(ctx.vars, offset);
                });
            } else if let Some(args) = SCROLL_DOWN_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.down, |_| {
                    let mut offset = self.layout_line.y;
                    if ScrollRequest::from_args(args).map(|f| f.alternate).unwrap_or(false) {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    ScrollContext::scroll_vertical(ctx.vars, offset);
                });
            } else if let Some(args) = SCROLL_LEFT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.left, |_| {
                    let mut offset = -self.layout_line.x;
                    if ScrollRequest::from_args(args).map(|f| f.alternate).unwrap_or(false) {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    ScrollContext::scroll_horizontal(ctx.vars, offset);
                });
            } else if let Some(args) = SCROLL_RIGHT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.right, |_| {
                    let mut offset = self.layout_line.x;
                    if ScrollRequest::from_args(args).map(|f| f.alternate).unwrap_or(false) {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    ScrollContext::scroll_horizontal(ctx.vars, offset);
                });
            }
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            self.child.measure(ctx)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let r = self.child.layout(ctx, wl);

            let viewport = SCROLL_VIEWPORT_SIZE_VAR.get();
            ctx.with_constrains(
                |_| PxConstrains2d::new_fill_size(viewport),
                |ctx| {
                    self.layout_line = PxVector::new(
                        HORIZONTAL_LINE_UNIT_VAR.get().layout(ctx.metrics.for_x(), |_| Px(20)),
                        VERTICAL_LINE_UNIT_VAR.get().layout(ctx.metrics.for_y(), |_| Px(20)),
                    );
                },
            );

            r
        }
    }
    ScrollCommandsNode {
        child: child.cfg_boxed(),

        up: CommandHandle::dummy(),
        down: CommandHandle::dummy(),
        left: CommandHandle::dummy(),
        right: CommandHandle::dummy(),

        layout_line: PxVector::zero(),
    }
    .cfg_boxed()
}

/// Create a node that implements [`PAGE_UP_CMD`], [`PAGE_DOWN_CMD`],
/// [`PAGE_LEFT_CMD`] and [`PAGE_RIGHT_CMD`] scoped on the widget.
pub fn page_commands_node(child: impl UiNode) -> impl UiNode {
    #[impl_ui_node(struct PageCommandsNode {
        child: impl UiNode,

        up: CommandHandle,
        down: CommandHandle,
        left: CommandHandle,
        right: CommandHandle,

        layout_page: PxVector,
    })]
    impl UiNode for PageCommandsNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            ctx.sub_var(&VERTICAL_PAGE_UNIT_VAR).sub_var(&HORIZONTAL_PAGE_UNIT_VAR);

            let scope = ctx.path.widget_id();

            self.up = PAGE_UP_CMD.scoped(scope).subscribe(ctx, ScrollContext::can_scroll_up());
            self.down = PAGE_DOWN_CMD.scoped(scope).subscribe(ctx, ScrollContext::can_scroll_down());
            self.left = PAGE_LEFT_CMD.scoped(scope).subscribe(ctx, ScrollContext::can_scroll_left());
            self.right = PAGE_RIGHT_CMD.scoped(scope).subscribe(ctx, ScrollContext::can_scroll_right());

            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);

            self.up = CommandHandle::dummy();
            self.down = CommandHandle::dummy();
            self.left = CommandHandle::dummy();
            self.right = CommandHandle::dummy();
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);

            self.up.set_enabled(ScrollContext::can_scroll_up());
            self.down.set_enabled(ScrollContext::can_scroll_down());
            self.left.set_enabled(ScrollContext::can_scroll_left());
            self.right.set_enabled(ScrollContext::can_scroll_right());

            if VERTICAL_PAGE_UNIT_VAR.is_new(ctx) || HORIZONTAL_PAGE_UNIT_VAR.is_new(ctx) {
                ctx.updates.layout();
            }
        }

        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            self.child.event(ctx, update);

            let scope = ctx.path.widget_id();

            if let Some(args) = PAGE_UP_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.up, |_| {
                    let mut offset = -self.layout_page.y;
                    if ScrollRequest::from_args(args).map(|f| f.alternate).unwrap_or(false) {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    ScrollContext::scroll_vertical(ctx.vars, offset);
                });
            } else if let Some(args) = PAGE_DOWN_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.down, |_| {
                    let mut offset = self.layout_page.y;
                    if ScrollRequest::from_args(args).map(|f| f.alternate).unwrap_or(false) {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    ScrollContext::scroll_vertical(ctx.vars, offset);
                });
            } else if let Some(args) = PAGE_LEFT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.left, |_| {
                    let mut offset = -self.layout_page.x;
                    if ScrollRequest::from_args(args).map(|f| f.alternate).unwrap_or(false) {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    ScrollContext::scroll_horizontal(ctx.vars, offset);
                });
            } else if let Some(args) = PAGE_RIGHT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.right, |_| {
                    let mut offset = self.layout_page.x;
                    if ScrollRequest::from_args(args).map(|f| f.alternate).unwrap_or(false) {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    ScrollContext::scroll_horizontal(ctx.vars, offset);
                });
            }
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            self.child.measure(ctx)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let r = self.child.layout(ctx, wl);

            let viewport = SCROLL_VIEWPORT_SIZE_VAR.get();
            ctx.with_constrains(
                |_| PxConstrains2d::new_fill_size(viewport),
                |ctx| {
                    self.layout_page = PxVector::new(
                        HORIZONTAL_PAGE_UNIT_VAR.get().layout(ctx.metrics.for_x(), |_| Px(20)),
                        VERTICAL_PAGE_UNIT_VAR.get().layout(ctx.metrics.for_y(), |_| Px(20)),
                    );
                },
            );

            r
        }
    }
    PageCommandsNode {
        child,

        up: CommandHandle::dummy(),
        down: CommandHandle::dummy(),
        left: CommandHandle::dummy(),
        right: CommandHandle::dummy(),

        layout_page: PxVector::zero(),
    }
}

/// Create a node that implements [`SCROLL_TO_TOP_CMD`], [`SCROLL_TO_BOTTOM_CMD`],
/// [`SCROLL_TO_LEFTMOST_CMD`] and [`SCROLL_TO_RIGHTMOST_CMD`] scoped on the widget.
pub fn scroll_to_edge_commands_node(child: impl UiNode) -> impl UiNode {
    #[impl_ui_node(struct ScrollToEdgeCommandsNode {
        child: impl UiNode,

        top: CommandHandle,
        bottom: CommandHandle,
        leftmost: CommandHandle,
        rightmost: CommandHandle,
    })]
    impl UiNode for ScrollToEdgeCommandsNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            let scope = ctx.path.widget_id();

            self.top = SCROLL_TO_TOP_CMD.scoped(scope).subscribe(ctx, ScrollContext::can_scroll_up());
            self.bottom = SCROLL_TO_BOTTOM_CMD.scoped(scope).subscribe(ctx, ScrollContext::can_scroll_down());
            self.leftmost = SCROLL_TO_LEFTMOST_CMD
                .scoped(scope)
                .subscribe(ctx, ScrollContext::can_scroll_left());
            self.rightmost = SCROLL_TO_RIGHTMOST_CMD
                .scoped(scope)
                .subscribe(ctx, ScrollContext::can_scroll_right());

            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);

            self.top = CommandHandle::dummy();
            self.bottom = CommandHandle::dummy();
            self.leftmost = CommandHandle::dummy();
            self.rightmost = CommandHandle::dummy();
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);

            self.top.set_enabled(ScrollContext::can_scroll_up());
            self.bottom.set_enabled(ScrollContext::can_scroll_down());
            self.leftmost.set_enabled(ScrollContext::can_scroll_left());
            self.rightmost.set_enabled(ScrollContext::can_scroll_right());
        }

        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            self.child.event(ctx, update);

            let scope = ctx.path.widget_id();

            if let Some(args) = SCROLL_TO_TOP_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.top, |_| {
                    ScrollContext::chase_vertical(ctx, 0.fct());
                });
            } else if let Some(args) = SCROLL_TO_BOTTOM_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.bottom, |_| {
                    ScrollContext::chase_vertical(ctx, 1.fct());
                });
            } else if let Some(args) = SCROLL_TO_LEFTMOST_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.leftmost, |_| {
                    ScrollContext::chase_horizontal(ctx, 0.fct());
                });
            } else if let Some(args) = SCROLL_TO_RIGHTMOST_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.rightmost, |_| {
                    ScrollContext::chase_horizontal(ctx, 1.fct());
                });
            } else {
            }
        }
    }
    ScrollToEdgeCommandsNode {
        child: child.cfg_boxed(),

        top: CommandHandle::dummy(),
        bottom: CommandHandle::dummy(),
        leftmost: CommandHandle::dummy(),
        rightmost: CommandHandle::dummy(),
    }
    .cfg_boxed()
}

/// Create a node that implements [`SCROLL_TO_CMD`] scoped on the widget and scroll to focused.
pub fn scroll_to_node(child: impl UiNode) -> impl UiNode {
    #[impl_ui_node(struct ScrollToCommandNode {
        child: impl UiNode,

        handle: CommandHandle,
        scroll_to: Option<(WidgetBoundsInfo, ScrollToMode)>,
    })]
    impl UiNode for ScrollToCommandNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.handle = SCROLL_TO_CMD.scoped(ctx.path.widget_id()).subscribe(ctx, true);
            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.handle = CommandHandle::dummy();
            self.child.deinit(ctx);
        }

        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            let self_id = ctx.path.widget_id();
            if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                if let Some(path) = &args.new_focus {
                    if path.contains(self_id) && path.widget_id() != self_id {
                        // probable focus move inside.
                        if let Some(target) = ctx.info_tree.get(path.widget_id()) {
                            // target exits
                            if let Some(us) = target.ancestors().find(|w| w.widget_id() == self_id) {
                                // confirmed, target is descendant
                                if us.is_scroll() {
                                    // we are a scroll.

                                    let bounds = target.bounds_info();
                                    let mode = SCROLL_TO_FOCUSED_MODE_VAR.get();

                                    self.scroll_to = Some((bounds, mode));
                                    ctx.updates.layout();
                                }
                            }
                        }
                    }
                }
            } else if let Some(args) = SCROLL_TO_CMD.scoped(self_id).on(update) {
                // event send to us and enabled
                if let Some(request) = ScrollToRequest::from_args(args) {
                    // has unhandled request
                    if let Some(target) = ctx.info_tree.get(request.widget_id) {
                        // target exists
                        if let Some(us) = target.ancestors().find(|w| w.widget_id() == self_id) {
                            // target is descendant
                            if us.is_scroll() {
                                // we are a scroll.

                                let bounds = target.bounds_info();
                                let mode = request.mode;

                                // will scroll on the next arrange.
                                self.scroll_to = Some((bounds, mode));
                                ctx.updates.layout();

                                args.propagation().stop();
                            }
                        }
                    }
                }
            }
            self.child.event(ctx, update);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            self.child.measure(ctx)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let r = self.child.layout(ctx, wl);

            if let Some((bounds, mode)) = self.scroll_to.take() {
                let us = ctx.info_tree.get(ctx.path.widget_id()).unwrap();
                if let Some(viewport_bounds) = us.viewport() {
                    let target_bounds = bounds.inner_bounds();
                    match mode {
                        ScrollToMode::Minimal { margin } => {
                            let margin = ctx.with_constrains(
                                |_| PxConstrains2d::new_fill_size(target_bounds.size),
                                |ctx| margin.layout(ctx, |_| PxSideOffsets::zero()),
                            );
                            let mut target_bounds = target_bounds;
                            target_bounds.origin.x -= margin.left;
                            target_bounds.origin.y -= margin.top;
                            target_bounds.size.width += margin.horizontal();
                            target_bounds.size.height += margin.vertical();

                            if target_bounds.size.width < viewport_bounds.size.width {
                                if target_bounds.origin.x < viewport_bounds.origin.x {
                                    let diff = target_bounds.origin.x - viewport_bounds.origin.x;
                                    ScrollContext::scroll_horizontal(ctx, diff);
                                } else if target_bounds.max_x() > viewport_bounds.max_x() {
                                    let diff = target_bounds.max_x() - viewport_bounds.max_x();
                                    ScrollContext::scroll_horizontal(ctx, diff);
                                }
                            } else {
                                let target_center_x = (target_bounds.size.width / Px(2)) + target_bounds.origin.x;
                                let viewport_center_x = (target_bounds.size.width / Px(2)) + viewport_bounds.origin.x;

                                let diff = target_center_x - viewport_center_x;
                                ScrollContext::scroll_horizontal(ctx, diff);
                            }
                            if target_bounds.size.height < viewport_bounds.size.height {
                                if target_bounds.origin.y < viewport_bounds.origin.y {
                                    let diff = target_bounds.origin.y - viewport_bounds.origin.y;
                                    ScrollContext::scroll_vertical(ctx, diff);
                                } else if target_bounds.max_y() > viewport_bounds.max_y() {
                                    let diff = target_bounds.max_y() - viewport_bounds.max_y();
                                    ScrollContext::scroll_vertical(ctx, diff);
                                }
                            } else {
                                let target_center_y = (target_bounds.size.height / Px(2)) + target_bounds.origin.y;
                                let viewport_center_y = (target_bounds.size.height / Px(2)) + viewport_bounds.origin.y;

                                let diff = target_center_y - viewport_center_y;
                                ScrollContext::scroll_vertical(ctx, diff);
                            }
                        }
                        ScrollToMode::Center {
                            widget_point,
                            scroll_point,
                        } => {
                            let default = (target_bounds.size / Px(2)).to_vector().to_point();
                            let widget_point = ctx.with_constrains(
                                |_| PxConstrains2d::new_fill_size(target_bounds.size),
                                |ctx| widget_point.layout(ctx, |_| default),
                            );

                            let default = (viewport_bounds.size / Px(2)).to_vector().to_point();
                            let scroll_point = ctx.with_constrains(
                                |_| PxConstrains2d::new_fill_size(viewport_bounds.size),
                                |ctx| scroll_point.layout(ctx, |_| default),
                            );

                            let widget_point = widget_point + target_bounds.origin.to_vector();
                            let scroll_point = scroll_point + viewport_bounds.origin.to_vector();

                            let diff = widget_point - scroll_point;

                            ScrollContext::scroll_vertical(ctx, diff.y);
                            ScrollContext::scroll_horizontal(ctx, diff.x);
                        }
                    }
                }
            }

            r
        }
    }

    ScrollToCommandNode {
        child: child.cfg_boxed(),

        handle: CommandHandle::dummy(),
        scroll_to: None,
    }
    .cfg_boxed()
}

/// Create a node that implements scroll-wheel handling for the widget.
pub fn scroll_wheel_node(child: impl UiNode) -> impl UiNode {
    struct ScrollWheelNode<C> {
        child: C,
        offset: Vector,
        mouse_wheel_handle: Option<EventWidgetHandle>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for ScrollWheelNode<C> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.mouse_wheel_handle = Some(MOUSE_WHEEL_EVENT.subscribe(ctx.path.widget_id()));
            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.mouse_wheel_handle = None;
            self.child.deinit(ctx);
        }

        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            if let Some(args) = MOUSE_WHEEL_EVENT.on(update) {
                if let Some(delta) = args.scroll_delta(ALT_FACTOR_VAR.get()) {
                    args.handle(|_| {
                        match delta {
                            MouseScrollDelta::LineDelta(x, y) => {
                                self.offset.x -= HORIZONTAL_LINE_UNIT_VAR.get() * x.fct();
                                self.offset.y -= VERTICAL_LINE_UNIT_VAR.get() * y.fct();
                            }
                            MouseScrollDelta::PixelDelta(x, y) => {
                                self.offset.x -= x.px();
                                self.offset.y -= y.px();
                            }
                        }

                        ctx.updates.layout();
                    });
                }
            }
            self.child.event(ctx, update);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            self.child.measure(ctx)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let r = self.child.layout(ctx, wl);

            let viewport = SCROLL_VIEWPORT_SIZE_VAR.get();

            ctx.with_constrains(
                |_| PxConstrains2d::new_fill_size(viewport),
                |ctx| {
                    let offset = self.offset.layout(ctx, |_| viewport.to_vector());
                    self.offset = Vector::zero();

                    if offset.y != Px(0) {
                        ScrollContext::scroll_vertical(ctx, offset.y);
                    }
                    if offset.x != Px(0) {
                        ScrollContext::scroll_horizontal(ctx, offset.x);
                    }
                },
            );

            r
        }
    }
    ScrollWheelNode {
        child: child.cfg_boxed(),
        offset: Vector::zero(),
        mouse_wheel_handle: None,
    }
    .cfg_boxed()
}
