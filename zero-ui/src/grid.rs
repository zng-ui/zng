//! Grid layout widgets.
//!
//! # Full API
//!
//! See [`zero_ui_wgt_grid`] for the full widget API.

pub use zero_ui_wgt_grid::{node, AutoGrowFnArgs, AutoGrowMode, Cell, Column, Grid, Row};

/// Cell widget and properties.
pub mod cell {
    pub use zero_ui_wgt_grid::cell::{at, column, column_span, row, row_span, span, Cell, CellInfo, AT_AUTO};
}

/// Column widget and properties.
pub mod column {
    pub use zero_ui_wgt_grid::column::{
        get_index, get_index_fct, get_index_len, get_rev_index, is_even, is_first, is_last, is_odd, Column,
    };
}

/// Row widget and properties.
pub mod row {
    pub use zero_ui_wgt_grid::row::{get_index, get_index_fct, get_index_len, get_rev_index, is_even, is_first, is_last, is_odd, Row};
}
