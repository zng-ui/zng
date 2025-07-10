//! UI nodes used for building the scroll widget.
//!

use std::sync::Arc;

use parking_lot::Mutex;
use zng_app::{
    access::ACCESS_SCROLL_EVENT,
    view_process::raw_events::{RAW_MOUSE_INPUT_EVENT, RAW_MOUSE_MOVED_EVENT},
};
use zng_color::Rgba;
use zng_ext_input::{
    focus::{FOCUS, FOCUS_CHANGED_EVENT},
    keyboard::{KEY_INPUT_EVENT, Key, KeyState},
    mouse::{ButtonState, MOUSE_INPUT_EVENT, MOUSE_WHEEL_EVENT, MouseButton, MouseScrollDelta},
    touch::{TOUCH_TRANSFORM_EVENT, TouchPhase},
};
use zng_wgt::prelude::{
    gradient::{ExtendMode, RenderGradientStop},
    *,
};
use zng_wgt_container::Container;
use zng_wgt_layer::{AnchorMode, LAYERS, LayerIndex};

use super::cmd::*;
use super::scroll_properties::*;
use super::scrollbar::Orientation;
use super::types::*;

/// The actual content presenter.
pub fn viewport(child: impl UiNode, mode: impl IntoVar<ScrollMode>, child_align: impl IntoVar<Align>) -> impl UiNode {
    let mode = mode.into_var();
    let child_align = child_align.into_var();
    let binding_key = FrameValueKey::new_unique();

    let mut viewport_size = PxSize::zero();
    let mut content_offset = PxVector::zero();
    let mut content_scale = 1.fct();
    let mut auto_hide_extra = PxSideOffsets::zero();
    let mut last_render_offset = PxVector::zero();
    let mut scroll_info = None;
    let mut scroll_info = move || {
        scroll_info
            .get_or_insert_with(|| WIDGET.info().meta().get_clone(*SCROLL_INFO_ID).unwrap())
            .clone()
    };

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_layout(&mode)
                .sub_var_layout(&SCROLL_VERTICAL_OFFSET_VAR)
                .sub_var_layout(&SCROLL_HORIZONTAL_OFFSET_VAR)
                .sub_var_layout(&SCROLL_SCALE_VAR)
                .sub_var_layout(&child_align);
        }

        UiNodeOp::Measure { wm, desired_size } => {
            let constraints = LAYOUT.constraints();
            if constraints.is_fill_max().all() {
                *desired_size = constraints.fill_size();
                child.delegated();
                return;
            }

            let mode = mode.get();
            let child_align = child_align.get();

            let vp_unit = constraints.fill_size();
            let has_fill_size = !vp_unit.is_empty() && constraints.max_size() == Some(vp_unit);
            let define_vp_unit = has_fill_size && DEFINE_VIEWPORT_UNIT_VAR.get();

            let mut content_size = LAYOUT.with_constraints(
                {
                    let mut c = constraints;
                    if mode.contains(ScrollMode::VERTICAL) {
                        c = c.with_unbounded_y().with_new_min_y(vp_unit.height);
                    } else {
                        c = c.with_new_min_y(Px(0));
                        if has_fill_size {
                            c = c.with_new_max_y(vp_unit.height);
                        }
                    }
                    if mode.contains(ScrollMode::HORIZONTAL) {
                        c = c.with_unbounded_x().with_new_min_x(vp_unit.width);
                    } else {
                        c = c.with_new_min_x(Px(0));
                        if has_fill_size {
                            c = c.with_new_max_x(vp_unit.width);
                        }
                    }

                    child_align.child_constraints(c)
                },
                || {
                    if define_vp_unit {
                        LAYOUT.with_viewport(vp_unit, || child.measure(wm))
                    } else {
                        child.measure(wm)
                    }
                },
            );

            if mode.contains(ScrollMode::ZOOM) {
                let scale = SCROLL_SCALE_VAR.get();
                content_size.width *= scale;
                content_size.height *= scale;
            }

            *desired_size = constraints.fill_size_or(content_size);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let mode = mode.get();
            let child_align = child_align.get();

            let constraints = LAYOUT.constraints();
            let vp_unit = constraints.fill_size();

            let has_fill_size = !vp_unit.is_empty() && constraints.max_size() == Some(vp_unit);
            let define_vp_unit = has_fill_size && DEFINE_VIEWPORT_UNIT_VAR.get();

            let joiner_size = scroll_info().joiner_size();

            let mut content_size = LAYOUT.with_constraints(
                {
                    let mut c = constraints;
                    if mode.contains(ScrollMode::VERTICAL) {
                        // Align::FILL forces the min-size, because we have infinite space in scrollable dimensions.
                        c = c.with_unbounded_y().with_new_min_y(vp_unit.height);
                    } else {
                        // If not scrollable Align::FILL works like normal `Container!` widgets.
                        c = c.with_new_min_y(Px(0));
                        if has_fill_size {
                            c = c.with_new_max_y(vp_unit.height);
                        }
                    }
                    if mode.contains(ScrollMode::HORIZONTAL) {
                        c = c.with_unbounded_x().with_new_min_x(vp_unit.width);
                    } else {
                        c = c.with_new_min_x(Px(0));
                        if has_fill_size {
                            c = c.with_new_max_x(vp_unit.width);
                        }
                    }

                    child_align.child_constraints(c)
                },
                || {
                    if define_vp_unit {
                        LAYOUT.with_viewport(vp_unit, || child.layout(wl))
                    } else {
                        child.layout(wl)
                    }
                },
            );
            if mode.contains(ScrollMode::ZOOM) {
                content_scale = SCROLL_SCALE_VAR.get();
                content_size.width *= content_scale;
                content_size.height *= content_scale;
            } else {
                content_scale = 1.fct();
            }

            let vp_size = constraints.fill_size_or(content_size);
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

            scroll_info().set_viewport_size(vp_size);

            let align_offset = child_align.child_offset(content_size, viewport_size, LAYOUT.direction());

            let mut ct_offset = PxVector::zero();

            if mode.contains(ScrollMode::VERTICAL) && content_size.height > vp_size.height {
                let v_offset = SCROLL_VERTICAL_OFFSET_VAR.get();
                ct_offset.y = (viewport_size.height - content_size.height) * v_offset;
            } else {
                ct_offset.y = align_offset.y;
            }
            if mode.contains(ScrollMode::HORIZONTAL) && content_size.width > vp_size.width {
                let h_offset = SCROLL_HORIZONTAL_OFFSET_VAR.get();
                ct_offset.x = (viewport_size.width - content_size.width) * h_offset;
            } else {
                ct_offset.x = align_offset.x;
            }

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

            SCROLL_VERTICAL_RATIO_VAR.set(v_ratio.fct()).unwrap();
            SCROLL_HORIZONTAL_RATIO_VAR.set(h_ratio.fct()).unwrap();
            SCROLL_CONTENT_SIZE_VAR.set(content_size).unwrap();

            let full_size = viewport_size + joiner_size;

            SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR
                .set(mode.contains(ScrollMode::VERTICAL) && content_size.height > full_size.height)
                .unwrap();
            SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR
                .set(mode.contains(ScrollMode::HORIZONTAL) && content_size.width > full_size.width)
                .unwrap();

            *final_size = viewport_size;

            scroll_info().set_content(PxRect::new(content_offset.to_point(), content_size), content_scale);
        }
        UiNodeOp::Render { frame } => {
            scroll_info().set_viewport_transform(*frame.transform());
            last_render_offset = content_offset;

            let mut culling_rect = PxBox::from_size(viewport_size);
            culling_rect.min.y -= auto_hide_extra.top;
            culling_rect.max.x += auto_hide_extra.right;
            culling_rect.max.y += auto_hide_extra.bottom;
            culling_rect.min.x -= auto_hide_extra.left;
            let culling_rect = frame.transform().outer_transformed(culling_rect).unwrap_or(culling_rect).to_rect();

            let transform = if content_scale != 1.fct() {
                PxTransform::scale(content_scale.0, content_scale.0).then_translate(content_offset.cast())
            } else {
                content_offset.into()
            };
            frame.push_reference_frame(binding_key.into(), binding_key.bind(transform, true), true, false, |frame| {
                frame.with_auto_hide_rect(culling_rect, |frame| {
                    child.render(frame);
                });
            });
        }
        UiNodeOp::RenderUpdate { update } => {
            scroll_info().set_viewport_transform(*update.transform());

            let transform = if content_scale != 1.fct() {
                PxTransform::scale(content_scale.0, content_scale.0).then_translate(content_offset.cast())
            } else {
                content_offset.into()
            };
            update.with_transform(binding_key.update(transform, true), false, |update| {
                child.render_update(update);
            });
        }
        _ => {}
    })
}

