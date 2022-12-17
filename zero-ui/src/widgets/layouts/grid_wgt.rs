use crate::prelude::new_widget::*;

/// Grid layout with cells of variable sizes.
#[widget($crate::widgets::layouts::grid)]
pub mod grid {
    use super::*;

    #[doc(inline)]
    pub use super::{cell, column, row, AutoRowViewArgs};

    inherit!(widget_base::base);

    properties! {
        /// Cell widget items.
        ///
        /// Cells can select their own column, row using the properties in the [`cell!`] widget. Note that
        /// you don't need to use the `cell!` widget, only the properties.
        ///
        /// Cells can also be set to span multiple columns using the [`cell!`] properties. If
        ///
        /// If the column or row is not explicitly set the widget is positioned in the logical index.
        ///
        /// [`cell!`]: mod@cell
        pub widget_base::children as cells;

        /// Column definitions.
        ///
        /// You can define columns with any widget, but the [`column!`] widget is recommended. The [`column::width`] property defines
        /// the cells width if set, if it is not set, the column widget and all cells in the column with column span 1 are measured to
        /// fill and the widest width is used as the column width. If the [`column::width`] is set to [`Length::Default`] the widest
        /// cell width is used to layout the column widget, and the final width used for cells. This means that you can always constrain
        /// a column using the [`min_width`] and [`max_width`] properties.
        ///
        /// Note that the column widget is not the parent of the cells that match it, the column is layout before cells and
        /// is render under cell and row widgets. Properties like `padding` and `align` only affect the column visual, not the cells,
        /// similarly contextual properties like `text_color` don't affect the cells.
        ///
        /// [`column!`]: mod@column
        /// [`column::width`]: fn@column::width
        /// [`min_width`]: fn@min_width
        /// [`max_width`]: fn@max_width
        pub columns(impl UiNodeList);

        /// Row definitions.
        ///
        /// If left empty rows are auto-generated, using the [`auto_row_view`] if it is set or to an imaginary default row with
        /// height [`Length::Default`].
        ///
        /// If not empty the row widgets are used to layout the cells height the same way the [`columns`] are used to layout width.
        /// Like columns the rows are not the logical parent of cells, if the row widget renders any visual it is rendered under the
        /// cells assigned to it, but over the column widgets.
        ///
        /// [`auto_row_view`]: fn@auto_row_view
        /// [`columns`]: fn@columns
        pub rows(impl UiNodeList);

        /// View generator used to provide row widgets when [`rows`] is empty.
        ///
        /// Note that auto-rows are always generated when `rows` is empty, even if this generator is not set or is [`ViewGenerator::nil`].
        ///
        /// [`rows`]: fn@rows
        pub auto_row_view(impl IntoVar<ViewGenerator<AutoRowViewArgs>>);

        /// Space in-between items.
        pub spacing(impl IntoVar<GridSpacing>);

        /// Spacing around the items grid, inside the border.
        pub crate::properties::padding;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|w| {
            let child = grid_node(
                w.capture_ui_node_list_or_empty(property_id!(self::cells)),
                w.capture_ui_node_list_or_empty(property_id!(self::columns)),
                w.capture_ui_node_list_or_empty(property_id!(self::rows)),
                w.capture_var_or_else(property_id!(self::auto_row_view), ViewGenerator::nil),
                w.capture_var_or_default(property_id!(self::spacing)),
            );
            let child = widget_base::nodes::children_layout(child);

            w.set_child(child);
        });
    }
}

/// Grid column definition.
///
/// This widget is layout to define the actual column width, it is not the parent
/// of the cells, only the `width` and `align` properties affect the cells.
///
/// See the [`grid::columns`] property for more details.
///
/// [`grid::columns`]: fn@grid::columns
#[widget($crate::widgets::layouts::grid::column)]
pub mod column {
    use super::*;

    inherit!(widget_base::base);

    pub use crate::properties::{max_width, min_width};

