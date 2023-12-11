#![warn(unused_extern_crates)]
#![warn(missing_docs)]

//! Single child container base.

use std::fmt;

use zero_ui_wgt::{align, clip_to_bounds, margin, prelude::*, Wgt};

/// Base single content container.
#[widget($crate::Container {
    ($child:expr) => {
        child = $child;
    }
})]
pub struct Container(Wgt);
impl Container {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            if let Some(child) = wgt.capture_ui_node(property_id!(Self::child)) {
                wgt.set_child(child);
            }
        });
    }

    widget_impl! {
        /// The content.
        ///
        /// Can be any type that implements [`UiNode`], any widget.
        ///
        /// [`UiNode`]: zero_ui_app::widget::instance::UiNode
        pub zero_ui_app::widget::base::child(child: impl UiNode);

        /// Content overflow clipping.
        pub clip_to_bounds(clip: impl IntoVar<bool>);
    }
}

/// Margin space around the *content* of a widget.
///
/// This property is [`margin`](fn@margin) with nest group `CHILD_LAYOUT`.
#[property(CHILD_LAYOUT, default(0), widget_impl(Container))]
pub fn padding(child: impl UiNode, padding: impl IntoVar<SideOffsets>) -> impl UiNode {
    margin(child, padding)
}

/// Aligns the widget *content* within the available space.
///
/// This property is [`align`](fn@align) with nest group `CHILD_LAYOUT`.
#[property(CHILD_LAYOUT, default(Align::FILL), widget_impl(Container))]
pub fn child_align(child: impl UiNode, alignment: impl IntoVar<Align>) -> impl UiNode {
    align(child, alignment)
}

/// Placement of a node inserted by the [`child_insert`] property.
///
/// [`child_insert`]: fn@child_insert
#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ChildInsertPlace {
    /// Insert node above the child.
    Above,
    /// Insert node to the right of child.
    Right,
    /// Insert node below the child.
    Below,
    /// Insert node to the left of child.
    Left,

    /// Insert node to the left of child in [`LayoutDirection::LTR`] contexts and to the right of child
    /// in [`LayoutDirection::RTL`] contexts.
    Start,
    /// Insert node to the right of child in [`LayoutDirection::LTR`] contexts and to the left of child
    /// in [`LayoutDirection::RTL`] contexts.
    End,
}
impl fmt::Debug for ChildInsertPlace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "ChildInsertPlace::")?;
        }
        match self {
            Self::Above => write!(f, "Above"),
            Self::Right => write!(f, "Right"),
            Self::Below => write!(f, "Below"),
            Self::Left => write!(f, "Left"),
            Self::Start => write!(f, "Start"),
            Self::End => write!(f, "End"),
        }
    }
}
impl ChildInsertPlace {
    /// Convert [`ChildInsertPlace::Start`] and [`ChildInsertPlace::End`] to the fixed place they represent in the `direction` context.
    pub fn resolve_direction(self, direction: LayoutDirection) -> Self {
        match self {
            Self::Start => match direction {
                LayoutDirection::LTR => Self::Left,
                LayoutDirection::RTL => Self::Right,
            },
            Self::End => match direction {
                LayoutDirection::LTR => Self::Right,
                LayoutDirection::RTL => Self::Left,
            },
            p => p,
        }
    }

    /// Inserted node is to the left or right of child.
    pub fn is_x_axis(self) -> bool {
        !matches!(self, Self::Above | Self::Below)
    }

    /// Inserted node is above or bellow the child node.
    pub fn is_y_axis(self) -> bool {
        matches!(self, Self::Above | Self::Below)
    }
}

