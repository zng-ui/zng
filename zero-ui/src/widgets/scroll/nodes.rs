//! UI nodes used for building the scroll widget.
//!

use crate::prelude::new_widget::*;

use crate::core::{
    focus::FOCUS_CHANGED_EVENT,
    mouse::{MouseScrollDelta, MOUSE_WHEEL_EVENT},
};

use super::commands::*;
use super::scroll_properties::*;
use super::scrollbar::Orientation;
use super::types::*;

/// The actual content presenter.
pub fn viewport(child: impl UiNode, mode: impl IntoVar<ScrollMode>) -> impl UiNode {
    let mode = mode.into_var();
    let binding_key = FrameValueKey::new_unique();

    let mut viewport_size = PxSize::zero();
    let mut viewport_unit = PxSize::zero();
    let mut content_offset = PxVector::zero();
    let mut auto_hide_extra = PxSideOffsets::zero();
    let mut last_render_offset = PxVector::zero();
    let scroll_info = ScrollInfo::default();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var(&mode)
                .sub_var(&SCROLL_VERTICAL_OFFSET_VAR)
                .sub_var(&SCROLL_HORIZONTAL_OFFSET_VAR);
        }
        UiNodeOp::Info { info } => {
            info.set_meta(&SCROLL_INFO_ID, scroll_info.clone());
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);

            if mode.is_new() || SCROLL_VERTICAL_OFFSET_VAR.is_new() || SCROLL_HORIZONTAL_OFFSET_VAR.is_new() {
                WIDGET.layout();
            }
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let constraints = LAYOUT.constraints();
            if constraints.is_fill_max().all() {
                *desired_size = constraints.fill_size();
                child.delegated();
                return;
            }

            let mode = mode.get();

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
                        LAYOUT.with_viewport(viewport_unit, || child.measure(wm))
                    } else {
                        child.measure(wm)
                    }
                },
            );

            *desired_size = constraints.fill_size_or(ct_size);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let mode = mode.get();

            let constraints = LAYOUT.constraints();
            let vp_unit = constraints.fill_size();
            let define_vp_unit = DEFINE_VIEWPORT_UNIT_VAR.get() // requested
                && vp_unit.width > Px(0) // and has fill-size
                && vp_unit.height > Px(0)
                && constraints.max_size() == Some(vp_unit); // that is not just min size.

            let ct_size = LAYOUT.with_constraints(
                {
                    let mut c = constraints;
                    c = c.with_min_size(vp_unit);
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
                        LAYOUT.with_viewport(vp_unit, || {
                            viewport_unit = vp_unit;
                            child.layout(wl)
                        })
                    } else {
                        child.layout(wl)
                    }
                },
            );

            let vp_size = constraints.fill_size_or(ct_size);
            if viewport_size != vp_size {
                viewport_size = vp_size;
                SCROLL_VIEWPORT_SIZE_VAR.set(vp_size).unwrap();
                WIDGET.render();
            }

            auto_hide_extra = LAYOUT.with_viewport(vp_size, || {
                LAYOUT.with_constraints(PxConstraints2d::new_fill_size(vp_size), || {
                    AUTO_HIDE_EXTRA_VAR.layout_dft(PxSideOffsets::new(vp_size.height, vp_size.width, vp_size.height, vp_size.width))
                })
            });
            auto_hide_extra.top = auto_hide_extra.top.max(Px(0));
            auto_hide_extra.right = auto_hide_extra.right.max(Px(0));
            auto_hide_extra.bottom = auto_hide_extra.bottom.max(Px(0));
            auto_hide_extra.left = auto_hide_extra.left.max(Px(0));

            scroll_info.set_viewport_size(vp_size);

            let mut content_size = ct_size;
            if !mode.contains(ScrollMode::VERTICAL) {
                content_size.height = vp_size.height;
            }
            if !mode.contains(ScrollMode::HORIZONTAL) {
                content_size.width = vp_size.width;
            }

            let mut ct_offset = PxVector::zero();
            let v_offset = SCROLL_VERTICAL_OFFSET_VAR.get();
            ct_offset.y = (viewport_size.height - content_size.height) * v_offset;
            let h_offset = SCROLL_HORIZONTAL_OFFSET_VAR.get();
            ct_offset.x = (viewport_size.width - content_size.width) * h_offset;

            if ct_offset != content_offset {
                content_offset = ct_offset;

                // check if scrolled using only `render_update` to the end of the `auto_hide_extra` space.
                let update_only_offset = (last_render_offset - content_offset).abs();
                const OFFSET_EXTRA: Px = Px(20); // give a margin of error for widgets that render outside bounds.
                let mut need_full_render = if update_only_offset.y < Px(0) {
                    update_only_offset.y.abs() + OFFSET_EXTRA > auto_hide_extra.top
                } else {
                    update_only_offset.y + OFFSET_EXTRA > auto_hide_extra.bottom
                };
                if !need_full_render {
                    need_full_render = if update_only_offset.x < Px(0) {
                        update_only_offset.x.abs() + OFFSET_EXTRA > auto_hide_extra.left
                    } else {
                        update_only_offset.x + OFFSET_EXTRA > auto_hide_extra.right
                    };
                }

                if need_full_render {
                    // need to render more widgets, `auto_hide_extra` was reached using only `render_update`
                    WIDGET.render();
                } else {
                    WIDGET.render_update();
                }
            }

            let v_ratio = viewport_size.height.0 as f32 / content_size.height.0 as f32;
            let h_ratio = viewport_size.width.0 as f32 / content_size.width.0 as f32;

            SCROLL_VERTICAL_RATIO_VAR.set_ne(v_ratio.fct()).unwrap();
            SCROLL_HORIZONTAL_RATIO_VAR.set_ne(h_ratio.fct()).unwrap();
            SCROLL_CONTENT_SIZE_VAR.set_ne(content_size).unwrap();

            *final_size = viewport_size;
        }
        UiNodeOp::Render { frame } => {
            scroll_info.set_viewport_transform(*frame.transform());
            last_render_offset = content_offset;

            let mut culling_rect = PxBox::from_size(viewport_size);
            culling_rect.min.y -= auto_hide_extra.top;
            culling_rect.max.x += auto_hide_extra.right;
            culling_rect.max.y += auto_hide_extra.bottom;
            culling_rect.min.x -= auto_hide_extra.left;
            let culling_rect = frame.transform().outer_transformed(culling_rect).unwrap_or(culling_rect).to_rect();

            frame.push_reference_frame(
                binding_key.into(),
                binding_key.bind(content_offset.into(), true),
                true,
                false,
                |frame| {
                    frame.with_auto_hide_rect(culling_rect, |frame| {
                        child.render(frame);
                    });
                },
            );
        }
        UiNodeOp::RenderUpdate { update } => {
            scroll_info.set_viewport_transform(*update.transform());

            update.with_transform(binding_key.update(content_offset.into(), true), false, |update| {
                child.render_update(update);
            });
        }
        _ => {}
    })
}

