//! UI nodes used for building the scrollable widget.
//!
use crate::prelude::new_widget::*;

use crate::core::mouse::{MouseScrollDelta, MouseWheelEvent};

use super::commands::*;
use super::parts::*;
use super::properties::*;
use super::types::*;

/// The actual content presenter.
pub fn viewport(child: impl UiNode, mode: impl IntoVar<ScrollMode>) -> impl UiNode {
    struct ViewportNode<C, M> {
        scroll_id: ScrollId,
        child: C,
        mode: M,

        viewport_size: PxSize,
        content_size: PxSize,
        content_offset: PxVector,

        info: ScrollableInfo,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, M: Var<ScrollMode>> UiNode for ViewportNode<C, M> {
        fn info(&self, ctx: &mut InfoContext, builder: &mut WidgetInfoBuilder) {
            builder.meta().set(ScrollableInfoKey, self.info.clone());
            self.child.info(ctx, builder);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions
                .vars(ctx)
                .var(&self.mode)
                .var(&ScrollVerticalOffsetVar::new())
                .var(&ScrollHorizontalOffsetVar::new());
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            if self.mode.is_new(ctx) || ScrollVerticalOffsetVar::is_new(ctx) || ScrollHorizontalOffsetVar::is_new(ctx) {
                ctx.updates.layout();
            }
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            // TODO !!: reimplement after all others, and after we maybe do the `panel!` base widget?
            self.child.layout(ctx, wl)
        }

        /*
        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let mut c_available_size = available_size;

            let mode = self.mode.copy(ctx);
            if mode.contains(ScrollMode::VERTICAL) {
                c_available_size.height = AvailablePx::Infinite;
            }
            if mode.contains(ScrollMode::HORIZONTAL) {
                c_available_size.width = AvailablePx::Infinite;
            }

            let ct_size = self.child.measure(ctx, c_available_size);

            if mode.contains(ScrollMode::VERTICAL) && ct_size.height != self.content_size.height {
                self.content_size.height = ct_size.height;
                ctx.updates.render();
            }
            if mode.contains(ScrollMode::HORIZONTAL) && ct_size.width != self.content_size.width {
                self.content_size.width = ct_size.width;
                ctx.updates.render();
            }

            available_size.clip(ct_size)
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
            if self.viewport_size != final_size {
                self.viewport_size = final_size;
                ScrollViewportSizeVar::set(ctx, final_size).unwrap();
                ctx.updates.render();
            }
            let viewport = PxRect::new(
                widget_layout
                    .global_transform()
                    .transform_px_point(PxPoint::zero())
                    .unwrap_or_default(),
                    final_size,
            );

            self.info.set_viewport(viewport);

            let mode = self.mode.copy(ctx);
            if !mode.contains(ScrollMode::VERTICAL) {
                self.content_size.height = final_size.height;
            }
            if !mode.contains(ScrollMode::HORIZONTAL) {
                self.content_size.width = final_size.width;
            }

            let mut content_offset = self.content_offset;
            let v_offset = *ScrollVerticalOffsetVar::get(ctx.vars);
            content_offset.y = (self.viewport_size.height - self.content_size.height) * v_offset;
            let h_offset = *ScrollHorizontalOffsetVar::get(ctx.vars);
            content_offset.x = (self.viewport_size.width - self.content_size.width) * h_offset;

            widget_layout.with_custom_transform(&RenderTransform::translation_px(content_offset), |wl| {
                self.child.arrange(ctx, wl, self.content_size);
            });

            if self.content_offset != content_offset {
                self.content_offset = content_offset;
                ctx.updates.render_update();
            }

            let v_ratio = self.viewport_size.height.0 as f32 / self.content_size.height.0 as f32;
            let h_ratio = self.viewport_size.width.0 as f32 / self.content_size.width.0 as f32;

            ScrollVerticalRatioVar::new().set_ne(ctx, v_ratio.fct()).unwrap();
            ScrollHorizontalRatioVar::new().set_ne(ctx, h_ratio.fct()).unwrap();
            ScrollContentSizeVar::new().set_ne(ctx, self.content_size).unwrap();
        }
        */

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_scroll_frame(
                self.scroll_id,
                self.viewport_size,
                PxRect::new(self.content_offset.to_point(), self.content_size),
                |frame| {
                    self.child.render(ctx, frame);
                },
            )
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            update.update_scroll(self.scroll_id, self.content_offset);
            self.child.render_update(ctx, update);
        }
    }
    ViewportNode {
        child: child.cfg_boxed(),
        scroll_id: ScrollId::new_unique(),
        mode: mode.into_var(),
        viewport_size: PxSize::zero(),
        content_size: PxSize::zero(),
        content_offset: PxVector::zero(),
        info: ScrollableInfo::default(),
    }
    .cfg_boxed()
}

