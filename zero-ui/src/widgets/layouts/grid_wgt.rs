use crate::prelude::new_widget::*;

/// Grid layout with cells of variable sizes.
#[widget($crate::widgets::layouts::grid)]
pub mod grid {
    use super::*;

    #[doc(inline)]
    pub use super::{cell, column, row, AutoGrowMode, AutoGrowViewArgs};

    inherit!(widget_base::base);

    properties! {
        /// Cell widget items.
        ///
        /// Cells can select their own column, row using the properties in the [`cell!`] widget. Note that
        /// you don't need to use the `cell!` widget, only the properties.
        ///
        /// Cells can also be set to span multiple columns using the [`cell!`] properties.
        ///
        /// If the column or row is not explicitly set the widget is positioned in the logical index, the column
        /// `i % columns` and the row  `i / columns`.
        ///
        /// [`cell!`]: mod@cell
        pub widget_base::children as cells;

        /// Column definitions.
        ///
        /// You can define columns with any widget, but the [`column!`] widget is recommended. The column widget width defines
        /// the width of the cells assigned to it, the [`column::width`] property can be used to enforce a width, otherwise the
        /// column is sized by the widest cell.
        ///
        /// The grid uses the [`SizePropertyKind`] value to select one of three layout modes for columns:
        ///
        /// * *Cell*, used for columns that do not set width or set it to [`Length::Default`].
        /// * *Relative*, used for columns that set width to a factor or percentage.
        /// * *Exact*, used for columns that set the width to a different unit.
        ///
        /// The column layout follows these steps:
        ///
        /// 1 - All *Exact* column widgets are layout, their final width defines the column width.
        /// 2 - All cell widgets with span `1` in *Cell* columns are measured, the widest defines the fill width constrain,
        /// the columns is layout using this constrain, the final width defines the column width.
        /// 3 - All *Relative* cells are layout with the left-over grid width as fill constrain divided by the number of *Relative* cells,
        ///     the final width defines the column width.
        ///
        /// So given the columns `200 | 100.pct() | 100.pct()` and grid width of `1000` with spacing `5` the final widths are `200 | 395 | 395`,
        /// for `200 + 5 + 395 + 5 + 395 = 1000`.
        ///
        /// Note that the column widget is not the parent of the cells that match it, the column widget is rendered under cell and row widgets.
        /// Properties like `padding` and `align` only affect the column visual, not the cells, similarly contextual properties like `text_color`
        /// don't affect the cells.
        ///
        /// [`column!`]: mod@column
        /// [`column::width`]: fn@column::width
        pub columns(impl UiNodeList);

        /// Row definitions.
        ///
        /// Same behavior as [`columns`], but in the ***y*** dimension.
        ///
        /// [`columns`]: fn@columns
        pub rows(impl UiNodeList);

        /// View generator used when new columns are needed to cover a cell placement.
        ///
        /// The generator is used when a cell is placed in a column not covered by the [`columns`] and inside the [`auto_column_max`] range.
        /// Note that if the generator is [`ViewGenerator::nil`] or does not return a full widget an *imaginary* column is used instead.
        pub auto_grow_view(impl IntoVar<ViewGenerator<AutoGrowViewArgs>>);

        /// Maximum inclusive index that can be covered by auto-generated columns or rows. If a cell is outside this index and
        /// is not covered by predefined columns or rows a new one is auto generated for it, but if the cell is also outside this
        /// max it is *collapsed*.
        ///
        /// Is `AutoGrowMode::Rows(u32::MAX)` by default.
        pub auto_grow_mode(impl IntoVar<AutoGrowMode>);

        /// Space in-between cells.
        pub spacing(impl IntoVar<GridSpacing>);

        /// Spacing around the grid, inside the border.
        pub crate::properties::padding;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|w| {
            let child = grid_node(
                w.capture_ui_node_list_or_empty(property_id!(self::cells)),
                w.capture_ui_node_list_or_empty(property_id!(self::columns)),
                w.capture_ui_node_list_or_empty(property_id!(self::rows)),
                w.capture_var_or_else(property_id!(self::auto_grow_view), ViewGenerator::nil),
                w.capture_var_or_else(property_id!(self::auto_grow_mode), || AutoGrowMode::Rows(u32::MAX)),
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

    pub use crate::properties::{height, max_width, min_width};

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

    pub use crate::properties::{height, max_height, min_height};

    /// Row index in the parent widget set by the parent.
    pub(super) static INDEX_ID: StaticStateId<usize> = StaticStateId::new_unique();
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

    /// Represents values set by cell properties in a widget.
    #[derive(Clone, Copy, Debug)]
    pub struct CellInfo {
        /// The [`column`] value.
        ///
        /// [`column`]: fn@column
        pub column: usize,

        /// The [`column_span`] value.
        ///
        /// [`column_span`]: fn@column_span
        pub column_span: usize,

        /// The [`row`] value.
        ///
        /// [`row`]: fn@row
        pub row: usize,

        /// The [`row_span`] value.
        ///
        /// [`row_span`]: fn@row_span
        pub row_span: usize,
    }
    impl Default for CellInfo {
        fn default() -> Self {
            Self {
                column: usize::MAX,
                column_span: 1,
                row: usize::MAX,
                row_span: 1,
            }
        }
    }
    impl CellInfo {
        /// Compute or correct the column and row of the cell.
        ///
        /// The `logical_index` is the index of the cell widget in the cell node list.
        pub fn actual(mut self, logical_index: usize, columns_len: usize) -> Self {
            if self.column == usize::MAX {
                self.column = logical_index % columns_len;
            } else {
                self.column = self.column.min(columns_len - 1);
            }
            if self.row == usize::MAX {
                self.row = logical_index / columns_len
            }
            self
        }

        /// Get the cell info stored in the widget state.
        pub fn get(state: StateMapRef<state_map::Widget>) -> Self {
            state.get(&INFO_ID).copied().unwrap_or_default()
        }

        /// Get the cell info stored in the widget.
        pub fn get_wgt(wgt: &impl UiNode) -> Self {
            wgt.with_context(|ctx| Self::get(ctx.widget_state)).unwrap_or_default()
        }
    }

    /// Id for widget state set by cell properties.
    ///
    /// The parent grid uses this info to position and size the cell widget.
    pub static INFO_ID: StaticStateId<CellInfo> = StaticStateId::new_unique();

    /// Cell column index.
    ///
    /// If not set or set to [`usize::MAX`] the cell is positioned based on the logical index.
    ///
    /// This property sets the [`INFO_ID`].
    #[property(CONTEXT, default(usize::MAX))]
    pub fn column(child: impl UiNode, col: impl IntoVar<usize>) -> impl UiNode {
        with_widget_state_modify(child, &INFO_ID, col, CellInfo::default, |u, i, &c| {
            if i.column != c {
                i.column = c;
                u.layout();
            }
        })
    }

    /// Cell row index.
    ///
    /// If not set or out-of-bounds the cell is positioned based on the logical index.
    ///
    /// This property sets the [`INFO_ID`].
    #[property(CONTEXT, default(usize::MAX))]
    pub fn row(child: impl UiNode, row: impl IntoVar<usize>) -> impl UiNode {
        with_widget_state_modify(child, &INFO_ID, row, CellInfo::default, |u, i, &r| {
            if i.row != r {
                i.row = r;
                u.layout();
            }
        })
    }

    /// Cell column span.
    ///
    /// Number of *cells* this one spans over horizontally, starting from the column index and spanning to the right.
    ///
    /// Is `1` by default, the index is clamped between `1..max` where max is the maximum number of valid columns
    /// to the right of the cell column index.
    ///
    /// Note that the cell does not influence the column width if it spans over multiple columns.
    ///
    /// This property sets the [`INFO_ID`].
    #[property(CONTEXT, default(1))]
    pub fn column_span(child: impl UiNode, span: impl IntoVar<usize>) -> impl UiNode {
        with_widget_state_modify(child, &INFO_ID, span, CellInfo::default, |u, i, &s| {
            if i.column_span != s {
                i.column_span = s;
                u.layout();
            }
        })
    }

    /// Cell row span.
    ///
    /// Number of *cells* this one spans over vertically, starting from the row index and spanning down.
    ///
    /// Is `1` by default, the index is clamped between `1..max` where max is the maximum number of valid rows
    /// down from the cell column index.
    ///
    /// Note that the cell does not influence the row height if it spans over multiple rows.
    ///
    /// This property sets the [`INFO_ID`].
    #[property(CONTEXT, default(1))]
    pub fn row_span(child: impl UiNode, span: impl IntoVar<usize>) -> impl UiNode {
        with_widget_state_modify(child, &INFO_ID, span, CellInfo::default, |u, i, &s| {
            if i.row_span != s {
                i.row_span = s;
                u.layout();
            }
        })
    }
}

fn grid_node(
    cells: BoxedUiNodeList,
    columns: BoxedUiNodeList,
    rows: BoxedUiNodeList,
    auto_grow_view: BoxedVar<ViewGenerator<AutoGrowViewArgs>>,
    auto_grow_mode: BoxedVar<AutoGrowMode>,
    spacing: BoxedVar<GridSpacing>,
) -> impl UiNode {
    let auto_columns = EditableUiNodeList::new();
    let auto_columns_ref = auto_columns.reference();
    let auto_rows = EditableUiNodeList::new();
    let auto_rows_ref = auto_rows.reference();
    GridNode {
        children: vec![
            vec![columns, auto_columns.boxed()].boxed(),
            vec![rows, auto_rows.boxed()].boxed(),
            ZSortingList::new(cells).boxed(),
        ],
        auto_columns: auto_columns_ref,
        auto_rows: auto_rows_ref,
        spacing,
        auto_grow_view,
        auto_grow_mode,

        column_info: vec![],
        row_info: vec![],
        auto_imaginary: 0,
    }
}

#[derive(Clone, Copy)]
struct ColumnInfo {
    x: Px,
    width: Px,
}
impl Default for ColumnInfo {
    fn default() -> Self {
        Self {
            x: Px::MIN,
            width: Px::MIN,
        }
    }
}
#[derive(Clone, Copy)]
struct RowInfo {
    y: Px,
    height: Px,
}
impl Default for RowInfo {
    fn default() -> Self {
        Self {
            y: Px::MIN,
            height: Px::MIN,
        }
    }
}

#[ui_node(struct GridNode {
    // [[columns, auto_columns], [rows, auto_rows], cells]
    children: Vec<BoxedUiNodeList>,
    auto_columns: EditableUiNodeListRef,
    auto_rows: EditableUiNodeListRef,
    #[var] auto_grow_view: impl Var<ViewGenerator<AutoGrowViewArgs>>,
    #[var] auto_grow_mode: impl Var<AutoGrowMode>,
    #[var] spacing: impl Var<GridSpacing>,

    column_info: Vec<ColumnInfo>,
    row_info: Vec<RowInfo>,
    auto_imaginary: usize,
})]
impl UiNode for GridNode {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.auto_subs(ctx);
        self.children.init_all(ctx);

