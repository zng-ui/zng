#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Grid widgets, properties and nodes.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{fmt, mem};

use zng_layout::unit::{GridSpacing, PxGridSpacing};
use zng_wgt::prelude::*;
use zng_wgt_access::{AccessRole, access_role};
use zng_wgt_size_offset::*;

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
/// Note that you don't need to use the cell widget, only the [`cell`] properties.
///
/// If the column or row index is set to [`usize::MAX`] the widget is positioned using the
/// logical index *i*, the column *i % columns* and the row *i / columns*.
///
/// [`Cell!`]: struct@Cell
#[property(CHILD, capture, widget_impl(Grid))]
pub fn cells(cells: impl IntoUiNode) {}

/// Column definitions.
///
/// You can define columns with any widget, but the [`Column!`] widget is recommended. The column widget width defines
/// the width of the cells assigned to it, the [`Column::width`] property can be used to enforce a width, otherwise the
/// column is sized by the widest cell.
///
/// The grid uses the [`WIDGET_SIZE`] value to select one of three layout modes for columns:
///
/// * *Default*, used for columns that do not set width or set it to [`Length::Default`].
/// * *Exact*, used for columns that set the width to an unit that is exact or only depends on the grid context.
/// * *Leftover*, used for columns that set width to a [`lft`] value.
///
/// The column layout follows these steps:
///
/// 1 - All *Exact* column widgets are layout, their final width defines the column width.
/// 2 - All cell widgets with span `1` in *Default* columns are measured, the widest defines the fill width constrain,
/// the columns are layout using this constrain, the final width defines the column width.
/// 3 - All *Leftover* cells are layout with the leftover grid width divided among all columns in this mode.
///
/// So given the columns `200 | 1.lft() | 1.lft()` and grid width of `1000` with spacing `5` the final widths are `200 | 395 | 395`,
/// for `200 + 5 + 395 + 5 + 395 = 1000`.
///
/// Note that the column widget is not the parent of the cells that match it, the column widget is rendered behind cell and row widgets.
/// Properties like `padding` and `align` only affect the column visual, not the cells, similarly contextual properties like `text_color`
/// don't affect the cells.
///
/// [`Column!`]: struct@Column
/// [`lft`]: zng_layout::unit::LengthUnits::lft
/// [`WIDGET_SIZE`]: zng_wgt_size_offset::WIDGET_SIZE
/// [`Length::Default`]: zng_layout::unit::Length::Default
#[property(CHILD, capture, widget_impl(Grid))]
pub fn columns(cells: impl IntoUiNode) {}

/// Row definitions.
///
/// Same behavior as [`columns`], but in the ***y*** dimension.
///
/// [`columns`]: fn@columns
#[property(CHILD, capture, widget_impl(Grid))]
pub fn rows(cells: impl IntoUiNode) {}

/// Widget function used when new rows or columns are needed to cover a cell placement.
///
/// The function is used according to the [`auto_grow_mode`]. Note that *imaginary* rows or columns are used if
/// the function is [`WidgetFn::nil`].
///
/// [`auto_grow_mode`]: fn@auto_grow_mode
/// [`WidgetFn::nil`]: zng_wgt::prelude::WidgetFn::nil
#[property(CONTEXT, capture, default(WidgetFn::nil()), widget_impl(Grid))]
pub fn auto_grow_fn(auto_grow: impl IntoVar<WidgetFn<AutoGrowFnArgs>>) {}

/// Defines the direction the grid auto-grows and the maximum inclusive index that can be covered by auto-generated columns or rows.
/// If a cell is outside this index and is not covered by predefined columns or rows a new one is auto generated for it, but if the
/// cell is also outside this max it is *collapsed*.
///
/// Is `AutoGrowMode::rows() by default.
#[property(CONTEXT, capture, default(AutoGrowMode::rows()), widget_impl(Grid))]
pub fn auto_grow_mode(mode: impl IntoVar<AutoGrowMode>) {}

