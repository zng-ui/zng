use std::fmt;

use crate::cell;
use crate::column;
use crate::row;
use zng_layout::unit::Factor;
use zng_layout::unit::GridSpacing;
use zng_layout::unit::Px;
use zng_layout::unit::PxGridSpacing;
use zng_wgt::prelude::*;
use zng_wgt_size_offset::WIDGET_SIZE;
use zng_wgt_size_offset::WidgetLength;

use super::AutoGrowFnArgs;
use super::AutoGrowMode;

#[derive(Clone, Copy)]
pub(crate) struct ColRowMeta(f32);

impl ColRowMeta {
    /// `width` or `height` contains the largest cell or `Px::MIN` if cell measure is pending.
    pub(crate) fn is_default(self) -> bool {
        self.0.is_sign_negative() && self.0.is_infinite()
    }

    /// Return the leftover factor if the column or row must be measured on a fraction of the leftover space.
    pub(crate) fn is_leftover(self) -> Option<Factor> {
        if self.0 >= 0.0 { Some(Factor(self.0)) } else { None }
    }

    /// `width` or `height` contains the final length or is pending layout `Px::MIN`.
    pub(crate) fn is_exact(self) -> bool {
        self.0.is_nan()
    }

    pub(crate) fn exact() -> Self {
        Self(f32::NAN)
    }

    pub(crate) fn leftover(f: Factor) -> Self {
        Self(f.0.max(0.0))
    }
}

impl Default for ColRowMeta {
    fn default() -> Self {
        Self(f32::NEG_INFINITY)
    }
}

