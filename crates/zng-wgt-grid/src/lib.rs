#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Grid widgets, properties and nodes.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{fmt, mem};

use zng_layout::unit::GridSpacing;
use zng_wgt::prelude::*;
use zng_wgt_access::{AccessRole, access_role};
use zng_wgt_size_offset::*;

#[doc(inline)]
pub use column::Column;

/// Column widget and properties.
pub mod column;

#[doc(inline)]
pub use row::Row;

/// Row widget and properties.
pub mod row;

#[doc(inline)]
pub use cell::Cell;

/// Cell widget and properties.
pub mod cell;

mod layout;

/// Grid layout with cells of variable sizes.
#[widget($crate::Grid)]
pub struct Grid(WidgetBase);
impl Grid {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|w| {
            let child = node(
                w.capture_ui_node_or_nil(property_id!(Self::cells)),
                w.capture_ui_node_or_nil(property_id!(Self::columns)),
                w.capture_ui_node_or_nil(property_id!(Self::rows)),
                w.capture_var_or_else(property_id!(Self::auto_grow_fn), WidgetFn::nil),
                w.capture_var_or_else(property_id!(Self::auto_grow_mode), AutoGrowMode::rows),
                w.capture_var_or_default(property_id!(Self::spacing)),
            );
            w.set_child(child);
        });

        widget_set! {
            self;

            access_role = AccessRole::Grid;
        }
    }
}

/// Cell widget items.
///
/// Cells can select their own column, row, column-span and row-span using the properties in the [`Cell!`] widget.
/// Note that you don't need to use that widget, only the [`cell`] properties.
///
/// If the column or row index is set to [`usize::MAX`] the widget is positioned using the
/// logical index *i*, the column *i % columns* and the row *i / columns*.
///
/// [`Cell!`]: struct@Cell
#[property(CHILD, widget_impl(Grid))]
pub fn cells(wgt: &mut WidgetBuilding, cells: impl IntoUiNode) {
    let _ = cells;
    wgt.expect_property_capture();
}

/// Column definitions.
///
/// Columns are defined by widgets, the column widget width defines the width of cells assigned to the column,
/// the [`Column!`] widget is recommended, but you can use any widget to define a column. The column widget is rendered
/// as the background of the column region, behind cells and row backgrounds.
///
/// ### Layout Modes
///
/// The grid uses the [`WIDGET_SIZE`] value to select one of three layout modes for columns:
///
/// * *Default*, used for columns that do not set width or set it to [`Length::Default`].
/// * *Exact*, used for columns that set the width to an unit that is exact or only depends on the grid context.
/// * *Leftover*, used for columns that set width to a [`lft`] value.
///
/// The column (and row) measure follows these steps:
///
/// 1 - All *Exact* column widgets are measured, their final width defines the column width.
/// 2 - All *Default* sized column widgets are measured twice to find its min and max widths.
/// 3 - All cell widgets with span `1` in *Default* columns are measured to find the widest cell width. That defines the column width.
/// 4 - All *Leftover* columns receive the proportional leftover grid width for each.
///
/// So given the columns `200 | 1.lft() | 1.lft()` and grid width of `1000` with spacing `5` the final widths are `200 | 395 | 395`,
/// for `200 + 5 + 395 + 5 + 395 = 1000`.
///
/// #### Overflow Recovery
///
/// In case the columns width overflows and all rows are *Default* height and some columns are *Default* width these recovery steps are taken:
///
/// 1 - All cell widgets with span `1` in *Default* columns are measured to find the minimum width they can wrap down too.
/// 2 - All *Default* columns are sized to the minimum width plus the extra space now available, proportionally divided.
/// 3 - All cells widgets affected are measured again to define the row heights.
///
/// The proportion of each *Default* is the difference between the previous measured width with the new minimum, this is very similar to
/// the CSS table layout, except the previous measured width is used instead of another measure pass to find the cells maximum width.
///
/// ### Notes
///
/// Note that the column widget is not the parent of the cells that match it.
/// Properties like `padding` and `align` only affect the column visual, not the cells, similarly contextual properties like `text_color`
/// don't affect the cells.
///
/// Note that the *Default* layout mode scales with the cell count, the other modes scale with the column count. This
/// is fine for small grids (<100 cells) or for simple cell widgets, but for larger grids you should really consider using
/// an *Exact* width or *Leftover* proportion, specially if the grid width is bounded.
///
/// [`Column!`]: struct@Column
/// [`lft`]: zng_layout::unit::LengthUnits::lft
/// [`WIDGET_SIZE`]: zng_wgt_size_offset::WIDGET_SIZE
/// [`Length::Default`]: zng_layout::unit::Length::Default
#[property(CHILD, widget_impl(Grid))]
pub fn columns(wgt: &mut WidgetBuilding, columns: impl IntoUiNode) {
    let _ = columns;
    wgt.expect_property_capture();
}

