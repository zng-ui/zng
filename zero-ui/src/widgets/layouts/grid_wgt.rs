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
        /// If the column or row is not explicitly set the widget is positioned in the first *free* cell.
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

    context_var! {
        /// Column index as a read-only variable.
        ///
        /// Set to the zero-based index of the column widget for each widget. You can use this to implement interleaved background colors.
        pub static INDEX_VAR: usize = 0;
    }

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
            fn init(&mut self, ctx: &mut WidgetContext) {
                {
                    let i = INDEX_VAR.get();
                    let mut info = GRID_CONTEXT.write();
                    if i < info.column_info.len() {
                        info.column_info[i].sized_by_cell = self.width.get().is_default();
                    }
                }

                self.child.init(ctx);
            }

            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                if let Some(l) = self.width.get_new(ctx) {
                    let i = INDEX_VAR.get();
                    let mut info = GRID_CONTEXT.write();
                    if i < info.column_info.len() {
                        info.column_info[i].sized_by_cell = l.is_default();
                        ctx.updates.layout();
                    }
                }
                self.child.update(ctx, updates);
            }
        }
        WidthNode {
            child,
            width: width.into_var(),
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

    context_var! {
        /// Row index as a read-only variable.
        ///
        /// Set to the zero-based index of the row widget for each widget. You can use this to implement interleaved background colors.
        pub static INDEX_VAR: usize = 0;
    }

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
            fn init(&mut self, ctx: &mut WidgetContext) {
                {
                    let i = INDEX_VAR.get();
                    let mut info = GRID_CONTEXT.write();
                    if i < info.row_info.len() {
                        info.row_info[i].sized_by_cell = self.height.get().is_default();
                    }
                }

                self.child.init(ctx);
            }

            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                if let Some(l) = self.height.get_new(ctx) {
                    let i = INDEX_VAR.get();
                    let mut info = GRID_CONTEXT.write();
                    if i < info.row_info.len() {
                        info.row_info[i].sized_by_cell = l.is_default();
                        ctx.updates.layout();
                    }
                }
                self.child.update(ctx, updates);
            }
        }
        HeightNode {
            child,
            height: height.into_var(),
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

    /// Cell column index.
    ///
    /// If not set or out-of-bounds the cell is positioned on the first free cell.
    #[property(CONTEXT, default(usize::MAX))]
    pub fn column(child: impl UiNode, col: impl IntoVar<usize>) -> impl UiNode {
        #[ui_node(struct ColumnNode {
            child: impl UiNode,
            #[var] col: impl Var<usize>,
        })]
        impl UiNode for ColumnNode {
            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                if self.col.is_new(ctx) {
                    ctx.updates.layout();
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
    /// If not set or out-of-bounds the cell is positioned on the first free cell.
    #[property(CONTEXT, default(usize::MAX))]
    pub fn row(child: impl UiNode, row: impl IntoVar<usize>) -> impl UiNode {
        #[ui_node(struct RowNode {
            child: impl UiNode,
            #[var] row: impl Var<usize>,
        })]
        impl UiNode for RowNode {
            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                if self.row.is_new(ctx) {
                    ctx.updates.layout();
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
            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                if self.col.is_new(ctx) {
                    ctx.updates.layout();
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
            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                if self.row.is_new(ctx) {
                    ctx.updates.layout();
                }
                self.child.update(ctx, updates);
            }
        }
        RowSpanNode {
            child,
            row: row.into_var(),
        }
    }

    context_var! {
        /// Cell `(column, row)` index as a read-only variable.
        ///
        /// This is the actual index, corrected from the [`column`] and [`row`] values or auto-selected if these
        /// properties where not set in the widget.
        ///
        /// [`column`]: fn@column
        /// [`row`]: fn@row
        pub static INDEX_VAR: (usize, usize) = (0, 0);
    }
}

#[derive(Clone, Copy)]
pub struct ColumnInfo {
    /// Column is sized to fill the widest cell.
    pub sized_by_cell: bool,
    /// Computed width.
    pub width: Px,
}

#[derive(Clone, Copy)]
pub struct RowInfo {
    /// Row is sized to fill the tallest cell.
    pub sized_by_cell: bool,
    /// Computed height.
    pub height: Px,
}

