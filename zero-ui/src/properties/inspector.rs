//! Debug inspection properties.

use std::{cell::RefCell, fmt, rc::Rc};

use crate::core::{
    focus::*,
    mouse::{MOUSE_HOVERED_EVENT, MOUSE_MOVE_EVENT},
    widget_info::*,
    window::WINDOW_CTRL,
};
use crate::prelude::new_property::*;

/// Target of inspection properties.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum InspectMode {
    /// Just the widget where the inspector property is set.
    Widget,
    /// The widget where the inspector property is set and all descendants.
    ///
    /// This is the `true` value.
    WidgetAndDescendants,
    /// Disable inspection.
    ///
    /// This is the `false` value.
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
/// [center point]: crate::core::widget_info::WidgetInfo::center
#[property(CONTEXT, default(false))]
pub fn show_center_points(child: impl UiNode, mode: impl IntoVar<InspectMode>) -> impl UiNode {
    show_widget_tree(
        child,
        |_, wgt, frame| {
            frame.push_debug_dot(wgt.center(), colors::GREEN);
        },
        mode,
    )
}

/// Draws a border for every target widget's outer and inner bounds.
#[property(CONTEXT, default(false))]
pub fn show_bounds(child: impl UiNode, mode: impl IntoVar<InspectMode>) -> impl UiNode {
    show_widget_tree(
        child,
        |_, wgt, frame| {
            let p = Dip::new(1).to_px(frame.scale_factor().0);

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
        },
        mode,
    )
}

/// Draws a border over every inlined widget row in the window.
#[property(CONTEXT, default(false))]
pub fn show_rows(child: impl UiNode, mode: impl IntoVar<InspectMode>) -> impl UiNode {
    let spatial_id = SpatialFrameId::new_unique();
    show_widget_tree(
        child,
        move |i, wgt, frame| {
            let p = Dip::new(1).to_px(frame.scale_factor().0);

            let wgt = wgt.bounds_info();
            let transform = wgt.inner_transform();
            if let Some(inline) = wgt.inline() {
                frame.push_reference_frame((spatial_id, i as u32).into(), FrameValue::Value(transform), false, false, |frame| {
                    for row in &inline.rows {
                        frame.push_border(
                            *row,
                            PxSideOffsets::new_all_same(p),
                            BorderSides::dotted(colors::LIGHT_SALMON),
                            PxCornerRadius::zero(),
                        )
                    }
                })
            };
        },
        mode,
    )
}

fn show_widget_tree(
    child: impl UiNode,
    render: impl Fn(usize, WidgetInfo, &mut FrameBuilder) + Send + 'static,
    mode: impl IntoVar<InspectMode>,
) -> impl UiNode {
    #[ui_node(struct RenderWidgetTreeNode {
        child: impl UiNode,
        render: impl Fn(usize, WidgetInfo, &mut FrameBuilder) + Send + 'static,
        #[var] mode: impl Var<InspectMode>,
        cancel_space: SpatialFrameId,
    })]
    impl UiNode for RenderWidgetTreeNode {
        fn update(&mut self, updates: &mut WidgetUpdates) {
            if self.mode.is_new() {
                WIDGET.render();
            }
            self.child.update(updates);
        }

        fn render(&self, frame: &mut FrameBuilder) {
            self.child.render(frame);

            let mut render = |render: &mut dyn FnMut(WidgetInfo, &mut FrameBuilder)| {
                let tree = WINDOW.widget_tree();
                if let Some(wgt) = tree.get(WIDGET.id()) {
                    if WIDGET.parent_id().is_none() {
                        render(wgt, frame);
                    } else if let Some(t) = frame.transform().inverse() {
                        // cancel current transform
                        frame.push_reference_frame(self.cancel_space.into(), t.into(), false, false, |frame| {
                            render(wgt, frame);
                        })
                    } else {
                        tracing::error!("cannot inspect from `{:?}`, non-reversable transform", WIDGET.id())
                    }
                }
            };

            match self.mode.get() {
                InspectMode::Widget => {
                    render(&mut |wgt, frame| {
                        (self.render)(0, wgt, frame);
                    });
                }
                InspectMode::WidgetAndDescendants => {
                    render(&mut |wgt, frame| {
                        for (i, wgt) in wgt.self_and_descendants().enumerate() {
                            (self.render)(i, wgt, frame);
                        }
                    });
                }
                InspectMode::Disabled => {}
            }
        }
    }
    RenderWidgetTreeNode {
        child,
        render,
        mode: mode.into_var(),
        cancel_space: SpatialFrameId::new_unique(),
    }
}

