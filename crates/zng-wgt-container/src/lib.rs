#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Base container widget and properties.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::fmt;

use zng_wgt::{align, clip_to_bounds, margin, prelude::*};

/// Base container.
#[widget($crate::Container { ($child:expr) => { child = $child; } })]
pub struct Container(WidgetBase);
impl Container {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            if let Some(child) = wgt.capture_ui_node(property_id!(Self::child)) {
                wgt.set_child(child);
            }
        });
    }

    widget_impl! {
        /// Content overflow clipping.
        pub clip_to_bounds(clip: impl IntoVar<bool>);
    }
}

/// The widget's child.
///
/// Can be any type that implements [`UiNode`], any widget.
///
/// In `Container!` derived widgets or similar this property is captured and used as the actual child, in other widgets
/// this property is an alias for [`child_under`](fn@child_under).
///
/// [`UiNode`]: zng_app::widget::node::UiNode
#[property(CHILD, default(FillUiNode), widget_impl(Container))]
pub fn child(widget_child: impl IntoUiNode, child: impl IntoUiNode) -> UiNode {
    child_under(widget_child, child)
}

/// Margin space around the content of a widget.
///
/// This property is [`margin`](fn@margin) with nest group `CHILD_LAYOUT`.
#[property(CHILD_LAYOUT, default(0), widget_impl(Container))]
pub fn padding(child: impl IntoUiNode, padding: impl IntoVar<SideOffsets>) -> UiNode {
    margin(child, padding)
}

/// Aligns the widget *content* within the available space.
///
/// This property is [`align`](fn@align) with nest group `CHILD_LAYOUT`.
#[property(CHILD_LAYOUT, default(Align::FILL), widget_impl(Container))]
pub fn child_align(child: impl IntoUiNode, alignment: impl IntoVar<Align>) -> UiNode {
    align(child, alignment)
}

/// Placement of a node inserted by the [`child_insert`] property.
///
/// [`child_insert`]: fn@child_insert
#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ChildInsert {
    /// Insert node above the child.
    Top,
    /// Insert node to the right of child.
    Right,
    /// Insert node below the child.
    Bottom,
    /// Insert node to the left of child.
    Left,

    /// Insert node to the left of child in [`LayoutDirection::LTR`] contexts and to the right of child
    /// in [`LayoutDirection::RTL`] contexts.
    ///
    /// [`LayoutDirection::LTR`]: zng_wgt::prelude::LayoutDirection::LTR
    /// [`LayoutDirection::RTL`]: zng_wgt::prelude::LayoutDirection::RTL
    Start,
    /// Insert node to the right of child in [`LayoutDirection::LTR`] contexts and to the left of child
    /// in [`LayoutDirection::RTL`] contexts.
    ///
    /// [`LayoutDirection::LTR`]: zng_wgt::prelude::LayoutDirection::LTR
    /// [`LayoutDirection::RTL`]: zng_wgt::prelude::LayoutDirection::RTL
    End,

    /// Insert node over the child.
    ///
    /// Spacing is ignored for this placement.
    Over,
    /// Insert node under the child.
    ///
    /// Spacing is ignored for this placement.
    Under,
}
impl fmt::Debug for ChildInsert {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "ChildInsert::")?;
        }
        match self {
            Self::Top => write!(f, "Top"),
            Self::Right => write!(f, "Right"),
            Self::Bottom => write!(f, "Bottom"),
            Self::Left => write!(f, "Left"),
            Self::Start => write!(f, "Start"),
            Self::End => write!(f, "End"),
            Self::Over => write!(f, "Over"),
            Self::Under => write!(f, "Under"),
        }
    }
}
impl ChildInsert {
    /// Convert [`ChildInsert::Start`] and [`ChildInsert::End`] to the fixed place they represent in the `direction` context.
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
        !matches!(self, Self::Top | Self::Bottom)
    }

    /// Inserted node is above or bellow the child node.
    pub fn is_y_axis(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }

    /// Inserted node is over or under the child node.
    pub fn is_z_axis(self) -> bool {
        matches!(self, Self::Over | Self::Under)
    }

    /// Layout the spacing for the direction.
    ///
    /// Expects that [`resolve_direction`] was already called.
    ///
    /// [`resolve_direction`]: Self::resolve_direction
    pub fn spacing(self, spacing: &Var<SideOffsets>) -> Px {
        spacing.with(|s| match self {
            ChildInsert::Top => s.top.layout_y(),
            ChildInsert::Right => s.right.layout_x(),
            ChildInsert::Bottom => s.bottom.layout_y(),
            ChildInsert::Left => s.left.layout_x(),
            _ => Px(0),
        })
    }
}