        // Set index for column and row.
        self.children[0].for_each_mut(|i, c| {
            c.with_context_mut(|ctx| ctx.widget_state.set(&column::INDEX_ID, i));
            true
        });
        self.children[1].for_each_mut(|i, r| {
            r.with_context_mut(|ctx| ctx.widget_state.set(&row::INDEX_ID, i));
            true
        });

        // collect column/row count needed for auto-grow.
        let auto_mode = self.auto_grow_mode.get();
        let mut max_custom = 0;
        let mut max_auto_placed = 0;
        self.children[2].for_each_mut(|i, c| {
            let info = c.with_context(|ctx| cell::CellInfo::get(ctx.widget_state)).unwrap_or_default();
            if let AutoGrowMode::Rows(_) = auto_mode {
                if info.row != usize::MAX {
                    max_custom = max_custom.max(info.row);
                } else {
                    max_auto_placed = i;
                }
            } else if info.column != usize::MAX {
                max_custom = max_custom.max(info.column);
            } else {
                max_auto_placed = i;
            }

            true
        });

        // auto-grow
        let mut generated_len = 0;
        match auto_mode {
            AutoGrowMode::Rows(max) | AutoGrowMode::Columns(max) => {
                let needed_len = (max_custom.max(max_auto_placed / self.children[0].len()) + 1).min(max as usize);
                let fixed_len = self.children[1].len();
                if needed_len > fixed_len {
                    let view = self.auto_grow_view.get();
                    if !view.is_nil() {
                        let list = match auto_mode {
                            AutoGrowMode::Rows(_) => &self.auto_rows,
                            AutoGrowMode::Columns(_) => &self.auto_columns,
                        };
                        for i in fixed_len..needed_len {
                            let mut auto_item = view.generate(ctx, AutoGrowViewArgs { mode: auto_mode, index: i });
                            auto_item.with_context_mut(|ctx| ctx.widget_state.set(&row::INDEX_ID, i));
                            list.push(ctx, auto_item);
                        }
                        generated_len = needed_len - fixed_len;
                    } else {
                        self.auto_imaginary = needed_len - fixed_len;
                    }
                }
            }
        }