/// Draws the inner bounds that where tested for the mouse point.
///
/// # Window Only
///
/// This property only works if set in a window, if set in another widget it will log an error and don't render anything.
#[property(CONTEXT, default(false))]
pub fn show_hit_test(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    #[ui_node(struct ShowHitTestNode {
        child: impl UiNode,
        #[var] enabled: impl Var<bool>,

        valid: bool,

        fails: Vec<PxRect>,
        hits: Vec<PxRect>,

        handles: EventHandles,
    })]
    impl UiNode for ShowHitTestNode {
        fn init(&mut self) {
            self.valid = WIDGET.parent_id().is_none();
            if self.valid {
                self.auto_subs();

                if self.enabled.get() {
                    let id = WIDGET.id();
                    self.handles = [MOUSE_MOVE_EVENT.subscribe(id), MOUSE_HOVERED_EVENT.subscribe(id)].into();
                } else {
                    self.handles.clear();
                }
            } else {
                tracing::error!("properties that render widget info are only valid in a window");
            }

            self.child.init();
        }

        fn deinit(&mut self) {
            self.handles.clear();
            self.child.deinit();
        }

        fn event(&mut self, update: &mut EventUpdate) {
            if let Some(args) = MOUSE_MOVE_EVENT.on(update) {
                if self.valid && self.enabled.get() {
                    let factor = WINDOW_CTRL.vars().scale_factor().get();
                    let pt = args.position.to_px(factor.0);

                    let fails = Rc::new(RefCell::new(vec![]));
                    let hits = Rc::new(RefCell::new(vec![]));

                    let tree = WINDOW.widget_tree();
                    let _ = tree
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

                        WIDGET.render();
                    }
                }
            } else if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
                if args.target.is_none() && !self.fails.is_empty() && !self.hits.is_empty() {
                    self.fails.clear();
                    self.hits.clear();

                    WIDGET.render();
                }
            }
            self.child.event(update);
        }

        fn update(&mut self, updates: &mut WidgetUpdates) {
            if let Some(enabled) = self.enabled.get_new() {
                if enabled && self.valid {
                    let id = WIDGET.id();
                    self.handles = [MOUSE_MOVE_EVENT.subscribe(id), MOUSE_HOVERED_EVENT.subscribe(id)].into();
                } else {
                    self.handles.clear();
                }
                WIDGET.render();
            }
            self.child.update(updates);
        }

        fn render(&self, frame: &mut FrameBuilder) {
            self.child.render(frame);

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
        handles: EventHandles::default(),
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
#[property(CONTEXT, default(None))]
pub fn show_directional_query(child: impl UiNode, orientation: impl IntoVar<Option<Orientation2D>>) -> impl UiNode {
    #[ui_node(struct ShowDirectionalQueryNode {
        child: impl UiNode,
        #[var] orientation: impl Var<Option<Orientation2D>>,
        valid: bool,
        search_quads: Vec<PxRect>,
        mouse_hovered_handle: Option<EventHandle>,
    })]
    impl UiNode for ShowDirectionalQueryNode {
        fn init(&mut self) {
            self.valid = WIDGET.parent_id().is_none();
            if self.valid {
                self.auto_subs();
                if self.orientation.get().is_some() {
                    self.mouse_hovered_handle = Some(MOUSE_HOVERED_EVENT.subscribe(WIDGET.id()));
                }
            } else {
                tracing::error!("properties that render widget info are only valid in a window");
            }

            self.child.init();
        }

        fn deinit(&mut self) {
            self.mouse_hovered_handle = None;
            self.child.deinit();
        }

        fn event(&mut self, update: &mut EventUpdate) {
            if self.valid {
                if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
                    if let Some(orientation) = self.orientation.get() {
                        let mut none = true;
                        if let Some(target) = &args.target {
                            let tree = WINDOW.widget_tree();
                            for w_id in target.widgets_path().iter().rev() {
                                if let Some(w) = tree.get(*w_id) {
                                    if let Some(w) = w.as_focusable(true, true) {
                                        let search_quads: Vec<_> = orientation
                                            .search_bounds(w.info.center(), Px::MAX, tree.spatial_bounds().to_box2d())
                                            .map(|q| q.to_rect())
                                            .collect();

                                        if self.search_quads != search_quads {
                                            self.search_quads = search_quads;
                                            WIDGET.render();
                                        }

                                        none = false;
                                        break;
                                    }
                                }
                            }
                        }

                        if none && !self.search_quads.is_empty() {
                            self.search_quads.clear();
                            WIDGET.render();
                        }
                    }
                }
            }

            self.child.event(update);
        }

        fn update(&mut self, updates: &mut WidgetUpdates) {
            if self.valid {
                if let Some(ori) = self.orientation.get_new() {
                    self.search_quads.clear();

                    if ori.is_some() {
                        self.mouse_hovered_handle = Some(MOUSE_HOVERED_EVENT.subscribe(WIDGET.id()));
                    } else {
                        self.mouse_hovered_handle = None;
                    }

                    WIDGET.render();
                }
            }
            self.child.update(updates);
        }

        fn render(&self, frame: &mut FrameBuilder) {
            self.child.render(frame);

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
