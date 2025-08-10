//! Debug inspection properties.

use std::{cell::RefCell, fmt, rc::Rc};

use zng_ext_input::{
    focus::WidgetInfoFocusExt as _,
    mouse::{MOUSE_HOVERED_EVENT, MOUSE_MOVE_EVENT},
};
use zng_ext_window::WINDOW_Ext as _;
use zng_layout::unit::Orientation2D;
use zng_view_api::display_list::FrameValue;
use zng_wgt::prelude::*;

/// Target of inspection properties.
#[derive(Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum InspectMode {
    /// Just the widget where the inspector property is set.
    Widget,
    /// The widget where the inspector property is set and all descendants.
    ///
    /// The `true` value converts to this.
    WidgetAndDescendants,
    /// Disable inspection.
    ///
    /// The `false` value converts to this.
    #[default]
    Disabled,
}
impl fmt::Debug for InspectMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "InspectMode::")?;
        }
        match self {
            Self::Widget => write!(f, "Widget"),
            Self::WidgetAndDescendants => write!(f, "WidgetAndDescendants"),
            Self::Disabled => write!(f, "Disabled"),
        }
    }
}
impl_from_and_into_var! {
    fn from(widget_and_descendants: bool) -> InspectMode {
        if widget_and_descendants {
            InspectMode::WidgetAndDescendants
        } else {
            InspectMode::Disabled
        }
    }
}

/// Draws a debug dot in target widget's [center point].
///
/// [center point]: zng_wgt::prelude::WidgetInfo::center
#[property(CONTEXT, default(false))]
pub fn show_center_points(child: impl IntoUiNode, mode: impl IntoVar<InspectMode>) -> UiNode {
    show_widget_tree(
        child,
        |_, wgt, frame| {
            frame.push_debug_dot(wgt.center(), colors::GREEN);
        },
        mode,
    )
}

/// Draws a border for every target widget's outer and inner bounds.
///
/// The outer bounds is drawn dotted and in pink, the inner bounds is drawn solid and in blue.
#[property(CONTEXT, default(false))]
pub fn show_bounds(child: impl IntoUiNode, mode: impl IntoVar<InspectMode>) -> UiNode {
    show_widget_tree(
        child,
        |_, wgt, frame| {
            let p = Dip::new(1).to_px(frame.scale_factor());

            let outer_bounds = wgt.outer_bounds();
            let inner_bounds = wgt.inner_bounds();

            if outer_bounds != inner_bounds && !outer_bounds.is_empty() {
                frame.push_border(
                    wgt.outer_bounds(),
                    PxSideOffsets::new_all_same(p),
                    BorderSides::dotted(web_colors::PINK),
                    PxCornerRadius::zero(),
                );
            }

            if !inner_bounds.size.is_empty() {
                frame.push_border(
                    inner_bounds,
                    PxSideOffsets::new_all_same(p),
                    BorderSides::solid(web_colors::ROYAL_BLUE),
                    PxCornerRadius::zero(),
                );
            }
        },
        mode,
    )
}

/// Draws a border over every inlined widget row in the window.
#[property(CONTEXT, default(false))]
pub fn show_rows(child: impl IntoUiNode, mode: impl IntoVar<InspectMode>) -> UiNode {
    let spatial_id = SpatialFrameId::new_unique();
    show_widget_tree(
        child,
        move |i, wgt, frame| {
            let p = Dip::new(1).to_px(frame.scale_factor());

            let wgt = wgt.bounds_info();
            let transform = wgt.inner_transform();
            if let Some(inline) = wgt.inline() {
                frame.push_reference_frame((spatial_id, i as u32).into(), FrameValue::Value(transform), false, false, |frame| {
                    for row in &inline.rows {
                        if !row.size.is_empty() {
                            frame.push_border(
                                *row,
                                PxSideOffsets::new_all_same(p),
                                BorderSides::dotted(web_colors::LIGHT_SALMON),
                                PxCornerRadius::zero(),
                            );
                        }
                    }
                })
            };
        },
        mode,
    )
}