        match auto_mode {
            AutoGrowMode::Rows(_) => {
                self.column_info.resize(self.children[0].len(), ColumnInfo::default());
                self.row_info
                    .resize(self.children[1].len() + generated_len + self.auto_imaginary, RowInfo::default());
            }
            AutoGrowMode::Columns(_) => {
                self.column_info
                    .resize(self.children[0].len() + generated_len + self.auto_imaginary, ColumnInfo::default());
                self.row_info.resize(self.children[1].len(), RowInfo::default());
            }
        }
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.children.deinit_all(ctx);

        match self.auto_grow_mode.get() {
            AutoGrowMode::Rows(_) => {
                self.auto_rows.clear(ctx);
            }
            AutoGrowMode::Columns(_) => {
                self.auto_columns.clear(ctx);
            }
        }

        self.auto_imaginary = 0;

        self.column_info.clear();
        self.row_info.clear();
    }

    fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
        if self.spacing.is_new(ctx) {
            ctx.updates.layout();
        }

        let mut any = false;
        self.children.update_all(ctx, updates, &mut any);

        if any || self.auto_grow_view.is_new(ctx) || self.auto_grow_mode.is_new(ctx) {
            // !!: TODO, support new columns/rows
            ctx.updates.layout();
        }
    }

    fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
        todo!("layout can get very elaborate, how do we avoid massive duplication here?")
    }

    fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        let spacing = self.spacing.get().layout(ctx.metrics, |_| PxGridSpacing::zero());
        let constrains = ctx.constrains();

        let fill_x = constrains.x.fill_or_exact();
        let fill_y = constrains.y.fill_or_exact();

        let mut children = self.children.iter_mut();
        let columns = children.next().unwrap();
        let rows = children.next().unwrap();
        let cells = children.next().unwrap();

        // layout exact columns&rows, mark others for next passes.

        const KIND_RELATIVE: Px = Px(i32::MIN + 1);
        const KIND_DEFAULT: Px = Px(i32::MIN + 2);

        let mut has_default = false;
        let mut has_relative_cols = false;
        let mut has_relative_rows = false;

        columns.for_each_mut(|ci, col| {
            let col_kind = if fill_x.is_some() {
                SizePropertyKind::get_wgt(col).width
            } else {
                SizePropertyKind::Default
            };

            let col_info = &mut self.column_info[ci];

            col_info.x = Px::MIN;

            match col_kind {
                SizePropertyKind::None | SizePropertyKind::Default => {
                    col_info.width = KIND_DEFAULT;
                    has_default = true;
                }
                SizePropertyKind::Relative => {
                    col_info.width = KIND_RELATIVE;
                    has_relative_cols = true;
                }
                SizePropertyKind::Exact => {
                    col_info.width = col.layout(ctx, wl).width;
                }
            }

            true
        });
        rows.for_each_mut(|ri, row| {
            let row_kind = if fill_y.is_some() {
                SizePropertyKind::get_wgt(row).height
            } else {
                SizePropertyKind::Default
            };

            let row_info = &mut self.row_info[ri];

            row_info.y = Px::MIN;

            match row_kind {
                SizePropertyKind::None | SizePropertyKind::Default => {
                    row_info.height = KIND_DEFAULT;
                    has_default = true;
                }
                SizePropertyKind::Relative => {
                    row_info.height = KIND_RELATIVE;
                    has_relative_rows = true;
                }
                SizePropertyKind::Exact => {
                    row_info.height = row.layout(ctx, wl).height;
                }
            }

            true
        });

        // measure cells in DEFAULT columns&rows, collect widest/tallest as a negative length (neg marks partial DEFAULT)
        let columns_len = self.column_info.len();
        if has_default {
            cells.for_each(|i, cell| {
                let info = cell::CellInfo::get_wgt(cell);
                if info.column_span > 1 || info.row_span > 1 {
                    return true; // continue;
                }
                let info = info.actual(i, columns_len);

                let col = &mut self.column_info[info.column];
                let row = &mut self.row_info[info.row];

                // we use negative length to mark partial DEFAULT measures, to calculate the absolute max.
                let abs_max = |a: Px, b: Px| if a == KIND_DEFAULT { -b } else { a.min(-b) };
                if col.width < Px(0) && col.width >= KIND_DEFAULT {
                    if row.height < Px(0) && row.height >= KIND_DEFAULT {
                        // (default, default)
                        let size = cell.measure(&mut ctx.as_measure(), &mut WidgetMeasure::new());

                        col.width = abs_max(col.width, size.width);
                        row.height = abs_max(row.height, size.height);
                    } else if row.height != KIND_RELATIVE {
                        // (default, exact)
                        let size = ctx
                            .as_measure()
                            .with_constrains(|c| c.with_exact_y(row.height), |ctx| cell.measure(ctx, &mut WidgetMeasure::new()));

                        col.width = abs_max(col.width, size.width);
                    } else {
                        todo!("(default, relative)?")
                    }
                } else if col.width != KIND_RELATIVE && row.height < Px(0) && row.height >= KIND_DEFAULT {
                    // (exact, default)
                    let size = ctx
                        .as_measure()
                        .with_constrains(|c| c.with_exact_x(col.width), |ctx| cell.measure(ctx, &mut WidgetMeasure::new()));

                    row.height = abs_max(row.height, size.height);
                } else {
                    todo!("(relative, default)?")
                }
                true
            });

            // finalize, convert to abs length, layout column&row widgets.
            let view_columns_len = columns.len();
            for (i, col) in self.column_info.iter_mut().enumerate() {
                if col.width < Px(0) && col.width > KIND_DEFAULT {
                    col.width = col.width.abs();

                    if i < view_columns_len {
                        // layout column view
                        let size = ctx.with_constrains(
                            |c| c.with_exact_x(col.width),
                            |ctx| columns.with_node_mut(i, |col| col.layout(ctx, wl)),
                        );
                        // final width can change due to min/max
                        col.width = size.width;
                    }
                }
            }
            let view_rows_len = rows.len();
            for (i, row) in self.row_info.iter_mut().enumerate() {
                if row.height < Px(0) && row.height > KIND_DEFAULT {
                    row.height = row.height.abs();

                    if i < view_rows_len {
                        // layout row view
                        let size = ctx.with_constrains(
                            |c| c.with_exact_y(row.height),
                            |ctx| rows.with_node_mut(i, |row| row.layout(ctx, wl)),
                        );
                        // final height can change due to min/max
                        row.height = size.height;
                    }
                }
            }
        }

        // distribute left-over grid space to relative columns
        if has_relative_cols {
            let mut width = fill_x.unwrap(); // relative is converted to auto if not fill.
            let mut relative_count = 0;
            for col in &self.column_info {
                if col.width > Px(0) {
                    width -= col.width - spacing.column;
                } else if col.width == KIND_RELATIVE {
                    relative_count += 1;
                }
            }
            width /= Px(relative_count);
            width -= spacing.column * Px(relative_count - 1);
            width = width.max(Px(0));

            let view_columns_len = columns.len();

            // !!: Problems
            //   - We can have 6 columns with an arrangement of `max_width` that reaches limit one after the other.
            //   - Only 100% columns take on more width, `width = 90.pct()` with no `max_width` should take more too.
            let mut settle_passes = 5;
            loop {
                settle_passes -= 1;
                let is_final_pass = settle_passes == 0;
                let mut extra_width = Px(0);
                let mut settling_count = 0;

                for (i, col) in self.column_info.iter_mut().enumerate() {
                    if col.width != KIND_RELATIVE {
                        continue;
                    }

                    if is_final_pass {
                        col.width = width;
                    }

                    if i < view_columns_len {
                        // measure/layout column view to get extra constrains, like max_width.

                        if is_final_pass {
                            // layout
                            let size = ctx.with_constrains(
                                |c| c.with_fill_x(true).with_max_x(width),
                                |ctx| columns.with_node_mut(i, |col| col.layout(ctx, wl)),
                            );
                            col.width = size.width;
                        } else {
                            // measure to see if column gives away extra width.
                            let size = ctx.as_measure().with_constrains(
                                |c| c.with_fill_x(true).with_max_x(width),
                                |ctx| columns.with_node(i, |col| col.measure(ctx, &mut WidgetMeasure::new())),
                            );

                            let given_width = (width - size.width).max(Px(0));
                            if (given_width) > Px(0) {
                                // reached limit, stopped settling, layout

                                let size = ctx.with_constrains(
                                    |c| c.with_fill_x(true).with_max_x(width),
                                    |ctx| columns.with_node_mut(i, |col| col.layout(ctx, wl)),
                                );
                                col.width = size.width;
                                extra_width += given_width;
                            } else {
                                // can take more width, probably
                                settling_count += 1;
                            }
                        }
                    } else {
                        // relative imaginary column, not a thing currently
                        settling_count += 1;
                    }
                }

                if is_final_pass || settling_count == 0 {
                    break;
                } else if extra_width == Px(0) {
                    // jump to final pass.
                    settle_passes = 1;
                } else {
                    width += extra_width / Px(settling_count);
                }
            }
        }
        // distribute left-over grid space to relative rows
        if has_relative_rows {
            let mut height = fill_y.unwrap();
            let mut relative_count = 0;
            for row in &self.row_info {
                if row.height > Px(0) {
                    height -= row.height - spacing.row;
                } else if row.height == KIND_RELATIVE {
                    relative_count += 1;
                }
            }
            height /= Px(relative_count);
            height -= spacing.row * Px(relative_count - 1);
            height = height.max(Px(0));

            let view_rows_len = rows.len();

            let mut settle_passes = 5;
            loop {
                settle_passes -= 1;
                let is_final_pass = settle_passes == 0;
                let mut extra_height = Px(0);
                let mut settling_count = 0;

                for (i, row) in self.row_info.iter_mut().enumerate() {
                    if row.height != KIND_RELATIVE {
                        continue;
                    }

                    if is_final_pass {
                        row.height = height;
                    }

                    if i < view_rows_len {
                        // measure/layout row view to get extra constrains, like max_height.

                        if is_final_pass {
                            // layout
                            let size = ctx.with_constrains(
                                |c| c.with_fill_y(true).with_max_y(height),
                                |ctx| rows.with_node_mut(i, |row| row.layout(ctx, wl)),
                            );
                            row.height = size.height;
                        } else {
                            // measure to see if row gives away extra height.
                            let size = ctx.as_measure().with_constrains(
                                |c| c.with_fill_y(true).with_max_y(height),
                                |ctx| rows.with_node(i, |row| row.measure(ctx, &mut WidgetMeasure::new())),
                            );

                            let given_height = (height - size.height).max(Px(0));
                            if (given_height) > Px(0) {
                                // reached limit, stopped settling, layout

                                let size = ctx.with_constrains(
                                    |c| c.with_fill_y(true).with_max_y(height),
                                    |ctx| rows.with_node_mut(i, |row| row.layout(ctx, wl)),
                                );
                                row.height = size.height;
                                extra_height += given_height;
                            } else {
                                // can take more height, probably
                                settling_count += 1;
                            }
                        }
                    } else {
                        // relative imaginary row, not a thing currently
                        settling_count += 1;
                    }
                }

                if is_final_pass || settling_count == 0 {
                    break;
                } else if extra_height == Px(0) {
                    // jump to final pass.
                    settle_passes = 1;
                } else {
                    height += extra_height / Px(settling_count);
                }
            }
        }

        // compute column&row offsets
        let mut x = Px(0);
        for col in &mut self.column_info {
            col.x = x;
            x += col.width + spacing.column;
        }
        let mut y = Px(0);
        for row in &mut self.row_info {
            row.y = y;
            y += row.height + spacing.row;
        }
        // translate column&row views
        columns.for_each_mut(|ci, col| {
            let x = self.column_info[ci].x;
            wl.with_outer(col, false, |wl, _| wl.translate(PxVector::new(x, Px(0))));
            true
        });
        rows.for_each_mut(|ri, row| {
            let y = self.row_info[ri].y;
            wl.with_outer(row, false, |wl, _| wl.translate(PxVector::new(Px(0), y)));
            true
        });

        let grid_size = PxSize::new((x - spacing.column).max(Px(0)), (y - spacing.row).max(Px(0)));

        // layout and translate cells
        cells.for_each_mut(|i, cell| {
            let info = cell::CellInfo::get_wgt(cell).actual(i, columns_len);

            if info.column >= self.column_info.len() || info.row >= self.row_info.len() {
                todo!("collapse cell");
                return true;
            }

            let col = self.column_info[info.column];
            let row = self.row_info[info.row];

            if col.width < Px(0) || row.height < Px(0) {
                todo!("left-over specials, collapse?");
            }

            ctx.with_constrains(|c| c.with_exact(col.width, row.height), |ctx| cell.layout(ctx, wl));
            wl.with_outer(cell, false, |wl, _| wl.translate(PxVector::new(col.x, row.y)));

            true
        });

        constrains.fill_size_or(grid_size)
    }
}