static_id! {
    /// Identifies the [`child_spacing`] set on the widget.
    ///
    /// [`child_spacing`]: fn@child_spacing
    pub static ref CHILD_SPACING_ID: StateId<Var<SideOffsets>>;
    /// Identifies the [`child_out_spacing`] set on the widget.
    ///
    /// [`child_out_spacing`]: fn@child_out_spacing
    pub static ref CHILD_OUT_SPACING_ID: StateId<Var<SideOffsets>>;
}

/// Spacing between [`child`] and one of the [`child_insert`] properties.
///
/// The spacing is only applied if the child insert property in the direction is set.
///
/// [`child`]: fn@child
/// [`child_insert`]: fn@child_insert
#[property(CONTEXT, default(0), widget_impl(Container))]
pub fn child_spacing(child: impl IntoUiNode, spacing: impl IntoVar<SideOffsets>) -> UiNode {
    let spacing = spacing.into_var();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&spacing);
            WIDGET.set_state(*CHILD_SPACING_ID, spacing.clone());
        }
        UiNodeOp::Deinit => {
            c.deinit();
            WIDGET.set_state(*CHILD_SPACING_ID, const_var(SideOffsets::zero()));
        }
        _ => {}
    })
}

/// Spacing between child and child layout nodes and one of the [`child_out_insert`] properties.
///
/// The spacing is only applied if the child insert property in the direction is set.
///
/// [`child`]: fn@child
/// [`child_insert`]: fn@child_insert
#[property(CONTEXT, default(0), widget_impl(Container))]
pub fn child_out_spacing(child: impl IntoUiNode, spacing: impl IntoVar<SideOffsets>) -> UiNode {
    let spacing = spacing.into_var();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&spacing);
            WIDGET.set_state(*CHILD_OUT_SPACING_ID, spacing.clone());
        }
        UiNodeOp::Deinit => {
            c.deinit();
            WIDGET.set_state(*CHILD_OUT_SPACING_ID, const_var(SideOffsets::zero()));
        }
        _ => {}
    })
}

/// Insert `node` in the `placement` relative to the widget's child.
///
/// The `node` is inserted inside the `CHILD_LAYOUT` scope, meaning inside [`padding`], just like the [`child`].
/// See also [`child_out_insert`] for inserting a node outside the child layout.
///
/// Spacing between the widget's child and node can be configured using [`child_spacing`].
///
/// A property for each direction is also provided, see [`child_start`], [`child_end`], [`child_left`],
/// [`child_right`], [`child_top`], [`child_bottom`], [`child_over`] and [`child_under`].
///
/// This property disables inline layout for the widget.
///
/// [`padding`]: fn@padding
/// [`child`]: fn@child
/// [`child_spacing`]: fn@child_spacing
/// [`child_out_insert`]: fn@child_out_insert
/// [`child_start`]: fn@child_start
/// [`child_end`]: fn@child_end
/// [`child_left`]: fn@child_left
/// [`child_right`]: fn@child_right
/// [`child_top`]: fn@child_top
/// [`child_bottom`]: fn@child_bottom
/// [`child_over`]: fn@child_over
/// [`child_under`]: fn@child_under
#[property(CHILD, default(ChildInsert::Start, UiNode::nil()), widget_impl(Container))]
pub fn child_insert(child: impl IntoUiNode, placement: impl IntoVar<ChildInsert>, node: impl IntoUiNode) -> UiNode {
    fn init_spacing(s: &mut Var<SideOffsets>) {
        if let Some(v) = WIDGET.get_state(*CHILD_SPACING_ID) {
            *s = v;
        }
    }
    child_insert_node(child.into_node(), placement.into_var(), node.into_node(), init_spacing)
}