fn show_widget_tree(
    child: impl IntoUiNode,
    mut render: impl FnMut(usize, WidgetInfo, &mut FrameBuilder) + Send + 'static,
    mode: impl IntoVar<InspectMode>,
) -> UiNode {
    let mode = mode.into_var();
    let cancel_space = SpatialFrameId::new_unique();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&mode);
        }
        UiNodeOp::Render { frame } => {
            child.render(frame);

            let mut r = |render: &mut dyn FnMut(WidgetInfo, &mut FrameBuilder)| {
                let tree = WINDOW.info();
                if let Some(wgt) = tree.get(WIDGET.id()) {
                    if WIDGET.parent_id().is_none() {
                        render(wgt, frame);
                    } else if let Some(t) = frame.transform().inverse() {
                        // cancel current transform
                        frame.push_reference_frame(cancel_space.into(), t.into(), false, false, |frame| {
                            render(wgt, frame);
                        })
                    } else {
                        tracing::error!("cannot inspect from `{:?}`, non-invertible transform", WIDGET.id())
                    }
                }
            };

            match mode.get() {
                InspectMode::Widget => {
                    r(&mut |wgt, frame| {
                        render(0, wgt, frame);
                    });
                }
                InspectMode::WidgetAndDescendants => {
                    r(&mut |wgt, frame| {
                        for (i, wgt) in wgt.self_and_descendants().enumerate() {
                            render(i, wgt, frame);
                        }
                    });
                }
                InspectMode::Disabled => {}
            }
        }
        _ => {}
    })
}

/// Draws the inner bounds that where tested for the mouse point.
///
/// # Window Only
///
/// This property only works if set in a window, if set in another widget it will log an error and not render anything.
#[property(CONTEXT, default(false))]
pub fn show_hit_test(child: impl IntoUiNode, enabled: impl IntoVar<bool>) -> UiNode {
    let enabled = enabled.into_var();
    let mut handles = EventHandles::default();
    let mut valid = false;
    let mut fails = vec![];
    let mut hits = vec![];

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            valid = WIDGET.parent_id().is_none();
            if valid {
                WIDGET.sub_var(&enabled);

                if enabled.get() {
                    let id = WIDGET.id();
                    handles = [MOUSE_MOVE_EVENT.subscribe(id), MOUSE_HOVERED_EVENT.subscribe(id)].into();
                } else {
                    handles.clear();
                }
            } else {
                tracing::error!("property `show_hit_test` is only valid in a window");
            }
        }
        UiNodeOp::Deinit => {
            handles.clear();
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = MOUSE_MOVE_EVENT.on(update) {
                if valid && enabled.get() {
                    let factor = WINDOW.vars().scale_factor().get();
                    let pt = args.position.to_px(factor);

                    let new_fails = Rc::new(RefCell::new(vec![]));
                    let new_hits = Rc::new(RefCell::new(vec![]));

                    let tree = WINDOW.info();
                    let _ = tree
                        .root()
                        .spatial_iter(clmv!(new_fails, new_hits, |w| {
                            let bounds = w.inner_bounds();
                            let hit = bounds.contains(pt);
                            if hit {
                                new_hits.borrow_mut().push(bounds);
                            } else {
                                new_fails.borrow_mut().push(bounds);
                            }
                            hit
                        }))
                        .count();

                    let new_fails = Rc::try_unwrap(new_fails).unwrap().into_inner();
                    let new_hits = Rc::try_unwrap(new_hits).unwrap().into_inner();

                    if fails != new_fails || hits != new_hits {
                        fails = new_fails;
                        hits = new_hits;

                        WIDGET.render();
                    }
                }
            } else if let Some(args) = MOUSE_HOVERED_EVENT.on(update)
                && args.target.is_none()
                && !fails.is_empty()
                && !hits.is_empty()
            {
                fails.clear();
                hits.clear();

                WIDGET.render();
            }
        }
        UiNodeOp::Update { .. } => {
            if let Some(enabled) = enabled.get_new() {
                if enabled && valid {
                    let id = WIDGET.id();
                    handles = [MOUSE_MOVE_EVENT.subscribe(id), MOUSE_HOVERED_EVENT.subscribe(id)].into();
                } else {
                    handles.clear();
                }
                WIDGET.render();
            }
        }
        UiNodeOp::Render { frame } => {
            child.render(frame);

            if valid && enabled.get() {
                let widths = PxSideOffsets::new_all_same(Px(1));
                let fail_sides = BorderSides::solid(colors::RED);
                let hits_sides = BorderSides::solid(web_colors::LIME_GREEN);

                frame.with_hit_tests_disabled(|frame| {
                    for fail in &fails {
                        if !fail.size.is_empty() {
                            frame.push_border(*fail, widths, fail_sides, PxCornerRadius::zero());
                        }
                    }

                    for hit in &hits {
                        if !hit.size.is_empty() {
                            frame.push_border(*hit, widths, hits_sides, PxCornerRadius::zero());
                        }
                    }
                });
            }
        }
        _ => {}
    })
}