/// Create a node that generates and presents the [vertical scrollbar].
///
/// [vertical scrollbar]: VERTICAL_SCROLLBAR_GEN_VAR
pub fn v_scrollbar_presenter() -> impl UiNode {
    scrollbar_presenter(VERTICAL_SCROLLBAR_GEN_VAR, Orientation::Vertical)
}

/// Create a node that generates and presents the [horizontal scrollbar].
///
/// [horizontal scrollbar]: HORIZONTAL_SCROLLBAR_GEN_VAR
pub fn h_scrollbar_presenter() -> impl UiNode {
    scrollbar_presenter(HORIZONTAL_SCROLLBAR_GEN_VAR, Orientation::Horizontal)
}

fn scrollbar_presenter(var: impl IntoVar<WidgetFn<ScrollBarArgs>>, orientation: Orientation) -> impl UiNode {
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
    let mut up = CommandHandle::dummy();
    let mut down = CommandHandle::dummy();
    let mut left = CommandHandle::dummy();
    let mut right = CommandHandle::dummy();

    let mut layout_line = PxVector::zero();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&VERTICAL_LINE_UNIT_VAR).sub_var(&HORIZONTAL_LINE_UNIT_VAR);

            let scope = WIDGET.id();

            up = SCROLL_UP_CMD.scoped(scope).subscribe(SCROLL.can_scroll_up());
            down = SCROLL_DOWN_CMD.scoped(scope).subscribe(SCROLL.can_scroll_down());
            left = SCROLL_LEFT_CMD.scoped(scope).subscribe(SCROLL.can_scroll_left());
            right = SCROLL_RIGHT_CMD.scoped(scope).subscribe(SCROLL.can_scroll_right());
        }
        UiNodeOp::Deinit => {
            child.deinit();

            up = CommandHandle::dummy();
            down = CommandHandle::dummy();
            left = CommandHandle::dummy();
            right = CommandHandle::dummy();
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);

            up.set_enabled(SCROLL.can_scroll_up());
            down.set_enabled(SCROLL.can_scroll_down());
            left.set_enabled(SCROLL.can_scroll_left());
            right.set_enabled(SCROLL.can_scroll_right());

            if VERTICAL_LINE_UNIT_VAR.is_new() || HORIZONTAL_LINE_UNIT_VAR.is_new() {
                WIDGET.layout();
            }
        }
        UiNodeOp::Event { update } => {
            child.event(update);

            let scope = WIDGET.id();

            if let Some(args) = SCROLL_UP_CMD.scoped(scope).on(update) {
                args.handle_enabled(&up, |_| {
                    let mut offset = -layout_line.y;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_vertical_clamp(offset, args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = SCROLL_DOWN_CMD.scoped(scope).on(update) {
                args.handle_enabled(&down, |_| {
                    let mut offset = layout_line.y;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_vertical_clamp(offset, args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = SCROLL_LEFT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&left, |_| {
                    let mut offset = -layout_line.x;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_horizontal_clamp(offset, args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = SCROLL_RIGHT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&right, |_| {
                    let mut offset = layout_line.x;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_horizontal_clamp(offset, args.clamp.0, args.clamp.1);
                });
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = child.layout(wl);

            let viewport = SCROLL_VIEWPORT_SIZE_VAR.get();
            LAYOUT.with_constraints(PxConstraints2d::new_fill_size(viewport), || {
                layout_line = PxVector::new(
                    HORIZONTAL_LINE_UNIT_VAR.layout_dft_x(Px(20)),
                    VERTICAL_LINE_UNIT_VAR.layout_dft_y(Px(20)),
                );
            });
        }
        _ => {}
    })
}

/// Create a node that implements [`PAGE_UP_CMD`], [`PAGE_DOWN_CMD`],
/// [`PAGE_LEFT_CMD`] and [`PAGE_RIGHT_CMD`] scoped on the widget.
pub fn page_commands_node(child: impl UiNode) -> impl UiNode {
    let mut up = CommandHandle::dummy();
    let mut down = CommandHandle::dummy();
    let mut left = CommandHandle::dummy();
    let mut right = CommandHandle::dummy();

    let mut layout_page = PxVector::zero();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&VERTICAL_PAGE_UNIT_VAR).sub_var(&HORIZONTAL_PAGE_UNIT_VAR);

            let scope = WIDGET.id();

            up = PAGE_UP_CMD.scoped(scope).subscribe(SCROLL.can_scroll_up());
            down = PAGE_DOWN_CMD.scoped(scope).subscribe(SCROLL.can_scroll_down());
            left = PAGE_LEFT_CMD.scoped(scope).subscribe(SCROLL.can_scroll_left());
            right = PAGE_RIGHT_CMD.scoped(scope).subscribe(SCROLL.can_scroll_right());
        }
        UiNodeOp::Deinit => {
            child.deinit();

            up = CommandHandle::dummy();
            down = CommandHandle::dummy();
            left = CommandHandle::dummy();
            right = CommandHandle::dummy();
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);

            up.set_enabled(SCROLL.can_scroll_up());
            down.set_enabled(SCROLL.can_scroll_down());
            left.set_enabled(SCROLL.can_scroll_left());
            right.set_enabled(SCROLL.can_scroll_right());

            if VERTICAL_PAGE_UNIT_VAR.is_new() || HORIZONTAL_PAGE_UNIT_VAR.is_new() {
                WIDGET.layout();
            }
        }
        UiNodeOp::Event { update } => {
            child.event(update);

            let scope = WIDGET.id();

            if let Some(args) = PAGE_UP_CMD.scoped(scope).on(update) {
                args.handle_enabled(&up, |_| {
                    let mut offset = -layout_page.y;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_vertical_clamp(offset, args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = PAGE_DOWN_CMD.scoped(scope).on(update) {
                args.handle_enabled(&down, |_| {
                    let mut offset = layout_page.y;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_vertical_clamp(offset, args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = PAGE_LEFT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&left, |_| {
                    let mut offset = -layout_page.x;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_horizontal_clamp(offset, args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = PAGE_RIGHT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&right, |_| {
                    let mut offset = layout_page.x;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_horizontal_clamp(offset, args.clamp.0, args.clamp.1);
                });
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = child.layout(wl);

            let viewport = SCROLL_VIEWPORT_SIZE_VAR.get();
            LAYOUT.with_constraints(PxConstraints2d::new_fill_size(viewport), || {
                layout_page = PxVector::new(
                    HORIZONTAL_PAGE_UNIT_VAR.layout_dft_x(Px(20)),
                    VERTICAL_PAGE_UNIT_VAR.layout_dft_y(Px(20)),
                );
            });
        }
        _ => {}
    })
}

/// Create a node that implements [`SCROLL_TO_TOP_CMD`], [`SCROLL_TO_BOTTOM_CMD`],
/// [`SCROLL_TO_LEFTMOST_CMD`] and [`SCROLL_TO_RIGHTMOST_CMD`] scoped on the widget.
pub fn scroll_to_edge_commands_node(child: impl UiNode) -> impl UiNode {
    let mut top = CommandHandle::dummy();
    let mut bottom = CommandHandle::dummy();
    let mut leftmost = CommandHandle::dummy();
    let mut rightmost = CommandHandle::dummy();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            let scope = WIDGET.id();

            top = SCROLL_TO_TOP_CMD.scoped(scope).subscribe(SCROLL.can_scroll_up());
            bottom = SCROLL_TO_BOTTOM_CMD.scoped(scope).subscribe(SCROLL.can_scroll_down());
            leftmost = SCROLL_TO_LEFTMOST_CMD.scoped(scope).subscribe(SCROLL.can_scroll_left());
            rightmost = SCROLL_TO_RIGHTMOST_CMD.scoped(scope).subscribe(SCROLL.can_scroll_right());
        }
        UiNodeOp::Deinit => {
            child.deinit();

            top = CommandHandle::dummy();
            bottom = CommandHandle::dummy();
            leftmost = CommandHandle::dummy();
            rightmost = CommandHandle::dummy();
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);

            top.set_enabled(SCROLL.can_scroll_up());
            bottom.set_enabled(SCROLL.can_scroll_down());
            leftmost.set_enabled(SCROLL.can_scroll_left());
            rightmost.set_enabled(SCROLL.can_scroll_right());
        }
        UiNodeOp::Event { update } => {
            child.event(update);

            let scope = WIDGET.id();

            if let Some(args) = SCROLL_TO_TOP_CMD.scoped(scope).on(update) {
                args.handle_enabled(&top, |_| {
                    SCROLL.chase_vertical(|_| 0.fct());
                });
            } else if let Some(args) = SCROLL_TO_BOTTOM_CMD.scoped(scope).on(update) {
                args.handle_enabled(&bottom, |_| {
                    SCROLL.chase_vertical(|_| 1.fct());
                });
            } else if let Some(args) = SCROLL_TO_LEFTMOST_CMD.scoped(scope).on(update) {
                args.handle_enabled(&leftmost, |_| {
                    SCROLL.chase_horizontal(|_| 0.fct());
                });
            } else if let Some(args) = SCROLL_TO_RIGHTMOST_CMD.scoped(scope).on(update) {
                args.handle_enabled(&rightmost, |_| {
                    SCROLL.chase_horizontal(|_| 1.fct());
                });
            }
        }
        _ => {}
    })
}

/// Create a node that implements [`SCROLL_TO_CMD`] scoped on the widget and scroll to focused.
pub fn scroll_to_node(child: impl UiNode) -> impl UiNode {
    let mut _handle = CommandHandle::dummy();
    let mut scroll_to = None;

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            _handle = SCROLL_TO_CMD.scoped(WIDGET.id()).subscribe(true);
        }
        UiNodeOp::Deinit => {
            _handle = CommandHandle::dummy();
        }
        UiNodeOp::Event { update } => {
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

                                    scroll_to = Some((bounds, mode));
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
                                scroll_to = Some((bounds, mode));
                                WIDGET.layout();

                                args.propagation().stop();
                            }
                        }
                    }
                }
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = child.layout(wl);

            if let Some((bounds, mode)) = scroll_to.take() {
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
        }
        _ => {}
    })
}