/// Row definitions.
///
/// Rows are defined by widgets, the row widget height defines the height of cells assigned to the row, the [`Row!`] widget is recommended,
/// but you can use any widget to define a row. The row widget is rendered as the background of the row region, behind cells and in front
/// of column backgrounds.
///
/// ## Layout Modes
///
/// The grid uses the [`WIDGET_SIZE`] value to select one of three layout modes for rows:
///
/// * *Default*, used for rows that do not set height or set it to [`Length::Default`].
/// * *Exact*, used for rows that set the height to an unit that is exact or only depends on the grid context.
/// * *Leftover*, used for rows that set height to a [`lft`] value.
///
/// The row measure follows the same steps as [`columns`], the only difference is that there is no
/// overflow recovery for row heights exceeding the available height.
///
/// ### Notes
///
/// Note that the row widget is not the parent of the cells that match it.
/// Properties like `padding` and `align` only affect the row visual, not the cells, similarly contextual properties like `text_color`
/// don't affect the cells.
///
/// Note that the *Default* layout mode scales with the cell count, the other modes scale with the row count. This has less impact
/// for rows, but you should consider setting a fixed row height for larger grids. Also note that you can define the [`auto_grow_fn`]
/// instead of manually adding rows. With fixed heights a data table of up to 1000 rows with simple text cells should have good performance.
///
/// For massive data tables consider a paginating layout with a separate grid instance per *page*, the page grids don't need to be actually
/// presented as pages, you can use lazy loading and a simple stack layout to seamless virtualize data loading and presentation.
///
/// [`columns`]: fn@columns
/// [`auto_grow_fn`]: fn@auto_grow_fn
/// [`Row!`]: struct@Row
/// [`lft`]: zng_layout::unit::LengthUnits::lft
/// [`WIDGET_SIZE`]: zng_wgt_size_offset::WIDGET_SIZE
/// [`Length::Default`]: zng_layout::unit::Length::Default
#[property(CHILD, widget_impl(Grid))]
pub fn rows(wgt: &mut WidgetBuilding, rows: impl IntoUiNode) {
    let _ = rows;
    wgt.expect_property_capture();
}

/// Widget function used when new rows or columns are needed to cover a cell placement.
///
/// The function is used according to the [`auto_grow_mode`]. Note that *imaginary* rows or columns are used if
/// the function is [`WidgetFn::nil`].
///
/// [`auto_grow_mode`]: fn@auto_grow_mode
/// [`WidgetFn::nil`]: zng_wgt::prelude::WidgetFn::nil
#[property(CONTEXT, default(WidgetFn::nil()), widget_impl(Grid))]
pub fn auto_grow_fn(wgt: &mut WidgetBuilding, auto_grow: impl IntoVar<WidgetFn<AutoGrowFnArgs>>) {
    let _ = auto_grow;
    wgt.expect_property_capture();
}