/// Arguments for [`grid::auto_grow_view`].
///
/// [`grid::auto_grow_view`]: fn@grid::auto_grow_view.
#[derive(Clone, Debug)]
pub struct AutoGrowViewArgs {
    /// Auto-grow direction.
    pub mode: AutoGrowMode,
    /// Column index.
    pub index: usize,
}

/// Grid auto-grow direction.
///
/// The associated value is the maximum columns or rows that are allowed in the grid.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AutoGrowMode {
    /// Auto generate columns.
    Columns(u32),
    /// Auto generate rows.
    Rows(u32),
}
impl AutoGrowMode {
    /// Value that does not generate any new row or column.
    pub fn disabled() -> Self {
        Self::Rows(0)
    }

    /// Columns, not specific maximum limit.
    pub fn columns() -> Self {
        Self::Columns(u32::MAX)
    }

    /// Rows, not specific maximum limit.
    pub fn rows() -> Self {
        Self::Columns(u32::MAX)
    }

    /// Set the maximum columns or rows allowed.
    pub fn with_limit(self, limit: u32) -> Self {
        match self {
            AutoGrowMode::Columns(_) => AutoGrowMode::Columns(limit),
            AutoGrowMode::Rows(_) => AutoGrowMode::Rows(limit),
        }
    }
}