/// Create a node that implements scroll-wheel handling for the widget.
pub fn scroll_wheel_node(child: impl UiNode) -> impl UiNode {
    let mut offset = Vector::zero();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_event(&MOUSE_WHEEL_EVENT);
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = MOUSE_WHEEL_EVENT.on(update) {
                if let Some(delta) = args.scroll_delta(ALT_FACTOR_VAR.get()) {
                    args.handle(|_| {
                        match delta {
                            MouseScrollDelta::LineDelta(x, y) => {
                                offset.x -= HORIZONTAL_WHEEL_UNIT_VAR.get() * x.fct();
                                offset.y -= VERTICAL_WHEEL_UNIT_VAR.get() * y.fct();
                            }
                            MouseScrollDelta::PixelDelta(x, y) => {
                                offset.x -= x.px();
                                offset.y -= y.px();
                            }
                        }

                        WIDGET.layout();
                    });
                }
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = child.layout(wl);

            let viewport = SCROLL_VIEWPORT_SIZE_VAR.get();

            LAYOUT.with_constraints(PxConstraints2d::new_fill_size(viewport), || {
                let o = offset.layout_dft(viewport.to_vector());
                offset = Vector::zero();

                if o.y != Px(0) {
                    SCROLL.scroll_vertical(o.y);
                }
                if o.x != Px(0) {
                    SCROLL.scroll_horizontal(o.x);
                }
            });
        }
        _ => {}
    })
}