    // !!: what we need
    //    - Widget state is perfect to communicate the index.
    //    - Except we want the `INDEX_VAR` for custom `when` conditions.
    //    - We need a "state" property that takes on different values.
    //    - Was there not something about `get_` prefix?
    //        - Not in TODO anymore, idea was to have `get_index(impl UiNode, ArcVar<usize>) -> impl UiNode`.
    //        - The prefix `get_` signals the same kind of thing the `is_` prefix does.
    //        - Default required?

    /// Column index in the parent widget set by the parent.
    pub(super) static INDEX_ID: StaticStateId<usize> = StaticStateId::new_unique();

    /// Column width, defines the actual cells width and the column widget width if set and not [`Length::Default`].
    ///
    /// The fill constrain is the grid width fill divided by columns, so `100.pct()` width in a column in a grid with 3 columns is 1/3 the
    /// grid fill width. You can set the width to more then `100.pct()` as long as the different is removed from the other columns.
    ///
    /// Note that the column it self is always sized to fill as a *background* for the cells assigned to it, this property
    /// informs the [`grid!`] widget on how to constrain the cells width.
    ///
    /// If this property is set to a read-write variable with value [`Length::Default`] the first layout width is set back on the variable.
    /// You can use this to implement user resizable columns that init sized to cell content.
    ///
    /// [`grid!`]: mod@crate::widgets::layouts::grid
    #[property(LAYOUT, default(Length::Default))]
    pub fn width(child: impl UiNode, width: impl IntoVar<Length>) -> impl UiNode {
        #[ui_node(struct WidthNode {
            child: impl UiNode,
            #[var] width: impl Var<Length>,
        })]
        impl UiNode for WidthNode {
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                if let Some(&i) = ctx.widget_state.get(&INDEX_ID) {
                    let mut info = GRID_CONTEXT.write();
                    info.init_column_info(i);
                    info.column_info[i].sized_by_cell = self.width.get().is_default();
                }

                self.child.info(ctx, info);
            }

            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                if let Some(l) = self.width.get_new(ctx) {
                    if let Some(&i) = ctx.widget_state.get(&INDEX_ID) {
                        let mut info = GRID_CONTEXT.write();
                        info.column_info[i].sized_by_cell = l.is_default();
                        ctx.updates.layout();
                    }
                }
                self.child.update(ctx, updates);
            }
        }

        let width = width.into_var();
        WidthNode {
            child: crate::properties::width(child, width.clone()),
            width,
        }
    }
}

/// Grid row definition.
///
/// This widget is layout to define the actual row height, it is not the parent
/// of the cells, only the `height` property affect the cells.
///
/// See the [`grid::rows`] property for more details.
///
/// [`grid::rows`]: fn@grid::rows
#[widget($crate::widgets::layouts::grid::row)]
pub mod row {
    use super::*;

    inherit!(widget_base::base);

    pub use crate::properties::{max_height, min_height};

    /// Row index in the parent widget set by the parent.
    pub(super) static INDEX_ID: StaticStateId<usize> = StaticStateId::new_unique();

    /// Row height, defines the actual cells height and the row widget height if set and not [`Length::Default`].
    ///
    /// The fill constrain is the grid height fill divided by rows, so `100.pct()` height in a row in a grid with 3 rows is 1/3 the
    /// grid fill height. You can set the height to more then `100.pct()` as long as the different is removed from the other rows.
    ///
    /// Note that the row it self is always sized to fill as a *background* for the cells assigned to it, this property
    /// informs the [`grid!`] widget on how to constrain the cells height.
    ///
    /// If this property is set to a read-write variable with value [`Length::Default`] the first layout height is set back on the variable.
    /// You can use this to implement user resizable rows that init sized to cell content.
    ///
    /// [`grid!`]: mod@crate::widgets::layouts::grid
    #[property(LAYOUT, default(Length::Default))]
    pub fn height(child: impl UiNode, height: impl IntoVar<Length>) -> impl UiNode {
        #[ui_node(struct HeightNode {
            child: impl UiNode,
            #[var] height: impl Var<Length>,
        })]
        impl UiNode for HeightNode {
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                if let Some(&i) = ctx.widget_state.get(&INDEX_ID) {
                    let mut info = GRID_CONTEXT.write();
                    info.init_row_info(i);
                    info.row_info[i].sized_by_cell = self.height.get().is_default();
                }

                self.child.info(ctx, info);
            }

            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                if let Some(l) = self.height.get_new(ctx) {
                    if let Some(&i) = ctx.widget_state.get(&INDEX_ID) {
                        let mut info = GRID_CONTEXT.write();
                        info.row_info[i].sized_by_cell = l.is_default();
                        ctx.updates.layout();
                    }
                }
                self.child.update(ctx, updates);
            }
        }

        let height = height.into_var();
        HeightNode {
            child: crate::properties::height(child, height.clone()),
            height,
        }
    }
}

