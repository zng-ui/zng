use super::*;

/// Grid cell container.
///
/// This widget defines properties that position and size widgets in a [`Grid!`].
///
/// See the [`Grid::cells`] property for more details.
///
/// [`Grid!`]: struct@Grid
#[widget($crate::Cell)]
pub struct Cell(zng_wgt_container::Container);
impl Cell {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            access_role = AccessRole::GridCell;
        }
    }
}

/// Represents values set by cell properties in a widget.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
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
            column: 0,
            column_span: 1,
            row: 0,
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

    /// Get the cell info stored in the [`WIDGET`] state.
    ///
    /// [`WIDGET`]: zng_wgt::prelude::WIDGET
    pub fn get() -> Self {
        WIDGET.get_state(*INFO_ID).unwrap_or_default()
    }

    /// Get the cell info stored in the `wgt` state.
    pub fn get_wgt(wgt: &mut UiNode) -> Self {
        match wgt.as_widget() {
            Some(mut wgt) => wgt.with_context(WidgetUpdateMode::Ignore, Self::get),
            None => CellInfo::default(),
        }
    }
}

static_id! {
    /// Id for widget state set by cell properties.
    ///
    /// The parent grid uses this info to position and size the cell widget.
    pub static ref INFO_ID: StateId<CellInfo>;
}

/// Cell column index.
///
/// If set to [`usize::MAX`] the cell is positioned based on the logical index.
///
/// Is `0` by default.
///
/// This property sets the [`INFO_ID`].
///
/// See also the [`at`] property to bind both indexes at the same time.
///
/// [`at`]: fn@at
#[property(CONTEXT, default(0), widget_impl(Cell))]
pub fn column(child: impl IntoUiNode, col: impl IntoVar<usize>) -> UiNode {
    with_widget_state_modify(child, *INFO_ID, col, CellInfo::default, |i, &c| {
        if i.column != c {
            i.column = c;
            WIDGET.layout();
        }
    })
}

/// Cell row index.
///
/// If set to [`usize::MAX`] the cell is positioned based on the logical index.
///
/// Is `0` by default.
///
/// This property sets the [`INFO_ID`].
///
/// See also the [`at`] property to bind both indexes at the same time.
///
/// [`at`]: fn@at
#[property(CONTEXT, default(0), widget_impl(Cell))]
pub fn row(child: impl IntoUiNode, row: impl IntoVar<usize>) -> UiNode {
    with_widget_state_modify(child, *INFO_ID, row, CellInfo::default, |i, &r| {
        if i.row != r {
            i.row = r;
            WIDGET.layout();
        }
    })
}

/// Cell column and row indexes.
///
/// If set to [`AT_AUTO`] the cell is positioned based on the logical index.
///
/// Is `(0, 0)` by default.
///
/// This property sets the [`INFO_ID`].
///
/// See also the [`column`] or [`row`] properties to bind each index individually.
///
/// [`column`]: fn@column
/// [`row`]: fn@row
#[property(CONTEXT, default((0, 0)), widget_impl(Cell))]
pub fn at(child: impl IntoUiNode, column_row: impl IntoVar<(usize, usize)>) -> UiNode {
    with_widget_state_modify(child, *INFO_ID, column_row, CellInfo::default, |i, &(col, row)| {
        if i.column != col || i.row != row {
            i.column = col;
            i.row = row;
            WIDGET.layout();
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
/// Note that the cell will not influence the column width if it spans over multiple columns.
///
/// This property sets the [`INFO_ID`].
///
/// See also the [`span`] property to bind both spans at the same time.
///
/// [`span`]: fn@span
#[property(CONTEXT, default(1), widget_impl(Cell))]
pub fn column_span(child: impl IntoUiNode, span: impl IntoVar<usize>) -> UiNode {
    with_widget_state_modify(child, *INFO_ID, span, CellInfo::default, |i, &s| {
        if i.column_span != s {
            i.column_span = s;
            WIDGET.layout();
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
/// Note that the cell will not influence the row height if it spans over multiple rows.
///
/// This property sets the [`INFO_ID`].
///
/// See also the [`span`] property to bind both spans at the same time.
///
/// [`span`]: fn@span
#[property(CONTEXT, default(1), widget_impl(Cell))]
pub fn row_span(child: impl IntoUiNode, span: impl IntoVar<usize>) -> UiNode {
    with_widget_state_modify(child, *INFO_ID, span, CellInfo::default, |i, &s| {
        if i.row_span != s {
            i.row_span = s;
            WIDGET.layout();
        }
    })
}

/// Cell column and row span.
///
/// Is `(1, 1)` by default.
///
/// This property sets the [`INFO_ID`].
///
/// See also the [`column_span`] or [`row_span`] properties to bind each index individually.
///
/// [`column_span`]: fn@column_span
/// [`row_span`]: fn@row_span
#[property(CONTEXT, default((1, 1)), widget_impl(Cell))]
pub fn span(child: impl IntoUiNode, span: impl IntoVar<(usize, usize)>) -> UiNode {
    with_widget_state_modify(child, *INFO_ID, span, CellInfo::default, |i, &(cs, rs)| {
        if i.column_span != rs || i.row_span != rs {
            i.column_span = cs;
            i.row_span = rs;
            WIDGET.layout();
        }
    })
}

/// Value for [`at`] that causes the cell to be positioned based on the logical index *i*,
/// for columns *i % columns* and for rows *i / columns*.
///
/// [`at`]: fn@at
pub const AT_AUTO: (usize, usize) = (usize::MAX, usize::MAX);
