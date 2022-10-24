use crate::prelude::new_widget::*;

/// Grid layout where all cells are the same size.
///
/// # Z-Index
///
/// By default the widgets are layout without overlap, but you can use properties like [`transform`] to cause
/// a widget overlap, in this case the widget will be rendered above its previous sibling and below its next sibling,
/// you can change this by setting the [`z_index`] property in the item widget.
///
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// let grid = uniform_grid!{
///     columns = 3;
///     rows = 2;
///     items = ui_list![
///         text("0,0"), text("1,0"), text("2,0"),
///         text("0,1"), text("1,1")
///     ];
/// };
/// ```
/// Produces a 3x2 grid:
///
/// ```text
/// 0,0 | 1.0 | 2,0
/// ----|-----|----
/// 0,1 | 1,1 |
/// ```
///
/// [`transform`]: fn@transform
/// [`z_index`]: fn@z_index
#[widget($crate::widgets::layouts::uniform_grid)]
pub mod uniform_grid {
    use super::*;

    inherit!(widget_base::base);

    properties! {
        /// Widget items.
        pub widget_base::children;

        /// Number of columns.
        ///
        /// Set to zero (`0`) for auto.
        pub columns(impl IntoVar<u32>);
        /// Number of rows.
        ///
        /// Set to zero (`0`) for auto.
        pub rows(impl IntoVar<u32>);
        /// Number of empty cells in the first row.
        ///
        /// Value clamped to `columns` if `columns` is not auto and `first_column >= columns`. If `rows` is not
        /// auto the `first_column` is clamped to the number of empty cells to the end, so that the last cell is filled.
        ///
        /// # Examples
        ///
        /// ```
        /// # use zero_ui::prelude::*;
        /// let grid = uniform_grid!{
        ///     columns = 3;
        ///     rows = 2;
        ///     first_column = 1;
        ///     items = ui_list![
        ///                      text("1,0"), text("2,0"),
        ///         text("0,1"), text("1,1"), text("2,1")
        ///     ];
        /// };
        /// ```
        /// Produces a 3x2 grid with an empty first cell:
        ///
        /// ```text
        ///     | 1,0 | 2,0
        /// ----|-----|----
        /// 0,1 | 1,1 | 2,1
        /// ```
        pub first_column(impl IntoVar<u32>);

        /// Space in-between items.
        pub spacing(impl IntoVar<GridSpacing>);

        /// Spacing around the items grid, inside the border.
        pub padding;
    }


    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            let children = wgt.capture_ui_node_list(property_id!(self.children));
            let columns = wgt.capture_var_or_default(property_id!(self.columns));
            let rows = wgt.capture_var_or_default(property_id!(self.rows));
            let first_column = wgt.capture_var_or_default(property_id!(self.first_column));
            let spacing = wgt.capture_var_or_default(property_id!(self.spacing));

            let node = UniformGridNode {
                children: ZSortedWidgetList::new(children),
    
                columns: columns.into_var(),
                rows: rows.into_var(),
                first_column: first_column.into_var(),
                spacing: spacing.into_var(),
            };
            let child = widget_base::nodes::children_layout(node);

