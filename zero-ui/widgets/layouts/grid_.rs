use crate::prelude::new_widget::*;

/// Grid layout with configurable rows and columns.
///
/// # Columns & Rows
///
/// Columns and rows are defined by instances of the specialized widgets [`column!`] and [`row!`], these
/// widgets define the size and background visual of the columns and rows, you can use any property that
/// affects size or visual, but these widgets don't actually have any content, the width of columns is combined
/// with the height of rows to define the available size for each item.
///
/// ## Limitations
///
/// Not all properties work as you may expect, the most important detail to remember is that the column and row
/// widgets are not the parent of the item widgets, they only affect the size and position of the items, properties
/// like color filters only apply to the column or row background visual.
///
/// Only sizes in the same dimension affects the items, giving a column a height or a row a width only affects
/// the background visual, the items available size is a combination of the column width and row height.
///  
/// # Items
///
/// Items can be any widget, each item selects the cells it will occupy by using the [`grid::index`] and [`grid::span`]
/// properties. If the index is not on the grid of the span is zero the item is not visible, if the span is more than
/// one the item available size encompasses the sum of columns and rows plus the spacing in between then.
///
/// Spans grow to the right and bottom, and are clamped to the number of columns and rows, the item positioning is governed
/// solely by the indexed column and row, if it spans more then one cell only the outer dimensions of the other columns and
/// rows is considered.
///
/// [`column!`]: mod@grid::column
/// [`row!`]: mod@grid::row
/// [`grid::index`]: fn@grid::index
/// [`grid::span`]: fn@grid::span
#[widget($crate::widgets::layouts::grid)]
pub mod grid {
    use super::*;

    #[doc(inline)]
    pub use super::{column, row};

    properties! {
        /// Widget items.
        #[allowed_in_when = false]
        items(impl WidgetList) = widgets![];

        /// Column definitions.
        ///
        /// At least one column is required, panics if empty, does not compile if not set.
        /// By default it is a single column.
        ///
        /// See [`column!`] for details.
        ///
        /// [`column`]: mod@column
        #[allowed_in_when = false]
        columns(Vec<column::Definition>) = vec![column!()];

        /// Row definitions.
        ///
        /// At least one row is required, panics if empty, does not compile if not set.
        /// By default it is a single row.
        ///
        /// See [`row!`] for details.
        ///
        /// [`row`]: mod@column
        #[allowed_in_when = false]
        rows(Vec<row::Definition>) = vec![row!()];

        /// Space in-between items.
        ///
        /// Relative values are to the full grid size.
        spacing(impl IntoVar<GridSpacing>) = 0.0;
    }