/// Create a node that generates and presents the [vertical scrollbar].
///
/// [vertical scrollbar]: VerticalScrollBarViewVar
pub fn v_scrollbar_presenter() -> impl UiNode {
    scrollbar_presenter(VerticalScrollBarViewVar, scrollbar::Orientation::Vertical)
}

/// Create a node that generates and presents the [horizontal scrollbar].
///
/// [horizontal scrollbar]: HorizontalScrollBarViewVar
pub fn h_scrollbar_presenter() -> impl UiNode {
    scrollbar_presenter(HorizontalScrollBarViewVar, scrollbar::Orientation::Horizontal)
}

fn scrollbar_presenter(var: impl IntoVar<ViewGenerator<ScrollBarArgs>>, orientation: scrollbar::Orientation) -> impl UiNode {
    ViewGenerator::presenter(
        var,
        |_, _| {},
        move |_, is_new| {
            if is_new {
                DataUpdate::Update(ScrollBarArgs::new(orientation))
            } else {
                DataUpdate::Same
            }
        },
    )
}

/// Create a node that generates and presents the [scrollbar joiner].
///
/// [scrollbar joiner]: ScrollBarJoinerViewVar
pub fn scrollbar_joiner_presenter() -> impl UiNode {
    ViewGenerator::presenter_default(ScrollBarJoinerViewVar)
}

/// Create a node that implements [`ScrollUpCommand`], [`ScrollDownCommand`],
/// [`ScrollLeftCommand`] and [`ScrollRightCommand`] scoped on the widget.
pub fn scroll_commands_node(child: impl UiNode) -> impl UiNode {
    struct ScrollCommandsNode<C> {
        child: C,

        up: CommandHandle,
        down: CommandHandle,
        left: CommandHandle,
        right: CommandHandle,

        offset: Vector,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for ScrollCommandsNode<C> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            let scope = ctx.path.widget_id();

            self.up = ScrollUpCommand.scoped(scope).new_handle(ctx, ScrollContext::can_scroll_up(ctx));
            self.down = ScrollDownCommand.scoped(scope).new_handle(ctx, ScrollContext::can_scroll_down(ctx));
            self.left = ScrollLeftCommand.scoped(scope).new_handle(ctx, ScrollContext::can_scroll_left(ctx));
            self.right = ScrollRightCommand
                .scoped(scope)
                .new_handle(ctx, ScrollContext::can_scroll_right(ctx));

            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);

            self.up = CommandHandle::dummy();
            self.down = CommandHandle::dummy();
            self.left = CommandHandle::dummy();
            self.right = CommandHandle::dummy();
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            let scope = ctx.path.widget_id();

            subscriptions
                .event(ScrollUpCommand.scoped(scope))
                .event(ScrollDownCommand.scoped(scope))
                .event(ScrollLeftCommand.scoped(scope))
                .event(ScrollRightCommand.scoped(scope));

            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            self.up.set_enabled(ScrollContext::can_scroll_up(ctx));
            self.down.set_enabled(ScrollContext::can_scroll_down(ctx));
            self.left.set_enabled(ScrollContext::can_scroll_left(ctx));
            self.right.set_enabled(ScrollContext::can_scroll_right(ctx));
        }

        fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            let scope = ctx.path.widget_id();

            if let Some(args) = ScrollUpCommand.scoped(scope).update(args) {
                self.child.event(ctx, args);

                args.handle(|_| {
                    if ScrollRequest::from_args(args).map(|f| f.alternate).unwrap_or(false) {
                        self.offset.y -= VerticalLineUnitVar::get_clone(ctx) * AltFactorVar::get_clone(ctx);
                    } else {
                        self.offset.y -= VerticalLineUnitVar::get_clone(ctx);
                    }
                    ctx.updates.layout();
                });
            } else if let Some(args) = ScrollDownCommand.scoped(scope).update(args) {
                self.child.event(ctx, args);

                args.handle(|_| {
                    if ScrollRequest::from_args(args).map(|f| f.alternate).unwrap_or(false) {
                        self.offset.y += VerticalLineUnitVar::get_clone(ctx) * AltFactorVar::get_clone(ctx);
                    } else {
                        self.offset.y += VerticalLineUnitVar::get_clone(ctx);
                    }
                    ctx.updates.layout();
                });
            } else if let Some(args) = ScrollLeftCommand.scoped(scope).update(args) {
                self.child.event(ctx, args);

                args.handle(|_| {
                    if ScrollRequest::from_args(args).map(|f| f.alternate).unwrap_or(false) {
                        self.offset.x -= HorizontalLineUnitVar::get_clone(ctx) * AltFactorVar::get_clone(ctx);
                    } else {
                        self.offset.x -= HorizontalLineUnitVar::get_clone(ctx);
                    }
                    ctx.updates.layout();
                });
            } else if let Some(args) = ScrollRightCommand.scoped(scope).update(args) {
                self.child.event(ctx, args);

                args.handle(|_| {
                    if ScrollRequest::from_args(args).map(|f| f.alternate).unwrap_or(false) {
                        self.offset.x += HorizontalLineUnitVar::get_clone(ctx) * AltFactorVar::get_clone(ctx);
                    } else {
                        self.offset.x += HorizontalLineUnitVar::get_clone(ctx);
                    }
                    ctx.updates.layout();
                });
            } else {
                self.child.event(ctx, args);
            }
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let r = self.child.layout(ctx, wl);

            let viewport = *ScrollViewportSizeVar::get(ctx);
            let available_size = AvailableSize::finite(viewport);

            ctx.with_available_size(available_size, |ctx| {
                let default = 1.em().layout(ctx.for_y(), Px(0));
                let offset = self.offset.layout(ctx, PxVector::new(default, default));
                self.offset = Vector::zero();

                if offset.y != Px(0) {
                    ScrollContext::scroll_vertical(ctx, offset.y);
                }
                if offset.x != Px(0) {
                    ScrollContext::scroll_horizontal(ctx, offset.x);
                }
            });

            r
        }
    }

    ScrollCommandsNode {
        child: child.cfg_boxed(),

        up: CommandHandle::dummy(),
        down: CommandHandle::dummy(),
        left: CommandHandle::dummy(),
        right: CommandHandle::dummy(),

        offset: Vector::zero(),
    }
    .cfg_boxed()
}