/// Insert `node` in the `placement` relative to the widget's child, outside of the `CHILD_LAYOUT` scope, meaning outside [`padding`], but
/// still inside the widget.
///
/// Spacing between the widget's child layout nodes and the `node` can be configured using [`child_out_spacing`].
///
/// A property for each direction is also provided, see [`child_out_start`], [`child_out_end`], [`child_out_left`],
/// [`child_out_right`], [`child_out_top`], [`child_out_bottom`], [`child_out_over`] and [`child_out_under`].
///
/// This property disables inline layout for the widget.
///
/// [`padding`]: fn@padding
/// [`child_out_spacing`]: fn@child_out_spacing
/// [`child_out_start`]: fn@child_out_start
/// [`child_out_end`]: fn@child_out_end
/// [`child_out_left`]: fn@child_out_left
/// [`child_out_right`]: fn@child_out_right
/// [`child_out_top`]: fn@child_out_top
/// [`child_out_bottom`]: fn@child_out_bottom
/// [`child_out_over`]: fn@child_out_over
/// [`child_out_under`]: fn@child_out_under
#[property(CHILD_LAYOUT - 1, default(ChildInsert::Start, UiNode::nil()), widget_impl(Container))]
pub fn child_out_insert(child: impl IntoUiNode, placement: impl IntoVar<ChildInsert>, node: impl IntoUiNode) -> UiNode {
    fn init_spacing(s: &mut Var<SideOffsets>) {
        if let Some(v) = WIDGET.get_state(*CHILD_OUT_SPACING_ID) {
            *s = v;
        }
    }
    child_insert_node(child.into_node(), placement.into_var(), node.into_node(), init_spacing)
}

/// Insert `node` to the left of the widget's child.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(UiNode::nil()), widget_impl(Container))]
pub fn child_left(child: impl IntoUiNode, node: impl IntoUiNode) -> UiNode {
    child_insert(child, ChildInsert::Left, node)
}

/// Insert `node` to the right of the widget's child.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(UiNode::nil()), widget_impl(Container))]
pub fn child_right(child: impl IntoUiNode, node: impl IntoUiNode) -> UiNode {
    child_insert(child, ChildInsert::Right, node)
}

/// Insert `node` above the widget's child.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(UiNode::nil()), widget_impl(Container))]
pub fn child_top(child: impl IntoUiNode, node: impl IntoUiNode) -> UiNode {
    child_insert(child, ChildInsert::Top, node)
}

/// Insert `node` below the widget's child.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(UiNode::nil()), widget_impl(Container))]
pub fn child_bottom(child: impl IntoUiNode, node: impl IntoUiNode) -> UiNode {
    child_insert(child, ChildInsert::Bottom, node)
}

/// Insert `node` to the left of the widget's child in LTR contexts or to the right in RTL contexts.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(UiNode::nil()), widget_impl(Container))]
pub fn child_start(child: impl IntoUiNode, node: impl IntoUiNode) -> UiNode {
    child_insert(child, ChildInsert::Start, node)
}

/// Insert `node` to the right of the widget's child in LTR contexts or to the right of the widget's child in RTL contexts.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(UiNode::nil()), widget_impl(Container))]
pub fn child_end(child: impl IntoUiNode, node: impl IntoUiNode) -> UiNode {
    child_insert(child, ChildInsert::End, node)
}

/// Insert `node` over the widget's child.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(UiNode::nil()), widget_impl(Container))]
pub fn child_over(child: impl IntoUiNode, node: impl IntoUiNode) -> UiNode {
    child_insert(child, ChildInsert::Over, node)
}