    fn new_child(
        columns: Vec<column::Definition>,
        rows: Vec<row::Definition>,
        items: impl WidgetList,
        spacing: impl IntoVar<GridSpacing>,
    ) -> impl UiNode {
        struct GridNode<I, S> {
            items: I,
            columns: Vec<column::Definition>,
            rows: Vec<row::Definition>,
            spacing: S,
            column_origins: Vec<PxPoint>,
            row_origins: Vec<PxPoint>,
            item_rects: Vec<PxRect>,
        }
        impl<I: WidgetList, S: Var<GridSpacing>> UiNode for GridNode<I, S> {
            fn init(&mut self, ctx: &mut WidgetContext) {
                for column in &mut self.columns {
                    column.widget_mut().init(ctx);
                }
                for row in &mut self.rows {
                    row.widget_mut().init(ctx);
                }
                self.items.init_all(ctx);
            }

            fn deinit(&mut self, ctx: &mut WidgetContext) {
                for column in &mut self.columns {
                    column.widget_mut().deinit(ctx);
                }
                for row in &mut self.rows {
                    row.widget_mut().deinit(ctx);
                }
                self.items.deinit_all(ctx);
            }

            fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                for column in &mut self.columns {
                    column.widget_mut().event(ctx, args);
                }
                for row in &mut self.rows {
                    row.widget_mut().event(ctx, args);
                }
                self.items.event_all(ctx, args);
            }

            fn update(&mut self, ctx: &mut WidgetContext) {
                for column in &mut self.columns {
                    column.widget_mut().update(ctx);
                }
                for row in &mut self.rows {
                    row.widget_mut().update(ctx);
                }
                self.items.update_all(ctx);

                if self.spacing.is_new(ctx) {
                    ctx.updates.layout();
                }
            }

            fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                // # Measure Steps
                //
                // ? - Find column/rows with fixed sized the divide the rest?
                // 1 - Measure columns and rows without any content size.
                // 2 - Measure items using the column/row sizes.
                // 3 - Measure columns again, now with max single-span size.
                //
                // The grid's desired_size is the sum of visible columns with spacing in between then.

                // ## 1 - Measure Columns & Rows
                //
                // The available space is divided equally for each column.
                let mut c_available_size = available_size;
                c_available_size.width *= (self.columns.len() as f32 / 1.0).fct();

                let mut r_available_size = available_size;
                r_available_size.height *= (self.rows.len() as f32 / 1.0).fct();

                let spacing = self.spacing.get(ctx.vars).to_layout(ctx, available_size, PxGridSpacing::zero());

                // these are [(Px, AvailablePx)] of the (outer, inner) sizes.
                let column_widths: Vec<_> = self.columns.iter_mut().map(|c| c.measure(ctx, c_available_size)).collect();
                let row_heights: Vec<_> = self.rows.iter_mut().map(|r| r.measure(ctx, r_available_size)).collect();

                let mut desired_size = PxSize::zero();
                let mut c_desired_widths = vec![Px(0); column_widths.len()];
                let mut r_desired_heights = vec![Px(0); row_heights.len()];

                let columns_len = self.columns.len() as u32;
                let rows_len = self.rows.len() as u32;

                for i in 0..self.items.len() {
                    let state = self.items.widget_state(i);
                    let (c, r) = state.get(IndexKey).copied().unwrap_or((0, 0));
                    let (mut c_span, mut r_span) = state.get(SpanKey).copied().unwrap_or((1, 1));

                    if c >= columns_len || r >= rows_len || c_span == 0 || r_span == 0 {
                        tracing::debug!(
                            "grid child index `({:?})`, span `({:?})` is not placeable in a {}x{} grid and will not be rendered",
                            (c, r),
                            (c_span, r_span),
                            self.columns.len(),
                            self.rows.len()
                        );
                        self.item_rects[i].size = PxSize::zero();
                        continue;
                    }

                    if c + c_span > columns_len {
                        tracing::debug!(
                            "grid child column `{}` and span `{}` overflows the a grid with `{}` columns, span corrected",
                            c,
                            c_span,
                            self.columns.len()
                        );
                        c_span = columns_len - c;
                    }
                    if r + r_span > rows_len {
                        tracing::debug!(
                            "grid child row `{}` and span `{}` overflows the a grid with `{}` rows, span corrected",
                            c,
                            c_span,
                            self.rows.len()
                        );
                        r_span = rows_len - c;
                    }

                    let (outer_width, inner_width) = column_widths[c as usize];
                    let (outer_height, inner_height) = row_heights[r as usize];

                    if outer_width == Px(0) || outer_height == Px(0) {
                        // column or row collapsed
                        self.item_rects[i].size = PxSize::zero();
                        continue;
                    }

                    let cell_available_size = AvailableSize::new(
                        if c_span == 1 {
                            inner_width
                        } else {
                            let mut w = outer_width;
                            let mut visible_columns = 0;
                            for (ow, _) in &column_widths[(c + 1) as usize..(c + c_span) as usize] {
                                w += *ow;
                                if *ow > Px(0) {
                                    visible_columns += 1;
                                }
                            }
                            w += spacing.column * Px((visible_columns - 1).max(0) as i32);
                            AvailablePx::Finite(w)
                        },
                        if r_span == 1 {
                            inner_height
                        } else {
                            let mut h = outer_height;
                            for (rh, _) in &row_heights[(r + 1) as usize..(r + r_span) as usize] {
                                h += *rh;
                            }
                            h += spacing.row * Px((r_span - 1) as i32);
                            AvailablePx::Finite(h)
                        },
                    );

                    let cell_desired_size = self.items.widget_measure(i, ctx, cell_available_size);

                    if c_span == 1 {
                        c_desired_widths[i] = c_desired_widths[i].max(cell_desired_size.width);
                    }

                    if r_span == 1 {
                        r_desired_heights[i] = r_desired_heights[i].max(cell_desired_size.height);
                    }
                }

                desired_size
            }

            fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
                todo!()
            }

            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                for (column, origin) in self.columns.iter().zip(&self.column_origins) {
                    frame.push_reference_frame(*origin, |frame| column.widget().render(ctx, frame));
                }
                for (row, origin) in self.rows.iter().zip(&self.row_origins) {
                    frame.push_reference_frame(*origin, |frame| row.widget().render(ctx, frame));
                }
                self.items.render_filtered(
                    |i, _| {
                        let rect = self.item_rects[i];
                        if rect.size.width > Px(0) && rect.size.height > Px(0) {
                            Some(rect.origin)
                        } else {
                            None
                        }
                    },
                    ctx,
                    frame,
                );
            }

            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                for column in &self.columns {
                    column.widget().render_update(ctx, update);
                }
                for row in &self.rows {
                    row.widget().render_update(ctx, update);
                }
                self.items.render_update_all(ctx, update);
            }
        }

        assert!(!columns.is_empty());
        assert!(!rows.is_empty());

        GridNode {
            column_origins: vec![PxPoint::zero(); columns.len()],
            columns,
            row_origins: vec![PxPoint::zero(); rows.len()],
            rows,
            item_rects: vec![PxRect::zero(); items.len()],
            items,
            spacing: spacing.into_var(),
        }
    }

    /// Sets the `(column, row)` index of a widget in the grid.
    ///
    /// This property must be set in widgets placed in the grid [`items`].
    ///
    /// The default index is `(0, 0)`.
    ///
    /// [`items`]: #wp-items
    #[property(context, default((0, 0)))]
    pub fn index(child: impl UiNode, index: impl IntoVar<(u32, u32)>) -> impl UiNode {
        set_widget_state_update(child, IndexKey, index, |ctx, _| ctx.updates.layout_and_render())
    }

    /// Sets the `(column, row)` counts of columns and rows the widget takes.
    ///
    /// This property must be set in widgets placed in the grid [`items`].
    ///
    /// The default span is `(1, 1)`.
    ///
    /// [`items`]: #wp-items
    #[property(context, default((1, 1)))]
    pub fn span(child: impl UiNode, column_row: impl IntoVar<(u32, u32)>) -> impl UiNode {
        set_widget_state_update(child, SpanKey, column_row, |ctx, _| ctx.updates.layout_and_render())
    }

    /// Sets the z-order index of a widget in the grid.
    ///
    /// Item widgets are rendered sorted by this index then by the order they appear in [`items`]. The default
    /// index is `0` with `i32::MAX` being the top-most, or, rendered last.
    ///
    /// This property can also be set in the column and row *widgets*, by default rows are rendered on top o columns,
    /// but this property can override that. Note that item widgets are always rendered on-top columns and rows, this cannot be changed.
    #[property(context, default(0))]
    pub fn z_index(child: impl UiNode, z_index: impl IntoVar<i32>) -> impl UiNode {
        set_widget_state_update(child, ZIndexKey, z_index, |ctx, _| ctx.updates.render())
    }

    state_key! {
        /// Widget state key for the [`index`] value.
        ///
        /// [`index`]: fn@index
        pub struct IndexKey: (u32, u32);

        /// Widget state key for the [`span`] value.
        ///
        /// [`span`]: fn@span
        pub struct SpanKey: (u32, u32);

        /// Widget state key for the [`z_index`] value.
        ///
        /// [`z_index`]: fn@z_index
        pub struct ZIndexKey: i32;
    }
}

/// Row definition.
///
/// This is a specialized widget used to define a [grid!] row, it can only be used as one of the values in the [`rows`] property.
///
/// It does not actually contain any "cell" widget, but it affects their width and the row visual is rendered behind the items.
///
/// [`grid!`]: mod@crate::widgets::layouts::grid
/// [`rows`]: wp@crate::widgets::layouts::grid#wp-rows
#[widget($crate::widgets::layouts::grid::row)]
pub mod row {
    use super::*;

    properties! {
        /// Row height.
        ///
        /// Set to `100.pct()` by default which is the height of the grid divided by the row count.
        height = 100.pct();

        /// Row minimum height.
        ///
        /// Relative values work like the [`height`] property.
        ///
        /// [`height`]: #wp-height
        min_height;

        /// Row maximum height.
        ///
        /// Relative values work like the [`height`] property.
        ///
        /// [`height`]: #wp-height
        max_height;
    }

