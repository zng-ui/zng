use crate::prelude::new_widget::*;

/// Grid layout where all cells are the same size.
///
/// # Example
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
#[widget($crate::widgets::layouts::uniform_grid)]
pub mod uniform_grid {
    use super::*;

    properties! {
        child {
            /// Widget items.
            #[allowed_in_when = false]
            items(impl WidgetList) = widgets![];

            /// Number of columns.
            ///
            /// Set to zero (`0`) for auto TODO.
            columns(impl IntoVar<usize>) = 0;
            /// Number of rows.
            rows(impl IntoVar<usize>) = 0;
            /// Number of empty cells in the first row.
            ///
            /// Value is ignored if is `>= columns`.
            ///
            /// # Example
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
            first_column(impl IntoVar<usize>) = 0;

            /// Space in-between items.
            spacing(impl IntoVar<GridSpacing>) = 0.0;

            /// Margin around all items.
            margin as padding;
        }
    }

    /// New uniform grid layout.
    #[inline]
    fn new_child(
        items: impl WidgetList,
        columns: impl IntoVar<usize>,
        rows: impl IntoVar<usize>,
        first_column: impl IntoVar<usize>,
        spacing: impl IntoVar<GridSpacing>,
    ) -> impl UiNode {
        UniformGridNode {
            children: items,

            columns: columns.into_local(),
            rows: rows.into_local(),
            first_column: first_column.into_local(),
            spacing: spacing.into_local(),

            cells_iter: CellsIter::default(),
        }
    }

    struct UniformGridNode<U, C, R, FC, S> {
        children: U,
        columns: C,
        rows: R,
        first_column: FC,
        spacing: S,
        cells_iter: CellsIter,
    }
    #[impl_ui_node(children)]
    impl<U, C, R, FC, S> UniformGridNode<U, C, R, FC, S>
    where
        U: WidgetList,
        C: VarLocal<usize>,
        R: VarLocal<usize>,
        FC: VarLocal<usize>,
        S: VarLocal<GridSpacing>,
    {
        /// cells count for `grid_len`.
        fn cells_count(&self) -> f32 {
            match self.children.count_not_collapsed() {
                0 => 1.0,
                n => n as f32,
            }
        }

        /// (columns, rows)
        fn grid_len(&self) -> (f32, f32) {
            let mut columns = *self.columns.get_local() as f32;
            let mut rows = *self.rows.get_local() as f32;

            if columns < 1.0 {
                if rows < 1.0 {
                    // columns and rows are 0=AUTO, make a square
                    rows = self.cells_count().sqrt().ceil();
                    columns = rows;
                } else {
                    // only columns is 0=AUTO
                    columns = (self.cells_count() / rows).ceil();
                }
            } else if rows < 1.0 {
                // only rows is 0=AUTO
                rows = (self.cells_count() / columns).ceil();
            }

            debug_assert!(columns > 0.0 && rows > 0.0);

            (columns, rows)
        }

        #[UiNode]
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.children.init_all(ctx);

            self.columns.init_local(ctx.vars);
            self.rows.init_local(ctx.vars);
            self.first_column.init_local(ctx.vars);
            self.spacing.init_local(ctx.vars);
        }
        #[UiNode]
        fn update(&mut self, ctx: &mut WidgetContext) {
            self.children.update_all(ctx);

            if self.columns.update_local(ctx.vars).is_some()
                | self.rows.update_local(ctx.vars).is_some()
                | self.first_column.update_local(ctx.vars).is_some()
                | self.spacing.update_local(ctx.vars).is_some()
            {
                ctx.updates.layout();
            }
        }
        #[UiNode]
        fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
            let (columns, rows) = self.grid_len();

            let layout_spacing = self.spacing.get_local().to_layout(available_size, ctx);

            let available_size = LayoutSize::new(
                (available_size.width - layout_spacing.column / 2.0) / columns,
                (available_size.height - layout_spacing.row / 2.0) / rows,
            )
            .snap_to(*ctx.pixel_grid);

            let mut cell_size = LayoutSize::zero();

            self.children
                .measure_all(|_, _| available_size, |_, s, _| cell_size = cell_size.max(s), ctx);

            LayoutSize::new(
                cell_size.width * columns + layout_spacing.column * (columns - 1.0),
                cell_size.height * rows + layout_spacing.row * (rows - 1.0),
            )
            .snap_to(*ctx.pixel_grid)
        }
        #[UiNode]
        fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
            let (columns, rows) = self.grid_len();

            let layout_spacing = self.spacing.get_local().to_layout(final_size, ctx);

            let cell_size = LayoutSize::new(
                (final_size.width - layout_spacing.column * (columns - 1.0)) / columns,
                (final_size.height - layout_spacing.row * (rows - 1.0)) / rows,
            )
            .snap_to(*ctx.pixel_grid);

            self.children.arrange_all(|_, _| cell_size, ctx);

            let mut first_column = *self.first_column.get_local() as f32;
            if first_column >= columns {
                first_column = 0.0;
            }

            self.cells_iter = CellsIter::new(cell_size, columns, first_column, layout_spacing);
        }
        #[UiNode]
        fn render(&self, frame: &mut FrameBuilder) {
            let mut cells = self.cells_iter.clone();
            let grid = frame.pixel_grid();
            self.children
                .render_not_collapsed(move |_| cells.next().unwrap().snap_to(grid), frame);
        }
    }
    #[derive(Clone, Default)]
    struct CellsIter {
        r: LayoutPoint,
        advance: LayoutPoint,
        max_width: f32,
    }
    impl CellsIter {
        pub fn new(cell_size: LayoutSize, columns: f32, first_column: f32, spacing: LayoutGridSpacing) -> Self {
            let advance = LayoutPoint::new(cell_size.width + spacing.column, cell_size.height + spacing.row);
            CellsIter {
                r: LayoutPoint::new(advance.x * (first_column - 1.0), 0.0),
                max_width: advance.x * columns,
                advance,
            }
        }
    }
    impl Iterator for CellsIter {
        type Item = LayoutPoint;

        fn next(&mut self) -> Option<Self::Item> {
            self.r.x += self.advance.x;
            if self.r.x >= self.max_width {
                self.r.x = 0.0;
                self.r.y += self.advance.y;
            }
            Some(self.r)
        }
    }
}

/// Grid layout where all cells are the same size.
///
/// # Example
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
#[inline]
pub fn uniform_grid(items: impl WidgetList) -> impl Widget {
    uniform_grid! { items; }
}