/// Insert `node` under the widget's child.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(UiNode::nil()), widget_impl(Container))]
pub fn child_under(child: impl IntoUiNode, node: impl IntoUiNode) -> UiNode {
    child_insert(child, ChildInsert::Under, node)
}

/// Insert `node` to the left of the widget's child, outside of the child layout.
///
/// This property disables inline layout for the widget. See [`child_out_insert`] for more details.
///
/// [`child_out_insert`]: fn@child_insert
#[property(CHILD_LAYOUT - 1, default(UiNode::nil()), widget_impl(Container))]
pub fn child_out_left(child: impl IntoUiNode, node: impl IntoUiNode) -> UiNode {
    child_out_insert(child, ChildInsert::Left, node)
}

/// Insert `node` to the right of the widget's child, outside of the child layout.
///
/// This property disables inline layout for the widget. See [`child_out_insert`] for more details.
///
/// [`child_out_insert`]: fn@child_out_insert
#[property(CHILD_LAYOUT - 1, default(UiNode::nil()), widget_impl(Container))]
pub fn child_out_right(child: impl IntoUiNode, node: impl IntoUiNode) -> UiNode {
    child_out_insert(child, ChildInsert::Right, node)
}

/// Insert `node` above the widget's child, outside of the child layout.
///
/// This property disables inline layout for the widget. See [`child_out_insert`] for more details.
///
/// [`child_out_insert`]: fn@child_out_insert
#[property(CHILD_LAYOUT - 1, default(UiNode::nil()), widget_impl(Container))]
pub fn child_out_top(child: impl IntoUiNode, node: impl IntoUiNode) -> UiNode {
    child_out_insert(child, ChildInsert::Top, node)
}

/// Insert `node` below the widget's child, outside of the child layout.
///
/// This property disables inline layout for the widget. See [`child_out_insert`] for more details.
///
/// [`child_out_insert`]: fn@child_out_insert
#[property(CHILD_LAYOUT - 1, default(UiNode::nil()), widget_impl(Container))]
pub fn child_out_bottom(child: impl IntoUiNode, node: impl IntoUiNode) -> UiNode {
    child_out_insert(child, ChildInsert::Bottom, node)
}

/// Insert `node` to the left of the widget's child in LTR contexts or to the right in RTL contexts, outside of the child layout.
///
/// This property disables inline layout for the widget. See [`child_out_insert`] for more details.
///
/// [`child_out_insert`]: fn@child_out_insert
#[property(CHILD_LAYOUT - 1, default(UiNode::nil()), widget_impl(Container))]
pub fn child_out_start(child: impl IntoUiNode, node: impl IntoUiNode) -> UiNode {
    child_out_insert(child, ChildInsert::Start, node)
}

/// Insert `node` to the right of the widget's child in LTR contexts or to the right of the widget's child in RTL contexts, outside of the child layout.
///
/// This property disables inline layout for the widget. See [`child_out_insert`] for more details.
///
/// [`child_out_insert`]: fn@child_out_insert
#[property(CHILD_LAYOUT - 1, default(UiNode::nil()), widget_impl(Container))]
pub fn child_out_end(child: impl IntoUiNode, node: impl IntoUiNode) -> UiNode {
    child_out_insert(child, ChildInsert::End, node)
}

/// Insert `node` over the widget's child, not affected by child layout.
///
/// This property disables inline layout for the widget. See [`child_out_insert`] for more details.
///
/// [`child_out_insert`]: fn@child_out_insert
#[property(CHILD_LAYOUT - 1, default(UiNode::nil()), widget_impl(Container))]
pub fn child_out_over(child: impl IntoUiNode, node: impl IntoUiNode) -> UiNode {
    child_out_insert(child, ChildInsert::Over, node)
}

