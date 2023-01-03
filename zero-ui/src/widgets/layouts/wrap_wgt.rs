use crate::prelude::new_widget::*;

use std::mem;
use task::parking_lot::Mutex;

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
                row_joiners: Mutex::new(vec![]),
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
    row_joiners: Mutex<Vec<RowJoinerInfo>>,
})]
impl WrapNode {
    #[UiNode]
    fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
        let mut any = false;
        self.children.update_all(ctx, updates, &mut any);

        if any || self.spacing.is_new(ctx) || self.children_align.is_new(ctx) {
            ctx.updates.layout();
        }
    }

    #[UiNode]
    fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
        let inline_constrains = ctx.inline_constrains().map(|c| c.measure());
        let constrains = ctx.constrains();

        if let (None, Some(known)) = (inline_constrains, constrains.fill_or_exact()) {
            // block, known size
            return known;
        }
        if self.children.is_empty() {
            return if inline_constrains.is_some() {
                if let Some(inline) = wm.inline() {
                    *inline = WidgetInlineMeasure::default();
                }
                PxSize::zero()
            } else {
                constrains.min_size()
            };
        }

        let (max_row_width, panel_height) = self.measure_row_joiners(ctx, wm);
        let panel_width = if let Some(width) = constrains.x.fill_or_exact() {
            // constrains requests a width
            width
        } else {
            // our width, or min allowed
            constrains.x.clamp(max_row_width)
        };

        if let Some(inline) = wm.inline() {
            let row_joiners = self.row_joiners.lock();
            if let Some(first) = row_joiners.first() {
                inline.first = first.size;
            }
            if let Some(last) = row_joiners.last() {
                inline.last = last.size;
            }
        }

        constrains.clamp_size(PxSize::new(panel_width, panel_height))
    }

    #[UiNode]
    fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        let inline_constrains = ctx.inline_constrains().map(|c| c.layout());
        let constrains = ctx.constrains();
        if self.children.is_empty() {
            return if inline_constrains.is_some() {
                if let Some(inline) = wl.inline() {
                    inline.rows.clear();
                }
                PxSize::zero()
            } else {
                // block
                constrains.fill_or_exact().unwrap_or_default()
            };
        }

        let (max_row_width, _s) = self.measure_row_joiners(&mut ctx.as_measure(), &mut WidgetMeasure::new());
        let panel_width = if let Some(width) = constrains.x.fill_or_exact() {
            // constrains requests a width
            width
        } else {
            // our width, or min allowed
            constrains.x.clamp(max_row_width)
        };
        let mut panel_height = Px(0);

        let mut next_row = 0;
        let mut row_offset = PxVector::zero();
        let mut row_size = PxSize::zero();
        let mut row_end = 0;
        let child_align = self.children_align.get();
        let child_align_x = child_align.x(ctx.direction());
        let child_align_y = child_align.y();
        // !!: TODO baseline align
        let child_constrains = constrains
            .with_fill(child_align.is_fill_x(), false)
            .with_new_min(Px(0), Px(0))
            .with_max_x(panel_width);

        let row_joiners = &*self.row_joiners.get_mut();

        self.children.for_each_mut(|i, child| {
            if i == row_end && next_row < row_joiners.len() {
                // panel wrap
                panel_height += row_size.height;
                row_offset.y += row_size.height;
                row_size = row_joiners[next_row].size;
                row_offset.x = (panel_width - row_size.width) * child_align_x;

                next_row += 1;
                if next_row < row_joiners.len() {
                    row_end = row_joiners[next_row].first_child;
                } else {
                    row_end = usize::MAX;
                }
            }

            if let Some((Some(inline), size)) =
                child.with_context(|ctx| (ctx.widget_info.bounds.inline_measure(), ctx.widget_info.bounds.outer_size()))
            {
                if inline.last != size {
                    // child wrap
                    let first_row = PxRect::new(
                        PxPoint::new(row_offset.x, row_offset.y + (row_size.height - inline.first.height) * child_align_y),
                        inline.first,
                    );
                    let mid_clear = row_size.height - first_row.size.height;

                    // !!: use the measured overall child size to calculate offset?
                    let last_row = if let Some(nr) = <[_]>::get(row_joiners, next_row) {
                        row_offset.y += row_size.height; // !!: what about the mid-rows?
                        row_size = nr.size;
                        row_offset.x = (panel_width - nr.size.height) * child_align_x;

                        next_row += 1;
                        if next_row < row_joiners.len() {
                            row_end = row_joiners[next_row].first_child;
                        } else {
                            row_end = usize::MAX;
                        }

                        let mut r = PxRect::new(row_offset.to_point(), row_size);
                        r.origin.y += (row_size.height - r.size.height) * child_align_y;
                        row_offset.x += r.size.width;

                        r
                    } else {
                        // last panel row (not a joiner)
                        let mut r = PxRect::from_size(inline.last);
                        if child_align.is_fill_x() {
                            r.size.width = panel_width;
                        }
                        r.origin.y = size.height - r.size.height;
                        r.origin.x = (panel_width - r.size.width) * child_align_x;
                        r
                    };

                    let size = ctx.with_inline(first_row, mid_clear, last_row, |ctx| child.layout(ctx, wl));

                    panel_height += size.height - first_row.size.height - mid_clear;
                } else {
                    // child inline, but no wrap

                    let rect = PxRect::new(
                        PxPoint::new(row_offset.x, row_offset.y + (row_size.height - inline.first.height) * child_align_y),
                        inline.first,
                    );

                    let size = ctx.with_inline(rect, Px(0), rect, |ctx| child.layout(ctx, wl));
                }
            } else {
                // layout inline-block
                let mut constrains = child_constrains;

                if child_align.is_fill_y() {
                    constrains.y = constrains.y.with_fill(true).with_max(row_size.height);
                }

                let size = ctx.with_constrains(|_| constrains, |ctx| child.layout(ctx, wl));

                let mut offset = row_offset;
                offset.y += (row_size.height - size.height) * child_align_y;
                wl.with_outer(child, false, |wl, _| {
                    wl.translate(row_offset);
                });

                row_offset.x += size.width;
            }

            true
        });
        panel_height += row_size.height;

        let panel_size = constrains.clamp_size(PxSize::new(panel_width, panel_height));

        panel_size
    }

    /// Updates the `self.row_joiners` and returns the `(max_row_width, panel_height)`.
    fn measure_row_joiners(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> (Px, Px) {
        let constrains = ctx.constrains();

        let mut row_joiners = self.row_joiners.lock();
        let row_joiners = &mut *row_joiners;

        let max_allowed_width = constrains.x.max().unwrap_or(Px::MAX);
        row_joiners.clear();
        let mut current_row = RowJoinerInfo::default();
        let mut max_row_width = Px(0);
        let mut panel_height = Px(0);

        // measure children to find all "joiner" rows and their size.
        self.children.for_each(|i, child| {
            let leftover = if max_allowed_width == Px::MAX {
                // unbounded
                Px::MAX
            } else {
                // bounded
                max_allowed_width - current_row.size.width
            };

            let (inline, size) = ctx.measure_inline(leftover, child);

            let measured_row_size = if let Some(inline) = inline { inline.first } else { size };

            // add to current row, or wrap into new.
            let row_width = current_row.size.width + measured_row_size.width;
            if row_width < max_allowed_width {
                panel_height += current_row.size.height;
                current_row.size.width = row_width;
                current_row.size.height = current_row.size.height.max(measured_row_size.height);
            } else {
                max_row_width = max_row_width.max(current_row.size.width);
                row_joiners.push(mem::replace(
                    &mut current_row,
                    RowJoinerInfo {
                        size: measured_row_size,
                        first_child: i,
                    },
                ));
            }

            if let Some(inline) = inline {
                if inline.last != size {
                    // child wrap.
                    panel_height += current_row.size.height;
                    max_row_width = max_row_width.max(current_row.size.width);
                    row_joiners.push(mem::replace(
                        &mut current_row,
                        RowJoinerInfo {
                            size: inline.last,
                            first_child: i,
                        },
                    ));
                }
            }

            true
        });
        max_row_width = max_row_width.max(current_row.size.width);
        panel_height += current_row.size.height;
        row_joiners.push(current_row);

        (max_row_width, panel_height)
    }
}

/// Info about a row that contains more then one widget.
#[derive(Default, Debug)]
struct RowJoinerInfo {
    size: PxSize,
    first_child: usize,
}
