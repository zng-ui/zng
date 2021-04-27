use crate::prelude::new_widget::*;
use crate::properties::grid::GridChildState;

struct GridNode {
    columns: Vec<ColumnDef>,
    rows: Vec<RowDef>,
    children: Vec<Box<dyn Widget>>,
}
#[impl_ui_node(children)]
impl UiNode for GridNode {
    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        let mut size = LayoutSize::zero();
        for child in &mut self.children {
            child.measure(available_size, ctx);
            let _column = child.column();
            // TODO
        }
        size
    }
}

/// A grid column definition.
#[derive(Debug, Clone)]
pub struct ColumnDef {
    /// Minimal exact width.
    pub min_width: f32,
    /// Maximum exact width.
    pub max_width: f32,
    /// Width configuration.
    pub width: GridLength,
}
impl Default for ColumnDef {
    fn default() -> Self {
        ColumnDef {
            min_width: 0.0,
            max_width: f32::MAX,
            width: GridLength::default(),
        }
    }
}
impl ColumnDef {
    pub fn new(width: GridLength) -> Self {
        ColumnDef {
            width,
            ..Default::default()
        }
    }
}

/// A grid row definition.
#[derive(Debug, Clone)]
pub struct RowDef {
    /// Minimal exact height.
    pub min_height: f32,
    /// Maximum exact height.
    pub max_height: f32,
    /// Height configuration.
    pub height: GridLength,
}
impl Default for RowDef {
    fn default() -> Self {
        RowDef {
            min_height: 0.0,
            max_height: f32::MAX,
            height: GridLength::default(),
        }
    }
}
impl RowDef {
    pub fn new(height: GridLength) -> Self {
        RowDef {
            height,
            ..Default::default()
        }
    }
}

/// Represents the length configuration of a grid column(width) or row(height).
#[derive(Debug, Clone, Copy)]
pub enum GridLength {
    /// The column/row fits to the maximum used cell size.
    Auto,
    /// The column/row is of a size relative to the other columns in the grid.
    /// If all columns are weighted `1.0` they are all the same size.
    Weight(f32),
    /// The column/row is an exact size.
    Exact(f32),
}
impl Default for GridLength {
    fn default() -> Self {
        GridLength::Auto
    }
}
impl GridLength {
    /// If length is automatic.
    pub fn is_auto(self) -> bool {
        match self {
            GridLength::Auto => true,
            _ => false,
        }
    }

    /// If length is relative weight.
    pub fn is_relative(self) -> bool {
        match self {
            GridLength::Weight(_) => true,
            _ => false,
        }
    }

    /// If length is exact value.
    pub fn is_exact(self) -> bool {
        match self {
            GridLength::Exact(_) => true,
            _ => false,
        }
    }
}
