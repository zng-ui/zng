use crate::prelude::new_widget::*;

use std::mem;

/// Wrapping inline layout.
#[widget($crate::widgets::layouts::wrap)]
pub mod wrap {
    use super::*;

    use crate::widgets::text::TEXT_ALIGN_VAR;

    inherit!(widget_base::base);

    properties! {
        /// Widget items.
        pub widget_base::children;

        /// Space in-between items.
        pub spacing(impl IntoVar<GridSpacing>);

        /// Children align.
        ///
        /// This property only defines the align for children inside this panel, but it is set
        /// to [`TEXT_ALIGN_VAR`] by default, so you can use the [`txt_align`] property if you want
        /// to affect all nested wrap and text widgets.
        ///
        /// [`TEXT_ALIGN_VAR`]: crate::widgets::text::TEXT_ALIGN_VAR
        /// [`txt_align`]: crate::widgets::text::txt_align
        pub children_align(impl IntoVar<Align>);

        /// Alignment of children in this widget and of nested wrap panels and texts.
        ///
        /// Note that this only sets the [`children_align`] if that property is not set (default) or is set to [`TEXT_ALIGN_VAR`].
        ///
        /// [`children_align`]: fn@children_align
        pub crate::widgets::text::txt_align;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            let children = wgt.capture_ui_node_list_or_empty(property_id!(self::children));
            let spacing = wgt.capture_var_or_default(property_id!(self::spacing));
            let children_align = wgt.capture_var_or_else(property_id!(self::children_align), || TEXT_ALIGN_VAR);

            let node = WrapNode {
                children: ZSortingList::new(children),
                spacing,
                children_align,
                row_joiners: vec![],
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
    row_joiners: Vec<RowJoinerInfo>,
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
        if self.children.is_empty() {
            return constrains.fill_or_exact().unwrap_or_default();
        }

        let max_width = constrains.x.max().unwrap_or(Px::MAX);
        let mut actual_max_width = Px(0);
        let mut leftover = max_width;

        self.row_joiners.clear();
        let mut current_row = RowJoinerInfo::default();

        // measure children to find all "joiner" rows and their size.
        self.children.for_each(|i, child| {
            let (inline, size) = ctx.measure_inline(leftover, child);
            actual_max_width = actual_max_width.max(size.width);

            if let Some(inline) = inline {
                // current row, or new if overflows.
                let row_width = current_row.size.width + size.width;
                if row_width < max_width {
                    current_row.size.width = row_width;
                    current_row.size.height = current_row.size.height.max(size.height);
                } else {
                    self.row_joiners
                        .push(mem::replace(&mut current_row, RowJoinerInfo { size, first_child: i }));
                }

                // leftover space, only defined if the panel width is bounded.
                if max_width < Px::MAX {
                    if inline.last != size {
                        leftover = max_width;
                    }
                    leftover -= inline.last.width;
                    if leftover <= Px(0) {
                        leftover = max_width;
                    }
                }
            } else {
                todo!("inline-block layout");
            }
            true
        });
        self.row_joiners.push(current_row);

        // panel width is defined by constrains or by the widest row.
        let max_width = if max_width == Px::MAX {
            constrains.x.clamp(actual_max_width)
        } else {
            max_width
        };

        let mut next_row = 0;
        let mut row_size = PxSize::zero();
        let mut row_end = 0;
        self.children.for_each(|i, child| {
            if i == row_end && next_row < self.row_joiners.len() {
                row_size = self.row_joiners[next_row].size;

                next_row += 1;
                if next_row < self.row_joiners.len() {
                    row_end = self.row_joiners[next_row].first_child;
                } else {
                    row_end = usize::MAX;
                }
            }

            // !!: TODO, use the row_joiners info to define the first and last row of each child.
            // !!: TODO, way to get the child measure constrains (it is stored in the bounds info, but not public)
            //          - just make it public? Only useful here that we know we just measured.
            child.layout(ctx, wl);
            true
        });

        todo!("constrained panel size")
    }
}

/// Info about a row that contains more then one widget.
#[derive(Default)]
struct RowJoinerInfo {
    size: PxSize,
    first_child: usize,
}