/// Space in-between cells.
#[property(LAYOUT, capture, default(GridSpacing::default()), widget_impl(Grid))]
pub fn spacing(spacing: impl IntoVar<GridSpacing>) {}

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

    let mut grid = GridLayout::default();
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
            GridChildrenMut(c.node()).auto_columns().clear();
            GridChildrenMut(c.node()).auto_rows().clear();
            is_measured = false;
        }
        UiNodeOp::Update { updates } => {
            let mut any = false;
            c.update_list(updates, &mut any);

            if auto_grow_fn.is_new() || auto_grow_mode.is_new() {
                for mut auto in GridChildrenMut(c.node()).auto_columns().drain(..) {
                    auto.deinit();
                }
                for mut auto in GridChildrenMut(c.node()).auto_rows().drain(..) {
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

            *desired_size = if let Some(size) = LAYOUT.constraints().fill_or_exact() {
                size
            } else {
                is_measured = true;
                grid.grid_layout(wm, c.node(), &spacing).1
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

            let mut children = GridChildrenMut(c.node());
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
                    LAYOUT.with_constraints(constraints.with_exact(info.width, grid_size.height), || col.layout(wl))
                },
                |_, _| PxSize::zero(),
            );
            // layout rows
            let _ = rows.layout_list(
                wl,
                |ri, row, wl| {
                    let info = grid.rows[ri];
                    LAYOUT.with_constraints(constraints.with_exact(grid_size.width, info.height), || row.layout(wl))
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
                        LAYOUT.with_constraints(constraints.with_exact_size(cell_size), || wl.with_child(|wl| cell.layout(wl)));
                    o.child_offset = cell_offset;
                    o.define_reference_frame = define_ref_frame;

                    cell_size
                },
                |_, _| PxSize::zero(),
            );
            cells.commit_data().request_render();

            *final_size = constraints.fill_size_or(grid_size);
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

            let mut children = GridChildrenMut(c.node());
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

            let mut children = GridChildrenMut(c.node());
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

#[doc(inline)]
pub use column::Column;

/// Column widget and properties.
pub mod column {
    use super::*;

    /// Grid column definition.
    ///
    /// This widget is layout to define the actual column width, it is not the parent
    /// of the cells, only the `width` and `align` properties affect the cells.
    ///
    /// See the [`Grid::columns`] property for more details.
    ///
    /// # Shorthand
    ///
    /// The `Column!` macro provides a shorthand init that sets the width, `grid::Column!(1.lft())` instantiates
    /// a column with width of *1 leftover*.
    #[widget($crate::Column { ($width:expr) => { width = $width; }; })]
    pub struct Column(WidgetBase);
    impl Column {
        widget_impl! {
            /// Column max width.
            pub max_width(max: impl IntoVar<Length>);

            /// Column min width.
            pub min_width(min: impl IntoVar<Length>);

            /// Column width.
            pub width(width: impl IntoVar<Length>);
        }

        fn widget_intrinsic(&mut self) {
            widget_set! {
                self;
                access_role = AccessRole::Column;
            }
        }
    }

    static_id! {
        /// Column index, total in the parent widget set by the parent.
        pub(super) static ref INDEX_ID: StateId<(usize, usize)>;
    }

    /// If the column index is even.
    ///
    /// Column index is zero-based, so the first column is even, the next [`is_odd`].
    ///
    /// [`is_odd`]: fn@is_odd
    #[property(CONTEXT, widget_impl(Column))]
    pub fn is_even(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
        widget_state_is_state(child, |w| w.get(*INDEX_ID).copied().unwrap_or((0, 0)).0 % 2 == 0, |_| false, state)
    }

    /// If the column index is odd.
    ///
    /// Column index is zero-based, so the first column [`is_even`], the next one is odd.
    ///
    /// [`is_even`]: fn@is_even
    #[property(CONTEXT, widget_impl(Column))]
    pub fn is_odd(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
        widget_state_is_state(child, |w| w.get(*INDEX_ID).copied().unwrap_or((0, 0)).0 % 2 != 0, |_| false, state)
    }

    /// If the column is the first.
    #[property(CONTEXT, widget_impl(Column))]
    pub fn is_first(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
        widget_state_is_state(
            child,
            |w| {
                let (i, l) = w.get(*INDEX_ID).copied().unwrap_or((0, 0));
                i == 0 && l > 0
            },
            |_| false,
            state,
        )
    }

    /// If the column is the last.
    #[property(CONTEXT, widget_impl(Column))]
    pub fn is_last(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
        widget_state_is_state(
            child,
            |w| {
                let (i, l) = w.get(*INDEX_ID).copied().unwrap_or((0, 0));
                i < l && i == l - 1
            },
            |_| false,
            state,
        )
    }

    /// Get the column index.
    ///
    /// The column index is zero-based.
    #[property(CONTEXT, widget_impl(Column))]
    pub fn get_index(child: impl IntoUiNode, state: impl IntoVar<usize>) -> UiNode {
        widget_state_get_state(
            child,
            |w, &i| {
                let a = w.get(*INDEX_ID).copied().unwrap_or((0, 0)).0;
                if a != i { Some(a) } else { None }
            },
            |_, &i| if i != 0 { Some(0) } else { None },
            state,
        )
    }

    /// Get the column index and number of columns.
    #[property(CONTEXT, widget_impl(Column))]
    pub fn get_index_len(child: impl IntoUiNode, state: impl IntoVar<(usize, usize)>) -> UiNode {
        widget_state_get_state(
            child,
            |w, &i| {
                let a = w.get(*INDEX_ID).copied().unwrap_or((0, 0));
                if a != i { Some(a) } else { None }
            },
            |_, &i| if i != (0, 0) { Some((0, 0)) } else { None },
            state,
        )
    }

    /// Get the column index, starting from the last column at `0`.
    #[property(CONTEXT, widget_impl(Column))]
    pub fn get_rev_index(child: impl IntoUiNode, state: impl IntoVar<usize>) -> UiNode {
        widget_state_get_state(
            child,
            |w, &i| {
                let a = w.get(*INDEX_ID).copied().unwrap_or((0, 0));
                let a = a.1 - a.0;
                if a != i { Some(a) } else { None }
            },
            |_, &i| if i != 0 { Some(0) } else { None },
            state,
        )
    }
}

#[doc(inline)]
pub use row::Row;

/// Row widget and properties.
pub mod row {
    use super::*;

    /// Grid row definition.
    ///
    /// This widget is layout to define the actual row height, it is not the parent
    /// of the cells, only the `height` property affect the cells.
    ///
    /// See the [`Grid::rows`] property for more details.
    ///
    /// # Shorthand
    ///
    /// The `Row!` macro provides a shorthand init that sets the height, `grid::Row!(1.lft())` instantiates
    /// a row with height of *1 leftover*.
    #[widget($crate::Row { ($height:expr) => { height = $height; }; })]
    pub struct Row(WidgetBase);
    impl Row {
        widget_impl! {
            /// Row max height.
            pub max_height(max: impl IntoVar<Length>);

            /// Row min height.
            pub min_height(max: impl IntoVar<Length>);

            /// Row height.
            pub height(max: impl IntoVar<Length>);
        }

        fn widget_intrinsic(&mut self) {
            widget_set! {
                self;
                access_role = AccessRole::Row;
            }
        }
    }

    static_id! {
        /// Row index, total in the parent widget set by the parent.
        pub(super) static ref INDEX_ID: StateId<(usize, usize)>;
    }

    /// If the row index is even.
    ///
    /// Row index is zero-based, so the first row is even, the next [`is_odd`].
    ///
    /// [`is_odd`]: fn@is_odd
    #[property(CONTEXT, widget_impl(Row))]
    pub fn is_even(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
        widget_state_is_state(child, |w| w.get(*INDEX_ID).copied().unwrap_or((0, 0)).0 % 2 == 0, |_| false, state)
    }

    /// If the row index is odd.
    ///
    /// Row index is zero-based, so the first row [`is_even`], the next one is odd.
    ///
    /// [`is_even`]: fn@is_even
    #[property(CONTEXT, widget_impl(Row))]
    pub fn is_odd(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
        widget_state_is_state(child, |w| w.get(*INDEX_ID).copied().unwrap_or((0, 0)).0 % 2 != 0, |_| false, state)
    }

    /// If the row is the first.
    #[property(CONTEXT, widget_impl(Row))]
    pub fn is_first(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
        widget_state_is_state(
            child,
            |w| {
                let (i, l) = w.get(*INDEX_ID).copied().unwrap_or((0, 0));
                i == 0 && l > 0
            },
            |_| false,
            state,
        )
    }

    /// If the row is the last.
    #[property(CONTEXT, widget_impl(Row))]
    pub fn is_last(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
        widget_state_is_state(
            child,
            |w| {
                let (i, l) = w.get(*INDEX_ID).copied().unwrap_or((0, 0));
                i < l && i == l - 1
            },
            |_| false,
            state,
        )
    }

    /// Get the row index.
    ///
    /// The row index is zero-based.
    #[property(CONTEXT, widget_impl(Row))]
    pub fn get_index(child: impl IntoUiNode, state: impl IntoVar<usize>) -> UiNode {
        widget_state_get_state(
            child,
            |w, &i| {
                let a = w.get(*INDEX_ID).copied().unwrap_or((0, 0)).0;
                if a != i { Some(a) } else { None }
            },
            |_, &i| if i != 0 { Some(0) } else { None },
            state,
        )
    }

    /// Get the row index and number of rows.
    #[property(CONTEXT, widget_impl(Row))]
    pub fn get_index_len(child: impl IntoUiNode, state: impl IntoVar<(usize, usize)>) -> UiNode {
        widget_state_get_state(
            child,
            |w, &i| {
                let a = w.get(*INDEX_ID).copied().unwrap_or((0, 0));
                if a != i { Some(a) } else { None }
            },
            |_, &i| if i != (0, 0) { Some((0, 0)) } else { None },
            state,
        )
    }

    /// Get the row index, starting from the last row at `0`.
    #[property(CONTEXT, widget_impl(Row))]
    pub fn get_rev_index(child: impl IntoUiNode, state: impl IntoVar<usize>) -> UiNode {
        widget_state_get_state(
            child,
            |w, &i| {
                let a = w.get(*INDEX_ID).copied().unwrap_or((0, 0));
                let a = a.1 - a.0;
                if a != i { Some(a) } else { None }
            },
            |_, &i| if i != 0 { Some(0) } else { None },
            state,
        )
    }
}

#[doc(inline)]
pub use cell::Cell;

/// Cell widget and properties.
pub mod cell {
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
}

#[derive(Clone, Copy)]
struct ColRowMeta(f32);
impl ColRowMeta {
    /// `width` or `height` contains the largest cell or `Px::MIN` if cell measure is pending.
    fn is_default(self) -> bool {
        self.0.is_sign_negative() && self.0.is_infinite()
    }

    /// Return the leftover factor if the column or row must be measured on a fraction of the leftover space.
    fn is_leftover(self) -> Option<Factor> {
        if self.0 >= 0.0 { Some(Factor(self.0)) } else { None }
    }

    /// `width` or `height` contains the final length or is pending layout `Px::MIN`.
    fn is_exact(self) -> bool {
        self.0.is_nan()
    }

    fn exact() -> Self {
        Self(f32::NAN)
    }

    fn leftover(f: Factor) -> Self {
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
struct ColumnLayout {
    meta: ColRowMeta,
    was_leftover: bool,
    x: Px,
    width: Px,
    min_width: Px,
    max_width: Px,
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
struct RowLayout {
    meta: ColRowMeta,
    was_leftover: bool,
    y: Px,
    height: Px,
    min_height: Px,
    max_height: Px,
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
struct GridLayout {
    columns: Vec<ColumnLayout>,
    rows: Vec<RowLayout>,
}
impl GridLayout {
    fn is_collapse(&self) -> bool {
        self.columns.is_empty() || self.rows.is_empty()
    }

    fn collapse(&mut self) {
        self.columns.clear();
        self.rows.clear();
    }

    /// add/remove info entries, auto-grow/shrink
    fn update_entries(&mut self, children: &mut GridChildren, auto_mode: AutoGrowMode, auto_grow_fn: &Var<WidgetFn<AutoGrowFnArgs>>) {
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
    fn grid_layout(&mut self, wm: &mut WidgetMeasure, children: &mut GridChildren, spacing: &Var<GridSpacing>) -> (PxGridSpacing, PxSize) {
        if self.is_collapse() {
            return (PxGridSpacing::zero(), PxSize::zero());
        }

        let spacing = spacing.layout();
        let constraints = LAYOUT.constraints();

        let fill_x = constraints.x.fill_or_exact();
        let fill_y = constraints.y.fill_or_exact();

        let mut children = GridChildrenMut(children);
        let mut children = children.children().iter_mut();
        let columns = children.next().unwrap();
        let rows = children.next().unwrap();
        let cells = children.next().unwrap();

        // layout exact columns&rows, mark others for next passes.

        let mut has_default = false;
        let mut has_leftover_cols = false;
        let mut has_leftover_rows = false;
        const MAX_PROBE: i32 = Px::MAX.0 - 1000;

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
                        PxConstraints2d::new_fill(Px(MAX_PROBE), Px(MAX_PROBE)).with_fill_inner(true, true),
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
                        PxConstraints2d::new_fill(Px(MAX_PROBE), Px(MAX_PROBE)).with_fill_inner(true, true),
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

            let c = LAYOUT.constraints();

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
        for col in &mut self.columns {
            col.x = x;
            if col.width > Px(0) {
                x += col.width + spacing.column;
            }
        }
        let mut y = Px(0);
        for row in &mut self.rows {
            row.y = y;
            if row.height > Px(0) {
                y += row.height + spacing.row;
            }
        }

        x = (x - spacing.column).max(Px(0));
        let max_width = constraints.x.fill();
        if max_width > Px(0) && x > max_width {
            println!("!!: OVERFLOW, wrap autos {:?}", (x, max_width))
        }

        (spacing, PxSize::new((x - spacing.column).max(Px(0)), (y - spacing.row).max(Px(0))))
    }
}

/// [[columns, auto_columns], [rows, auto_rows], cells]
type GridChildren = UiNode;
struct GridChildrenMut<'a>(&'a mut GridChildren);
impl<'a> GridChildrenMut<'a> {
    fn children(&mut self) -> &mut UiVec {
        self.0.downcast_mut().unwrap()
    }

    fn all_columns_node(&mut self) -> &mut UiNode {
        &mut self.children()[0]
    }
    fn all_columns(&mut self) -> &mut ChainList {
        self.all_columns_node().downcast_mut().unwrap()
    }
    fn auto_columns(&mut self) -> &mut UiVec {
        self.all_columns().0[1].downcast_mut().unwrap()
    }

    fn all_rows_node(&mut self) -> &mut UiNode {
        &mut self.children()[1]
    }
    fn all_rows(&mut self) -> &mut ChainList {
        self.all_rows_node().downcast_mut().unwrap()
    }
    fn auto_rows(&mut self) -> &mut UiVec {
        self.all_rows().0[1].downcast_mut().unwrap()
    }

    fn cells(&mut self) -> &mut PanelList {
        self.children()[2].downcast_mut().unwrap()
    }
}
