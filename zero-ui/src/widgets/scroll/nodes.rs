//! UI nodes used for building the scroll widget.
//!
use std::cell::Cell;

use crate::prelude::new_widget::*;

use crate::core::{
    focus::FOCUS_CHANGED_EVENT,
    mouse::{MouseScrollDelta, MOUSE_WHEEL_EVENT},
};

use super::commands::*;
use super::scroll_properties::*;
use super::types::*;

/// The actual content presenter.
pub fn viewport(child: impl UiNode, mode: impl IntoVar<ScrollMode>) -> impl UiNode {
    #[ui_node(struct ViewportNode {
        child: impl UiNode,
        mode: impl Var<ScrollMode>,

        viewport_unit: PxSize,
        viewport_size: PxSize,
        content_size: PxSize,
        content_offset: PxVector,

        auto_hide_extra: PxSideOffsets,
        last_render_offset: Cell<PxVector>,

        binding_key: FrameValueKey<PxTransform>,

        info: ScrollInfo,
    })]
    impl UiNode for ViewportNode {
        fn init(&mut self) {
            WIDGET
                .sub_var(&self.mode)
                .sub_var(&SCROLL_VERTICAL_OFFSET_VAR)
                .sub_var(&SCROLL_HORIZONTAL_OFFSET_VAR);
            self.child.init();
        }

        fn info(&self, builder: &mut WidgetInfoBuilder) {
            builder.meta().set(&SCROLL_INFO_ID, self.info.clone());
            self.child.info(builder);
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            self.child.update(updates);

            if self.mode.is_new() || SCROLL_VERTICAL_OFFSET_VAR.is_new() || SCROLL_HORIZONTAL_OFFSET_VAR.is_new() {
                WIDGET.layout();
            }
        }

        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            let constraints = LAYOUT.constraints();
            if constraints.is_fill_max().all() {
                return constraints.fill_size();
            }

            let mode = self.mode.get();

            let viewport_unit = constraints.fill_size();
            let define_vp_unit = DEFINE_VIEWPORT_UNIT_VAR.get() // requested
                && viewport_unit.width > Px(0) // and has fill-size
                && viewport_unit.height > Px(0)
                && constraints.max_size() == Some(viewport_unit); // that is not just min size.

            let ct_size = LAYOUT.with_constraints(
                {
                    let mut c = constraints;
                    c = c.with_min_size(viewport_unit);
                    if mode.contains(ScrollMode::VERTICAL) {
                        c = c.with_unbounded_y();
                    }
                    if mode.contains(ScrollMode::HORIZONTAL) {
                        c = c.with_unbounded_x();
                    }
                    c
                },
                || {
                    if define_vp_unit {
                        LAYOUT.with_viewport(viewport_unit, || self.child.measure(wm))
                    } else {
                        self.child.measure(wm)
                    }
                },
            );

            constraints.fill_size_or(ct_size)
        }
        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            let mode = self.mode.get();

            let constraints = LAYOUT.constraints();
            let viewport_unit = constraints.fill_size();
            let define_vp_unit = DEFINE_VIEWPORT_UNIT_VAR.get() // requested
                && viewport_unit.width > Px(0) // and has fill-size
                && viewport_unit.height > Px(0)
                && constraints.max_size() == Some(viewport_unit); // that is not just min size.

            let ct_size = LAYOUT.with_constraints(
                {
                    let mut c = constraints;
                    c = c.with_min_size(viewport_unit);
                    if mode.contains(ScrollMode::VERTICAL) {
                        c = c.with_unbounded_y();
                    }
                    if mode.contains(ScrollMode::HORIZONTAL) {
                        c = c.with_unbounded_x();
                    }
                    c
                },
                || {
                    if define_vp_unit {
                        LAYOUT.with_viewport(viewport_unit, || {
                            self.viewport_unit = viewport_unit;
                            self.child.layout(wl)
                        })
                    } else {
                        self.child.layout(wl)
                    }
                },
            );

            let viewport_size = constraints.fill_size_or(ct_size);
            if self.viewport_size != viewport_size {
                self.viewport_size = viewport_size;
                SCROLL_VIEWPORT_SIZE_VAR.set(viewport_size).unwrap();
                WIDGET.render();
            }

            self.auto_hide_extra = LAYOUT.with_viewport(viewport_size, || {
                LAYOUT.with_constraints(PxConstraints2d::new_fill_size(viewport_size), || {
                    AUTO_HIDE_EXTRA_VAR.layout_dft(PxSideOffsets::new(
                        viewport_size.height,
                        viewport_size.width,
                        viewport_size.height,
                        viewport_size.width,
                    ))
                })
            });
            self.auto_hide_extra.top = self.auto_hide_extra.top.max(Px(0));
            self.auto_hide_extra.right = self.auto_hide_extra.right.max(Px(0));
            self.auto_hide_extra.bottom = self.auto_hide_extra.bottom.max(Px(0));
            self.auto_hide_extra.left = self.auto_hide_extra.left.max(Px(0));

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

                // check if scrolled using only `render_update` to the end of the `auto_hide_extra` space.
                let update_only_offset = (self.last_render_offset.get() - self.content_offset).abs();
                const OFFSET_EXTRA: Px = Px(20); // give a margin of error for widgets that render outside bounds.
                let mut need_full_render = if update_only_offset.y < Px(0) {
                    update_only_offset.y.abs() + OFFSET_EXTRA > self.auto_hide_extra.top
                } else {
                    update_only_offset.y + OFFSET_EXTRA > self.auto_hide_extra.bottom
                };
                if !need_full_render {
                    need_full_render = if update_only_offset.x < Px(0) {
                        update_only_offset.x.abs() + OFFSET_EXTRA > self.auto_hide_extra.left
                    } else {
                        update_only_offset.x + OFFSET_EXTRA > self.auto_hide_extra.right
                    };
                }

                if need_full_render {
                    // need to render more widgets, `auto_hide_extra` was reached using only `render_update`
                    WIDGET.render();
                } else {
                    WIDGET.render_update();
                }
            }

            let v_ratio = self.viewport_size.height.0 as f32 / self.content_size.height.0 as f32;
            let h_ratio = self.viewport_size.width.0 as f32 / self.content_size.width.0 as f32;

            SCROLL_VERTICAL_RATIO_VAR.set_ne(v_ratio.fct()).unwrap();
            SCROLL_HORIZONTAL_RATIO_VAR.set_ne(h_ratio.fct()).unwrap();
            SCROLL_CONTENT_SIZE_VAR.set_ne(self.content_size).unwrap();

            self.viewport_size
        }

        fn render(&self, frame: &mut FrameBuilder) {
            self.info.set_viewport_transform(*frame.transform());
            self.last_render_offset.set(self.content_offset);

            let mut culling_rect = PxBox::from_size(self.viewport_size);
            culling_rect.min.y -= self.auto_hide_extra.top;
            culling_rect.max.x += self.auto_hide_extra.right;
            culling_rect.max.y += self.auto_hide_extra.bottom;
            culling_rect.min.x -= self.auto_hide_extra.left;
            let culling_rect = frame.transform().outer_transformed(culling_rect).unwrap_or(culling_rect).to_rect();

            frame.push_reference_frame(
                self.binding_key.into(),
                self.binding_key.bind(self.content_offset.into(), true),
                true,
                false,
                |frame| {
                    frame.with_auto_hide_rect(culling_rect, |frame| {
                        self.child.render(frame);
                    });
                },
            );
        }

        fn render_update(&self, update: &mut FrameUpdate) {
            self.info.set_viewport_transform(*update.transform());

            update.with_transform(self.binding_key.update(self.content_offset.into(), true), false, |update| {
                self.child.render_update(update);
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
        auto_hide_extra: PxSideOffsets::zero(),
        last_render_offset: Cell::new(PxVector::zero()),
        info: ScrollInfo::default(),

        binding_key: FrameValueKey::new_unique(),
    }
    .cfg_boxed()
}

/// Create a node that generates and presents the [vertical scrollbar].
///
/// [vertical scrollbar]: VERTICAL_SCROLLBAR_GEN_VAR
pub fn v_scrollbar_presenter() -> impl UiNode {
    scrollbar_presenter(VERTICAL_SCROLLBAR_GEN_VAR, scrollbar::Orientation::Vertical)
}

/// Create a node that generates and presents the [horizontal scrollbar].
///
/// [horizontal scrollbar]: HORIZONTAL_SCROLLBAR_GEN_VAR
pub fn h_scrollbar_presenter() -> impl UiNode {
    scrollbar_presenter(HORIZONTAL_SCROLLBAR_GEN_VAR, scrollbar::Orientation::Horizontal)
}

fn scrollbar_presenter(var: impl IntoVar<WidgetFn<ScrollBarArgs>>, orientation: scrollbar::Orientation) -> impl UiNode {
    WidgetFn::presenter(var, move |is_new| {
        if is_new {
            DataUpdate::Update(ScrollBarArgs::new(orientation))
        } else {
            DataUpdate::Same
        }
    })
}

/// Create a node that generates and presents the [scrollbar joiner].
///
/// [scrollbar joiner]: SCROLLBAR_JOINER_GEN_VAR
pub fn scrollbar_joiner_presenter() -> impl UiNode {
    WidgetFn::presenter_default(SCROLLBAR_JOINER_GEN_VAR)
}

/// Create a node that implements [`SCROLL_UP_CMD`], [`SCROLL_DOWN_CMD`],
/// [`SCROLL_LEFT_CMD`] and [`SCROLL_RIGHT_CMD`] scoped on the widget.
pub fn scroll_commands_node(child: impl UiNode) -> impl UiNode {
    #[ui_node(struct ScrollCommandsNode {
        child: impl UiNode,

        up: CommandHandle,
        down: CommandHandle,
        left: CommandHandle,
        right: CommandHandle,

        layout_line: PxVector,
    })]
    impl UiNode for ScrollCommandsNode {
        fn init(&mut self) {
            WIDGET.sub_var(&VERTICAL_LINE_UNIT_VAR).sub_var(&HORIZONTAL_LINE_UNIT_VAR);

            let scope = WIDGET.id();

            self.up = SCROLL_UP_CMD.scoped(scope).subscribe(SCROLL.can_scroll_up());
            self.down = SCROLL_DOWN_CMD.scoped(scope).subscribe(SCROLL.can_scroll_down());
            self.left = SCROLL_LEFT_CMD.scoped(scope).subscribe(SCROLL.can_scroll_left());
            self.right = SCROLL_RIGHT_CMD.scoped(scope).subscribe(SCROLL.can_scroll_right());

            self.child.init();
        }

        fn deinit(&mut self) {
            self.child.deinit();

            self.up = CommandHandle::dummy();
            self.down = CommandHandle::dummy();
            self.left = CommandHandle::dummy();
            self.right = CommandHandle::dummy();
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            self.child.update(updates);

            self.up.set_enabled(SCROLL.can_scroll_up());
            self.down.set_enabled(SCROLL.can_scroll_down());
            self.left.set_enabled(SCROLL.can_scroll_left());
            self.right.set_enabled(SCROLL.can_scroll_right());

            if VERTICAL_LINE_UNIT_VAR.is_new() || HORIZONTAL_LINE_UNIT_VAR.is_new() {
                WIDGET.layout();
            }
        }

        fn event(&mut self, update: &EventUpdate) {
            self.child.event(update);

            let scope = WIDGET.id();

            if let Some(args) = SCROLL_UP_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.up, |_| {
                    let mut offset = -self.layout_line.y;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_vertical_clamp(offset, args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = SCROLL_DOWN_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.down, |_| {
                    let mut offset = self.layout_line.y;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_vertical_clamp(offset, args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = SCROLL_LEFT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.left, |_| {
                    let mut offset = -self.layout_line.x;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_horizontal_clamp(offset, args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = SCROLL_RIGHT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.right, |_| {
                    let mut offset = self.layout_line.x;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_horizontal_clamp(offset, args.clamp.0, args.clamp.1);
                });
            }
        }

        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            self.child.measure(wm)
        }
        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            let r = self.child.layout(wl);

            let viewport = SCROLL_VIEWPORT_SIZE_VAR.get();
            LAYOUT.with_constraints(PxConstraints2d::new_fill_size(viewport), || {
                self.layout_line = PxVector::new(
                    HORIZONTAL_LINE_UNIT_VAR.layout_dft_x(Px(20)),
                    VERTICAL_LINE_UNIT_VAR.layout_dft_y(Px(20)),
                );
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

        layout_line: PxVector::zero(),
    }
    .cfg_boxed()
}

/// Create a node that implements [`PAGE_UP_CMD`], [`PAGE_DOWN_CMD`],
/// [`PAGE_LEFT_CMD`] and [`PAGE_RIGHT_CMD`] scoped on the widget.
pub fn page_commands_node(child: impl UiNode) -> impl UiNode {
    #[ui_node(struct PageCommandsNode {
        child: impl UiNode,

        up: CommandHandle,
        down: CommandHandle,
        left: CommandHandle,
        right: CommandHandle,

        layout_page: PxVector,
    })]
    impl UiNode for PageCommandsNode {
        fn init(&mut self) {
            WIDGET.sub_var(&VERTICAL_PAGE_UNIT_VAR).sub_var(&HORIZONTAL_PAGE_UNIT_VAR);

            let scope = WIDGET.id();

            self.up = PAGE_UP_CMD.scoped(scope).subscribe(SCROLL.can_scroll_up());
            self.down = PAGE_DOWN_CMD.scoped(scope).subscribe(SCROLL.can_scroll_down());
            self.left = PAGE_LEFT_CMD.scoped(scope).subscribe(SCROLL.can_scroll_left());
            self.right = PAGE_RIGHT_CMD.scoped(scope).subscribe(SCROLL.can_scroll_right());

            self.child.init();
        }

        fn deinit(&mut self) {
            self.child.deinit();

            self.up = CommandHandle::dummy();
            self.down = CommandHandle::dummy();
            self.left = CommandHandle::dummy();
            self.right = CommandHandle::dummy();
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            self.child.update(updates);

            self.up.set_enabled(SCROLL.can_scroll_up());
            self.down.set_enabled(SCROLL.can_scroll_down());
            self.left.set_enabled(SCROLL.can_scroll_left());
            self.right.set_enabled(SCROLL.can_scroll_right());

            if VERTICAL_PAGE_UNIT_VAR.is_new() || HORIZONTAL_PAGE_UNIT_VAR.is_new() {
                WIDGET.layout();
            }
        }

        fn event(&mut self, update: &EventUpdate) {
            self.child.event(update);

            let scope = WIDGET.id();

            if let Some(args) = PAGE_UP_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.up, |_| {
                    let mut offset = -self.layout_page.y;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_vertical_clamp(offset, args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = PAGE_DOWN_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.down, |_| {
                    let mut offset = self.layout_page.y;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_vertical_clamp(offset, args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = PAGE_LEFT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.left, |_| {
                    let mut offset = -self.layout_page.x;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_horizontal_clamp(offset, args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = PAGE_RIGHT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.right, |_| {
                    let mut offset = self.layout_page.x;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_horizontal_clamp(offset, args.clamp.0, args.clamp.1);
                });
            }
        }

        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            self.child.measure(wm)
        }
        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            let r = self.child.layout(wl);

            let viewport = SCROLL_VIEWPORT_SIZE_VAR.get();
            LAYOUT.with_constraints(PxConstraints2d::new_fill_size(viewport), || {
                self.layout_page = PxVector::new(
                    HORIZONTAL_PAGE_UNIT_VAR.layout_dft_x(Px(20)),
                    VERTICAL_PAGE_UNIT_VAR.layout_dft_y(Px(20)),
                );
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

        layout_page: PxVector::zero(),
    }
}

/// Create a node that implements [`SCROLL_TO_TOP_CMD`], [`SCROLL_TO_BOTTOM_CMD`],
/// [`SCROLL_TO_LEFTMOST_CMD`] and [`SCROLL_TO_RIGHTMOST_CMD`] scoped on the widget.
pub fn scroll_to_edge_commands_node(child: impl UiNode) -> impl UiNode {
    #[ui_node(struct ScrollToEdgeCommandsNode {
        child: impl UiNode,

        top: CommandHandle,
        bottom: CommandHandle,
        leftmost: CommandHandle,
        rightmost: CommandHandle,
    })]
    impl UiNode for ScrollToEdgeCommandsNode {
        fn init(&mut self) {
            let scope = WIDGET.id();

            self.top = SCROLL_TO_TOP_CMD.scoped(scope).subscribe(SCROLL.can_scroll_up());
            self.bottom = SCROLL_TO_BOTTOM_CMD.scoped(scope).subscribe(SCROLL.can_scroll_down());
            self.leftmost = SCROLL_TO_LEFTMOST_CMD.scoped(scope).subscribe(SCROLL.can_scroll_left());
            self.rightmost = SCROLL_TO_RIGHTMOST_CMD.scoped(scope).subscribe(SCROLL.can_scroll_right());

            self.child.init();
        }

        fn deinit(&mut self) {
            self.child.deinit();

            self.top = CommandHandle::dummy();
            self.bottom = CommandHandle::dummy();
            self.leftmost = CommandHandle::dummy();
            self.rightmost = CommandHandle::dummy();
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            self.child.update(updates);

            self.top.set_enabled(SCROLL.can_scroll_up());
            self.bottom.set_enabled(SCROLL.can_scroll_down());
            self.leftmost.set_enabled(SCROLL.can_scroll_left());
            self.rightmost.set_enabled(SCROLL.can_scroll_right());
        }

        fn event(&mut self, update: &EventUpdate) {
            self.child.event(update);

            let scope = WIDGET.id();

            if let Some(args) = SCROLL_TO_TOP_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.top, |_| {
                    SCROLL.chase_vertical(|_| 0.fct());
                });
            } else if let Some(args) = SCROLL_TO_BOTTOM_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.bottom, |_| {
                    SCROLL.chase_vertical(|_| 1.fct());
                });
            } else if let Some(args) = SCROLL_TO_LEFTMOST_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.leftmost, |_| {
                    SCROLL.chase_horizontal(|_| 0.fct());
                });
            } else if let Some(args) = SCROLL_TO_RIGHTMOST_CMD.scoped(scope).on(update) {
                args.handle_enabled(&self.rightmost, |_| {
                    SCROLL.chase_horizontal(|_| 1.fct());
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
    #[ui_node(struct ScrollToCommandNode {
        child: impl UiNode,

        handle: CommandHandle,
        scroll_to: Option<(WidgetBoundsInfo, ScrollToMode)>,
    })]
    impl UiNode for ScrollToCommandNode {
        fn init(&mut self) {
            self.handle = SCROLL_TO_CMD.scoped(WIDGET.id()).subscribe(true);
            self.child.init();
        }

        fn deinit(&mut self) {
            self.handle = CommandHandle::dummy();
            self.child.deinit();
        }

        fn event(&mut self, update: &EventUpdate) {
            let self_id = WIDGET.id();
            if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                if let Some(path) = &args.new_focus {
                    if path.contains(self_id) && path.widget_id() != self_id {
                        // probable focus move inside.
                        let tree = WINDOW.widget_tree();
                        if let Some(target) = tree.get(path.widget_id()) {
                            // target exits
                            if let Some(us) = target.ancestors().find(|w| w.id() == self_id) {
                                // confirmed, target is descendant
                                if us.is_scroll() {
                                    // we are a scroll.

                                    let bounds = target.bounds_info();
                                    let mode = SCROLL_TO_FOCUSED_MODE_VAR.get();

                                    self.scroll_to = Some((bounds, mode));
                                    WIDGET.layout();
                                }
                            }
                        }
                    }
                }
            } else if let Some(args) = SCROLL_TO_CMD.scoped(self_id).on(update) {
                // event send to us and enabled
                if let Some(request) = ScrollToRequest::from_args(args) {
                    // has unhandled request
                    let tree = WINDOW.widget_tree();
                    if let Some(target) = tree.get(request.widget_id) {
                        // target exists
                        if let Some(us) = target.ancestors().find(|w| w.id() == self_id) {
                            // target is descendant
                            if us.is_scroll() {
                                // we are a scroll.

                                let bounds = target.bounds_info();
                                let mode = request.mode;

                                // will scroll on the next arrange.
                                self.scroll_to = Some((bounds, mode));
                                WIDGET.layout();

                                args.propagation().stop();
                            }
                        }
                    }
                }
            }
            self.child.event(update);
        }

        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            self.child.measure(wm)
        }
        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            let r = self.child.layout(wl);

            if let Some((bounds, mode)) = self.scroll_to.take() {
                let tree = WINDOW.widget_tree();
                let us = tree.get(WIDGET.id()).unwrap();
                if let Some(viewport_bounds) = us.viewport() {
                    let target_bounds = bounds.inner_bounds();
                    match mode {
                        ScrollToMode::Minimal { margin } => {
                            let margin = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(target_bounds.size), || margin.layout());
                            let mut target_bounds = target_bounds;
                            target_bounds.origin.x -= margin.left;
                            target_bounds.origin.y -= margin.top;
                            target_bounds.size.width += margin.horizontal();
                            target_bounds.size.height += margin.vertical();

                            if target_bounds.size.width < viewport_bounds.size.width {
                                if target_bounds.origin.x < viewport_bounds.origin.x {
                                    let diff = target_bounds.origin.x - viewport_bounds.origin.x;
                                    SCROLL.scroll_horizontal(diff);
                                } else if target_bounds.max_x() > viewport_bounds.max_x() {
                                    let diff = target_bounds.max_x() - viewport_bounds.max_x();
                                    SCROLL.scroll_horizontal(diff);
                                }
                            } else {
                                let target_center_x = (target_bounds.size.width / Px(2)) + target_bounds.origin.x;
                                let viewport_center_x = (target_bounds.size.width / Px(2)) + viewport_bounds.origin.x;

                                let diff = target_center_x - viewport_center_x;
                                SCROLL.scroll_horizontal(diff);
                            }
                            if target_bounds.size.height < viewport_bounds.size.height {
                                if target_bounds.origin.y < viewport_bounds.origin.y {
                                    let diff = target_bounds.origin.y - viewport_bounds.origin.y;
                                    SCROLL.scroll_vertical(diff);
                                } else if target_bounds.max_y() > viewport_bounds.max_y() {
                                    let diff = target_bounds.max_y() - viewport_bounds.max_y();
                                    SCROLL.scroll_vertical(diff);
                                }
                            } else {
                                let target_center_y = (target_bounds.size.height / Px(2)) + target_bounds.origin.y;
                                let viewport_center_y = (target_bounds.size.height / Px(2)) + viewport_bounds.origin.y;

                                let diff = target_center_y - viewport_center_y;
                                SCROLL.scroll_vertical(diff);
                            }
                        }
                        ScrollToMode::Center {
                            widget_point,
                            scroll_point,
                        } => {
                            let default = (target_bounds.size / Px(2)).to_vector().to_point();
                            let widget_point = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(target_bounds.size), || {
                                widget_point.layout_dft(default)
                            });

                            let default = (viewport_bounds.size / Px(2)).to_vector().to_point();
                            let scroll_point = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(viewport_bounds.size), || {
                                scroll_point.layout_dft(default)
                            });

                            let widget_point = widget_point + target_bounds.origin.to_vector();
                            let scroll_point = scroll_point + viewport_bounds.origin.to_vector();

                            let diff = widget_point - scroll_point;

                            SCROLL.scroll_vertical(diff.y);
                            SCROLL.scroll_horizontal(diff.x);
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
    #[ui_node(struct ScrollWheelNode {
        child: impl UiNode,
        offset: Vector,
        mouse_wheel_handle: Option<EventHandle>,
    })]
    impl UiNode for ScrollWheelNode {
        fn init(&mut self) {
            self.mouse_wheel_handle = Some(MOUSE_WHEEL_EVENT.subscribe(WIDGET.id()));
            self.child.init();
        }

        fn deinit(&mut self) {
            self.mouse_wheel_handle = None;
            self.child.deinit();
        }

        fn event(&mut self, update: &EventUpdate) {
            if let Some(args) = MOUSE_WHEEL_EVENT.on(update) {
                if let Some(delta) = args.scroll_delta(ALT_FACTOR_VAR.get()) {
                    args.handle(|_| {
                        match delta {
                            MouseScrollDelta::LineDelta(x, y) => {
                                self.offset.x -= HORIZONTAL_WHEEL_UNIT_VAR.get() * x.fct();
                                self.offset.y -= VERTICAL_WHEEL_UNIT_VAR.get() * y.fct();
                            }
                            MouseScrollDelta::PixelDelta(x, y) => {
                                self.offset.x -= x.px();
                                self.offset.y -= y.px();
                            }
                        }

                        WIDGET.layout();
                    });
                }
            }
            self.child.event(update);
        }

        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            self.child.measure(wm)
        }
        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            let r = self.child.layout(wl);

            let viewport = SCROLL_VIEWPORT_SIZE_VAR.get();

            LAYOUT.with_constraints(PxConstraints2d::new_fill_size(viewport), || {
                let offset = self.offset.layout_dft(viewport.to_vector());
                self.offset = Vector::zero();

                if offset.y != Px(0) {
                    SCROLL.scroll_vertical(offset.y);
                }
                if offset.x != Px(0) {
                    SCROLL.scroll_horizontal(offset.x);
                }
            });

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