impl fmt::Debug for ColRowMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_default() {
            write!(f, "default")
        } else if self.is_exact() {
            write!(f, "exact")
        } else if let Some(l) = self.is_leftover() {
            write!(f, "leftover({l})")
        } else {
            write!(f, "ColRowMeta({})", self.0)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ColumnLayout {
    pub(crate) meta: ColRowMeta,
    pub(crate) was_leftover: bool,
    pub(crate) x: Px,
    pub(crate) width: Px,
    pub(crate) min_width: Px,
    pub(crate) max_width: Px,
}

impl Default for ColumnLayout {
    fn default() -> Self {
        Self {
            meta: ColRowMeta::default(),
            was_leftover: false,
            x: Px::MIN,
            width: Px::MIN,
            min_width: Px::MIN,
            max_width: Px::MAX,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RowLayout {
    pub(crate) meta: ColRowMeta,
    pub(crate) was_leftover: bool,
    pub(crate) y: Px,
    pub(crate) height: Px,
    pub(crate) min_height: Px,
    pub(crate) max_height: Px,
}

impl Default for RowLayout {
    fn default() -> Self {
        Self {
            meta: ColRowMeta::default(),
            was_leftover: false,
            y: Px::MIN,
            height: Px::MIN,
            min_height: Px::MIN,
            max_height: Px::MAX,
        }
    }
}

#[derive(Default)]
pub(crate) struct GridLayout {
    pub(crate) columns: Vec<ColumnLayout>,
    pub(crate) rows: Vec<RowLayout>,
}

impl GridLayout {
    pub(crate) fn is_collapse(&self) -> bool {
        self.columns.is_empty() || self.rows.is_empty()
    }

    pub(crate) fn collapse(&mut self) {
        self.columns.clear();
        self.rows.clear();
    }

    /// add/remove info entries, auto-grow/shrink
    pub(crate) fn update_entries(
        &mut self,
        children: &mut GridChildren,
        auto_mode: AutoGrowMode,
        auto_grow_fn: &Var<WidgetFn<AutoGrowFnArgs>>,
    ) {
        let mut children = GridChildrenMut(children);

        // max needed column or row in the auto_mode axis.
        let mut max_custom = 0;
        let mut max_auto_placed_i = 0;
        children.cells().for_each_child(|i, c, _| {
            let info = cell::CellInfo::get_wgt(c);

            let n = match auto_mode {
                AutoGrowMode::Rows(_) => info.row,
                AutoGrowMode::Columns(_) => info.column,
            };
            if n == usize::MAX {
                max_auto_placed_i = i;
            } else {
                max_custom = max_custom.max(n);
            }
        });

        let mut imaginary_cols = 0;
        let mut imaginary_rows = 0;

        match auto_mode {
            AutoGrowMode::Rows(max) => {
                let columns_len = children.all_columns().children_len();
                if columns_len == 0 {
                    tracing::warn!(
                        "grid {} has no columns and auto_grow_mode={:?}, no cell will be visible",
                        WIDGET.id(),
                        auto_mode,
                    );
                    self.collapse();
                    return;
                }

                let max_auto_placed = max_auto_placed_i / columns_len;
                let max_needed_len = max_auto_placed.max(max_custom).min(max as usize) + 1;

                let rows_len = children.all_rows().children_len();

                if rows_len < max_needed_len {
                    let auto = children.auto_rows();
                    let mut index = rows_len;

                    let view = auto_grow_fn.get();
                    if view.is_nil() {
                        imaginary_rows = max_needed_len - rows_len;
                    } else {
                        while index < max_needed_len {
                            let mut row = view(AutoGrowFnArgs { mode: auto_mode, index });
                            row.init();
                            auto.push(row);
                            index += 1;
                        }
                    }
                } else if rows_len > max_needed_len {
                    let remove = rows_len - max_needed_len;
                    let auto = children.auto_rows();
                    let s = auto.len().saturating_sub(remove);
                    for mut auto in auto.drain(s..) {
                        auto.deinit();
                    }
                }
            }
            AutoGrowMode::Columns(max) => {
                let rows_len = children.all_rows().children_len();
                if rows_len == 0 {
                    tracing::warn!(
                        "grid {} has no rows and auto_grow_mode={:?}, no cell will be visible",
                        WIDGET.id(),
                        auto_mode,
                    );
                    self.collapse();
                    return;
                }

                let max_auto_placed = max_auto_placed_i / rows_len;
                let max_needed_len = max_auto_placed.max(max_custom).min(max as usize) + 1;

                let cols_len = children.all_columns().children_len();

                if cols_len < max_needed_len {
                    let auto = children.auto_columns();
                    let mut index = cols_len;

                    let view = auto_grow_fn.get();
                    if view.is_nil() {
                        imaginary_cols = max_needed_len - cols_len;
                    } else {
                        while index < max_needed_len {
                            let mut column = view(AutoGrowFnArgs { mode: auto_mode, index });
                            column.init();
                            auto.push(column);
                            index += 1;
                        }
                    }
                } else if cols_len > max_needed_len {
                    let remove = cols_len - max_needed_len;
                    let auto = children.auto_columns();
                    let s = auto.len().saturating_sub(remove);
                    for mut auto in auto.drain(s..) {
                        auto.deinit();
                    }
                }
            }
        }

        // Set index for column and row.
        let columns_len = children.all_columns().children_len() + imaginary_cols;
        children.all_columns_node().for_each_child(|i, c| {
            if let Some(mut wgt) = c.as_widget() {
                wgt.with_context(WidgetUpdateMode::Bubble, || {
                    let prev = WIDGET.set_state(*column::INDEX_ID, (i, columns_len));
                    if prev != Some((i, columns_len)) {
                        WIDGET.update();
                    }
                });
            }
        });
        let rows_len = children.all_rows().children_len() + imaginary_rows;
        children.all_rows_node().for_each_child(|i, r| {
            if let Some(mut wgt) = r.as_widget() {
                wgt.with_context(WidgetUpdateMode::Bubble, || {
                    let prev = WIDGET.set_state(*row::INDEX_ID, (i, rows_len));
                    if prev != Some((i, rows_len)) {
                        WIDGET.update();
                    }
                });
            }
        });

        self.columns.resize(columns_len, ColumnLayout::default());
        self.rows.resize(rows_len, RowLayout::default());
    }

    #[must_use]
    pub(crate) fn grid_layout(
        &mut self,
        wm: &mut WidgetMeasure,
        children: &mut GridChildren,
        spacing: &Var<GridSpacing>,
    ) -> (PxGridSpacing, PxSize) {
        if self.is_collapse() {
            return (PxGridSpacing::zero(), PxSize::zero());
        }

        let spacing = spacing.layout();
        let constraints = LAYOUT.constraints();

        let fill_x = constraints.x.fill_or_exact();
        let fill_y = constraints.y.fill_or_exact();

        let constraints = constraints.with_new_min(Px(0), Px(0));

        let mut children = GridChildrenMut(children);
        let mut children = children.children().iter_mut();
        let columns = children.next().unwrap();
        let rows = children.next().unwrap();
        let cells = children.next().unwrap();

        // layout exact columns&rows, mark others for next passes.

        let mut has_default = false;
        let mut has_leftover_cols = false;
        let mut has_leftover_rows = false;
        pub(crate) const MAX_PROBE: i32 = Px::MAX.0 - 1000;

        columns.for_each_child(|ci, col| {
            let col_kind = WIDGET_SIZE.get_wgt(col).width;

            let col_info = &mut self.columns[ci];

            col_info.x = Px::MIN;
            col_info.width = Px::MIN;
            col_info.min_width = Px::MIN;
            col_info.max_width = Px::MAX;

            match col_kind {
                WidgetLength::Default => {
                    col_info.meta = ColRowMeta::default();
                    has_default = true;
                }
                WidgetLength::Leftover(f) => {
                    col_info.meta = ColRowMeta::leftover(f);
                    col_info.was_leftover = true;
                    has_leftover_cols = true;
                }
                WidgetLength::Exact => {
                    col_info.width = LAYOUT.with_constraints(Align::TOP_LEFT.child_constraints(constraints), || col.measure(wm).width);
                    col_info.meta = ColRowMeta::exact();
                }
            }
            if matches!(col_kind, WidgetLength::Default | WidgetLength::Leftover(_)) {
                col_info.min_width = LAYOUT.with_constraints(PxConstraints2d::new_unbounded(), || col.measure(wm)).width;
                col_info.max_width = LAYOUT
                    .with_constraints(
                        PxConstraints2d::new_fill(Px(MAX_PROBE), Px(MAX_PROBE)).with_inner(true, true),
                        || col.measure(wm),
                    )
                    .width;
                if col_info.max_width == MAX_PROBE {
                    col_info.max_width = Px::MAX;
                }
            }
        });
        rows.for_each_child(|ri, row| {
            let row_kind = WIDGET_SIZE.get_wgt(row).height;

            let row_info = &mut self.rows[ri];

            row_info.y = Px::MIN;
            row_info.height = Px::MIN;

            match row_kind {
                WidgetLength::Default => {
                    row_info.meta = ColRowMeta::default();
                    has_default = true;
                }
                WidgetLength::Leftover(f) => {
                    row_info.meta = ColRowMeta::leftover(f);
                    row_info.was_leftover = true;
                    has_leftover_rows = true;
                }
                WidgetLength::Exact => {
                    row_info.height = LAYOUT.with_constraints(Align::TOP_LEFT.child_constraints(constraints), || row.measure(wm).height);
                    row_info.meta = ColRowMeta::exact();
                }
            }
            if matches!(row_kind, WidgetLength::Default | WidgetLength::Leftover(_)) {
                row_info.min_height = LAYOUT.with_constraints(PxConstraints2d::new_unbounded(), || row.measure(wm)).height;
                row_info.max_height = LAYOUT
                    .with_constraints(
                        PxConstraints2d::new_fill(Px(MAX_PROBE), Px(MAX_PROBE)).with_inner(true, true),
                        || row.measure(wm),
                    )
                    .width;
                if row_info.max_height == MAX_PROBE {
                    row_info.max_height = Px::MAX;
                }
            }
        });

        // reset imaginary
        for col in &mut self.columns[columns.children_len()..] {
            col.meta = ColRowMeta::default();
            col.x = Px::MIN;
            col.width = Px::MIN;
            col.min_width = Px::MIN;
            col.max_width = Px::MAX;
            has_default = true;
        }
        for row in &mut self.rows[rows.children_len()..] {
            row.meta = ColRowMeta::default();
            row.y = Px::MIN;
            row.height = Px::MIN;
            row.min_height = Px::MIN;
            row.max_height = Px::MAX;
            has_default = true;
        }

        // Measure cells when needed, collect widest/tallest.
        // - For `Default` columns&rows to get their size.
        // - For `leftover` columns&rows when the grid is not fill or exact size, to get the `1.lft()` length.
        // - For leftover x default a second pass later in case the constrained leftover causes a different default.
        let mut has_leftover_x_default = false;
        let columns_len = self.columns.len();
        if has_default || (fill_x.is_none() && has_leftover_cols) || (fill_y.is_none() && has_leftover_rows) {
            cells.for_each_child(|i, cell| {
                let cell_info = cell::CellInfo::get_wgt(cell);
                if cell_info.column_span > 1 || cell_info.row_span > 1 {
                    return; // continue;
                }
                let cell_info = cell_info.actual(i, columns_len);

                let col = &mut self.columns[cell_info.column];
                let row = &mut self.rows[cell_info.row];

                let col_is_default = col.meta.is_default() || (fill_x.is_none() && col.meta.is_leftover().is_some());
                let col_is_exact = !col_is_default && col.meta.is_exact();
                let col_is_leftover = !col_is_default && col.meta.is_leftover().is_some();

                let row_is_default = row.meta.is_default() || (fill_y.is_none() && row.meta.is_leftover().is_some());
                let row_is_exact = !row_is_default && row.meta.is_exact();
                let row_is_leftover = !row_is_default && row.meta.is_leftover().is_some();

                if col_is_default {
                    if row_is_default {
                        // (default, default)
                        let size = LAYOUT.with_constraints(
                            PxConstraints2d::new_range(col.min_width, col.max_width, row.min_height, row.max_height),
                            || cell.measure(wm),
                        );

                        col.width = col.width.max(size.width.clamp(col.min_width, col.max_width));
                        row.height = row.height.max(size.height);
                    } else if row_is_exact {
                        // (default, exact)
                        let size = LAYOUT.with_constraints(
                            PxConstraints2d::new_range(col.min_width, col.max_width, row.height, row.height),
                            || cell.measure(wm),
                        );

                        col.width = col.width.max(size.width.clamp(col.min_width, col.max_width));
                    } else {
                        debug_assert!(row_is_leftover);
                        // (default, leftover)
                        let size = LAYOUT.with_constraints(
                            PxConstraints2d::new_range(col.min_width, col.max_width, row.min_height, row.max_height),
                            || cell.measure(wm),
                        );

                        col.width = col.width.max(size.width.clamp(col.min_width, col.max_width));

                        has_leftover_x_default = true;
                    }
                } else if col_is_exact {
                    if row_is_default {
                        // (exact, default)
                        let size = LAYOUT.with_constraints(
                            PxConstraints2d::new_range(col.width, col.width, row.min_height, row.max_height),
                            || cell.measure(wm),
                        );

                        row.height = row.height.max(size.height.clamp(row.min_height, row.max_height));
                    }
                } else if row_is_default {
                    debug_assert!(col_is_leftover);
                    // (leftover, default)
                    let size = LAYOUT.with_constraints(
                        PxConstraints2d::new_range(col.min_width, col.max_width, row.min_height, row.max_height),
                        || cell.measure(wm),
                    );

                    row.height = row.height.max(size.height.clamp(row.min_height, row.max_height));

                    has_leftover_x_default = true;
                }
            });
        }

        // distribute leftover grid space to columns
        if has_leftover_cols {
            let mut no_fill_1_lft = Px(0);
            let mut used_width = Px(0);
            let mut total_factor = Factor(0.0);
            let mut leftover_count = 0;
            let mut max_factor = 0.0_f32;

            for col in &mut self.columns {
                if let Some(f) = col.meta.is_leftover() {
                    if fill_x.is_none() {
                        no_fill_1_lft = no_fill_1_lft.max(col.width);
                        col.width = Px::MIN;
                    }
                    max_factor = max_factor.max(f.0);
                    total_factor += f;
                    leftover_count += 1;
                } else if col.width > Px(0) {
                    used_width += col.width;
                }
            }

            // handle big leftover factors
            if total_factor.0.is_infinite() {
                total_factor = Factor(0.0);

                if max_factor.is_infinite() {
                    // +inf takes all space
                    for col in &mut self.columns {
                        if let Some(f) = col.meta.is_leftover() {
                            if f.0.is_infinite() {
                                col.meta = ColRowMeta::leftover(Factor(1.0));
                                total_factor.0 += 1.0;
                            } else {
                                col.meta = ColRowMeta::leftover(Factor(0.0));
                            }
                        }
                    }
                } else {
                    // scale down every factor to fit
                    let scale = f32::MAX / max_factor / leftover_count as f32;
                    for col in &mut self.columns {
                        if let Some(f) = col.meta.is_leftover() {
                            let f = Factor(f.0 * scale);
                            col.meta = ColRowMeta::leftover(f);
                            total_factor += f;
                        }
                    }
                }
            }

            // individual factors under `1.0` behave like `Length::Factor`.
            if total_factor < Factor(1.0) {
                total_factor = Factor(1.0);
            }

            let mut leftover_width = if let Some(w) = fill_x {
                let vis_columns = self.columns.iter().filter(|c| c.width != Px(0)).count() as i32;
                w - used_width - spacing.column * Px(vis_columns - 1).max(Px(0))
            } else {
                // grid has no width, so `1.lft()` is defined by the widest cell measured using `Default` constraints.
                let mut unbounded_width = used_width;
                for col in &self.columns {
                    if let Some(f) = col.meta.is_leftover() {
                        unbounded_width += no_fill_1_lft * f;
                    }
                }
                let bounded_width = constraints.x.clamp(unbounded_width);
                bounded_width - used_width
            };
            leftover_width = leftover_width.max(Px(0));

            let view_columns_len = columns.children_len();

            // find extra leftover space from columns that can't fully fill their requested leftover length.
            let mut settled_all = false;
            while !settled_all && leftover_width > Px(0) {
                settled_all = true;

                for col in self.columns[..view_columns_len].iter_mut() {
                    let lft = if let Some(lft) = col.meta.is_leftover() {
                        lft
                    } else {
                        continue;
                    };

                    let width = lft.0 * leftover_width.0 as f32 / total_factor.0;
                    let width = Px(width as i32);
                    col.width = width.clamp(col.min_width, col.max_width);

                    if col.width != width {
                        // reached a max/min, convert this column to "exact" and remove it from
                        // the leftover pool.
                        settled_all = false;

                        col.meta = ColRowMeta::exact();

                        if col.width != Px(0) {
                            leftover_width -= col.width + spacing.column;
                            total_factor -= lft;
                            if total_factor < Factor(1.0) {
                                total_factor = Factor(1.0);
                            }
                        }
                    }
                }
            }

            leftover_width = leftover_width.max(Px(0));

            // finish settled leftover columns that can fill the requested leftover length.
            for col in &mut self.columns {
                let lft = if let Some(lft) = col.meta.is_leftover() {
                    lft
                } else {
                    continue;
                };

                let width = lft.0 * leftover_width.0 as f32 / total_factor.0;
                col.width = Px(width as i32).clamp(col.min_width, col.max_width);
                col.meta = ColRowMeta::exact();
            }
        }

        // distribute leftover grid space to rows
        if has_leftover_rows {
            let mut no_fill_1_lft = Px(0);
            let mut used_height = Px(0);
            let mut total_factor = Factor(0.0);
            let mut leftover_count = 0;
            let mut max_factor = 0.0_f32;

            for row in &mut self.rows {
                if let Some(f) = row.meta.is_leftover() {
                    if fill_y.is_none() {
                        no_fill_1_lft = no_fill_1_lft.max(row.height);
                        row.height = Px::MIN;
                    }
                    max_factor = max_factor.max(f.0);
                    total_factor += f;
                    leftover_count += 1;
                } else if row.height > Px(0) {
                    used_height += row.height;
                }
            }

            // handle big leftover factors
            if total_factor.0.is_infinite() {
                total_factor = Factor(0.0);

                if max_factor.is_infinite() {
                    // +inf takes all space
                    for row in &mut self.rows {
                        if let Some(f) = row.meta.is_leftover() {
                            if f.0.is_infinite() {
                                row.meta = ColRowMeta::leftover(Factor(1.0));
                                total_factor.0 += 1.0;
                            } else {
                                row.meta = ColRowMeta::leftover(Factor(0.0));
                            }
                        }
                    }
                } else {
                    // scale down every factor to fit
                    let scale = f32::MAX / max_factor / leftover_count as f32;
                    for row in &mut self.rows {
                        if let Some(f) = row.meta.is_leftover() {
                            let f = Factor(f.0 * scale);
                            row.meta = ColRowMeta::leftover(f);
                            total_factor += f;
                        }
                    }
                }
            }

            // individual factors under `1.0` behave like `Length::Factor`.
            if total_factor < Factor(1.0) {
                total_factor = Factor(1.0);
            }

            let mut leftover_height = if let Some(h) = fill_y {
                let vis_rows = self.rows.iter().filter(|c| c.height != Px(0)).count() as i32;
                h - used_height - spacing.row * Px(vis_rows - 1).max(Px(0))
            } else {
                // grid has no height, so `1.lft()` is defined by the tallest cell measured using `Default` constraints.
                let mut unbounded_height = used_height;
                for row in &self.rows {
                    if let Some(f) = row.meta.is_leftover() {
                        unbounded_height += no_fill_1_lft * f;
                    }
                }
                let bounded_height = constraints.x.clamp(unbounded_height);
                bounded_height - used_height
            };
            leftover_height = leftover_height.max(Px(0));

            let view_rows_len = rows.children_len();

            // find extra leftover space from leftover that can't fully fill their requested leftover length.
            let mut settled_all = false;
            while !settled_all && leftover_height > Px(0) {
                settled_all = true;

                for row in self.rows[..view_rows_len].iter_mut() {
                    let lft = if let Some(lft) = row.meta.is_leftover() {
                        lft
                    } else {
                        continue;
                    };

                    let height = lft.0 * leftover_height.0 as f32 / total_factor.0;
                    let height = Px(height as i32);
                    row.height = height.clamp(row.min_height, row.max_height);

                    if row.height != height {
                        // reached a max/min, convert this row to "exact" and remove it from
                        // the leftover pool.
                        settled_all = false;

                        row.meta = ColRowMeta::exact();

                        if row.height != Px(0) {
                            leftover_height -= row.height + spacing.row;
                            total_factor -= lft;
                            if total_factor < Factor(1.0) {
                                total_factor = Factor(1.0);
                            }
                        }
                    }
                }
            }

            leftover_height = leftover_height.max(Px(0));

            // finish settled leftover rows that can fill the requested leftover length.
            for row in &mut self.rows {
                let lft = if let Some(lft) = row.meta.is_leftover() {
                    lft
                } else {
                    continue;
                };

                let height = lft.0 * leftover_height.0 as f32 / total_factor.0;
                row.height = Px(height as i32).clamp(row.min_height, row.max_height);
                row.meta = ColRowMeta::exact();
            }
        }

        if has_leftover_x_default {
            // second measure pass with constrained leftovers to get a more accurate default

            let c = constraints;

            cells.for_each_child(|i, cell| {
                let cell_info = cell::CellInfo::get_wgt(cell);
                if cell_info.column_span > 1 || cell_info.row_span > 1 {
                    return; // continue;
                }

                let cell_info = cell_info.actual(i, columns_len);

                let col = &mut self.columns[cell_info.column];
                let row = &mut self.rows[cell_info.row];

                let col_is_default = col.meta.is_default() || (fill_x.is_none() && col.was_leftover);
                let col_is_leftover = col.was_leftover;

                let row_is_default = row.meta.is_default() || (fill_y.is_none() && row.was_leftover);
                let row_is_leftover = row.was_leftover;

                if col_is_default {
                    if row_is_leftover {
                        // (default, leftover)

                        let size = LAYOUT.with_constraints(c.with_fill(false, false).with_exact_y(row.height), || cell.measure(wm));

                        col.width = col.width.max(size.width);
                    }
                } else if row_is_default && col_is_leftover {
                    // (leftover, default)

                    let size = LAYOUT.with_constraints(c.with_fill(false, false).with_exact_x(col.width), || cell.measure(wm));

                    row.height = row.height.max(size.height);
                }
            });
        }

        // compute column&row offsets
        let mut x = Px(0);
        let mut s = Px(0);
        for col in &mut self.columns {
            if col.width <= Px(0) {
                // collapsed or never measured (no cells)
                continue;
            }
            x += s;
            col.x = x;
            s = spacing.column;
            x += col.width;
        }
        let mut y = Px(0);
        let mut s = Px(0);
        for row in &mut self.rows {
            if row.height <= Px(0) {
                // collapsed or never measured (no cells)
                continue;
            }
            y += s;
            row.y = y;
            s = spacing.row;
            y += row.height;
        }

        let max_width = constraints.x.max().unwrap_or(Px::MAX);
        if max_width > Px(0) && x > max_width {
            // width overflow

            let max_height = constraints.y.max().unwrap_or(Px::MAX);
            if y < max_height && self.columns.iter().any(|c| c.meta.is_default()) && self.rows.iter().all(|r| r.meta.is_default()) {
                // height has space to grow
                // AND has at least one column that can still change width
                // AND all rows can still change height

                // find cell minimum width
                cells.for_each_child(|i, cell| {
                    let cell_info = cell::CellInfo::get_wgt(cell);
                    if cell_info.column_span > 1 || cell_info.row_span > 1 {
                        return; // continue;
                    }

                    let cell_info = cell_info.actual(i, columns_len);
                    let col = &mut self.columns[cell_info.column];

                    if col.meta.is_default() {
                        let row = &mut self.rows[cell_info.row];
                        debug_assert!(row.meta.is_default());

                        // get cell minimum width (0 max constraint means collapse so we give it at least one pixel)
                        let min_w_size = LAYOUT.with_constraints(
                            PxConstraints2d::new_range(col.min_width, col.min_width.max(Px(1)), row.min_height, row.max_height)
                                .with_inner(true, true),
                            || cell.measure(wm),
                        );

                        col.min_width = col.min_width.max(min_w_size.width);
                    }
                });

                // starting with all default columns at col.min_width, distribute the available space proportionate to
                // the "give" that is `col.width - col.min_width`

                // grid width if all default sized columns are set to min_width
                let mut min_width = Px(0);
                let mut s = Px(0);
                for col in &self.columns {
                    if col.width <= Px(0) {
                        continue;
                    }
                    min_width += s;
                    s = spacing.column;
                    min_width += if col.meta.is_default() { col.min_width } else { col.width };
                }
                let min_width = min_width;

                // sum total of default sized columns "give"
                let total_give: Px = self
                    .columns
                    .iter()
                    .filter(|c| c.meta.is_default())
                    .map(|c| (c.width - c.min_width).max(Px(0)))
                    .sum();

                // available grid growth
                let available_width = max_width - min_width;

                if available_width > Px(0) && total_give > Px(0) {
                    // proportionally distribute the available growth width
                    // columns with a large "give" get more space
                    let available_width = available_width.0 as f32;
                    let total_give = total_give.0 as f32;
                    for col in &mut self.columns {
                        if col.meta.is_default() {
                            let give = (col.width - col.min_width).max(Px(0)).0 as f32;
                            let share = available_width * (give / total_give);
                            col.width = col.min_width + Px(share as i32);
                        }
                    }
                } else {
                    // sum of mins already overflows or default sized columns have no give,
                    // just collapse everything to minimums
                    for col in &mut self.columns {
                        if col.meta.is_default() {
                            col.width = col.min_width;
                        }
                    }
                }

                // measure with final column widths to find final row heights
                for row in &mut self.rows {
                    row.height = row.min_height;
                }
                cells.for_each_child(|i, cell| {
                    let cell_info = cell::CellInfo::get_wgt(cell);
                    if cell_info.column_span > 1 || cell_info.row_span > 1 {
                        return; // continue;
                    }

                    let cell_info = cell_info.actual(i, columns_len);
                    let col = &mut self.columns[cell_info.column];

                    if col.meta.is_default() {
                        let row = &mut self.rows[cell_info.row];
                        let height = LAYOUT
                            .with_constraints(
                                PxConstraints2d::new_range(col.width, col.width, row.min_height, row.max_height),
                                || cell.measure(wm),
                            )
                            .height;

                        row.height = row.height.max(height.clamp(row.min_height, row.max_height));
                    }
                });

                // compute column&row offsets again
                x = Px(0);
                let mut s = Px(0);
                for col in &mut self.columns {
                    if col.width <= Px(0) {
                        continue;
                    }
                    x += s;
                    col.x = x;
                    s = spacing.column;
                    x += col.width;
                }
                y = Px(0);
                let mut s = Px(0);
                for row in &mut self.rows {
                    if row.height <= Px(0) {
                        continue;
                    }
                    y += s;
                    row.y = y;
                    s = spacing.row;
                    y += row.height;
                }
            }
        }

        (spacing, PxSize::new(x.max(Px(0)), y.max(Px(0))))
    }
}

/// [[columns, auto_columns], [rows, auto_rows], cells]
pub(crate) type GridChildren = UiNode;

pub(crate) struct GridChildrenMut<'a>(pub(crate) &'a mut GridChildren);

impl<'a> GridChildrenMut<'a> {
    pub(crate) fn children(&mut self) -> &mut UiVec {
        self.0.downcast_mut().unwrap()
    }

    pub(crate) fn all_columns_node(&mut self) -> &mut UiNode {
        &mut self.children()[0]
    }
    pub(crate) fn all_columns(&mut self) -> &mut ChainList {
        self.all_columns_node().downcast_mut().unwrap()
    }
    pub(crate) fn auto_columns(&mut self) -> &mut UiVec {
        self.all_columns().0[1].downcast_mut().unwrap()
    }

    pub(crate) fn all_rows_node(&mut self) -> &mut UiNode {
        &mut self.children()[1]
    }
    pub(crate) fn all_rows(&mut self) -> &mut ChainList {
        self.all_rows_node().downcast_mut().unwrap()
    }
    pub(crate) fn auto_rows(&mut self) -> &mut UiVec {
        self.all_rows().0[1].downcast_mut().unwrap()
    }

    pub(crate) fn cells(&mut self) -> &mut PanelList {
        self.children()[2].downcast_mut().unwrap()
    }
}
