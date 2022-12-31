use crate::prelude::new_widget::*;

/// Wrapping layout where children are layout next to the other, wrapping into
/// a new line or column once there is no more space.
#[widget($crate::widgets::layouts::wrap)]
pub mod wrap {
    use super::*;

    inherit!(widget_base::base);

    properties! {
        /// Widget items.
        pub widget_base::children;

        /// Space in-between items.
        pub spacing(impl IntoVar<GridSpacing>);

        pub children_align(impl IntoVar<Align>) = text::TEXT_ALIGN_VAR;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            let children = wgt.capture_ui_node_list_or_empty(property_id!(self::children));
            let spacing = wgt.capture_var_or_default(property_id!(self::spacing));
            let children_align = wgt.capture_var_or_default(property_id!(self::children_align));

            let node = WrapNode {
                children: ZSortingList::new(children),
                spacing,
                children_align,
            };
            let child = widget_base::nodes::children_layout(node);

            wgt.set_child(child);
        });
    }
}

#[ui_node(struct WrapNode {
    children: impl UiNodeList,
    #[var] spacing: impl Var<GridSpacing>,
    #[var] children_align: impl Var<Align>,
})]
impl UiNode for WrapNode {
    fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
        let mut any = false;
        self.children.update_all(ctx, updates, &mut any);

        if any || self.spacing.is_new(ctx) || self.children_align.is_new(ctx) {
            ctx.updates.layout();
        }
    }

    fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
        todo!()
    }

    fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        let constrains = ctx.constrains();
        let mut row_width = constrains.x.max().unwrap_or(Px::MAX);
        let mut leftover = row_width;
        let mut max_width = Px(0);
        let width_bounded = row_width < Px::MAX;

        self.children.for_each(|i, child| {
            let (inline, size) = ctx.as_measure().measure_inline(&mut WidgetMeasure::new(), leftover, child);
            max_width = max_width.max(size.width);

            if let Some(inline) = inline {
                if width_bounded {
                    if inline.last != size {
                        leftover = row_width;
                    }
                    leftover -= inline.last.width;
                    if leftover <= Px(0) {
                        leftover = row_width;
                    }
                }
            } else {
                todo!();
            }
            true
        });
        if row_width == Px::MAX {
            row_width = constrains.x.clamp(max_width);
        }

        self.children.for_each(|i, child| {
            child.layout(ctx, wl);
            true
        });
        PxSize::zero()
    }
}
