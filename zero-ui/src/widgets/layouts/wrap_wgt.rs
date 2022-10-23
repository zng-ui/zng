use crate::prelude::new_widget::*;

/// Wrapping layout where children are layout next to the other, wrapping into
/// a new line or column once there is no more space.
#[widget($crate::widgets::layouts::wrap)]
pub mod wrap {
    use super::*;

    inherit!(widget_base::base);

    properties! {
        /// Widget items.
        #[allowed_in_when = false]
        items(impl UiNodeList) = ui_list![];

        /// Space in-between items.
        spacing(impl IntoVar<GridSpacing>) = 0.0;
    }

    fn new_child(items: impl UiNodeList, spacing: impl IntoVar<GridSpacing>) -> impl UiNode {
        let node = WrapNode {
            children: ZSortedWidgetList::new(items),
            spacing: spacing.into_var(),
        };
        widget_base::nodes::children_layout(node)
    }

    #[ui_node(struct WrapNode {
        children: impl UiNodeList,
        #[var] spacing: impl Var<GridSpacing>,
    })]
    impl UiNode for WrapNode {
        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let constrains = ctx.constrains();

            if let Some(size) = constrains.fill_or_exact() {
                return size;
            }

            let mut panel_size = PxSize::zero();
            let spacing = self.spacing.get().layout(ctx.metrics, |_| PxGridSpacing::zero());
            let max_width = constrains.x.max().unwrap_or(Px::MAX);
            let mut row_size = PxSize::zero();

            ctx.with_constrains(
                |c| c.with_fill(false, false).with_new_min(Px(0), Px(0)),
                |ctx| {
                    self.children.for_each(|_, n| {
                        let s = n.measure(ctx);
                        if s == PxSize::zero() {
                            return true;
                        }

                        let width = row_size.width + s.width + spacing.column;
                        if width < max_width {
                            row_size.width = width;
                            row_size.height = row_size.height.max(s.height);
                        } else {
                            if row_size.width > Px(0) {
                                row_size.width -= spacing.column;
                            }
                            panel_size.width = panel_size.width.max(row_size.width);
                            panel_size.height += row_size.height + spacing.row;
                            row_size = PxSize::zero();
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
                panel_size.height += row_size.height;
            } else if panel_size.height > Px(0) {
                panel_size.height -= spacing.row;
            }

            constrains.fill_size_or(panel_size)
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let constrains = ctx.constrains();

            let mut panel_size = PxSize::zero();
            let spacing = self.spacing.get().layout(ctx.metrics, |_| PxGridSpacing::zero());
            let max_width = constrains.x.max().unwrap_or(Px::MAX);
            let mut row_size = PxSize::zero();

            ctx.with_constrains(
                |c| c.with_fill(false, false).with_new_min(Px(0), Px(0)),
                |ctx| {
                    self.children.for_each_mut(|_, n| {
                        let s = n.layout(ctx, wl);
                        if s == PxSize::zero() {
                            return true;
                        }

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

                        true
                    });
                },
            );

            if row_size.height > Px(0) {
                if row_size.width > Px(0) {
                    row_size.width -= spacing.column;
                }
                panel_size.width = panel_size.width.max(row_size.width);
                panel_size.height += row_size.height;
            } else if panel_size.height > Px(0) {
                panel_size.height -= spacing.row;
            }

            constrains.fill_size_or(panel_size)
        }
    }
}