/// Defines the direction the grid auto-grows and the maximum inclusive index that can be covered by auto-generated columns or rows.
/// If a cell is outside this index and is not covered by predefined columns or rows a new one is auto generated for it, but if the
/// cell is also outside this max it is *collapsed*.
///
/// Is `AutoGrowMode::rows() by default.
#[property(CONTEXT, default(AutoGrowMode::rows()), widget_impl(Grid))]
pub fn auto_grow_mode(wgt: &mut WidgetBuilding, mode: impl IntoVar<AutoGrowMode>) {
    let _ = mode;
    wgt.expect_property_capture();
}

/// Space in-between cells.
#[property(LAYOUT, default(GridSpacing::default()), widget_impl(Grid))]
pub fn spacing(wgt: &mut WidgetBuilding, spacing: impl IntoVar<GridSpacing>) {
    let _ = spacing;
    wgt.expect_property_capture();
}

/// Grid node.
///
/// Can be used directly to layout widgets without declaring a grid widget info. This node is the child
/// of the `Grid!` widget.
pub fn node(
    cells: impl IntoUiNode,
    columns: impl IntoUiNode,
    rows: impl IntoUiNode,
    auto_grow_fn: impl IntoVar<WidgetFn<AutoGrowFnArgs>>,
    auto_grow_mode: impl IntoVar<AutoGrowMode>,
    spacing: impl IntoVar<GridSpacing>,
) -> UiNode {
    let auto_columns = ui_vec![];
    let auto_rows = ui_vec![];
    let children = ui_vec![
        ChainList(ui_vec![columns.into_node().into_list(), auto_columns]),
        ChainList(ui_vec![rows.into_node().into_list(), auto_rows]),
        PanelList::new(cells),
    ];
    let spacing = spacing.into_var();
    let auto_grow_fn = auto_grow_fn.into_var();
    let auto_grow_mode = auto_grow_mode.into_var();

    let mut grid = layout::GridLayout::default();
    let mut is_measured = false;
    let mut last_layout = LayoutMetrics::new(1.fct(), PxSize::zero(), Px(0));

    match_node(children, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&auto_grow_fn).sub_var(&auto_grow_mode).sub_var_layout(&spacing);
            c.init();
            grid.update_entries(c.node(), auto_grow_mode.get(), &auto_grow_fn);
        }
        UiNodeOp::Deinit => {
            c.deinit();
            layout::GridChildrenMut(c.node()).auto_columns().clear();
            layout::GridChildrenMut(c.node()).auto_rows().clear();
            is_measured = false;
        }
        UiNodeOp::Update { updates } => {
            let mut any = false;
            c.update_list(updates, &mut any);

            if auto_grow_fn.is_new() || auto_grow_mode.is_new() {
                for mut auto in layout::GridChildrenMut(c.node()).auto_columns().drain(..) {
                    auto.deinit();
                }
                for mut auto in layout::GridChildrenMut(c.node()).auto_rows().drain(..) {
                    auto.deinit();
                }
                any = true;
            }
            if any {
                grid.update_entries(c.node(), auto_grow_mode.get(), &auto_grow_fn);
                WIDGET.layout();
            }
        }
        UiNodeOp::Measure { wm, desired_size } => {
            c.delegated();

            let constraints = LAYOUT.constraints().inner();

            *desired_size = if let Some(size) = constraints.fill_or_exact() {
                size
            } else {
                is_measured = true;
                let s = grid.grid_layout(wm, c.node(), &spacing).1;
                constraints.clamp_size(s)
            };
        }
        UiNodeOp::Layout { wl, final_size } => {
            c.delegated();
            is_measured = false;
            last_layout = LAYOUT.metrics();

            let (spacing, grid_size) = grid.grid_layout(&mut wl.to_measure(None), c.node(), &spacing);
            let constraints = last_layout.constraints();

            if grid.is_collapse() {
                wl.collapse_descendants();
                *final_size = constraints.fill_or_exact().unwrap_or_default();
                return;
            }

            let mut children = layout::GridChildrenMut(c.node());
            let mut children = children.children().iter_mut();
            let columns = children.next().unwrap();
            let rows = children.next().unwrap();
            let cells = children.next().unwrap();
            let cells: &mut PanelList = cells.downcast_mut().unwrap();

            let grid = &grid;

            // layout columns
            let _ = columns.layout_list(
                wl,
                |ci, col, wl| {
                    let info = grid.columns[ci];
                    LAYOUT.with_constraints(PxConstraints2d::new_exact(info.width, grid_size.height), || col.layout(wl))
                },
                |_, _| PxSize::zero(),
            );
            // layout rows
            let _ = rows.layout_list(
                wl,
                |ri, row, wl| {
                    let info = grid.rows[ri];
                    LAYOUT.with_constraints(PxConstraints2d::new_exact(grid_size.width, info.height), || row.layout(wl))
                },
                |_, _| PxSize::zero(),
            );
            // layout and translate cells
            let cells_offset = columns.children_len() + rows.children_len();

            cells.layout_list(
                wl,
                |i, cell, o, wl| {
                    let cell_info = cell::CellInfo::get_wgt(cell).actual(i, grid.columns.len());

                    if cell_info.column >= grid.columns.len() || cell_info.row >= grid.rows.len() {
                        wl.collapse_child(cells_offset + i);
                        return PxSize::zero(); // continue;
                    }

                    let cell_offset = PxVector::new(grid.columns[cell_info.column].x, grid.rows[cell_info.row].y);
                    let mut cell_size = PxSize::zero();

                    for col in cell_info.column..(cell_info.column + cell_info.column_span).min(grid.columns.len()) {
                        if grid.columns[col].width != Px(0) {
                            cell_size.width += grid.columns[col].width + spacing.column;
                        }
                    }
                    cell_size.width -= spacing.column;

                    for row in cell_info.row..(cell_info.row + cell_info.row_span).min(grid.rows.len()) {
                        if grid.rows[row].height != Px(0) {
                            cell_size.height += grid.rows[row].height + spacing.row;
                        }
                    }
                    cell_size.height -= spacing.row;

                    if cell_size.is_empty() {
                        wl.collapse_child(cells_offset + i);
                        return PxSize::zero(); // continue;
                    }

                    let (_, define_ref_frame) =
                        LAYOUT.with_constraints(PxConstraints2d::new_exact_size(cell_size), || wl.with_child(|wl| cell.layout(wl)));
                    o.child_offset = cell_offset;
                    o.define_reference_frame = define_ref_frame;

                    cell_size
                },
                |_, _| PxSize::zero(),
            );
            cells.commit_data().request_render();

            *final_size = constraints.inner().fill_size_or(grid_size);
        }
        UiNodeOp::Render { frame } => {
            c.delegated();

            if mem::take(&mut is_measured) {
                LAYOUT.with_context(last_layout.clone(), || {
                    let _ = grid.grid_layout(&mut WidgetMeasure::new_reuse(None), c.node(), &spacing);
                });
            }

            let grid = &grid;

            if grid.is_collapse() {
                return;
            }

            let mut children = layout::GridChildrenMut(c.node());
            let mut children = children.children().iter_mut();
            let columns = children.next().unwrap();
            let rows = children.next().unwrap();
            let cells: &mut PanelList = children.next().unwrap().downcast_mut().unwrap();
            let offset_key = cells.offset_key();

            columns.for_each_child(|i, child| {
                let offset = PxVector::new(grid.columns[i].x, Px(0));
                frame.push_reference_frame(
                    (offset_key, i as u32).into(),
                    FrameValue::Value(offset.into()),
                    true,
                    true,
                    |frame| {
                        child.render(frame);
                    },
                );
            });
            let i_extra = columns.children_len();
            rows.for_each_child(|i, child| {
                let offset = PxVector::new(Px(0), grid.rows[i].y);
                frame.push_reference_frame(
                    (offset_key, (i + i_extra) as u32).into(),
                    FrameValue::Value(offset.into()),
                    true,
                    true,
                    |frame| {
                        child.render(frame);
                    },
                );
            });
            let i_extra = i_extra + rows.children_len();
            cells.for_each_z_sorted(|i, child, data| {
                if data.define_reference_frame {
                    frame.push_reference_frame(
                        (offset_key, (i + i_extra) as u32).into(),
                        FrameValue::Value(data.child_offset.into()),
                        true,
                        true,
                        |frame| {
                            child.render(frame);
                        },
                    );
                } else {
                    frame.push_child(data.child_offset, |frame| child.render(frame));
                }
            });
        }
        UiNodeOp::RenderUpdate { update } => {
            c.delegated();

            if mem::take(&mut is_measured) {
                LAYOUT.with_context(last_layout.clone(), || {
                    let _ = grid.grid_layout(&mut WidgetMeasure::new_reuse(None), c.node(), &spacing);
                });
            }

            let grid = &grid;

            if grid.is_collapse() {
                return;
            }

            let mut children = layout::GridChildrenMut(c.node());
            let mut children = children.children().iter_mut();
            let columns = children.next().unwrap();
            let rows = children.next().unwrap();
            let cells: &mut PanelList = children.next().unwrap().downcast_mut().unwrap();

            columns.for_each_child(|i, child| {
                let offset = PxVector::new(grid.columns[i].x, Px(0));
                update.with_transform_value(&offset.into(), |update| {
                    child.render_update(update);
                });
            });
            rows.for_each_child(|i, child| {
                let offset = PxVector::new(Px(0), grid.rows[i].y);
                update.with_transform_value(&offset.into(), |update| {
                    child.render_update(update);
                });
            });
            cells.for_each_child(|_, child, data| {
                if data.define_reference_frame {
                    update.with_transform_value(&data.child_offset.into(), |update| {
                        child.render_update(update);
                    });
                } else {
                    update.with_child(data.child_offset, |update| {
                        child.render_update(update);
                    })
                }
            })
        }
        _ => {}
    })
}