/// Create a node that implements [`PageUpCommand`], [`PageDownCommand`],
/// [`PageLeftCommand`] and [`PageRightCommand`] scoped on the widget.
pub fn page_commands_node(child: impl UiNode) -> impl UiNode {
    struct PageCommandsNode<C> {
        child: C,

        up: CommandHandle,
        down: CommandHandle,
        left: CommandHandle,
        right: CommandHandle,

        offset: Vector,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for PageCommandsNode<C> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            let scope = ctx.path.widget_id();

            self.up = PageUpCommand.scoped(scope).new_handle(ctx, ScrollContext::can_scroll_up(ctx));
            self.down = PageDownCommand.scoped(scope).new_handle(ctx, ScrollContext::can_scroll_down(ctx));
            self.left = PageLeftCommand.scoped(scope).new_handle(ctx, ScrollContext::can_scroll_left(ctx));
            self.right = PageRightCommand.scoped(scope).new_handle(ctx, ScrollContext::can_scroll_right(ctx));

            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);

            self.up = CommandHandle::dummy();
            self.down = CommandHandle::dummy();
            self.left = CommandHandle::dummy();
            self.right = CommandHandle::dummy();
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            let scope = ctx.path.widget_id();

            subscriptions
                .event(PageUpCommand.scoped(scope))
                .event(PageDownCommand.scoped(scope))
                .event(PageLeftCommand.scoped(scope))
                .event(PageRightCommand.scoped(scope));

            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            self.up.set_enabled(ScrollContext::can_scroll_up(ctx));
            self.down.set_enabled(ScrollContext::can_scroll_down(ctx));
            self.left.set_enabled(ScrollContext::can_scroll_left(ctx));
            self.right.set_enabled(ScrollContext::can_scroll_right(ctx));
        }

        fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            let scope = ctx.path.widget_id();

            if let Some(args) = PageUpCommand.scoped(scope).update(args) {
                self.child.event(ctx, args);

                args.handle(|_| {
                    if ScrollRequest::from_args(args).map(|f| f.alternate).unwrap_or(false) {
                        self.offset.y -= VerticalPageUnitVar::get_clone(ctx) * AltFactorVar::get_clone(ctx);
                    } else {
                        self.offset.y -= VerticalPageUnitVar::get_clone(ctx);
                    }
                    ctx.updates.layout();
                });
            } else if let Some(args) = PageDownCommand.scoped(scope).update(args) {
                self.child.event(ctx, args);

                args.handle(|_| {
                    if ScrollRequest::from_args(args).map(|f| f.alternate).unwrap_or(false) {
                        self.offset.y += VerticalPageUnitVar::get_clone(ctx) * AltFactorVar::get_clone(ctx);
                    } else {
                        self.offset.y += VerticalPageUnitVar::get_clone(ctx);
                    }
                    ctx.updates.layout();
                });
            } else if let Some(args) = PageLeftCommand.scoped(scope).update(args) {
                self.child.event(ctx, args);

                args.handle(|_| {
                    if ScrollRequest::from_args(args).map(|f| f.alternate).unwrap_or(false) {
                        self.offset.x -= HorizontalPageUnitVar::get_clone(ctx) * AltFactorVar::get_clone(ctx);
                    } else {
                        self.offset.x -= HorizontalPageUnitVar::get_clone(ctx);
                    }
                    ctx.updates.layout();
                });
            } else if let Some(args) = PageRightCommand.scoped(scope).update(args) {
                self.child.event(ctx, args);

                args.handle(|_| {
                    if ScrollRequest::from_args(args).map(|f| f.alternate).unwrap_or(false) {
                        self.offset.x += HorizontalPageUnitVar::get_clone(ctx) * AltFactorVar::get_clone(ctx);
                    } else {
                        self.offset.x += HorizontalPageUnitVar::get_clone(ctx);
                    }
                    ctx.updates.layout();
                });
            } else {
                self.child.event(ctx, args);
            }
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let r = self.child.layout(ctx, wl);

            let viewport = *ScrollViewportSizeVar::get(ctx);
            let available_size = AvailableSize::finite(viewport);

            ctx.with_available_size(available_size, |ctx| {
                let offset = self.offset.layout(ctx, viewport.to_vector());
                self.offset = Vector::zero();

                if offset.y != Px(0) {
                    ScrollContext::scroll_vertical(ctx, offset.y);
                }
                if offset.x != Px(0) {
                    ScrollContext::scroll_horizontal(ctx, offset.x);
                }
            });

            r
        }
    }

    PageCommandsNode {
        child,

        up: CommandHandle::dummy(),
        down: CommandHandle::dummy(),
        left: CommandHandle::dummy(),
        right: CommandHandle::dummy(),

        offset: Vector::zero(),
    }
}

