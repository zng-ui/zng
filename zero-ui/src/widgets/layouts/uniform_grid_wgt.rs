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
///     items = widgets![
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

    properties! {
        /// Widget items.
        #[allowed_in_when = false]
        items(impl WidgetList) = widgets![];

        /// Number of columns.
        ///
        /// Set to zero (`0`) for auto.
        columns(impl IntoVar<u32>) = 0;
        /// Number of rows.
        ///
        /// Set to zero (`0`) for auto.
        rows(impl IntoVar<u32>) = 0;
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
        ///     items = widgets![
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
        first_column(impl IntoVar<u32>) = 0;

        /// Space in-between items.
        spacing(impl IntoVar<GridSpacing>) = 0.0;

        /// Spacing around the items grid, inside the border.
        padding;
    }

    /// New uniform grid layout.
    fn new_child(
        items: impl WidgetList,
        columns: impl IntoVar<u32>,
        rows: impl IntoVar<u32>,
        first_column: impl IntoVar<u32>,
        spacing: impl IntoVar<GridSpacing>,
    ) -> impl UiNode {
        let node = UniformGridNode {
            children: ZSortedWidgetList::new(items),

            columns: columns.into_var(),
            rows: rows.into_var(),
            first_column: first_column.into_var(),
            spacing: spacing.into_var(),
        };
        implicit_base::nodes::children_layout(node)
    }

    struct UniformGridNode<U, C, R, FC, S> {
        children: U,
        columns: C,
        rows: R,
        first_column: FC,
        spacing: S,
    }
    #[impl_ui_node(children)]
    impl<U, C, R, FC, S> UniformGridNode<U, C, R, FC, S>
    where
        U: WidgetList,
        C: Var<u32>,
        R: Var<u32>,
        FC: Var<u32>,
        S: Var<GridSpacing>,
    {
        /// (columns, rows, first_column)
        fn grid_len(&self, vars: &VarsRead, cells_count: usize) -> (i32, i32, i32) {
            let mut columns = self.columns.copy(vars) as i32;
            let mut rows = self.rows.copy(vars) as i32;
            let mut first_column = self.first_column.copy(vars) as i32;
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
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.vars(ctx)
                .var(&self.columns)
                .var(&self.rows)
                .var(&self.first_column)
                .var(&self.spacing);

            self.children.subscriptions_all(ctx, subs);
        }

        #[UiNode]
        fn update(&mut self, ctx: &mut WidgetContext) {
            let mut changed = false;
            self.children.update_all(ctx, &mut changed);

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
            self.children.measure_all(
                ctx,
                |_, _| {},
                |_, a| {
                    if a.size != PxSize::zero() {
                        count += 1;
                        cell_size = cell_size.max(a.size);
                    }
                },
            );

            if count == 0 {
                return constrains.min_size();
            }

            let (columns, rows, _) = self.grid_len(ctx.vars, count);

            let spacing = self.spacing.get(ctx.vars).layout(ctx.metrics, |_| PxGridSpacing::zero());

            let panel_size = PxSize::new(
                (cell_size.width + spacing.column) * Px(columns) - spacing.column,
                (cell_size.height + spacing.row) * Px(rows) - spacing.row,
            );

            constrains.fill_size_or(panel_size)
        }

        #[UiNode]
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let constrains = ctx.constrains();
            let spacing = self.spacing.get(ctx.vars).layout(ctx.metrics, |_| PxGridSpacing::zero());

            let final_panel_size;
            let final_cell_size;
            let final_columns;
            let final_first_column;

            if let Some(panel_size) = constrains.fill_or_exact() {
                // panel size not defined by cells

                final_panel_size = panel_size;

                let count = match self.children.count(|c| c.bounds_info.outer_size() != PxSize::zero()) {
                    0 => self.children.len(),
                    n => n,
                };

                let (columns, rows, first_column) = self.grid_len(ctx.vars, count);
                let cell_size = PxSize::new(
                    (panel_size.width + spacing.column) / Px(columns) - spacing.column,
                    (panel_size.height + spacing.row) / Px(rows) - spacing.row,
                );

                let mut actual_count = 0;
                ctx.with_constrains(
                    |_| PxConstrains2d::new_fill_size(cell_size),
                    |ctx| {
                        self.children.layout_all(
                            ctx,
                            wl,
                            |_, _, _| {},
                            |_, _, a| {
                                if a.size != PxSize::zero() {
                                    actual_count += 1;
                                }
                            },
                        );
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

                    let (columns, rows, first_column) = self.grid_len(ctx.vars, actual_count);
                    final_columns = columns;
                    final_first_column = first_column;
                    final_cell_size = PxSize::new(
                        (panel_size.width + spacing.column) / Px(columns) - spacing.column,
                        (panel_size.height + spacing.row) / Px(rows) - spacing.row,
                    );

                    if final_cell_size != cell_size {
                        ctx.with_constrains(
                            |_| PxConstrains2d::new_fill_size(final_cell_size),
                            |ctx| self.children.layout_all(ctx, wl, |_, _, _| {}, |_, _, _| {}),
                        );
                    }
                }
            } else {
                // panel size (partially) defined by cells.

                let mut count = 0;
                let mut cell_size = PxSize::zero();
                self.children.measure_all(
                    &mut ctx.as_measure(),
                    |_, _| {},
                    |_, a| {
                        if a.size != PxSize::zero() {
                            count += 1;
                            cell_size = cell_size.max(a.size);
                        }
                    },
                );

                if count == 0 {
                    return constrains.min_size();
                }

                let (columns, rows, first_column) = self.grid_len(ctx.vars, count);
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
                    |ctx| self.children.layout_all(ctx, wl, |_, _, _| {}, |_, _, _| {}),
                );
            }
            let mut cells = CellsIter::new(final_cell_size, final_columns, final_first_column, spacing);

            self.children.outer_all(wl, false, |wlt, a| {
                if a.size != PxSize::zero() {
                    if let Some(offset) = cells.next() {
                        wlt.translate(offset);
                    }
                }
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
/// let grid = uniform_grid(widgets![
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
pub fn uniform_grid(items: impl WidgetList) -> impl Widget {
    uniform_grid! { items; }
}
