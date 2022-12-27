use crate::prelude::new_widget::*;

#[widget($crate::widgets::layouts::stack)]
pub mod stack {
    pub use super::StackDirection;
    use super::*;

    inherit!(widget_base::base);

    properties! {
        /// Widget items.
        pub widget_base::children;

        /// Stack direction.
        pub direction(impl IntoVar<StackDirection>);

        /// Space in-between items.
        pub spacing(impl IntoVar<Length>);

        /// Spacing around the items stack, inside the border.
        pub crate::properties::padding;

        /// Items alignment.
        ///
        /// The default is [`FILL`].
        ///
        /// [`FILL`]: Align::FILL
        pub children_align(impl IntoVar<Align>) = Align::FILL;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            let children = wgt.capture_ui_node_list_or_empty(property_id!(self::children));
            let spacing = wgt.capture_var_or_default(property_id!(self::spacing));
            let direction = wgt.capture_var_or_default(property_id!(self::direction));
            let children_align = wgt.capture_var_or_else(property_id!(self::children_align), || Align::FILL);

            let node = StackNode {
                children: ZSortingList::new(children),
                direction,
                spacing,
                children_align,
            };
            let child = widget_base::nodes::children_layout(node);

            wgt.set_child(child);
        });
    }
}

#[ui_node(struct StackNode {
    children: impl UiNodeList,

    #[var] direction: impl Var<StackDirection>,
    #[var] spacing: impl Var<Length>,
    #[var] children_align: impl Var<Align>,
})]
impl UiNode for StackNode {
    fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
        let mut changed = false;
        self.children.update_all(ctx, updates, &mut changed);

        if changed || self.direction.is_new(ctx) || self.spacing.is_new(ctx) || self.children_align.is_new(ctx) {
            ctx.updates.layout_render();
        }
    }

    fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
        let constrains = ctx.constrains();
        if let Some(known) = constrains.fill_or_exact() {
            return known;
        }

        todo! {}
    }

    fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        let mut prev_rect = PxRect::zero();
        self.children.for_each_mut(|i, c| {
            let direction = self.direction.get().layout(ctx, |_| PxVector::zero());
            true
        });
        let spacing = self.spacing.get().layout(todo!(), |_| Px(0));
    }
}

#[derive(Debug, Default, Clone)]
pub struct StackDirection {
    pub x: Length,
    pub y: Length,
}
impl StackDirection {
    /// horizontal stack
    pub fn h() -> Self {
        Self {
            x: 100.pct().into(),
            y: 0.into(),
        }
    }
    /// vertical stack
    pub fn v() -> Self {
        Self {
            x: 0.into(),
            y: 100.pct().into(),
        }
    }

    /// depth stack
    pub fn z() -> Self {
        Self { x: 0.into(), y: 0.into() }
    }

    /// Compute the vector in a layout context.
    pub fn layout(&self, ctx: &LayoutMetrics, mut default_value: impl FnMut(&LayoutMetrics) -> PxVector) -> PxVector {
        PxVector::new(
            self.x.layout(ctx.for_x(), |ctx| default_value(ctx.metrics).x),
            self.y.layout(ctx.for_y(), |ctx| default_value(ctx.metrics).y),
        )
    }
}