/// Grid cell container.
///
/// This widget defines properties that position and size widgets in a [`grid!`].
///
/// See the [`grid::cells`] property for more details.
///
/// [`grid::cells`]: fn@grid::cells
#[widget($crate::widgets::layouts::grid::cell)]
pub mod cell {
    use super::*;

    inherit!(crate::widgets::container);

    /// Cell logical index in the parent widget set by the parent.
    pub(super) static INDEX_ID: StaticStateId<usize> = StaticStateId::new_unique();

    /// Cell column index.
    ///
    /// If not set or out-of-bounds the cell is positioned based on the logical index.
    #[property(CONTEXT, default(usize::MAX))]
    pub fn column(child: impl UiNode, col: impl IntoVar<usize>) -> impl UiNode {
        #[ui_node(struct ColumnNode {
            child: impl UiNode,
            #[var] col: impl Var<usize>,
        })]
        impl UiNode for ColumnNode {
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                if let Some(&i) = ctx.widget_state.get(&INDEX_ID) {
                    let mut info = GRID_CONTEXT.write();
                    info.init_cell_info(i);
                    info.cell_info[i].column = self.col.get();
                }

                self.child.info(ctx, info);
            }

            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                if let Some(c) = self.col.get_new(ctx) {
                    if let Some(&i) = ctx.widget_state.get(&INDEX_ID) {
                        let mut info = GRID_CONTEXT.write();
                        info.cell_info[i].column = c;
                        ctx.updates.layout();
                    }
                }
                self.child.update(ctx, updates);
            }
        }
        ColumnNode {
            child,
            col: col.into_var(),
        }
    }

    /// Cell row index.
    ///
    /// If not set or out-of-bounds the cell is positioned based on the logical index.
    #[property(CONTEXT, default(usize::MAX))]
    pub fn row(child: impl UiNode, row: impl IntoVar<usize>) -> impl UiNode {
        #[ui_node(struct RowNode {
            child: impl UiNode,
            #[var] row: impl Var<usize>,
        })]
        impl UiNode for RowNode {
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                if let Some(&i) = ctx.widget_state.get(&INDEX_ID) {
                    let mut info = GRID_CONTEXT.write();
                    info.init_cell_info(i);
                    info.cell_info[i].row = self.row.get();
                }

                self.child.info(ctx, info);
            }

            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                if let Some(r) = self.row.get_new(ctx) {
                    if let Some(&i) = ctx.widget_state.get(&INDEX_ID) {
                        let mut info = GRID_CONTEXT.write();
                        info.cell_info[i].row = r;
                        ctx.updates.layout();
                    }
                }
                self.child.update(ctx, updates);
            }
        }
        RowNode {
            child,
            row: row.into_var(),
        }
    }

    /// Cell column span.
    ///
    /// Number of *cells* this one spans over horizontally, starting from the column index and spanning to the right.
    ///
    /// Is `1` by default, the index is clamped between `1..max` where max is the maximum number of valid columns
    /// to the right of the cell column index.
    ///
    /// Note that the cell does not influence the column width if it spans over multiple columns.
    #[property(CONTEXT, default(1))]
    pub fn column_span(child: impl UiNode, col: impl IntoVar<usize>) -> impl UiNode {
        #[ui_node(struct ColumnSpanNode {
            child: impl UiNode,
            #[var] col: impl Var<usize>,
        })]
        impl UiNode for ColumnSpanNode {
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                if let Some(&i) = ctx.widget_state.get(&INDEX_ID) {
                    let mut info = GRID_CONTEXT.write();
                    info.init_cell_info(i);
                    info.cell_info[i].column_span = self.col.get();
                }

                self.child.info(ctx, info);
            }

            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                if let Some(c) = self.col.get_new(ctx) {
                    if let Some(&i) = ctx.widget_state.get(&INDEX_ID) {
                        let mut info = GRID_CONTEXT.write();
                        info.cell_info[i].column_span = c;
                        ctx.updates.layout();
                    }
                }
                self.child.update(ctx, updates);
            }
        }
        ColumnSpanNode {
            child,
            col: col.into_var(),
        }
    }

    /// Cell row span.
    ///
    /// Number of *cells* this one spans over vertically, starting from the row index and spanning down.
    ///
    /// Is `1` by default, the index is clamped between `1..max` where max is the maximum number of valid rows
    /// down from the cell column index.
    ///
    /// Note that the cell does not influence the row height if it spans over multiple rows.
    #[property(CONTEXT, default(1))]
    pub fn row_span(child: impl UiNode, row: impl IntoVar<usize>) -> impl UiNode {
        #[ui_node(struct RowSpanNode {
            child: impl UiNode,
            #[var] row: impl Var<usize>,
        })]
        impl UiNode for RowSpanNode {
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                if let Some(&i) = ctx.widget_state.get(&INDEX_ID) {
                    let mut info = GRID_CONTEXT.write();
                    info.init_cell_info(i);
                    info.cell_info[i].row_span = self.row.get();
                }

                self.child.info(ctx, info);
            }

            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                if let Some(r) = self.row.get_new(ctx) {
                    if let Some(&i) = ctx.widget_state.get(&INDEX_ID) {
                        let mut info = GRID_CONTEXT.write();
                        info.cell_info[i].row_span = r;
                        ctx.updates.layout();
                    }
                }
                self.child.update(ctx, updates);
            }
        }
        RowSpanNode {
            child,
            row: row.into_var(),
        }
    }
}

