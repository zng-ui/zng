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
            columns(impl IntoVar<u32>) = 0;
            /// Number of rows.
            rows(impl IntoVar<u32>) = 0;
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
            first_column(impl IntoVar<u32>) = 0;

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
        columns: impl IntoVar<u32>,
        rows: impl IntoVar<u32>,
        first_column: impl IntoVar<u32>,
        spacing: impl IntoVar<GridSpacing>,
    ) -> impl UiNode {
        UniformGridNode {
            children_info: vec![ChildInfo::default(); items.len()],
            children: items,

            columns: columns.into_var(),
            rows: rows.into_var(),
            first_column: first_column.into_var(),
            spacing: spacing.into_var(),
        }
    }

    #[derive(Default, Clone, Copy)]
    struct ChildInfo {
        /// If last desired size was not zero.
        visible: bool,
    }

    struct UniformGridNode<U, C, R, FC, S> {
        children_info: Vec<ChildInfo>,
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
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions
                .vars(ctx)
                .var(&self.columns)
                .var(&self.rows)
                .var(&self.first_column)
                .var(&self.spacing);

            self.children.subscriptions_all(ctx, subscriptions);
        }

        #[UiNode]
        fn update(&mut self, ctx: &mut WidgetContext) {
            let mut changed = false;
            self.children.update_all(ctx, &mut changed);

            if changed {
                self.children_info.resize(self.children.len(), ChildInfo::default());
            }

            if changed || self.columns.is_new(ctx) || self.rows.is_new(ctx) || self.first_column.is_new(ctx) || self.spacing.is_new(ctx) {
                ctx.updates.layout_and_render();
            }
        }
        #[UiNode]
        fn measure(&mut self, ctx: &mut LayoutContext, mut available_size: AvailableSize) -> PxSize {
            let layout_spacing = self.spacing.get(ctx).to_layout(ctx, available_size, PxGridSpacing::zero());

            let (columns, rows) = self.grid_len(ctx.vars, self.children.len());

            if let AvailablePx::Finite(f) = &mut available_size.width {
                *f = (*f - layout_spacing.column / Px(2)) / Px(columns);
            }
            if let AvailablePx::Finite(f) = &mut available_size.height {
                *f = (*f - layout_spacing.row / Px(2)) / Px(rows);
            }

            let mut cell_size = PxSize::zero();

            self.children.measure_all(
                ctx,
                |_, _| available_size,
                |_, args| {
                    cell_size = cell_size.max(args.desired_size);
                    self.children_info[args.index].visible = args.desired_size != PxSize::zero();
                },
            );

            PxSize::new(
                cell_size.width * columns + layout_spacing.column * (columns - 1),
                cell_size.height * rows + layout_spacing.row * (rows - 1),
            )
        }
        #[UiNode]
        fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
            let cell_count = self.children_info.iter().filter(|o| o.visible).count();

            let (columns, rows) = self.grid_len(ctx.vars, cell_count);

            let layout_spacing = self
                .spacing
                .get(ctx)
                .to_layout(ctx, AvailableSize::finite(final_size), PxGridSpacing::zero());

            let cell_size = PxSize::new(
                (final_size.width - layout_spacing.column * Px(columns - 1)) / Px(columns),
                (final_size.height - layout_spacing.row * Px(rows - 1)) / Px(rows),
            );

            let mut first_column = self.first_column.copy(ctx);
            if first_column as i32 >= columns {
                first_column = 0;
            }

            let mut cells = CellsIter::new(cell_size, columns, first_column as i32, layout_spacing);

            self.children.arrange_all(ctx, widget_layout, |_, args| {
                if self.children_info[args.index].visible {
                    args.pre_translate = cells.next();
                }
                cell_size
            });
        }
        #[UiNode]
        #[allow_(zero_ui::missing_delegate)] // false positive
        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.children
                .render_filtered(move |args| self.children_info[args.index].visible, ctx, frame);
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
