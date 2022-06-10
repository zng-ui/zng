use crate::prelude::new_widget::*;

use std::mem;

/// Grid layout where all cells are the same size.
///
/// # Z-Index
///
/// By default the widgets are layout without overlap, but you can use properties like [`transform`] to cause
/// an widget overlap, in this case the widget will be rendered above its previous sibling and below its next sibling,
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
        /// Set to zero (`0`) for auto TODO.
        columns(impl IntoVar<u32>) = 0;
        /// Number of rows.
        rows(impl IntoVar<u32>) = 0;
        /// Number of empty cells in the first row.
        ///
        /// Value is ignored if is `>= columns`.
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
        /// (columns, rows)
        fn grid_len(&self, vars: &VarsRead, cells_count: usize) -> (i32, i32) {
            let mut columns = *self.columns.get(vars) as i32;
            let mut rows = *self.rows.get(vars) as i32;

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

            (columns, rows)
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
            todo!("!!: impl measure and implement layout to use measure on children")
        }
        #[UiNode]
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let spacing = self.spacing.get(ctx.vars).layout(ctx.metrics, |_| PxGridSpacing::zero());

            // we don't assign cells for collapsed widgets, if the widget has not changed
            // from the previous layout everything is done in one pass, otherwise we do
            // a second pass with the updated count.
            let mut count = self.children.count(|c| c.bounds_info.outer_size() != PxSize::zero());
            if count == 0 {
                count = self.children.len();
            }
            let mut count_final = false;

            let (mut columns, mut rows) = self.grid_len(ctx.vars, count);
            let mut cell_size = PxSize::zero();
            let mut size_final = false;

            let mut panel_size = PxSize::zero();

            let constrains = ctx.constrains();

            if let Some(size) = constrains.fill_or_exact() {
                if size.width == Px(0) || size.height == Px(0) {
                    return size;
                }

                panel_size = size;
                size_final = true;

                cell_size = PxSize::new(
                    (panel_size.width + spacing.column) / Px(columns) - spacing.column,
                    (panel_size.height + spacing.row) / Px(rows) - spacing.row,
                );
            }

            let mut layout = true;
            while mem::take(&mut layout) {
                let mut actual_count = 0;

                ctx.with_constrains(
                    move |c| {
                        if size_final {
                            c.with_max_size(cell_size).with_fill(true, true)
                        } else {
                            c
                        }
                    },
                    |ctx| {
                        self.children.layout_all(
                            ctx,
                            wl,
                            |_, _, _| {},
                            |_, _, a| {
                                if a.size != PxSize::zero() {
                                    actual_count += 1;
                                    if !size_final {
                                        cell_size = cell_size.max(a.size);
                                    }
                                }
                            },
                        );
                    },
                );

                if actual_count == 0 {
                    // no children or all collapsed.
                    return ctx.constrains().min_size();
                }

                if !count_final {
                    count_final = true;

                    if actual_count != count {
                        count = actual_count;
                        let (n_columns, n_rows) = self.grid_len(ctx.vars, actual_count);
                        if n_columns != columns || n_rows != rows {
                            columns = n_columns;
                            rows = n_rows;
                            layout = true;
                        }
                    }
                }

                if !size_final {
                    size_final = true;

                    panel_size = PxSize::new(
                        (cell_size.width + spacing.column) * Px(columns) - spacing.column,
                        (cell_size.height + spacing.row) * Px(rows) - spacing.row,
                    );
                    let clamped = constrains.fill_size_or(panel_size);
                    if clamped != panel_size {
                        panel_size = clamped;

                        cell_size = PxSize::new(
                            (panel_size.width + spacing.column) / Px(columns) - spacing.column,
                            (panel_size.height + spacing.row) / Px(rows) - spacing.row,
                        );
                    }

                    layout = true;
                }
            }

            let mut first_column = self.first_column.copy(ctx);
            if first_column as i32 >= columns {
                first_column = 0;
            }

            let mut cells = CellsIter::new(cell_size, columns, first_column as i32, spacing);

            self.children.outer_all(wl, false, |wlt, a| {
                if a.size != PxSize::zero() {
                    if let Some(offset) = cells.next() {
                        wlt.translate(offset);
                    }
                }
            });

            panel_size
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