/// Create a node that implements [`ScrollToTopCommand`], [`ScrollToBottomCommand`],
/// [`ScrollToLeftmostCommand`] and [`ScrollToRightmostCommand`] scoped on the widget.
pub fn scroll_to_edge_commands_node(child: impl UiNode) -> impl UiNode {
    struct ScrollToEdgeCommandsNode<C> {
        child: C,

        top: CommandHandle,
        bottom: CommandHandle,
        leftmost: CommandHandle,
        rightmost: CommandHandle,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for ScrollToEdgeCommandsNode<C> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            let scope = ctx.path.widget_id();

            self.top = ScrollToTopCommand.scoped(scope).new_handle(ctx, ScrollContext::can_scroll_up(ctx));
            self.bottom = ScrollToBottomCommand
                .scoped(scope)
                .new_handle(ctx, ScrollContext::can_scroll_down(ctx));
            self.leftmost = ScrollToLeftmostCommand
                .scoped(scope)
                .new_handle(ctx, ScrollContext::can_scroll_left(ctx));
            self.rightmost = ScrollToRightmostCommand
                .scoped(scope)
                .new_handle(ctx, ScrollContext::can_scroll_right(ctx));

            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);

            self.top = CommandHandle::dummy();
            self.bottom = CommandHandle::dummy();
            self.leftmost = CommandHandle::dummy();
            self.rightmost = CommandHandle::dummy();
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            let scope = ctx.path.widget_id();

            subscriptions
                .event(ScrollToTopCommand.scoped(scope))
                .event(ScrollToBottomCommand.scoped(scope))
                .event(ScrollToLeftmostCommand.scoped(scope))
                .event(ScrollToRightmostCommand.scoped(scope));

            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            self.top.set_enabled(ScrollContext::can_scroll_up(ctx));
            self.bottom.set_enabled(ScrollContext::can_scroll_down(ctx));
            self.leftmost.set_enabled(ScrollContext::can_scroll_left(ctx));
            self.rightmost.set_enabled(ScrollContext::can_scroll_right(ctx));
        }

        fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            let scope = ctx.path.widget_id();

            if let Some(args) = ScrollToTopCommand.scoped(scope).update(args) {
                self.child.event(ctx, args);

                args.handle(|_| {
                    ScrollVerticalOffsetVar::new().set_ne(ctx, 0.fct()).unwrap();
                });
            } else if let Some(args) = ScrollToBottomCommand.scoped(scope).update(args) {
                self.child.event(ctx, args);

                args.handle(|_| {
                    ScrollVerticalOffsetVar::new().set_ne(ctx, 1.fct()).unwrap();
                });
            } else if let Some(args) = ScrollToLeftmostCommand.scoped(scope).update(args) {
                self.child.event(ctx, args);

                args.handle(|_| {
                    ScrollHorizontalOffsetVar::new().set_ne(ctx, 0.fct()).unwrap();
                });
            } else if let Some(args) = ScrollToRightmostCommand.scoped(scope).update(args) {
                self.child.event(ctx, args);

                args.handle(|_| {
                    ScrollHorizontalOffsetVar::new().set_ne(ctx, 1.fct()).unwrap();
                });
            } else {
                self.child.event(ctx, args);
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

/// Create a node that implements [`ScrollToCommand`] scoped on the widget.
pub fn scroll_to_command_node(child: impl UiNode) -> impl UiNode {
    struct ScrollToCommandNode<C> {
        child: C,

        handle: CommandHandle,
        scroll_to: Option<(WidgetLayoutInfo, ScrollToMode)>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for ScrollToCommandNode<C> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.handle = ScrollToCommand.scoped(ctx.path.widget_id()).new_handle(ctx, true);
            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.handle = CommandHandle::dummy();
            self.child.deinit(ctx);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.event(ScrollToCommand.scoped(ctx.path.widget_id()));
            self.child.subscriptions(ctx, subscriptions);
        }

        fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            let self_id = ctx.path.widget_id();
            if let Some(args) = ScrollToCommand.scoped(self_id).update(args) {
                // event send to us
                if let Some(request) = ScrollToRequest::from_args(args) {
                    // has unhandled request
                    if let Some(target) = ctx.info_tree.find(request.widget_id) {
                        // target exists
                        if let Some(us) = target.ancestors().find(|w| w.widget_id() == self_id) {
                            // target is descendant
                            if us.is_scrollable() {
                                // we are a scrollable.

                                let target = target.inner_info();
                                let mode = request.mode;

                                // will scroll on the next arrange.
                                self.scroll_to = Some((target, mode));
                                ctx.updates.layout();

                                args.stop_propagation();
                            }
                        }
                    }
                }
                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let r = self.child.layout(ctx, wl);

            if let Some((target, mode)) = self.scroll_to.take() {
                let us = ctx.info_tree.find(ctx.path.widget_id()).unwrap();
                if let Some(viewport_bounds) = us.viewport() {
                    let target_bounds = target.bounds();
                    match mode {
                        ScrollToMode::Minimal { margin } => {
                            let margin = margin.layout(ctx, AvailableSize::from_size(target_bounds.size), PxSideOffsets::zero());
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
                            scrollable_point,
                        } => {
                            let default = (target_bounds.size / Px(2)).to_vector().to_point();
                            let widget_point = widget_point.layout(ctx, AvailableSize::from_size(target_bounds.size), default);

                            let default = (viewport_bounds.size / Px(2)).to_vector().to_point();
                            let scrollable_point = scrollable_point.layout(ctx, AvailableSize::from_size(viewport_bounds.size), default);

                            let widget_point = widget_point + target_bounds.origin.to_vector();
                            let scrollable_point = scrollable_point + viewport_bounds.origin.to_vector(); // TODO origin non-zero?

                            let diff = widget_point - scrollable_point;

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
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for ScrollWheelNode<C> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.event(MouseWheelEvent);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            if let Some(args) = MouseWheelEvent.update(args) {
                if args.concerns_widget(ctx) {
                    if let Some(delta) = args.scroll_delta(*AltFactorVar::get(ctx)) {
                        args.handle(|_| {
                            match delta {
                                MouseScrollDelta::LineDelta(x, y) => {
                                    self.offset.x -= HorizontalLineUnitVar::get_clone(ctx) * x.fct();
                                    self.offset.y -= VerticalLineUnitVar::get_clone(ctx) * y.fct();
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
                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let r = self.child.layout(ctx, wl);

            let viewport = *ScrollViewportSizeVar::get(ctx);
            let available_size = AvailableSize::finite(viewport);

            ctx.with_available_size(available_size, |ctx| {
                let offset = self.offset.layout(ctx, available_size, viewport.to_vector());
                self.offset = Vector::zero();

                if offset.y != Px(0) {
                    ScrollContext::scroll_vertical(ctx, offset.y);
                }
                if offset.x != Px(0) {
                    ScrollContext::scroll_horizontal(ctx, offset.x);
                }
            });

            r
        }
    }
    ScrollWheelNode {
        child: child.cfg_boxed(),
        offset: Vector::zero(),
    }
    .cfg_boxed()
}
