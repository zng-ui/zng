use crate::prelude::new_widget::*;

mod direction;
use direction::*;

#[widget($crate::widgets::layouts::stack)]
pub mod stack {
    pub use super::direction::StackDirection;
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
        let constrains = ctx.constrains();
        let direction = self.direction.get();
        let direction_vector = direction.vector(ctx.direction());
        let children_align = self.children_align.get();
        let child_align = direction.filter_align(children_align);

        // !!: review this
        let spacing = self.spacing.get();
        let mut spacing = match (direction_vector.x == 0, direction_vector.y == 0) {
            (false, false) => PxVector::new(spacing.layout(ctx.for_x(), |_| Px(0)), spacing.layout(ctx.for_y(), |_| Px(0))),
            (true, false) => PxVector::new(Px(0), spacing.layout(ctx.for_y(), |_| Px(0))),
            (false, true) => PxVector::new(spacing.layout(ctx.for_x(), |_| Px(0)), Px(0)),
            (true, true) => PxVector::zero(),
        };
        if direction_vector.x < 0 {
            spacing.x = -spacing.x;
        }
        if direction_vector.y < 0 {
            spacing.y = -spacing.y;
        }

        // need measure when children fill, but the panel does not.
        let mut need_measure = false;
        let mut max_size = PxSize::zero();
        let mut measure_constrains = constrains;
        match (constrains.x.fill_or_exact(), constrains.y.fill_or_exact()) {
            (None, None) => {
                need_measure = child_align.is_fill_x() || child_align.is_fill_y();
                if !need_measure {
                    max_size = constrains.max_size().unwrap_or_else(|| PxSize::new(Px::MAX, Px::MAX));
                }
            }
            (None, Some(h)) => {
                max_size.height = h;
                need_measure = child_align.is_fill_x();
                measure_constrains = constrains.with_fill_x(false);
            }
            (Some(w), None) => {
                max_size.width = w;
                need_measure = child_align.is_fill_y();
                measure_constrains = constrains.with_fill_y(false);
            }
            (Some(w), Some(h)) => max_size = PxSize::new(w, h),
        }

        // find largest child, the others will fill to its size.
        if need_measure {
            ctx.as_measure().with_constrains(
                move |_| measure_constrains,
                |ctx| {
                    self.children.for_each(|_, c| {
                        let size = c.measure(ctx, &mut WidgetMeasure::new());
                        max_size = max_size.max(size);
                        true
                    });
                },
            );

            max_size = constrains.clamp_size(max_size);
        }

        // layout children, size, raw position + spacing only.
        let mut item_bounds = euclid::Box2D::zero();
        ctx.with_constrains(
            move |_| {
                constrains
                    .with_fill(child_align.is_fill_x(), child_align.is_fill_y())
                    .with_max_size(max_size)
            },
            |ctx| {
                let mut item_rect = PxRect::zero();
                let mut child_spacing = PxVector::zero();
                self.children.for_each_mut(|_, c| {
                    let size = c.layout(ctx, wl);
                    let offset = direction.layout(ctx, item_rect, size) + child_spacing;

                    wl.with_outer(c, false, |wl, _| wl.translate(offset));

                    item_rect.origin = offset.to_point();
                    item_rect.size = size;

                    let item_box = item_rect.to_box2d();
                    item_bounds.min = item_bounds.min.min(item_box.min);
                    item_bounds.max = item_bounds.max.max(item_box.max);
                    child_spacing = spacing;

                    true
                });
            },
        );

        // final position, align child inside item_bounds and item_bounds in the panel area.
        let child_align = child_align.xy(ctx.direction());
        let items_size = item_bounds.size();
        let panel_size = constrains.fill_size_or(item_bounds.size());
        let children_offset = -item_bounds.min.to_vector() + (panel_size - items_size).to_vector() * children_align.xy(ctx.direction());

        // !!: underline align?
        self.children.for_each_mut(|_, c| {
            let size = c.with_context(|ctx| ctx.widget_info.bounds.outer_size()).unwrap_or_default();
            let child_offset = (items_size - size).to_vector() * child_align;
            wl.with_outer(c, true, |wl, _| wl.translate(children_offset + child_offset));

            true
        });

        panel_size
    }
}