/// Insert the `insert` node in the `place` relative to the widget's child.
///
/// This property disables inline layout for the widget.
#[property(CHILD, default(ChildInsertPlace::Start, NilUiNode, 0), widget_impl(Container))]
pub fn child_insert(
    child: impl UiNode,
    place: impl IntoVar<ChildInsertPlace>,
    insert: impl UiNode,
    spacing: impl IntoVar<Length>,
) -> impl UiNode {
    let place = place.into_var();
    let spacing = spacing.into_var();
    let offset_key = FrameValueKey::new_unique();
    let mut offset_child = 0;
    let mut offset = PxVector::zero();

    match_node_list(ui_vec![child, insert], move |children, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&place).sub_var_layout(&spacing);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let c = LAYOUT.constraints();
            *desired_size = if place.get().is_x_axis() {
                let mut spacing = spacing.layout_x();
                let insert_size = children.with_node(1, |n| {
                    LAYOUT.with_constraints(c.with_new_min(Px(0), Px(0)).with_fill_x(false), || wm.measure_block(n))
                });
                if insert_size.width == Px(0) {
                    spacing = Px(0);
                }
                let child_size = children.with_node(0, |n| {
                    LAYOUT.with_constraints(c.with_less_x(insert_size.width + spacing), || wm.measure_block(n))
                });

                PxSize::new(
                    insert_size.width + spacing + child_size.width,
                    insert_size.height.max(child_size.height),
                )
            } else {
                let mut spacing = spacing.layout_y();
                let insert_size = children.with_node(1, |n| {
                    LAYOUT.with_constraints(c.with_new_min(Px(0), Px(0)).with_fill_y(false), || wm.measure_block(n))
                });
                if insert_size.height == Px(0) {
                    spacing = Px(0);
                }
                let child_size = children.with_node(0, |n| {
                    LAYOUT.with_constraints(c.with_less_y(insert_size.height + spacing), || wm.measure_block(n))
                });
                if child_size.height == Px(0) {
                    spacing = Px(0);
                }
                PxSize::new(
                    insert_size.width.max(child_size.width),
                    insert_size.height + spacing + child_size.height,
                )
            };
        }
        UiNodeOp::Layout { wl, final_size } => {
            let place = place.get().resolve_direction(LAYOUT.direction());
            let c = LAYOUT.constraints();

            *final_size = match place {
                ChildInsertPlace::Left | ChildInsertPlace::Right => {
                    let spacing = spacing.layout_x();

                    let mut constraints_y = LAYOUT.constraints().y;
                    if constraints_y.fill_or_exact().is_none() {
                        // measure to find fill height
                        let mut wm = wl.to_measure(None);
                        let wm = &mut wm;
                        let mut spacing = spacing;

                        let insert_size = children.with_node(1, |n| {
                            LAYOUT.with_constraints(c.with_new_min(Px(0), Px(0)).with_fill_x(false), || n.measure(wm))
                        });
                        if insert_size.width == Px(0) {
                            spacing = Px(0);
                        }
                        let child_size = children.with_node(0, |n| {
                            LAYOUT.with_constraints(c.with_less_x(insert_size.width + spacing), || n.measure(wm))
                        });

                        constraints_y = constraints_y.with_fill(true).with_max(child_size.height.max(insert_size.height));
                    }

                    let mut spacing = spacing;
                    let insert_size = children.with_node(1, |n| {
                        LAYOUT.with_constraints(
                            {
                                let mut c = c;
                                c.y = constraints_y;
                                c.with_new_min(Px(0), Px(0)).with_fill_x(false)
                            },
                            || n.layout(wl),
                        )
                    });
                    if insert_size.width == Px(0) {
                        spacing = Px(0);
                    }
                    let child_size = children.with_node(0, |n| {
                        LAYOUT.with_constraints(
                            {
                                let mut c = c;
                                c.y = constraints_y;
                                c.with_less_x(insert_size.width + spacing)
                            },
                            || n.layout(wl),
                        )
                    });
                    if child_size.width == Px(0) {
                        spacing = Px(0);
                    }

                    // position
                    let (child, o) = match place {
                        ChildInsertPlace::Left => (0, insert_size.width + spacing),
                        ChildInsertPlace::Right => (1, child_size.width + spacing),
                        _ => unreachable!(),
                    };
                    let o = PxVector::new(o, Px(0));
                    if offset != o || offset_child != child {
                        offset_child = child;
                        offset = o;
                        WIDGET.render_update();
                    }

                    PxSize::new(
                        insert_size.width + spacing + child_size.width,
                        insert_size.height.max(child_size.height),
                    )
                }
                ChildInsertPlace::Above | ChildInsertPlace::Below => {
                    let spacing = spacing.layout_y();

                    let mut constraints_x = c.x;
                    if constraints_x.fill_or_exact().is_none() {
                        // measure fill width

                        let mut wm = wl.to_measure(None);
                        let wm = &mut wm;
                        let mut spacing = spacing;

                        let insert_size = children.with_node(1, |n| {
                            LAYOUT.with_constraints(c.with_new_min(Px(0), Px(0)).with_fill_y(false), || n.measure(wm))
                        });
                        if insert_size.height == Px(0) {
                            spacing = Px(0);
                        }
                        let child_size = children.with_node(0, |n| {
                            LAYOUT.with_constraints(c.with_less_y(insert_size.height + spacing), || n.measure(wm))
                        });

                        constraints_x = constraints_x.with_fill(true).with_max(child_size.width.max(insert_size.width));
                    }

                    let mut spacing = spacing;
                    let insert_size = children.with_node(1, |n| {
                        LAYOUT.with_constraints(
                            {
                                let mut c = c;
                                c.x = constraints_x;
                                c.with_new_min(Px(0), Px(0)).with_fill_y(false)
                            },
                            || n.layout(wl),
                        )
                    });
                    if insert_size.height == Px(0) {
                        spacing = Px(0);
                    }
                    let child_size = children.with_node(0, |n| {
                        LAYOUT.with_constraints(
                            {
                                let mut c = c;
                                c.x = constraints_x;
                                c.with_less_y(insert_size.height + spacing)
                            },
                            || n.layout(wl),
                        )
                    });

                    // position
                    let (child, o) = match place {
                        ChildInsertPlace::Above => (0, insert_size.height + spacing),
                        ChildInsertPlace::Below => (1, child_size.height + spacing),
                        _ => unreachable!(),
                    };
                    let o = PxVector::new(Px(0), o);
                    if offset != o || offset_child != child {
                        offset_child = child;
                        offset = o;
                        WIDGET.render_update();
                    }

                    PxSize::new(
                        insert_size.width.max(child_size.width),
                        insert_size.height + spacing + child_size.height,
                    )
                }
                _ => {
                    unreachable!()
                }
            };
        }
        UiNodeOp::Render { frame } => children.for_each(|i, child| {
            if i as u8 == offset_child {
                frame.push_reference_frame(offset_key.into(), offset_key.bind(offset.into(), false), true, true, |frame| {
                    child.render(frame);
                });
            } else {
                child.render(frame);
            }
        }),
        UiNodeOp::RenderUpdate { update } => {
            children.for_each(|i, child| {
                if i as u8 == offset_child {
                    update.with_transform(offset_key.update(offset.into(), false), true, |update| {
                        child.render_update(update);
                    });
                } else {
                    child.render_update(update);
                }
            });
        }
        _ => {}
    })
}