#[derive(Default)]
struct GridContext {
    column_info: Vec<ColumnInfo>,
    row_info: Vec<RowInfo>,
    occupied: Vec<bool>,
}
context_local! {
    static GRID_CONTEXT: GridContext = GridContext::default();
}

fn grid_node(
    cells: BoxedUiNodeList,
    columns: BoxedUiNodeList,
    rows: BoxedUiNodeList,
    auto_row_view: BoxedVar<ViewGenerator<AutoRowViewArgs>>,
    spacing: BoxedVar<GridSpacing>,
) -> impl UiNode {
    let cell_len = columns.len();
    let col_len = columns.len();
    let mut row_len = rows.len();
    if row_len == 0 {
        let c = col_len.max(1);
        row_len = cell_len / c + 1;
    }

    let node = GridNode {
        children: vec![columns, rows, cells],
        spacing: spacing.into_var(),
        auto_row_view: auto_row_view.into_var(),
    };

    with_context_local(
        node,
        &GRID_CONTEXT,
        GridContext {
            column_info: Vec::with_capacity(col_len),
            row_info: Vec::with_capacity(row_len),
            occupied: Vec::with_capacity(cell_len),
        },
    )
}

#[ui_node(struct GridNode {
    // [columns, rows, cells]
    children: Vec<BoxedUiNodeList>,
    #[var] auto_row_view: impl Var<ViewGenerator<AutoRowViewArgs>>,
    #[var] spacing: impl Var<GridSpacing>,
})]
impl UiNode for GridNode {
    fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
        if self.spacing.is_new(ctx) {
            ctx.updates.layout();
        }

        let mut any = false;
        self.children.update_all(ctx, updates, &mut any);
        if any {
            ctx.updates.layout();
        }

        // !!: TODO associate grid cells with cell widgets.
        // !!: TODO generate rows.
    }

    fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
        let mut grid_size = PxSize::zero();

        let columns = self.children[0].len().max(1);
        ctx.with_constrains(
            |mut c| {
                if let Some(mut m) = c.x.max() {
                    m /= Px(columns as i32);
                    c = c.with_max_x(m);
                }
                c
            },
            |ctx| {
                self.children[0].for_each(|_, column| {
                    let s = column.measure(ctx, wm);
                    grid_size.width += s.width;
                    true
                });
            },
        );

        let rows = self.children[1].len();
        ctx.with_constrains(
            |mut c| {
                if rows == 0 {
                    c.y = c.y.with_unbounded();
                } else if let Some(mut m) = c.y.max() {
                    m /= Px(rows as i32);
                    c = c.with_max_y(m);
                }
                c
            },
            |ctx| {
                self.children[1].for_each(|_, row| {
                    let s = row.measure(ctx, wm);
                    grid_size.height += s.height;
                    true
                });
            },
        );

        grid_size
    }

    fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        let mut grid_size = PxSize::zero();

        let spacing = self.spacing.get().layout(ctx.metrics, |_| PxGridSpacing::zero());
        let fill_size = ctx.metrics.constrains().fill_size();

        // measure cells for columns/rows that need it.
        // this is needed for columns/rows flagged `sized_by_cell` or if the grid is not fill/exact in a dimension.
        // !!: TODO

        // compute final column widths.
        let column_100pct = Px::MAX; // !!: TODO divided width - spacing.
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
            GRID_CONTEXT.write().column_info[i].width = info.width;

            // !!: TODO, position column

            true
        });

        // compute final row widths.
        let row_100pct = Px::MAX; // !!: TODO divided height - spacing.
        self.children[1].for_each_mut(|i, r| {
            let info = GRID_CONTEXT.read().row_info[i];

            let s = if info.sized_by_cell || column_100pct == Px::MAX {
                // row has the tallest cell height.
                ctx.with_constrains(|c| c.with_exact_x(info.height), |ctx| r.layout(ctx, wl))
            } else {
                // row defines the height.
                ctx.with_constrains(|c| c.with_max_x(column_100pct), |ctx| r.layout(ctx, wl))
            };
            // height defined by the row or corrected by it (min/max_height).
            GRID_CONTEXT.write().row_info[i].height = info.height;

            // !!: TODO, position row

            true
        });

        // layout cells.
        self.children[2].for_each_mut(|i, c| {
            // !!: TODO get cell coordinates.

            true
        });

        grid_size
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
