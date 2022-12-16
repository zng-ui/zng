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
        /// If the column or row is not explicitly set the widget is positioned in the first *free* cell.
        ///
        /// [`cell!`]: mod@cell
        pub widget_base::children as cells;

        /// Column definitions.
        ///
        /// You can define columns with any widget, but the [`column!`] widget is recommended. The column widget width defines
        /// the cells width, and the [`column::cell_align`] property defines their alignment. Note that the column widget is not
        /// the parent of the cells that match it, the column is layout before cells and is render under cell and row widgets. Properties
        /// like `padding` only affect the column visual, not the cells, similarly contextual properties like `text_color` don't affect the cells.
        ///
        /// Each column is layout with constrains that make `width = 100.pct()` resolve to a width that equally divides the available
        /// grid width into equal partitions for each column. It is ok to layout a column larger then `100.pct()`, as long as the different
        /// is removed from another column. If the columns sum exceeds the available width
        ///
        /// If the column is [`Visibility::Collapsed`] the cells will also be collapsed, but if it hidden the cells are not hidden.
        ///
        /// [`column!`]: mod@column
        /// [`column::cell_align`]: fn@column::cell_align
        pub columns(impl UiNodeList);

        /// Row definitions.
        ///
        /// If left empty rows are auto-generated, with height defined by the tallest cell and no visual, or using the [`auto_row_view`]
        /// if it is set.
        ///
        /// [`auto_row_view`]: fn@auto_row_view
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
            let cells = w.capture_ui_node_list_or_empty(property_id!(self::cells));
            let columns = w.capture_ui_node_list_or_empty(property_id!(self::columns));
            let rows = w.capture_ui_node_list_or_empty(property_id!(self::rows));
            let spacing = w.capture_var_or_default(property_id!(self::spacing));

            w.set_child(GridNode {
                cells,
                columns,
                rows,
                spacing: spacing.into_var(),
            });
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

    properties! {
        /// Column width, set to `100.pct()` by default.
        pub crate::properties::width = 100.pct();
    }

    context_var! {
        /// Column index as a read-only variable.
        ///
        /// Set to the zero-based index of the column widget for each widget. You can use this to implement interleaved background colors.
        pub static INDEX_VAR: usize = 0;
    }

    /// Alignment of all cells that match this grid column.
    #[property(LAYOUT, default(Align::FILL))]
    pub fn cell_align(child: impl UiNode, align: impl IntoVar<Align>) -> impl UiNode {
        #[ui_node(struct CellAlignNode {
            child: impl UiNode,
            #[var] align: impl Var<Align>,
        })]
        impl UiNode for CellAlignNode {
            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                if self.align.is_new(ctx) {
                    ctx.updates.layout();
                }
                self.child.update(ctx, updates);
            }
        }
        CellAlignNode {
            child,
            align: align.into_var(),
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

    properties! {
        /// Row height, set to `100.pct()` by default
        pub crate::properties::height = 100.pct();
    }

    context_var! {
        /// Row index as a read-only variable.
        ///
        /// Set to the zero-based index of the row widget for each widget. You can use this to implement interleaved background colors.
        pub static INDEX_VAR: usize = 0;
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
        /// Cell column, row index as a read-only variable.
        ///
        /// This is the actual index, corrected from the [`column`] and [`row`] values or auto-selected if these
        /// properties where not set in the widget.
        ///
        /// [`column`]: fn@column
        /// [`row`]: fn@row
        pub static INDEX_VAR: (usize, usize) = (0, 0);
    }
}

#[ui_node(struct GridNode {
    cells: impl UiNodeList,
    columns: impl UiNodeList,
    rows: impl UiNodeList,
    #[var] spacing: impl Var<GridSpacing>,
})]
impl UiNode for GridNode {}

/// Arguments for [`grid::auto_row_view`].
///
/// [`grid::auto_row_view`]: fn@grid::auto_row_view.
#[derive(Clone, Debug)]
pub struct AutoRowViewArgs {
    /// Row index.
    pub index: usize,
}