            wgt.set_child(child);
        });
    }

    #[ui_node(struct UniformGridNode {
        children: impl UiNodeList,
        #[var] columns: impl Var<u32>,
        #[var] rows: impl Var<u32>,
        #[var] first_column: impl Var<u32>,
        #[var] spacing: impl Var<GridSpacing>,
    })]
    impl UniformGridNode {
        /// (columns, rows, first_column)
        fn grid_len(&self, cells_count: usize) -> (i32, i32, i32) {
            let mut columns = self.columns.get() as i32;
            let mut rows = self.rows.get() as i32;
            let mut first_column = self.first_column.get() as i32;
            let rows_is_bound = rows > 0;

            if columns == 0 {
                if rows == 0 {
                    // columns and rows are 0=AUTO, make a square
                    rows = (cells_count as f32).sqrt().ceil() as i32;
                    columns = rows;
                } else {
                    // only columns is 0=AUTO
                    columns = (cells_count as f32 / rows as f32).ceil() as i32;
                }
            } else if rows == 0 {
                // only rows is 0=AUTO
                rows = (cells_count as f32 / columns as f32).ceil() as i32;
            }

            if first_column > 0 {
                if first_column > columns {
                    first_column = columns;
                }

                let cells_count = cells_count as i32;
                let extra = (columns * rows) - cells_count;
                if rows_is_bound {
                    first_column = first_column.min(extra);
                } else if extra < first_column {
                    rows += 1;
                }
            }

            (columns, rows, first_column)
        }

        #[UiNode]
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            let mut changed = false;
            self.children.update_all(ctx, updates, &mut changed);

            if changed || self.columns.is_new(ctx) || self.rows.is_new(ctx) || self.first_column.is_new(ctx) || self.spacing.is_new(ctx) {
                ctx.updates.layout_and_render();
            }
        }

        #[UiNode]
        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let constrains = ctx.constrains();

            if let Some(size) = constrains.fill_or_exact() {
                return size;
            }

            let mut count = 0;
            let mut cell_size = PxSize::zero();
            self.children.for_each(|i, n| {
                let s = n.measure(ctx);
                if s != PxSize::zero() {
                    count += 1;
                    cell_size = cell_size.max(s);
                }
                true
            });

            if count == 0 {
                return constrains.min_size();
            }

            let (columns, rows, _) = self.grid_len(count);

            let spacing = self.spacing.get().layout(ctx.metrics, |_| PxGridSpacing::zero());

            let panel_size = PxSize::new(
                (cell_size.width + spacing.column) * Px(columns) - spacing.column,
                (cell_size.height + spacing.row) * Px(rows) - spacing.row,
            );

            constrains.fill_size_or(panel_size)
        }

        #[UiNode]
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let constrains = ctx.constrains();
            let spacing = self.spacing.get().layout(ctx.metrics, |_| PxGridSpacing::zero());

            let final_panel_size;
            let final_cell_size;
            let final_columns;
            let final_first_column;

            if let Some(panel_size) = constrains.fill_or_exact() {
                // panel size not defined by cells

                final_panel_size = panel_size;

                let mut count = 0;
                self.children.for_each(|_, n| {
                    let s = n.with_context(|ctx| ctx.widget_info.bounds.outer_size()).unwrap_or_default();
                    if s != PxSize::zero() {
                        count += 1;
                    }
                    true
                });
                let count = match count {
                    0 => self.children.len(),
                    n => n,
                };

                let (columns, rows, first_column) = self.grid_len(count);
                let cell_size = PxSize::new(
                    (panel_size.width + spacing.column) / Px(columns) - spacing.column,
                    (panel_size.height + spacing.row) / Px(rows) - spacing.row,
                );

                let mut actual_count = 0;
                ctx.with_constrains(
                    |_| PxConstrains2d::new_fill_size(cell_size),
                    |ctx| {
                        self.children.for_each_mut(|_, n| {
                            let s = n.layout(ctx, wl);
                            if s != PxSize::zero() {
                                actual_count += 1;
                            }
                            true
                        });
                    },
                );

                if actual_count == count {
                    final_cell_size = cell_size;
                    final_columns = columns;
                    final_first_column = first_column;
                } else {
                    // visibility of a child changed

                    if actual_count == 0 {
                        return constrains.min_size();
                    }

                    let (columns, rows, first_column) = self.grid_len(actual_count);
                    final_columns = columns;
                    final_first_column = first_column;
                    final_cell_size = PxSize::new(
                        (panel_size.width + spacing.column) / Px(columns) - spacing.column,
                        (panel_size.height + spacing.row) / Px(rows) - spacing.row,
                    );

                    if final_cell_size != cell_size {
                        ctx.with_constrains(
                            |_| PxConstrains2d::new_fill_size(final_cell_size),
                            |ctx| {
                                self.children.for_each_mut(|_, n| {
                                    n.layout(ctx, wl);
                                    true
                                });
                            },
                        );
                    }
                }
            } else {
                // panel size (partially) defined by cells.

                let mut count = 0;
                let mut cell_size = PxSize::zero();
                self.children.for_each(|_, n| {
                    let s = n.measure(&mut ctx.as_measure());
                    if s != PxSize::zero() {
                        count += 1;
                        cell_size = cell_size.max(s);
                    }
                    true
                });

                if count == 0 {
                    return constrains.min_size();
                }

                let (columns, rows, first_column) = self.grid_len(count);
                final_columns = columns;
                final_first_column = first_column;
                let panel_size = PxSize::new(
                    (cell_size.width + spacing.column) * Px(columns) - spacing.column,
                    (cell_size.height + spacing.row) * Px(rows) - spacing.row,
                );

                final_panel_size = constrains.fill_size_or(panel_size);

                if final_panel_size != panel_size {
                    cell_size = PxSize::new(
                        (final_panel_size.width + spacing.column) / Px(columns) - spacing.column,
                        (final_panel_size.height + spacing.row) / Px(rows) - spacing.row,
                    );
                }

                final_cell_size = cell_size;

                ctx.with_constrains(
                    |_| PxConstrains2d::new_fill_size(final_cell_size),
                    |ctx| {
                        self.children.for_each_mut(|_, n| {
                            n.layout(ctx, wl);
                            true
                        });
                    },
                );
            }
            let mut cells = CellsIter::new(final_cell_size, final_columns, final_first_column, spacing);

            self.children.for_each_mut(|_, n| {
                let s = n.with_context(|ctx| ctx.widget_info.bounds.outer_size()).unwrap_or_default();
                if s != PxSize::zero() {
                    if let Some(offset) = cells.next() {
                        wl.with_outer(n, false, |wlt, _| {
                            wlt.translate(offset);
                        });
                    }
                }
                true
            });

            final_panel_size
        }
    }
    #[derive(Clone, Default)]
    struct CellsIter {
        r: PxVector,
        advance: PxVector,
        max_width: Px,
    }
    impl CellsIter {
        pub fn new(cell_size: PxSize, columns: i32, first_column: i32, spacing: PxGridSpacing) -> Self {
            let advance = PxVector::new(cell_size.width + spacing.column, cell_size.height + spacing.row);
            CellsIter {
                r: PxVector::new(advance.x * (Px(first_column - 1)), Px(0)),
                max_width: advance.x * Px(columns),
                advance,
            }
        }
    }
    impl Iterator for CellsIter {
        type Item = PxVector;

        fn next(&mut self) -> Option<Self::Item> {
            self.r.x += self.advance.x;
            if self.r.x >= self.max_width {
                self.r.x = Px(0);
                self.r.y += self.advance.y;
            }
            Some(self.r)
        }
    }
}

/// Grid layout where all cells are the same size.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// let grid = uniform_grid(ui_list![
///     text("0,0"), text("1,0"),
///     text("0,1"), text("1,1"),
/// ]);
/// ```
/// Produces a 2x2 grid:
///
/// ```text
/// 0,0 | 1,0
/// ----|----
/// 0,1 | 1,1
/// ```
///
/// # `uniform_grid!`
///
/// This function is just a shortcut for [`uniform_grid!`](module@uniform_grid). Use the full widget
/// to better configure the grid widget.
pub fn uniform_grid(children: impl UiNodeList) -> impl UiNode {
    uniform_grid! { children; }
}