/// Draw the directional query for closest sibling of the hovered focusable widget.
///
/// # Window Only
///
/// This property only works if set in a window, if set in another widget it will log an error and not render anything.
#[property(CONTEXT, default(None))]
pub fn show_directional_query(child: impl IntoUiNode, orientation: impl IntoVar<Option<Orientation2D>>) -> UiNode {
    let orientation = orientation.into_var();
    let mut valid = false;
    let mut search_quads = vec![];
    let mut _mouse_hovered_handle = None;

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            valid = WIDGET.parent_id().is_none();
            if valid {
                WIDGET.sub_var(&orientation);
                if orientation.get().is_some() {
                    _mouse_hovered_handle = Some(MOUSE_HOVERED_EVENT.subscribe(WIDGET.id()));
                }
            } else {
                tracing::error!("property `show_directional_query` is only valid in a window");
            }
        }
        UiNodeOp::Deinit => {
            _mouse_hovered_handle = None;
        }
        UiNodeOp::Event { update } => {
            if !valid {
                return;
            }
            if let Some(args) = MOUSE_HOVERED_EVENT.on(update)
                && let Some(orientation) = orientation.get()
            {
                let mut none = true;
                if let Some(target) = &args.target {
                    let tree = WINDOW.info();
                    for w_id in target.widgets_path().iter().rev() {
                        if let Some(w) = tree.get(*w_id)
                            && let Some(w) = w.into_focusable(true, true)
                        {
                            let sq: Vec<_> = orientation
                                .search_bounds(w.info().center(), Px::MAX, tree.spatial_bounds().to_box2d())
                                .map(|q| q.to_rect())
                                .collect();

                            if search_quads != sq {
                                search_quads = sq;
                                WIDGET.render();
                            }

                            none = false;
                            break;
                        }
                    }
                }

                if none && !search_quads.is_empty() {
                    search_quads.clear();
                    WIDGET.render();
                }
            }
        }
        UiNodeOp::Update { .. } => {
            if !valid {
                return;
            }
            if let Some(ori) = orientation.get_new() {
                search_quads.clear();

                if ori.is_some() {
                    _mouse_hovered_handle = Some(MOUSE_HOVERED_EVENT.subscribe(WIDGET.id()));
                } else {
                    _mouse_hovered_handle = None;
                }

                WIDGET.render();
            }
        }
        UiNodeOp::Render { frame } => {
            child.render(frame);

            if valid && orientation.get().is_some() {
                let widths = PxSideOffsets::new_all_same(Px(1));
                let quad_sides = BorderSides::solid(colors::YELLOW);

                frame.with_hit_tests_disabled(|frame| {
                    for quad in &search_quads {
                        if !quad.size.is_empty() {
                            frame.push_border(*quad, widths, quad_sides, PxCornerRadius::zero());
                        }
                    }
                });
            }
        }
        _ => {}
    })
}
