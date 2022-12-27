use std::mem;

use crate::prelude::new_widget::*;

#[widget($crate::widgets::layouts::stack)]
pub mod stack {
    pub use super::StackDirection;
    use super::*;

    inherit!(widget_base::base);

    properties! {
        /// Widget items.
        pub widget_base::children;

        /// Stack direction.
        pub direction(impl IntoVar<StackDirection>);

        /// Space in-between items.
        pub spacing(impl IntoVar<Length>);

        /// Spacing around the items stack, inside the border.
        pub crate::properties::padding;

        /// Items alignment.
        ///
        /// The default is [`FILL`].
        ///
        /// [`FILL`]: Align::FILL
        pub children_align(impl IntoVar<Align>) = Align::FILL;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            let children = wgt.capture_ui_node_list_or_empty(property_id!(self::children));
            let spacing = wgt.capture_var_or_default(property_id!(self::spacing));
            let direction = wgt.capture_var_or_default(property_id!(self::direction));
            let children_align = wgt.capture_var_or_else(property_id!(self::children_align), || Align::FILL);

            let node = StackNode {
                children: ZSortingList::new(children),
                direction,
                spacing,
                children_align,
            };
            let child = widget_base::nodes::children_layout(node);

            wgt.set_child(child);
        });
    }
}

#[ui_node(struct StackNode {
    children: impl UiNodeList,

    #[var] direction: impl Var<StackDirection>,
    #[var] spacing: impl Var<Length>,
    #[var] children_align: impl Var<Align>,
})]
impl UiNode for StackNode {
    fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
        let mut changed = false;
        self.children.update_all(ctx, updates, &mut changed);

        if changed || self.direction.is_new(ctx) || self.spacing.is_new(ctx) || self.children_align.is_new(ctx) {
            ctx.updates.layout_render();
        }
    }

    fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
        let constrains = ctx.constrains();
        if let Some(known) = constrains.fill_or_exact() {
            return known;
        }

        todo! {}
    }

    fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        let mut prev_rect = PxRect::zero();
        self.children.for_each_mut(|i, c| {
            let direction = self.direction.get().layout(ctx, prev_rect, todo!());
            true
        });
        let spacing = self.spacing.get().layout(todo!(), |_| Px(0));
    }
}

/// Defines a placement point in the previous item and the origin point of the next.
///
/// # Alignment & Spacing
///
/// The direction type can express non-fill alignment and spacing by it self, but prefer using the [`stack::children_align`] and
/// [`stack::spacing`] properties as they are more readable and include fill alignment. The [`stack!`] widget implements alignment
/// along the axis that does not change, so if the computed layout vector is zero in a dimension the items can fill in that dimension.
///
/// [`stack::children_align`]: fn@stack::children_align
/// [`stack::spacing`]: fn@stack::spacing
/// [`stack!`]: mod@stack
///
/// # Examples
///
/// TODO, use the ASCII drawings?
#[derive(Debug, Default, Clone)]
pub struct StackDirection {
    /// Point on the previous item where the next item is placed.
    pub place: Point,
    /// Point on the next item that is offset to match `place`.
    pub origin: Point,

    /// If `place.x` and `origin.x` are swapped in [`LayoutDirection::RTL`] contexts.
    pub is_rtl_aware: bool,
}
impl StackDirection {
    /// `((100.pct(), 0), (0, 0))`, items are placed in a row from left to right.
    ///
    /// Alignment works on the `y` direction because it is not affected.
    pub fn left_to_right() -> Self {
        Self {
            place: (100.pct(), 0).into(),
            origin: (0, 0).into(),
            is_rtl_aware: false,
        }
    }

    /// `((0, 0), (100.pct(), 0))`, items are placed in a row from right to left.
    ///
    /// Alignment works on the `y` direction because it is not affected.
    pub fn right_to_left() -> Self {
        Self {
            place: (0, 0).into(),
            origin: (100.pct(), 0).into(),
            is_rtl_aware: false,
        }
    }

    /// `((100.pct(), 0), (0, 0), true)`, items are placed in a row from left to right or from right to left in RTL contexts.
    ///
    /// In [`LayoutDirection::RTL`] contexts the `place.x` and `origin.x` values are swapped before they are computed.
    ///
    /// Alignment works on the `y` direction because it is not affected.
    pub fn start_to_end() -> Self {
        Self {
            place: (100.pct(), 0).into(),
            origin: (0, 0).into(),
            is_rtl_aware: false,
        }
    }

    /// `((0, 0), (100.pct(), 0)), true)`, items are placed in a row from right to left or from left to right in RTL contexts.
    ///
    /// In [`LayoutDirection::RTL`] contexts the `place.x` and `origin.x` values are swapped before they are computed.
    ///
    /// Alignment works on the `y` direction because it is not affected.
    pub fn end_to_start() -> Self {
        Self {
            place: (0, 0).into(),
            origin: (100.pct(), 0).into(),
            is_rtl_aware: false,
        }
    }

    /// `((0, 100.pct()), (0, 0))`, items are placed in a column from top to bottom.
    ///  
    /// Alignment works on the `x` direction because it is not affected.
    pub fn top_to_bottom() -> Self {
        Self {
            place: (0, 100.pct()).into(),
            origin: (0, 0).into(),
            is_rtl_aware: false,
        }
    }

    /// `(0, 0), (0, 100.pct())`, items are placed in a column from bottom to top.
    ///  
    /// Alignment works on the `x` direction because it is not affected.
    pub fn bottom_to_top() -> Self {
        Self {
            place: (0, 0).into(),
            origin: (0, 100.pct()).into(),
            is_rtl_aware: false,
        }
    }

    /// `(0, 0)`, items are just stacked in the Z order.
    ///
    /// Fill alignment works in both dimensions because they don't change.
    ///
    /// Note that items are always rendered in the order defined by the [`z_index`] property.
    ///
    /// [`z_index`]: fn@z_index
    pub fn none() -> Self {
        Self {
            place: Point::zero(),
            origin: Point::zero(),
            is_rtl_aware: false,
        }
    }

    /// Compute offset of the next item.
    pub fn layout(&self, ctx: &LayoutMetrics, prev_item: PxRect, next_item: PxSize) -> PxVector {
        if self.is_rtl_aware && ctx.direction().is_ltr() {
            let mut d = self.clone();
            mem::swap(&mut d.place.x, &mut d.origin.x);
            d.is_rtl_aware = false;
            return d.layout_resolved_rtl(ctx, prev_item, next_item);
        }

        self.layout_resolved_rtl(ctx, prev_item, next_item)
    }

    fn layout_resolved_rtl(&self, ctx: &LayoutMetrics, prev_item: PxRect, next_item: PxSize) -> PxVector {
        let place = {
            let ctx = ctx.clone().with_constrains(|c| c.with_exact_size(prev_item.size));
            self.place.layout(&ctx, |_| PxPoint::zero())
        };
        let origin = {
            let ctx = ctx.clone().with_constrains(|c| c.with_exact_size(next_item));
            self.origin.layout(&ctx, |_| PxPoint::zero())
        };
        prev_item.origin.to_vector() + place.to_vector() - origin.to_vector()
    }
}