    #[doc(hidden)]
    pub fn new_child() -> impl UiNode {
        struct InnerHeightSamplerNode;
        #[impl_ui_node(none)]
        impl UiNode for InnerHeightSamplerNode {
            fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                // record the height.
                ctx.widget_state.set(InnerHeight, available_size.height);
                // min 1 to detect visibility collapsed.
                available_size.to_px_or(PxSize::splat(Px(1)))
            }
        }
        InnerHeightSamplerNode
    }

    #[doc(hidden)]
    pub fn new(child: impl UiNode, id: impl Into<WidgetId> + Clone) -> Definition {
        Definition {
            wgt: crate::core::widget_base::implicit_base::new(child, id).boxed_widget(),
        }
    }

    /// A constructed [`row!`] "widget".
    ///
    /// This struct deliberately does not implement [`Widget`] or [`UiNode`], it should only be used in the
    /// grid [`rows`] property.
    ///
    /// [`row!`]: mod@crate::widgets::layouts::grid::row
    /// [`rows`]: mod@crate::widgets::layouts::grid#wp-rows
    pub struct Definition {
        wgt: BoxedWidget,
    }
    impl Definition {
        /// Reference the definition as an [`Widget`] and [`UiNode`] implementer.
        pub fn widget(&self) -> &BoxedWidget {
            &self.wgt
        }

        /// Exclusive borrow the definition as an [`Widget`] and [`UiNode`] implementer.
        pub fn widget_mut(&mut self) -> &mut BoxedWidget {
            &mut self.wgt
        }

        /// Measure the row outer and inner heights.
        pub fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> (Px, AvailablePx) {
            let outer = self.wgt.measure(ctx, available_size).height;
            let inner = self.wgt.state().copy(InnerHeight).unwrap_or(AvailablePx::Infinite);
            (outer, inner)
        }
    }

    state_key! {
        struct InnerHeight: AvailablePx;
    }
}

/// Shorthand [`row!`] init with only the `height` non-default.
///
/// [`row!`]: mod@crate::widgets::layouts::grid::row
pub fn row(height: impl IntoVar<Length>) -> row::Definition {
    row!(height)
}

#[widget($crate::widgets::layouts::grid::column)]
pub mod column {
    use super::*;

    properties! {
        /// Row width.
        ///
        /// Set to `100.pct()` by default which is the width of the grid minus the width of
        /// columns with fixed widths and spacing divided by the number of columns with relative width.
        width = 100.pct();

        /// Row minimum width.
        ///
        /// Relative values work like the [`width`] property.
        ///
        /// [`width`]: #wp-width
        min_width;

        /// Row maximum width.
        ///
        /// Relative values work like the [`width`] property.
        ///
        /// [`width`]: #wp-width
        max_width;
    }

    #[doc(hidden)]
    pub fn new_child() -> impl UiNode {
        struct InnerWidthSamplerNode;
        #[impl_ui_node(none)]
        impl UiNode for InnerWidthSamplerNode {
            fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                // record the width.
                ctx.widget_state.set(InnerWidth, available_size.width);
                // min 1 to detect visibility collapsed.
                available_size.to_px_or(PxSize::splat(Px(1)))
            }
        }
        InnerWidthSamplerNode
    }

    #[doc(hidden)]
    pub fn new(child: impl UiNode, id: impl Into<WidgetId> + Clone) -> Definition {
        Definition {
            wgt: crate::core::widget_base::implicit_base::new(child, id).boxed_widget(),
        }
    }

    /// A constructed [`column!`] "widget".
    ///
    /// This struct deliberately does not implement [`Widget`] or [`UiNode`], it should only be used in the
    /// grid [`columns`] property.
    ///
    /// [`column!`]: mod@crate::widgets::layouts::grid::column
    /// [`columns`]: mod@crate::widgets::layouts::grid#wp-columns
    pub struct Definition {
        wgt: BoxedWidget,
    }
    impl Definition {
        /// Reference the definition as an [`Widget`] and [`UiNode`] implementer.
        pub fn widget(&self) -> &BoxedWidget {
            &self.wgt
        }

        /// Exclusive borrow the definition as an [`Widget`] and [`UiNode`] implementer.
        pub fn widget_mut(&mut self) -> &mut BoxedWidget {
            &mut self.wgt
        }

        /// Returns `true` if the width is not dependent on the available size.
        pub fn is_fixed_width(&mut self, ctx: &mut LayoutContext) -> bool {
            let measure_a = self.wgt.measure(ctx, AvailableSize::from_size(PxSize::zero()));
            let measure_b = self.wgt.measure(ctx, AvailableSize::from_size(PxSize::splat(Px(9000))));
            measure_a.width == measure_b.width
        }

        /// Measure the column outer and inner widths.
        pub fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> (Px, AvailablePx) {
            let outer = self.wgt.measure(ctx, available_size).width;
            let inner = self.wgt.state().copy(InnerWidth).unwrap_or(AvailablePx::Infinite);
            (outer, inner)
        }
    }

    state_key! {
        struct InnerWidth: AvailablePx;
    }
}