/// Arguments for [`auto_grow_fn`].
///
/// [`auto_grow_fn`]: fn@auto_grow_fn
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AutoGrowFnArgs {
    /// Auto-grow direction.
    pub mode: AutoGrowMode,
    /// Column index.
    pub index: usize,
}
impl AutoGrowFnArgs {
    /// New args.
    pub fn new(mode: AutoGrowMode, index: usize) -> Self {
        Self { mode, index }
    }
}

/// Grid auto-grow direction.
///
/// The associated value is the maximum columns or rows that are allowed in the grid.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AutoGrowMode {
    /// Auto generate columns.
    Columns(u32),
    /// Auto generate rows.
    Rows(u32),
}
impl AutoGrowMode {
    /// Value that does not generate any new row or column.
    pub const fn disabled() -> Self {
        Self::Rows(0)
    }

    /// Columns, not specific maximum limit.
    pub const fn columns() -> Self {
        Self::Columns(u32::MAX)
    }

    /// Rows, not specific maximum limit.
    pub const fn rows() -> Self {
        Self::Rows(u32::MAX)
    }

    /// Set the maximum columns or rows allowed.
    pub fn with_limit(self, limit: u32) -> Self {
        match self {
            AutoGrowMode::Columns(_) => AutoGrowMode::Columns(limit),
            AutoGrowMode::Rows(_) => AutoGrowMode::Rows(limit),
        }
    }
}
impl fmt::Debug for AutoGrowMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "AutoGrowMode::")?;
        }
        match self {
            AutoGrowMode::Rows(0) => write!(f, "disabled()"),
            AutoGrowMode::Columns(u32::MAX) => write!(f, "Columns(MAX)"),
            AutoGrowMode::Rows(u32::MAX) => write!(f, "Rows(MAX)"),
            AutoGrowMode::Columns(l) => write!(f, "Columns({l})"),
            AutoGrowMode::Rows(l) => write!(f, "Rows({l})"),
        }
    }
}