#[derive(Clone, Copy)]
struct ColumnInfo {
    /// Column is sized to fill the widest cell.
    pub sized_by_cell: bool,
    /// Computed width.
    pub width: Px,
}
impl Default for ColumnInfo {
    fn default() -> Self {
        Self {
            sized_by_cell: true,
            width: Px(0),
        }
    }
}

#[derive(Clone, Copy)]
struct RowInfo {
    /// Row is sized to fill the tallest cell.
    pub sized_by_cell: bool,
    /// Computed height.
    pub height: Px,
}
impl Default for RowInfo {
    fn default() -> Self {
        Self {
            sized_by_cell: true,
            height: Px(0),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct CellInfo {
    pub column: usize,
    pub column_span: usize,
    pub row: usize,
    pub row_span: usize,
}
impl Default for CellInfo {
    fn default() -> Self {
        Self {
            column: usize::MAX, // set by redistribute
            column_span: 1,
            row: usize::MAX,
            row_span: 1,
        }
    }
}
impl CellInfo {
    pub fn actual(mut self, i: usize, columns_len: usize) -> Self {
        if self.column >= columns_len {
            self.column = i % columns_len;
        }
        if self.row == usize::MAX {
            self.row = i / columns_len
        }
        self
    }
}

#[derive(Default)]
struct GridContext {
    column_info: Vec<ColumnInfo>,
    row_info: Vec<RowInfo>,
    cell_info: Vec<CellInfo>,
}
context_local! {
    static GRID_CONTEXT: GridContext = GridContext::default();
}
impl GridContext {
    fn init_column_info(&mut self, index: usize) {
        if self.column_info.len() <= index {
            self.column_info.resize(index + 1, ColumnInfo::default());
        }
    }
    fn init_row_info(&mut self, index: usize) {
        if self.row_info.len() <= index {
            self.row_info.resize(index + 1, RowInfo::default());
        }
    }
    fn init_cell_info(&mut self, index: usize) {
        if self.cell_info.len() <= index {
            self.cell_info.resize(index + 1, CellInfo::default());
        }
    }
}

fn grid_node(
    cells: BoxedUiNodeList,
    columns: BoxedUiNodeList,
    rows: BoxedUiNodeList,
    auto_row_view: BoxedVar<ViewGenerator<AutoRowViewArgs>>,
    spacing: BoxedVar<GridSpacing>,
) -> impl UiNode {
    let auto_rows = EditableUiNodeList::new();
    let auto_rows_ref = auto_rows.reference();
    let node = GridNode {
        children: vec![columns, vec![rows, auto_rows.boxed()].boxed(), cells],
        auto_rows: auto_rows_ref,
        spacing: spacing.into_var(),
        auto_row_view: auto_row_view.into_var(),
    };
    with_context_local(node, &GRID_CONTEXT, GridContext::default())
}

#[ui_node(struct GridNode {
    // [columns, [rows, auto_rows], cells]
    children: Vec<BoxedUiNodeList>,
    auto_rows: EditableUiNodeListRef,
    #[var] auto_row_view: impl Var<ViewGenerator<AutoRowViewArgs>>,
    #[var] spacing: impl Var<GridSpacing>,
})]
impl UiNode for GridNode {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.init_handles(ctx);
        self.children.init_all(ctx);

