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
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            let children = wgt.capture_ui_node_list_or_empty(property_id!(self::children));
            let spacing = wgt.capture_var_or_default(property_id!(self::spacing));

            let node = WrapNode {
                children: ZSortingList::new(children),
                spacing: spacing.into_var(),
            };
            let child = widget_base::nodes::children_layout(node);

            wgt.set_child(child);
        });
    }

    #[ui_node(struct WrapNode {
        children: impl UiNodeList,
        #[var] spacing: impl Var<GridSpacing>,
    })]
    impl UiNode for WrapNode {
        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            let constrains = ctx.constrains();

            if let Some(size) = constrains.fill_or_exact() {
                return size;
            }

            let mut panel_size = PxSize::zero();
            let spacing = self.spacing.get().layout(ctx.metrics, |_| PxGridSpacing::zero());
            let max_width = constrains.x.max().unwrap_or(Px::MAX);
            let mut row_size = PxSize::zero();
            let mut last_child_inlined = false;

            if wm.is_inline() {
                row_size = ctx.metrics.inline_advance();
            }

            ctx.with_constrains(
                |c| c.with_fill(false, false).with_new_min(Px(0), Px(0)),
                |ctx| {
                    self.children.for_each(|_, n| {
                        let (inline, s) = ctx.with_inline(wm, row_size, |ctx, wm| n.measure(ctx, wm));
                        if s == PxSize::zero() {
                            return true;
                        }

                        if let Some(inline) = inline {
                            if panel_size.height == Px(0) {
                                panel_size.height = inline.first_row.y;
                            }

                            panel_size.width = panel_size.width.max(inline.bounds.width);
                            panel_size.height += inline.bounds.height - inline.first_row.y + spacing.row - inline.last_row_spacing;

                            row_size = inline.last_rect().size;
                            if row_size.width > Px(0) {
                                row_size.width += spacing.column;
                            }

                            last_child_inlined = true;
                        } else {
                            let new_width = row_size.width + s.width;
                            if new_width <= max_width {
                                row_size.width = new_width + spacing.column;
                                row_size.height = row_size.height.max(s.height);
                            } else {
                                if row_size.width > Px(0) {
                                    row_size.width -= spacing.column;
                                }
                                panel_size.width = panel_size.width.max(row_size.width);
                                panel_size.height += row_size.height + spacing.row;

                                row_size = s;
                                row_size.width += spacing.column;
                            }

                            last_child_inlined = false;
                        }

                        true
                    });
                },
            );

            if row_size.height > Px(0) {
                if row_size.width > Px(0) {
                    row_size.width -= spacing.column;
                }
                panel_size.width = panel_size.width.max(row_size.width);

                if !last_child_inlined {
                    panel_size.height += row_size.height;
                }
            } else if panel_size.height > Px(0) {
                panel_size.height -= spacing.row;
            }

            let final_size = constrains.fill_size_or(panel_size);

            if let Some(inline) = wm.inline() {
                if final_size != panel_size {
                    todo!()
                }

                inline.bounds = final_size;
                inline.first_row = ctx.metrics.inline_advance().to_vector().to_point();

                if final_size.width <= row_size.width {
                    inline.last_row = PxPoint::new(Px(0), final_size.height);
                } else {
                    inline.last_row = PxPoint::new(row_size.width, final_size.height - row_size.height);
                    inline.last_row_spacing = spacing.column;
                }
            }

            final_size
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let constrains = ctx.constrains();

            let mut panel_size = PxSize::zero();
            let spacing = self.spacing.get().layout(ctx.metrics, |_| PxGridSpacing::zero());
            let max_width = constrains.x.max().unwrap_or(Px::MAX);
            let mut row_size = PxSize::zero();

            if wl.is_inline() {
                row_size = ctx.metrics.inline_advance();
            }
            let mut last_child_inlined = false;

            ctx.with_constrains(
                |c| c.with_fill(false, false).with_new_min(Px(0), Px(0)),
                |ctx| {
                    self.children.for_each_mut(|_, n| {
                        let (inline, s) = ctx.with_inline(wl, row_size, |ctx, wl| n.layout(ctx, wl));
                        if s == PxSize::zero() {
                            return true;
                        }

                        if let Some(inline) = inline {
                            if panel_size.height == Px(0) {
                                panel_size.height = inline.first_row.y;
                            }

                            // inline item
                            wl.translate(PxVector::new(row_size.width, panel_size.height) - inline.first_row.to_vector());

                            panel_size.width = panel_size.width.max(inline.bounds.width);
                            panel_size.height += inline.bounds.height - inline.first_row.y + spacing.row - inline.last_row_spacing;

                            row_size = inline.last_rect().size;
                            if row_size.width > Px(0) {
                                row_size.width += spacing.column;
                            }

                            last_child_inlined = true;
                        } else {
                            // *inline-block* item
                            let new_width = row_size.width + s.width;
                            if new_width <= max_width {
                                wl.translate(PxVector::new(row_size.width, panel_size.height));

                                row_size.width = new_width + spacing.column;
                                row_size.height = row_size.height.max(s.height);
                            } else {
                                if row_size.width > Px(0) {
                                    row_size.width -= spacing.column;
                                }
                                panel_size.width = panel_size.width.max(row_size.width);
                                panel_size.height += row_size.height + spacing.row;

                                row_size = s;
                                row_size.width += spacing.column;

                                wl.translate(PxVector::new(Px(0), panel_size.height));
                            }

                            last_child_inlined = false;
                        }
                        true
                    });
                },
            );

            if row_size.height > Px(0) {
                if row_size.width > Px(0) {
                    row_size.width -= spacing.column;
                }
                panel_size.width = panel_size.width.max(row_size.width);

                if !last_child_inlined {
                    panel_size.height += row_size.height;
                }
            } else if panel_size.height > Px(0) {
                panel_size.height -= spacing.row;
            }

            let final_size = constrains.fill_size_or(panel_size);

            if let Some(inline) = wl.inline() {
                inline.bounds = final_size;
                inline.first_row = ctx.metrics.inline_advance().to_vector().to_point();

                if final_size.width <= row_size.width {
                    inline.last_row = PxPoint::new(Px(0), final_size.height);
                } else {
                    inline.last_row = PxPoint::new(row_size.width, final_size.height - row_size.height);
                    inline.last_row_spacing = spacing.column;
                }
            }

            final_size
        }
    }
}