/// Insert the `insert` node in the `place` relative to the widget's child, but outside of the child layout.
///
/// This is still *inside* the parent widget, but outside of properties like padding.
///
/// This property disables inline layout for the widget.
#[property(CHILD_LAYOUT - 1, default(ChildInsertPlace::Start, NilUiNode, 0), widget_impl(Container))]
pub fn child_out_insert(
    child: impl UiNode,
    place: impl IntoVar<ChildInsertPlace>,
    insert: impl UiNode,
    spacing: impl IntoVar<Length>,
) -> impl UiNode {
    child_insert(child, place, insert, spacing)
}

/// Insert `insert` to the left of the widget's child.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(NilUiNode, 0), widget_impl(Container))]
pub fn child_insert_left(child: impl UiNode, insert: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    child_insert(child, ChildInsertPlace::Left, insert, spacing)
}

/// Insert `insert` to the right of the widget's child.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(NilUiNode, 0), widget_impl(Container))]
pub fn child_insert_right(child: impl UiNode, insert: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    child_insert(child, ChildInsertPlace::Right, insert, spacing)
}

/// Insert `insert` above the widget's child.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(NilUiNode, 0), widget_impl(Container))]
pub fn child_insert_above(child: impl UiNode, insert: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    child_insert(child, ChildInsertPlace::Above, insert, spacing)
}

/// Insert `insert` below the widget's child.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(NilUiNode, 0), widget_impl(Container))]
pub fn child_insert_below(child: impl UiNode, insert: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    child_insert(child, ChildInsertPlace::Below, insert, spacing)
}

/// Insert `insert` to the left of the widget's child in LTR contexts or to the right of the widget's child in RTL contexts.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(NilUiNode, 0), widget_impl(Container))]
pub fn child_insert_start(child: impl UiNode, insert: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    child_insert(child, ChildInsertPlace::Start, insert, spacing)
}

/// Insert `insert` to the right of the widget's child in LTR contexts or to the right of the widget's child in RTL contexts.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(NilUiNode, 0), widget_impl(Container))]
pub fn child_insert_end(child: impl UiNode, insert: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    child_insert(child, ChildInsertPlace::End, insert, spacing)
}