        // Set index for column, row and cell properties. These properties will *init* `GridContext` info
        // in the next `UiNode::info`.
        self.children[0].for_each_mut(|i, c| {
            c.with_context_mut(|ctx| ctx.widget_state.set(&column::INDEX_ID, i));
            true
        });
        self.children[1].for_each_mut(|i, r| {
            r.with_context_mut(|ctx| ctx.widget_state.set(&row::INDEX_ID, i));
            true
        });
        self.children[2].for_each_mut(|i, r| {
            r.with_context_mut(|ctx| ctx.widget_state.set(&row::INDEX_ID, i));
            true
        });
    }

    fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
        if self.spacing.is_new(ctx) {
            ctx.updates.layout();
        }

        let mut any = false;
        self.children.update_all(ctx, updates, &mut any);

        if any || self.auto_row_view.is_new(ctx) {
            // !!: TODO, support new columns/rows
            ctx.updates.layout();
        }
    }

    fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
        todo!()
    }

    fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        let spacing = self.spacing.get().layout(ctx.metrics, |_| PxGridSpacing::zero());
        let fill_size = ctx.metrics.constrains().fill_or_exact();

        let columns_len = self.children[0].len().max(1);

        // measure cells for columns/rows that need it.
        // this is needed for columns/rows flagged `sized_by_cell` or if the grid is not fill/exact in a dimension.
        self.children[2].for_each(|i, c| {
            let ci;
            let need_measure;
            {
                let mut info = GRID_CONTEXT.write();
                info.init_cell_info(i);
                ci = info.cell_info[i].actual(i, columns_len);

                info.init_column_info(ci.column);
                info.init_row_info(ci.row); // !!: generate rows?

                need_measure = fill_size.is_none() || info.column_info[ci.column].sized_by_cell || info.row_info[ci.row].sized_by_cell;
            }

            if need_measure {
                let cell_size = ctx
                    .as_measure()
                    .with_constrains(|c| c.with_fill(false, false), |ctx| c.measure(ctx, &mut WidgetMeasure::new()));

                let mut info = GRID_CONTEXT.write();
                let info = &mut *info;
                let col = &mut info.column_info[ci.column];
                let row = &mut info.row_info[ci.row];
                if fill_size.is_none() || col.sized_by_cell {
                    col.width = col.width.max(cell_size.width); // !!: ensure this is reset before first measure?
                }
                if fill_size.is_none() || row.sized_by_cell {
                    row.height = row.height.max(cell_size.height);
                }
            }

            true
        });

        let rows_len = GRID_CONTEXT.read().row_info.len();

        // compute final column widths.
        let mut offset = Px(0);
        let column_100pct = if let Some(s) = fill_size {
            let columns = Px(columns_len as i32);
            let total = s.width - spacing.column * (columns - Px(1));
            total / columns
        } else {
            Px::MAX
        };
        self.children[0].for_each_mut(|i, c| {
            let info = GRID_CONTEXT.read().column_info[i];

            let s = if info.sized_by_cell || column_100pct == Px::MAX {
                // column has the widest cell width.
                ctx.with_constrains(|c| c.with_exact_x(info.width), |ctx| c.layout(ctx, wl))
            } else {
                // column defines the width.
                ctx.with_constrains(|c| c.with_max_x(column_100pct), |ctx| c.layout(ctx, wl))
            };
            // width defined by the column or corrected by it (min/max_width).
            GRID_CONTEXT.write().column_info[i].width = s.width;

            wl.with_outer(c, false, |wl, _| wl.translate(PxVector::new(offset, Px(0))));
            offset += s.width + spacing.column;

            true
        });

        // compute final row heights.
        offset = Px(0);
        let row_100pct = if let Some(s) = fill_size {
            let rows = Px(rows_len as i32);
            let total = s.height - spacing.row * (rows - Px(1));
            total / rows
        } else {
            Px::MAX
        };
        self.children[1].for_each_mut(|i, r| {
            let info = GRID_CONTEXT.read().row_info[i];

            let s = if info.sized_by_cell || row_100pct == Px::MAX {
                // row has the tallest cell height.
                ctx.with_constrains(|c| c.with_exact_x(info.height), |ctx| r.layout(ctx, wl))
            } else {
                // row defines the height.
                ctx.with_constrains(|c| c.with_max_x(row_100pct), |ctx| r.layout(ctx, wl))
            };
            // height defined by the row or corrected by it (min/max_height).
            GRID_CONTEXT.write().row_info[i].height = s.height;

            wl.with_outer(r, false, |wl, _| wl.translate(PxVector::new(Px(0), offset)));
            offset += s.height + spacing.row;

            true
        });

        // layout cells.
        self.children[2].for_each_mut(|i, c| {
            let (cell_offset, cell_size) = {
                let info = GRID_CONTEXT.read();

                let ci = info.cell_info[i].actual(i, columns_len);

                let mut offset = PxVector::zero();
                for col in 0..ci.column {
                    offset.x += info.column_info[col].width + spacing.column;
                }
                offset.x -= spacing.column;
                for row in 0..ci.row {
                    offset.y += info.row_info[row].height + spacing.row;
                }
                offset.y -= spacing.row;

                let mut size = PxSize::zero();
                for col in ci.column..(ci.column + ci.column_span).min(columns_len) {
                    size.width += info.column_info[col].width + spacing.column;
                }
                size.width -= spacing.column;
                for row in ci.row..(ci.row + ci.row_span).min(rows_len) {
                    size.height += info.row_info[row].height + spacing.row;
                }
                size.height -= spacing.row;

                (offset, size)
            };

            ctx.with_constrains(|c| c.with_exact_size(cell_size), |ctx| c.layout(ctx, wl));
            wl.with_outer(c, false, |wl, _| wl.translate(cell_offset));

            true
        });

        let info = GRID_CONTEXT.read();

        PxSize::new(
            info.column_info
                .iter()
                .map(|c| c.width + spacing.column)
                .fold(Px(0), |acc, w| acc + w)
                - spacing.column,
            info.row_info.iter().map(|c| c.height + spacing.row).fold(Px(0), |acc, h| acc + h) - spacing.row,
        )
        .max(ctx.constrains().fill_size())
    }
}

/// Arguments for [`grid::auto_row_view`].
///
/// [`grid::auto_row_view`]: fn@grid::auto_row_view.
#[derive(Clone, Debug)]
pub struct AutoRowViewArgs {
    /// Row index.
    pub index: usize,
}