/// Create a node that generates and presents the [vertical scrollbar].
///
/// [vertical scrollbar]: VERTICAL_SCROLLBAR_FN_VAR
pub fn v_scrollbar_presenter() -> impl UiNode {
    scrollbar_presenter(VERTICAL_SCROLLBAR_FN_VAR, Orientation::Vertical)
}

/// Create a node that generates and presents the [horizontal scrollbar].
///
/// [horizontal scrollbar]: HORIZONTAL_SCROLLBAR_FN_VAR
pub fn h_scrollbar_presenter() -> impl UiNode {
    scrollbar_presenter(HORIZONTAL_SCROLLBAR_FN_VAR, Orientation::Horizontal)
}

fn scrollbar_presenter(var: impl IntoVar<WidgetFn<ScrollBarArgs>>, orientation: Orientation) -> impl UiNode {
    presenter(ScrollBarArgs::new(orientation), var)
}

/// Create a node that generates and presents the [scrollbar joiner].
///
/// [scrollbar joiner]: SCROLLBAR_JOINER_FN_VAR
pub fn scrollbar_joiner_presenter() -> impl UiNode {
    presenter((), SCROLLBAR_JOINER_FN_VAR)
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
            WIDGET
                .sub_var_layout(&VERTICAL_LINE_UNIT_VAR)
                .sub_var_layout(&HORIZONTAL_LINE_UNIT_VAR);

            let scope = WIDGET.id();

            up = SCROLL_UP_CMD.scoped(scope).subscribe(SCROLL.can_scroll_up().get());
            down = SCROLL_DOWN_CMD.scoped(scope).subscribe(SCROLL.can_scroll_down().get());
            left = SCROLL_LEFT_CMD.scoped(scope).subscribe(SCROLL.can_scroll_left().get());
            right = SCROLL_RIGHT_CMD.scoped(scope).subscribe(SCROLL.can_scroll_right().get());
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
                    SCROLL.scroll_vertical_clamp(ScrollFrom::VarTarget(offset), args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = SCROLL_DOWN_CMD.scoped(scope).on(update) {
                args.handle_enabled(&down, |_| {
                    let mut offset = layout_line.y;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_vertical_clamp(ScrollFrom::VarTarget(offset), args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = SCROLL_LEFT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&left, |_| {
                    let mut offset = -layout_line.x;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_horizontal_clamp(ScrollFrom::VarTarget(offset), args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = SCROLL_RIGHT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&right, |_| {
                    let mut offset = layout_line.x;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_horizontal_clamp(ScrollFrom::VarTarget(offset), args.clamp.0, args.clamp.1);
                });
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = child.layout(wl);

            up.set_enabled(SCROLL.can_scroll_up().get());
            down.set_enabled(SCROLL.can_scroll_down().get());
            left.set_enabled(SCROLL.can_scroll_left().get());
            right.set_enabled(SCROLL.can_scroll_right().get());

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
            WIDGET
                .sub_var_layout(&VERTICAL_PAGE_UNIT_VAR)
                .sub_var_layout(&HORIZONTAL_PAGE_UNIT_VAR);

            let scope = WIDGET.id();

            up = PAGE_UP_CMD.scoped(scope).subscribe(SCROLL.can_scroll_up().get());
            down = PAGE_DOWN_CMD.scoped(scope).subscribe(SCROLL.can_scroll_down().get());
            left = PAGE_LEFT_CMD.scoped(scope).subscribe(SCROLL.can_scroll_left().get());
            right = PAGE_RIGHT_CMD.scoped(scope).subscribe(SCROLL.can_scroll_right().get());
        }
        UiNodeOp::Deinit => {
            child.deinit();

            up = CommandHandle::dummy();
            down = CommandHandle::dummy();
            left = CommandHandle::dummy();
            right = CommandHandle::dummy();
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
                    SCROLL.scroll_vertical_clamp(ScrollFrom::VarTarget(offset), args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = PAGE_DOWN_CMD.scoped(scope).on(update) {
                args.handle_enabled(&down, |_| {
                    let mut offset = layout_page.y;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_vertical_clamp(ScrollFrom::VarTarget(offset), args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = PAGE_LEFT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&left, |_| {
                    let mut offset = -layout_page.x;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_horizontal_clamp(ScrollFrom::VarTarget(offset), args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = PAGE_RIGHT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&right, |_| {
                    let mut offset = layout_page.x;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_horizontal_clamp(ScrollFrom::VarTarget(offset), args.clamp.0, args.clamp.1);
                });
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = child.layout(wl);

            up.set_enabled(SCROLL.can_scroll_up().get());
            down.set_enabled(SCROLL.can_scroll_down().get());
            left.set_enabled(SCROLL.can_scroll_left().get());
            right.set_enabled(SCROLL.can_scroll_right().get());

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

            top = SCROLL_TO_TOP_CMD.scoped(scope).subscribe(SCROLL.can_scroll_up().get());
            bottom = SCROLL_TO_BOTTOM_CMD.scoped(scope).subscribe(SCROLL.can_scroll_down().get());
            leftmost = SCROLL_TO_LEFTMOST_CMD.scoped(scope).subscribe(SCROLL.can_scroll_left().get());
            rightmost = SCROLL_TO_RIGHTMOST_CMD.scoped(scope).subscribe(SCROLL.can_scroll_right().get());
        }
        UiNodeOp::Deinit => {
            child.deinit();

            top = CommandHandle::dummy();
            bottom = CommandHandle::dummy();
            leftmost = CommandHandle::dummy();
            rightmost = CommandHandle::dummy();
        }
        UiNodeOp::Layout { .. } => {
            top.set_enabled(SCROLL.can_scroll_up().get());
            bottom.set_enabled(SCROLL.can_scroll_down().get());
            leftmost.set_enabled(SCROLL.can_scroll_left().get());
            rightmost.set_enabled(SCROLL.can_scroll_right().get());
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

/// Create a node that implements [`ZOOM_IN_CMD`], [`ZOOM_OUT_CMD`], [`ZOOM_TO_FIT_CMD`],
/// and [`ZOOM_RESET_CMD`] scoped on the widget.
pub fn zoom_commands_node(child: impl UiNode) -> impl UiNode {
    let mut zoom_in = CommandHandle::dummy();
    let mut zoom_out = CommandHandle::dummy();
    let mut zoom_to_fit = CommandHandle::dummy();
    let mut zoom_reset = CommandHandle::dummy();

    let mut scale_delta = 0.fct();
    let mut origin = Point::default();

    fn fit_scale() -> Factor {
        let scroll = WIDGET.info().scroll_info().unwrap();
        let viewport = (scroll.viewport_size() + scroll.joiner_size()).to_f32(); // viewport without scrollbars
        let content = scroll.content().size.to_f32() / scroll.zoom_scale();
        (viewport.width / content.width).min(viewport.height / content.height).fct()
    }

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            let scope = WIDGET.id();

            zoom_in = ZOOM_IN_CMD.scoped(scope).subscribe(SCROLL.can_zoom_in());
            zoom_out = ZOOM_OUT_CMD.scoped(scope).subscribe(SCROLL.can_zoom_out());
            zoom_to_fit = ZOOM_TO_FIT_CMD.scoped(scope).subscribe(true);
            zoom_reset = ZOOM_RESET_CMD.scoped(scope).subscribe(true);
        }
        UiNodeOp::Deinit => {
            child.deinit();

            zoom_in = CommandHandle::dummy();
            zoom_out = CommandHandle::dummy();
            zoom_to_fit = CommandHandle::dummy();
            zoom_reset = CommandHandle::dummy();
        }
        UiNodeOp::Event { update } => {
            child.event(update);

            let scope = WIDGET.id();

            if let Some(args) = ZOOM_IN_CMD.scoped(scope).on(update) {
                args.handle_enabled(&zoom_in, |args| {
                    origin = args.param::<Point>().cloned().unwrap_or_default();
                    scale_delta += ZOOM_WHEEL_UNIT_VAR.get();

                    WIDGET.layout();
                });
            } else if let Some(args) = ZOOM_OUT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&zoom_out, |_| {
                    origin = args.param::<Point>().cloned().unwrap_or_default();
                    scale_delta -= ZOOM_WHEEL_UNIT_VAR.get();

                    WIDGET.layout();
                });
            } else if let Some(args) = ZOOM_TO_FIT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&zoom_to_fit, |_| {
                    let scale = fit_scale();
                    SCROLL.chase_zoom(|_| scale);
                });
            } else if let Some(args) = ZOOM_RESET_CMD.scoped(scope).on(update) {
                args.handle_enabled(&zoom_reset, |_| {
                    SCROLL.chase_zoom(|_| 1.fct());
                    scale_delta = 0.fct();
                });
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = child.layout(wl);

            zoom_in.set_enabled(SCROLL.can_zoom_in());
            zoom_out.set_enabled(SCROLL.can_zoom_out());
            let scale = SCROLL.zoom_scale().get();
            zoom_to_fit.set_enabled(scale != fit_scale());
            zoom_reset.set_enabled(scale != 1.fct());

            if scale_delta != 0.fct() {
                let scroll_info = WIDGET.info().scroll_info().unwrap();
                let viewport_size = scroll_info.viewport_size();

                let default = PxPoint::new(
                    Px(0),
                    match LAYOUT.direction() {
                        LayoutDirection::LTR => Px(0),
                        LayoutDirection::RTL => viewport_size.width,
                    },
                );
                let center_in_viewport =
                    LAYOUT.with_constraints(PxConstraints2d::new_fill_size(viewport_size), || origin.layout_dft(default));

                SCROLL.zoom(|f| f + scale_delta, center_in_viewport);
                scale_delta = 0.fct();
            }
        }
        _ => {}
    })
}

/// Create a node that implements [`SCROLL_TO_CMD`] scoped on the widget and scroll to focused.
pub fn scroll_to_node(child: impl UiNode) -> impl UiNode {
    let mut _handle = CommandHandle::dummy();
    let mut scroll_to = None;
    let mut scroll_to_from_cmd = false;

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            _handle = SCROLL_TO_CMD.scoped(WIDGET.id()).subscribe(true);
            WIDGET.sub_event(&FOCUS_CHANGED_EVENT);
        }
        UiNodeOp::Deinit => {
            _handle = CommandHandle::dummy();
        }
        UiNodeOp::Event { update } => {
            let self_id = WIDGET.id();
            if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                if let Some(path) = &args.new_focus {
                    if (scroll_to.is_none() || !scroll_to_from_cmd)
                        && path.contains(self_id)
                        && path.widget_id() != self_id
                        && !args.is_enabled_change()
                        && !args.is_highlight_changed()
                    {
                        // focus move inside.
                        if let Some(mode) = SCROLL_TO_FOCUSED_MODE_VAR.get() {
                            // scroll_to_focused enabled

                            let can_scroll_v = SCROLL.can_scroll_vertical().get();
                            let can_scroll_h = SCROLL.can_scroll_horizontal().get();
                            if can_scroll_v || can_scroll_h {
                                // auto scroll if can scroll AND focus did not change by a click on the
                                // Scroll! scope restoring focus back to a child AND the target is not already visible.

                                let tree = WINDOW.info();
                                if let Some(mut target) = tree.get(path.widget_id()) {
                                    let mut is_focus_restore = false;

                                    if args.prev_focus.as_ref().map(|p| p.widget_id()) == Some(self_id) {
                                        // focus moved to child from Scroll! scope (self)

                                        // Check if not caused by a click on a non-focusable child:
                                        // - On a click in a non-focusable child the focus goes back to the Scroll!.
                                        // - The Scroll! is a focus scope, it restores the focus to the previous focused child.
                                        // - The clicked non-focusable becomes the `navigation_origin`.
                                        // - We don't want to scroll back to the focusable child in this case.
                                        if let Some(id) = FOCUS.navigation_origin().get() {
                                            if let Some(origin) = tree.get(id) {
                                                for a in origin.ancestors() {
                                                    if a.id() == self_id {
                                                        is_focus_restore = true;
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    if !is_focus_restore {
                                        for a in target.ancestors() {
                                            if a.is_scroll() {
                                                if a.id() == self_id {
                                                    break;
                                                } else {
                                                    // actually focus move inside an inner scroll,
                                                    // the inner-most scroll scrolls to the target,
                                                    // the outer scrolls scroll to the child scroll.
                                                    target = a;
                                                }
                                            }
                                        }

                                        // check if widget is not large and already visible.
                                        let mut scroll = true;
                                        let scroll_bounds = tree.get(self_id).unwrap().inner_bounds();
                                        let target_bounds = target.inner_bounds();
                                        if let Some(r) = scroll_bounds.intersection(&target_bounds) {
                                            let is_large_visible_v =
                                                can_scroll_v && r.height() > Px(20) && target_bounds.height() > scroll_bounds.height();
                                            let is_large_visible_h =
                                                can_scroll_h && r.width() > Px(20) && target_bounds.width() > scroll_bounds.width();

                                            scroll = !is_large_visible_v && !is_large_visible_h;
                                        }
                                        if scroll {
                                            scroll_to = Some((Rect::from(target_bounds), mode, None, false));
                                            WIDGET.layout();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if let Some(args) = SCROLL_TO_CMD.scoped(self_id).on(update) {
                // event send to us and enabled
                if let Some(request) = ScrollToRequest::from_args(args) {
                    // has unhandled request
                    let tree = WINDOW.info();
                    match request.target {
                        ScrollToTarget::Descendant(target) => {
                            if let Some(target) = tree.get(target) {
                                // target exists
                                if let Some(us) = target.ancestors().find(|w| w.id() == self_id) {
                                    // target is descendant
                                    if us.is_scroll() {
                                        scroll_to = Some((Rect::from(target.inner_bounds()), request.mode, request.zoom, false));
                                        scroll_to_from_cmd = true;
                                        WIDGET.layout();

                                        args.propagation().stop();
                                    }
                                }
                            }
                        }
                        ScrollToTarget::Rect(rect) => {
                            scroll_to = Some((rect, request.mode, request.zoom, true));
                            scroll_to_from_cmd = true;
                            WIDGET.layout();

                            args.propagation().stop();
                        }
                    }
                }
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = child.layout(wl);

            if let Some((bounds, mode, mut zoom, in_content)) = scroll_to.take() {
                scroll_to_from_cmd = false;
                let tree = WINDOW.info();
                let us = tree.get(WIDGET.id()).unwrap();

                if let Some(scroll_info) = us.scroll_info() {
                    #[allow(deprecated)] // TODO(breaking) - remove this allow
                    if let Some(s) = &mut zoom {
                        *s = (*s).clamp(MIN_ZOOM_VAR.get(), MAX_ZOOM_VAR.get());
                    }

                    let rendered_content = scroll_info.content();

                    let mut bounds = {
                        let content = rendered_content;
                        let mut rect = LAYOUT.with_constraints(PxConstraints2d::new_exact_size(content.size), || bounds.layout());
                        if in_content {
                            rect.origin += content.origin.to_vector();
                        }
                        rect
                    };

                    // remove viewport transform
                    bounds = scroll_info
                        .viewport_transform()
                        .inverse()
                        .and_then(|t| t.outer_transformed(bounds.to_box2d()))
                        .map(|b| b.to_rect())
                        .unwrap_or(bounds);

                    let current_bounds = bounds;

                    // remove offset
                    let rendered_offset = rendered_content.origin.to_vector();
                    bounds.origin -= rendered_offset;

                    // replace scale
                    let rendered_scale = SCROLL.rendered_zoom_scale();
                    if let Some(s) = zoom {
                        let s = s / rendered_scale;
                        bounds.origin *= s;
                        bounds.size *= s;
                    }
                    // target bounds is now in the content space at future scale

                    let viewport_size = scroll_info.viewport_size();

                    let mut offset = PxVector::splat(Px::MAX);

                    match mode {
                        ScrollToMode::Minimal { margin } => {
                            // add minimal margin at new scale to target bounds
                            let scaled_margin = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(bounds.size), || margin.layout());
                            let bounds = inflate_margin(bounds, scaled_margin);

                            // add minimal margin, at current scale to the current bounds
                            let cur_margin = if zoom.is_some() {
                                LAYOUT.with_constraints(PxConstraints2d::new_fill_size(current_bounds.size), || margin.layout())
                            } else {
                                scaled_margin
                            };
                            let current_bounds = inflate_margin(current_bounds, cur_margin);

                            // vertical scroll
                            if bounds.size.height < viewport_size.height {
                                if current_bounds.origin.y < Px(0) {
                                    // scroll up
                                    offset.y = bounds.origin.y;
                                } else if current_bounds.max_y() > viewport_size.height {
                                    // scroll down
                                    offset.y = bounds.max_y() - viewport_size.height;
                                } else if zoom.is_some() {
                                    // scale around center
                                    let center_in_vp = current_bounds.center().y;
                                    let center = bounds.center().y;
                                    offset.y = center - center_in_vp;

                                    // offset minimal if needed
                                    let mut bounds_final = bounds;
                                    bounds_final.origin.y -= offset.y;
                                    if bounds_final.origin.y < Px(0) {
                                        offset.y = bounds.origin.y;
                                    } else if bounds_final.max_y() > viewport_size.height {
                                        offset.y = bounds.max_y() - viewport_size.height;
                                    }
                                }
                            } else {
                                // center
                                offset.y = viewport_size.height / Px(2) - bounds.center().y;
                            };

                            // horizontal scroll
                            if bounds.size.width < viewport_size.width {
                                if current_bounds.origin.x < Px(0) {
                                    // scroll left
                                    offset.x = bounds.origin.x;
                                } else if current_bounds.max_x() > viewport_size.width {
                                    // scroll right
                                    offset.x = bounds.max_x() - viewport_size.width;
                                } else if zoom.is_some() {
                                    // scale around center
                                    let center_in_vp = current_bounds.center().x;
                                    let center = bounds.center().x;
                                    offset.x = center - center_in_vp;

                                    // offset minimal if needed
                                    let mut bounds_final = bounds;
                                    bounds_final.origin.x -= offset.x;
                                    if bounds_final.origin.x < Px(0) {
                                        offset.x = bounds.origin.x;
                                    } else if bounds_final.max_x() > viewport_size.width {
                                        offset.x = bounds.max_x() - viewport_size.width;
                                    }
                                }
                            } else {
                                // center
                                offset.x = viewport_size.width / Px(2) - bounds.center().x;
                            };
                        }
                        ScrollToMode::Center {
                            widget_point,
                            scroll_point,
                        } => {
                            // find the two points
                            let default = (bounds.size / Px(2)).to_vector().to_point();
                            let widget_point =
                                LAYOUT.with_constraints(PxConstraints2d::new_fill_size(bounds.size), || widget_point.layout_dft(default));
                            let default = (viewport_size / Px(2)).to_vector().to_point();
                            let scroll_point =
                                LAYOUT.with_constraints(PxConstraints2d::new_fill_size(viewport_size), || scroll_point.layout_dft(default));

                            offset = (widget_point + bounds.origin.to_vector()) - scroll_point;
                        }
                    }

                    // scroll range
                    let mut content_size = SCROLL.content_size().get();
                    if let Some(scale) = zoom {
                        content_size *= scale / rendered_scale;
                    }
                    let max_scroll = content_size - viewport_size;

                    // apply
                    if let Some(scale) = zoom {
                        SCROLL.chase_zoom(|_| scale);
                    }
                    if offset.y != Px::MAX && max_scroll.height > Px(0) {
                        let offset_y = offset.y.0 as f32 / max_scroll.height.0 as f32;
                        SCROLL.chase_vertical(|_| offset_y.fct());
                    }
                    if offset.x != Px::MAX && max_scroll.width > Px(0) {
                        let offset_x = offset.x.0 as f32 / max_scroll.width.0 as f32;
                        SCROLL.chase_horizontal(|_| offset_x.fct());
                    }
                }
            }
        }
        _ => {}
    })
}
fn inflate_margin(mut r: PxRect, margin: PxSideOffsets) -> PxRect {
    r.origin.x -= margin.left;
    r.origin.y -= margin.top;
    r.size.width += margin.horizontal();
    r.size.height += margin.vertical();
    r
}

/// Create a node that implements scroll by touch gestures for the widget.
pub fn scroll_touch_node(child: impl UiNode) -> impl UiNode {
    let mut applied_offset = PxVector::zero();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_event(&TOUCH_TRANSFORM_EVENT);
        }
        UiNodeOp::Event { update } => {
            child.event(update);

            if let Some(args) = TOUCH_TRANSFORM_EVENT.on_unhandled(update) {
                let mut pending_translate = true;

                if SCROLL.mode().get().contains(ScrollMode::ZOOM) {
                    let f = args.scale();
                    if f != 1.fct() {
                        let center = WIDGET
                            .info()
                            .scroll_info()
                            .unwrap()
                            .viewport_transform()
                            .inverse()
                            .and_then(|t| t.transform_point_f32(args.latest_info.center))
                            .unwrap_or(args.latest_info.center);

                        SCROLL.zoom_touch(args.phase, f, center);
                        pending_translate = false;
                    }
                }

                if pending_translate {
                    let new_offset = args.translation().cast::<Px>();
                    let delta = new_offset - applied_offset;
                    applied_offset = new_offset;

                    if delta.y != Px(0) {
                        SCROLL.scroll_vertical_touch(-delta.y);
                    }
                    if delta.x != Px(0) {
                        SCROLL.scroll_horizontal_touch(-delta.x);
                    }
                }

                match args.phase {
                    TouchPhase::Start => {}
                    TouchPhase::Move => {}
                    TouchPhase::End => {
                        applied_offset = PxVector::zero();

                        let friction = Dip::new(1000);
                        let mode = SCROLL.mode().get();
                        if mode.contains(ScrollMode::VERTICAL) {
                            let (delta, duration) = args.translation_inertia_y(friction);

                            if delta != Px(0) {
                                SCROLL.scroll_vertical_touch_inertia(-delta, duration);
                            }
                            SCROLL.clear_vertical_overscroll();
                        }
                        if mode.contains(ScrollMode::HORIZONTAL) {
                            let (delta, duration) = args.translation_inertia_x(friction);
                            if delta != Px(0) {
                                SCROLL.scroll_horizontal_touch_inertia(-delta, duration);
                            }
                            SCROLL.clear_horizontal_overscroll();
                        }
                    }
                    TouchPhase::Cancel => {
                        applied_offset = PxVector::zero();

                        SCROLL.clear_vertical_overscroll();
                        SCROLL.clear_horizontal_overscroll();
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
    let mut scale_delta = 0.fct();
    let mut scale_position = DipPoint::zero();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_event(&MOUSE_WHEEL_EVENT);
        }
        UiNodeOp::Event { update } => {
            child.event(update);

            if let Some(args) = MOUSE_WHEEL_EVENT.on_unhandled(update) {
                if let Some(delta) = args.scroll_delta(ALT_FACTOR_VAR.get()) {
                    match delta {
                        MouseScrollDelta::LineDelta(x, y) => {
                            let scroll_x = if x > 0.0 {
                                SCROLL.can_scroll_left().get()
                            } else if x < 0.0 {
                                SCROLL.can_scroll_right().get()
                            } else {
                                false
                            };
                            let scroll_y = if y > 0.0 {
                                SCROLL.can_scroll_up().get()
                            } else if y < 0.0 {
                                SCROLL.can_scroll_down().get()
                            } else {
                                false
                            };

                            if scroll_x || scroll_y {
                                args.propagation().stop();

                                if scroll_x {
                                    offset.x -= HORIZONTAL_WHEEL_UNIT_VAR.get() * x.fct();
                                }
                                if scroll_y {
                                    offset.y -= VERTICAL_WHEEL_UNIT_VAR.get() * y.fct();
                                }
                            }
                        }
                        MouseScrollDelta::PixelDelta(x, y) => {
                            let scroll_x = if x > 0.0 {
                                SCROLL.can_scroll_left().get()
                            } else if x < 0.0 {
                                SCROLL.can_scroll_right().get()
                            } else {
                                false
                            };
                            let scroll_y = if y > 0.0 {
                                SCROLL.can_scroll_up().get()
                            } else if y < 0.0 {
                                SCROLL.can_scroll_down().get()
                            } else {
                                false
                            };

                            if scroll_x || scroll_y {
                                args.propagation().stop();

                                if scroll_x {
                                    offset.x -= x.px();
                                }
                                if scroll_y {
                                    offset.y -= y.px();
                                }
                            }
                        }
                        _ => {}
                    }

                    WIDGET.layout();
                } else if let Some(delta) = args.zoom_delta() {
                    if !SCROLL_MODE_VAR.get().contains(ScrollMode::ZOOM) {
                        return;
                    }

                    let delta = match delta {
                        MouseScrollDelta::LineDelta(x, y) => {
                            if y.abs() > x.abs() {
                                ZOOM_WHEEL_UNIT_VAR.get() * y.fct()
                            } else {
                                ZOOM_WHEEL_UNIT_VAR.get() * x.fct()
                            }
                        }
                        MouseScrollDelta::PixelDelta(x, y) => {
                            if y.abs() > x.abs() {
                                // 1% per "pixel".
                                0.001.fct() * y.fct()
                            } else {
                                0.001.fct() * x.fct()
                            }
                        }
                        _ => Factor(0.0),
                    };

                    let apply = if delta > 0.fct() {
                        SCROLL.can_zoom_in()
                    } else if delta < 0.fct() {
                        SCROLL.can_zoom_out()
                    } else {
                        false
                    };

                    if apply {
                        scale_delta += delta;
                        scale_position = args.position;
                        WIDGET.layout();
                    }
                }
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = child.layout(wl);

            if offset != Vector::zero() {
                let viewport = SCROLL_VIEWPORT_SIZE_VAR.get();

                LAYOUT.with_constraints(PxConstraints2d::new_fill_size(viewport), || {
                    let o = offset.layout_dft(viewport.to_vector());
                    offset = Vector::zero();

                    if o.y != Px(0) {
                        SCROLL.scroll_vertical(ScrollFrom::VarTarget(o.y));
                    }
                    if o.x != Px(0) {
                        SCROLL.scroll_horizontal(ScrollFrom::VarTarget(o.x));
                    }
                });
            }

            if scale_delta != 0.fct() {
                let scroll_info = WIDGET.info().scroll_info().unwrap();
                let default = scale_position.to_px(LAYOUT.scale_factor());
                let default = scroll_info
                    .viewport_transform()
                    .inverse()
                    .and_then(|t| t.transform_point(default))
                    .unwrap_or(default);

                let viewport_size = scroll_info.viewport_size();
                let center_in_viewport = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(viewport_size), || {
                    ZOOM_WHEEL_ORIGIN_VAR.layout_dft(default)
                });

                SCROLL.zoom(|f| f + scale_delta, center_in_viewport);
                scale_delta = 0.fct();
            }
        }
        _ => {}
    })
}

/// Overscroll visual indicator.
pub fn overscroll_node(child: impl UiNode) -> impl UiNode {
    let mut v_rect = PxRect::zero();
    let mut v_center = PxPoint::zero();
    let mut v_radius_w = Px(0);

    let mut h_rect = PxRect::zero();
    let mut h_center = PxPoint::zero();
    let mut h_radius_h = Px(0);

    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_layout(&OVERSCROLL_VERTICAL_OFFSET_VAR)
                .sub_var_layout(&OVERSCROLL_HORIZONTAL_OFFSET_VAR);
        }
        UiNodeOp::Layout { final_size, wl } => {
            *final_size = c.layout(wl);

            let mut new_v_rect = PxRect::zero();
            let v = OVERSCROLL_VERTICAL_OFFSET_VAR.get();
            if v < 0.fct() {
                // overscroll top
                new_v_rect.size = *final_size;
                new_v_rect.size.height *= v.abs() / 10.fct();
                v_center.y = Px(0);
            } else if v > 0.fct() {
                // overscroll bottom
                new_v_rect.size = *final_size;
                new_v_rect.size.height *= v.abs() / 10.fct();
                new_v_rect.origin.y = final_size.height - new_v_rect.size.height;
                v_center.y = new_v_rect.size.height;
            }

            let mut new_h_rect = PxRect::zero();
            let h = OVERSCROLL_HORIZONTAL_OFFSET_VAR.get();
            if h < 0.fct() {
                // overscroll left
                new_h_rect.size = *final_size;
                new_h_rect.size.width *= h.abs() / 10.fct();
                h_center.x = Px(0);
            } else if h > 0.fct() {
                // overscroll right
                new_h_rect.size = *final_size;
                new_h_rect.size.width *= h.abs() / 10.fct();
                new_h_rect.origin.x = final_size.width - new_h_rect.size.width;
                h_center.x = new_h_rect.size.width;
            }

            if new_v_rect != v_rect {
                v_rect = new_v_rect;
                // 50%
                v_center.x = v_rect.size.width / Px(2);
                // 110%
                let radius = v_center.x;
                v_radius_w = radius + radius * 0.1;

                WIDGET.render();
            }
            if new_h_rect != h_rect {
                h_rect = new_h_rect;
                h_center.y = h_rect.size.height / Px(2);
                let radius = h_center.y;
                h_radius_h = radius + radius * 0.1;
                WIDGET.render();
            }
        }
        UiNodeOp::Render { frame } => {
            c.render(frame);

            let stops = |color| {
                [
                    RenderGradientStop { offset: 0.0, color },
                    RenderGradientStop { offset: 0.99, color },
                    RenderGradientStop {
                        offset: 1.0,
                        color: {
                            let mut c = color;
                            c.alpha = 0.0;
                            c
                        },
                    },
                ]
            };

            frame.with_auto_hit_test(false, |frame| {
                if !v_rect.size.is_empty() {
                    let mut color: Rgba = OVERSCROLL_COLOR_VAR.get();
                    color.alpha *= (OVERSCROLL_VERTICAL_OFFSET_VAR.get().abs().0).min(1.0);
                    let stops = stops(color);

                    let mut radius = v_rect.size;
                    radius.width = v_radius_w;
                    frame.push_radial_gradient(
                        v_rect,
                        v_center,
                        radius,
                        &stops,
                        ExtendMode::Clamp.into(),
                        PxPoint::zero(),
                        v_rect.size,
                        PxSize::zero(),
                    );
                }
                if !h_rect.size.is_empty() {
                    let mut color: Rgba = OVERSCROLL_COLOR_VAR.get();
                    color.alpha *= (OVERSCROLL_HORIZONTAL_OFFSET_VAR.get().abs().0).min(1.0);
                    let stops = stops(color);

                    let mut radius = h_rect.size;
                    radius.height = h_radius_h;
                    frame.push_radial_gradient(
                        h_rect,
                        h_center,
                        radius,
                        &stops,
                        ExtendMode::Clamp.into(),
                        PxPoint::zero(),
                        h_rect.size,
                        PxSize::zero(),
                    );
                }
            });
        }
        _ => {}
    })
}

/// Create a node that converts [`ACCESS_SCROLL_EVENT`] to command requests.
///
/// [`ACCESS_SCROLL_EVENT`]: zng_app::access::ACCESS_SCROLL_EVENT
pub fn access_scroll_node(child: impl UiNode) -> impl UiNode {
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_event(&ACCESS_SCROLL_EVENT);
        }
        UiNodeOp::Event { update } => {
            c.event(update);

            if let Some(args) = ACCESS_SCROLL_EVENT.on_unhandled(update) {
                use zng_app::access::ScrollCmd::*;

                let id = WIDGET.id();
                if args.widget_id == id {
                    match args.command {
                        PageUp => PAGE_UP_CMD.scoped(id).notify(),
                        PageDown => PAGE_DOWN_CMD.scoped(id).notify(),
                        PageLeft => PAGE_LEFT_CMD.scoped(id).notify(),
                        PageRight => PAGE_RIGHT_CMD.scoped(id).notify(),
                        ScrollToRect(rect) => SCROLL_TO_CMD.scoped(id).notify_param(Rect::from(rect)),

                        ScrollTo => {
                            // parent scroll handles this
                            return;
                        }
                        _ => return,
                    }
                    args.propagation().stop();
                } else {
                    match args.command {
                        ScrollTo => super::cmd::scroll_to(args.widget_id, ScrollToMode::minimal(10)),
                        ScrollToRect(rect) => super::cmd::scroll_to(args.widget_id, ScrollToMode::minimal_rect(rect)),
                        _ => return,
                    }
                    args.propagation().stop();
                }
            }
        }
        _ => {}
    })
}

/// Create a note that spawns the auto scroller on middle click and fulfill `AUTO_SCROLL_CMD` requests.
pub fn auto_scroll_node(child: impl UiNode) -> impl UiNode {
    let mut middle_handle = EventHandle::dummy();
    let mut cmd_handle = CommandHandle::dummy();
    let mut auto_scrolling = None::<(WidgetId, Arc<Mutex<DInstant>>)>;
    match_node(child, move |c, op| {
        enum Task {
            CheckEnable,
            Disable,
        }
        let mut task = None;
        match op {
            UiNodeOp::Init => {
                cmd_handle = AUTO_SCROLL_CMD
                    .scoped(WIDGET.id())
                    .subscribe(SCROLL.can_scroll_horizontal().get() || SCROLL.can_scroll_vertical().get());
                WIDGET.sub_var(&AUTO_SCROLL_VAR);
                task = Some(Task::CheckEnable);
            }
            UiNodeOp::Deinit => {
                task = Some(Task::Disable);
            }
            UiNodeOp::Update { .. } => {
                if AUTO_SCROLL_VAR.is_new() {
                    task = Some(Task::CheckEnable);
                }
            }
            UiNodeOp::Event { update } => {
                c.event(update);

                if let Some(args) = MOUSE_INPUT_EVENT.on_unhandled(update) {
                    if args.is_mouse_down() && matches!(args.button, MouseButton::Middle) && AUTO_SCROLL_VAR.get() {
                        args.propagation().stop();

                        let mut open = true;
                        if let Some((id, closed)) = auto_scrolling.take() {
                            let closed = *closed.lock();
                            if closed == DInstant::MAX {
                                LAYERS.remove(id);
                                open = false;
                            } else {
                                open = closed.elapsed() > 50.ms();
                            }
                        }
                        if open {
                            let (wgt, wgt_id, closed) = auto_scroller_wgt();

                            let anchor = AnchorMode {
                                transform: zng_wgt_layer::AnchorTransform::CursorOnce {
                                    offset: zng_wgt_layer::AnchorOffset {
                                        place: Point::top_left(),
                                        origin: Point::center(),
                                    },
                                    include_touch: true,
                                    bounds: None,
                                },
                                min_size: zng_wgt_layer::AnchorSize::Unbounded,
                                max_size: zng_wgt_layer::AnchorSize::Window,
                                viewport_bound: true,
                                corner_radius: false,
                                visibility: true,
                                interactivity: false,
                            };
                            LAYERS.insert_anchored(LayerIndex::ADORNER, WIDGET.id(), anchor, wgt);
                            auto_scrolling = Some((wgt_id, closed));
                        }
                    }
                } else if let Some(args) = AUTO_SCROLL_CMD.scoped(WIDGET.id()).on_unhandled(update) {
                    if cmd_handle.is_enabled() {
                        args.propagation().stop();

                        let acc = args.param::<DipVector>().copied().unwrap_or_else(DipVector::zero);
                        SCROLL.auto_scroll(acc)
                    }
                }
            }
            UiNodeOp::Layout { wl, final_size } => {
                *final_size = c.layout(wl);
                cmd_handle.set_enabled(SCROLL.can_scroll_horizontal().get() || SCROLL.can_scroll_vertical().get());
            }
            _ => {}
        }

        while let Some(t) = task.take() {
            match t {
                Task::CheckEnable => {
                    if AUTO_SCROLL_VAR.get() {
                        if middle_handle.is_dummy() {
                            middle_handle = MOUSE_INPUT_EVENT.subscribe(WIDGET.id());
                        }
                    } else {
                        task = Some(Task::Disable);
                    }
                }
                Task::Disable => {
                    middle_handle = EventHandle::dummy();
                    if let Some((wgt_id, closed)) = auto_scrolling.take() {
                        if *closed.lock() == DInstant::MAX {
                            LAYERS.remove(wgt_id);
                        }
                    }
                }
            }
        }
    })
}

fn auto_scroller_wgt() -> (impl UiNode, WidgetId, Arc<Mutex<DInstant>>) {
    let id = WidgetId::new_unique();
    let mut wgt = Container::widget_new();
    let closed = Arc::new(Mutex::new(DInstant::MAX));
    widget_set! {
        wgt;
        id;
        zng_wgt_input::focus::focusable = true;
        zng_wgt_input::focus::focus_on_init = true;
        zng_wgt_container::child = presenter(AutoScrollArgs {}, AUTO_SCROLL_INDICATOR_VAR);
    }
    wgt.widget_builder().push_build_action(clmv!(closed, |w| {
        w.push_intrinsic(
            NestGroup::EVENT,
            "auto_scroller_node",
            clmv!(closed, |c| auto_scroller_node(c, closed)),
        );

        let mut ctx = LocalContext::capture_filtered(CaptureFilter::context_vars());
        let mut set = ContextValueSet::new();
        SCROLL.context_values_set(&mut set);
        ctx.extend(LocalContext::capture_filtered(CaptureFilter::Include(set)));

        w.push_intrinsic(NestGroup::CONTEXT, "scroll-ctx", |c| with_context_blend(ctx, true, c));
    }));

    (wgt.widget_build(), id, closed)
}
fn auto_scroller_node(child: impl UiNode, closed: Arc<Mutex<DInstant>>) -> impl UiNode {
    let mut requested_vel = DipVector::zero();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            // widget is focusable and focus_on_init.

            // RAW events to receive move outside widget without capturing pointer.
            WIDGET
                .sub_event(&RAW_MOUSE_MOVED_EVENT)
                .sub_event(&RAW_MOUSE_INPUT_EVENT)
                .sub_event(&FOCUS_CHANGED_EVENT);

            requested_vel = DipVector::zero();
        }
        UiNodeOp::Deinit => {
            SCROLL.auto_scroll(DipVector::zero());
            *closed.lock() = INSTANT.now();
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = RAW_MOUSE_MOVED_EVENT.on(update) {
                if args.window_id == WINDOW.id() {
                    let info = WIDGET.info();
                    let pos = args.position;
                    let bounds = info.inner_bounds().to_box2d().to_dip(info.tree().scale_factor());
                    let mut vel = DipVector::zero();

                    let limit = Dip::new(400);
                    if pos.x < bounds.min.x {
                        if SCROLL.can_scroll_left().get() {
                            vel.x = (pos.x - bounds.min.x).max(-limit);
                        }
                    } else if pos.x > bounds.max.x && SCROLL.can_scroll_right().get() {
                        vel.x = (pos.x - bounds.max.x).min(limit);
                    }
                    if pos.y < bounds.min.y {
                        if SCROLL.can_scroll_up().get() {
                            vel.y = (pos.y - bounds.min.y).max(-limit);
                        }
                    } else if pos.y > bounds.max.y && SCROLL.can_scroll_down().get() {
                        vel.y = (pos.y - bounds.max.y).min(limit);
                    }
                    vel *= 6.fct();

                    if vel != requested_vel {
                        SCROLL.auto_scroll(vel);
                        requested_vel = vel;
                    }
                }
            } else if let Some(args) = RAW_MOUSE_INPUT_EVENT.on(update) {
                if matches!((args.state, args.button), (ButtonState::Pressed, MouseButton::Middle)) {
                    args.propagation().stop();
                    LAYERS.remove(WIDGET.id());
                    SCROLL.auto_scroll(DipVector::zero());
                }
            } else if let Some(args) = KEY_INPUT_EVENT.on(update) {
                if matches!((args.state, &args.key), (KeyState::Pressed, Key::Escape)) {
                    args.propagation().stop();
                    LAYERS.remove(WIDGET.id());
                    SCROLL.auto_scroll(DipVector::zero());
                }
            } else if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                if args.is_blur(WIDGET.id()) {
                    LAYERS.remove(WIDGET.id());
                    SCROLL.auto_scroll(DipVector::zero());
                }
            }
        }
        _ => {}
    })
}

/// Renders a white circle with arrows that indicate what directions can be scrolled.
///
/// This is the default [`auto_scroll_indicator`].
///
/// [`auto_scroll_indicator`]: fn@crate::auto_scroll_indicator
pub fn default_auto_scroll_indicator() -> impl UiNode {
    match_node_leaf(|op| {
        match op {
            UiNodeOp::Init => {
                // vars used by SCROLL.can_scroll_*.
                WIDGET
                    .sub_var_render(&SCROLL_VIEWPORT_SIZE_VAR)
                    .sub_var_render(&SCROLL_CONTENT_SIZE_VAR)
                    .sub_var_render(&SCROLL_VERTICAL_OFFSET_VAR)
                    .sub_var_render(&SCROLL_HORIZONTAL_OFFSET_VAR);
            }
            UiNodeOp::Measure { desired_size, .. } => {
                *desired_size = PxSize::splat(Dip::new(40).to_px(LAYOUT.scale_factor()));
            }
            UiNodeOp::Layout { final_size, .. } => {
                *final_size = PxSize::splat(Dip::new(40).to_px(LAYOUT.scale_factor()));
            }
            UiNodeOp::Render { frame } => {
                let size = PxSize::splat(Dip::new(40).to_px(frame.scale_factor()));
                let corners = PxCornerRadius::new_all(size);
                // white circle
                frame.push_clip_rounded_rect(PxRect::from_size(size), corners, false, false, |frame| {
                    frame.push_color(PxRect::from_size(size), colors::WHITE.with_alpha(90.pct()).into());
                });
                // black border
                let widths = Dip::new(1).to_px(frame.scale_factor());
                frame.push_border(
                    PxRect::from_size(size),
                    PxSideOffsets::new_all_same(widths),
                    colors::BLACK.with_alpha(80.pct()).into(),
                    corners,
                );
                // black point middle
                let pt_size = PxSize::splat(Dip::new(4).to_px(frame.scale_factor()));
                frame.push_clip_rounded_rect(
                    PxRect::new((size / Px(2) - pt_size / Px(2)).to_vector().to_point(), pt_size),
                    PxCornerRadius::new_all(pt_size),
                    false,
                    false,
                    |frame| {
                        frame.push_color(PxRect::from_size(size), colors::BLACK.into());
                    },
                );

                // arrow
                let ar_size = PxSize::splat(Dip::new(20).to_px(frame.scale_factor()));
                let ar_center = ar_size / Px(2);

                // center circle
                let offset = (size / Px(2) - ar_center).to_vector();

                // 45º with origin center
                let transform = Transform::new_translate(-ar_center.width, -ar_center.height)
                    .rotate(45.deg())
                    .translate(ar_center.width + offset.x, ar_center.height + offset.y)
                    .layout()
                    .into();

                let widths = Dip::new(2).to_px(frame.scale_factor());
                let arrow_length = Dip::new(7).to_px(frame.scale_factor());
                let arrow_size = PxSize::splat(arrow_length);

                let mut arrow = |clip| {
                    frame.push_reference_frame(SpatialFrameId::new_unique().into(), transform, false, false, |frame| {
                        frame.push_clip_rect(clip, false, false, |frame| {
                            frame.push_border(
                                PxRect::from_size(ar_size),
                                PxSideOffsets::new_all_same(widths),
                                colors::BLACK.with_alpha(80.pct()).into(),
                                PxCornerRadius::zero(),
                            );
                        });
                    });
                };
                if SCROLL.can_scroll_up().get() {
                    arrow(PxRect::from_size(arrow_size));
                }
                if SCROLL.can_scroll_right().get() {
                    arrow(PxRect::new(PxPoint::new(ar_size.width - arrow_length, Px(0)), arrow_size));
                }
                if SCROLL.can_scroll_down().get() {
                    arrow(PxRect::new(
                        PxPoint::new(ar_size.width - arrow_length, ar_size.height - arrow_length),
                        arrow_size,
                    ));
                }
                if SCROLL.can_scroll_left().get() {
                    arrow(PxRect::new(PxPoint::new(Px(0), ar_size.height - arrow_length), arrow_size));
                }
            }
            _ => (),
        }
    })
}