/// Insert `node` under the widget's child, not affected by child layout.
///
/// This property disables inline layout for the widget. See [`child_out_insert`] for more details.
///
/// [`child_out_insert`]: fn@child_out_insert
#[property(CHILD_LAYOUT - 1, default(UiNode::nil()), widget_impl(Container))]
pub fn child_out_under(child: impl IntoUiNode, node: impl IntoUiNode) -> UiNode {
    child_out_insert(child, ChildInsert::Under, node)
}

fn child_insert_node(child: UiNode, placement: Var<ChildInsert>, node: UiNode, init_spacing: fn(&mut Var<SideOffsets>)) -> UiNode {
    let placement = placement.into_var();
    let mut spacing = const_var(SideOffsets::zero());
    let offset_key = FrameValueKey::new_unique();
    let mut offset_child = 0;
    let mut offset = PxVector::zero();

    match_node(ui_vec![child, node], move |children, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&placement);
            init_spacing(&mut spacing);
        }
        UiNodeOp::Deinit => {
            spacing = const_var(SideOffsets::zero());
        }
        UiNodeOp::Measure { wm, desired_size } => {
            children.delegated();

            let c = LAYOUT.constraints();
            let placement = placement.get().resolve_direction(LAYOUT.direction());
            let mut spacing = placement.spacing(&spacing);
            *desired_size = if placement.is_x_axis() {
                let insert_size = children.node().with_child(1, |n| {
                    LAYOUT.with_constraints(c.with_new_min(Px(0), Px(0)).with_fill_x(false), || wm.measure_block(n))
                });
                if insert_size.width == 0 {
                    spacing = Px(0);
                }
                let child_size = children.node().with_child(0, |n| {
                    LAYOUT.with_constraints(c.with_less_x(insert_size.width + spacing), || wm.measure_block(n))
                });

                PxSize::new(
                    insert_size.width + spacing + child_size.width,
                    insert_size.height.max(child_size.height),
                )
            } else if placement.is_y_axis() {
                let insert_size = children.node().with_child(1, |n| {
                    LAYOUT.with_constraints(c.with_new_min(Px(0), Px(0)).with_fill_y(false), || wm.measure_block(n))
                });
                if insert_size.height == 0 {
                    spacing = Px(0);
                }
                let child_size = children.node().with_child(0, |n| {
                    LAYOUT.with_constraints(c.with_less_y(insert_size.height + spacing), || wm.measure_block(n))
                });
                if child_size.height == 0 {
                    spacing = Px(0);
                }
                PxSize::new(
                    insert_size.width.max(child_size.width),
                    insert_size.height + spacing + child_size.height,
                )
            } else {
                children.node().with_child(0, |n| wm.measure_block(n))
            };
        }
        UiNodeOp::Layout { wl, final_size } => {
            children.delegated();
            wl.require_child_ref_frame();

            let placement = placement.get().resolve_direction(LAYOUT.direction());
            let spacing = placement.spacing(&spacing);

            let c = LAYOUT.constraints();

            *final_size = match placement {
                ChildInsert::Left | ChildInsert::Right => {
                    let mut constraints_y = LAYOUT.constraints().y;
                    if constraints_y.fill_or_exact().is_none() {
                        // measure to find fill height
                        let mut wm = wl.to_measure(None);
                        let wm = &mut wm;
                        let mut spacing = spacing;

                        let insert_size = children.node().with_child(1, |n| {
                            LAYOUT.with_constraints(c.with_new_min(Px(0), Px(0)).with_fill_x(false), || n.measure(wm))
                        });
                        if insert_size.width == 0 {
                            spacing = Px(0);
                        }
                        let child_size = children.node().with_child(0, |n| {
                            LAYOUT.with_constraints(c.with_less_x(insert_size.width + spacing), || n.measure(wm))
                        });

                        constraints_y = constraints_y.with_fill(true).with_max(child_size.height.max(insert_size.height));
                    }

                    let mut spacing = spacing;
                    let insert_size = children.node().with_child(1, |n| {
                        LAYOUT.with_constraints(
                            {
                                let mut c = c;
                                c.y = constraints_y;
                                c.with_new_min(Px(0), Px(0)).with_fill_x(false)
                            },
                            || n.layout(wl),
                        )
                    });
                    if insert_size.width == 0 {
                        spacing = Px(0);
                    }
                    let child_size = children.node().with_child(0, |n| {
                        LAYOUT.with_constraints(
                            {
                                let mut c = c;
                                c.y = constraints_y;
                                c.with_less_x(insert_size.width + spacing)
                            },
                            || n.layout(wl),
                        )
                    });
                    if child_size.width == 0 {
                        spacing = Px(0);
                    }

                    // position
                    let (child, o) = match placement {
                        ChildInsert::Left => (0, insert_size.width + spacing),
                        ChildInsert::Right => (1, child_size.width + spacing),
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
                ChildInsert::Top | ChildInsert::Bottom => {
                    let mut constraints_x = c.x;
                    if constraints_x.fill_or_exact().is_none() {
                        // measure fill width

                        let mut wm = wl.to_measure(None);
                        let wm = &mut wm;
                        let mut spacing = spacing;

                        let insert_size = children.node().with_child(1, |n| {
                            LAYOUT.with_constraints(c.with_new_min(Px(0), Px(0)).with_fill_y(false), || n.measure(wm))
                        });
                        if insert_size.height == 0 {
                            spacing = Px(0);
                        }
                        let child_size = children.node().with_child(0, |n| {
                            LAYOUT.with_constraints(c.with_less_y(insert_size.height + spacing), || n.measure(wm))
                        });

                        constraints_x = constraints_x.with_fill(true).with_max(child_size.width.max(insert_size.width));
                    }

                    let mut spacing = spacing;
                    let insert_size = children.node().with_child(1, |n| {
                        LAYOUT.with_constraints(
                            {
                                let mut c = c;
                                c.x = constraints_x;
                                c.with_new_min(Px(0), Px(0)).with_fill_y(false)
                            },
                            || n.layout(wl),
                        )
                    });
                    if insert_size.height == 0 {
                        spacing = Px(0);
                    }
                    let child_size = children.node().with_child(0, |n| {
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
                    let (child, o) = match placement {
                        ChildInsert::Top => (0, insert_size.height + spacing),
                        ChildInsert::Bottom => (1, child_size.height + spacing),
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
                ChildInsert::Over | ChildInsert::Under => {
                    let child_size = children.node().with_child(0, |n| n.layout(wl));
                    let insert_size = children.node().with_child(1, |n| n.layout(wl));
                    child_size.max(insert_size)
                }
                ChildInsert::Start | ChildInsert::End => unreachable!(), // already resolved
            };
        }
        UiNodeOp::Render { frame } => match placement.get() {
            ChildInsert::Over => children.render(frame),
            ChildInsert::Under => {
                children.delegated();
                children.node().with_child(1, |n| n.render(frame));
                children.node().with_child(0, |n| n.render(frame));
            }
            _ => {
                children.delegated();
                children.node().for_each_child(|i, child| {
                    if i as u8 == offset_child {
                        frame.push_reference_frame(offset_key.into(), offset_key.bind(offset.into(), false), true, true, |frame| {
                            child.render(frame);
                        });
                    } else {
                        child.render(frame);
                    }
                })
            }
        },
        UiNodeOp::RenderUpdate { update } => match placement.get() {
            ChildInsert::Over => children.render_update(update),
            ChildInsert::Under => {
                children.delegated();
                children.node().with_child(1, |n| n.render_update(update));
                children.node().with_child(0, |n| n.render_update(update));
            }
            _ => {
                children.delegated();
                children.node().for_each_child(|i, child| {
                    if i as u8 == offset_child {
                        update.with_transform(offset_key.update(offset.into(), false), true, |update| {
                            child.render_update(update);
                        });
                    } else {
                        child.render_update(update);
                    }
                });
            }
        },
        _ => {}
    })
}
