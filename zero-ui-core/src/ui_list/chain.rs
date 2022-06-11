use crate::{
    context::{InfoContext, LayoutContext, MeasureContext, RenderContext, StateMap, WidgetContext},
    event::EventUpdateArgs,
    render::{FrameBuilder, FrameUpdate},
    ui_list::{
        OffsetUiListObserver, PosLayoutArgs, PreLayoutArgs, UiListObserver, UiNodeList, UiNodeVec, WidgetFilterArgs, WidgetList, WidgetVec,
    },
    units::PxSize,
    widget_info::{
        WidgetBorderInfo, WidgetBoundsInfo, WidgetInfoBuilder, WidgetLayout, WidgetLayoutTranslation, WidgetRenderInfo, WidgetSubscriptions,
    },
    WidgetId,
};

/// Two [`WidgetList`] lists chained.
///
/// See [`WidgetList::chain`] for more information.
pub struct WidgetListChain<A: WidgetList, B: WidgetList>(pub(super) A, pub(super) B);

impl<A: WidgetList, B: WidgetList> UiNodeList for WidgetListChain<A, B> {
    fn is_fixed(&self) -> bool {
        self.0.is_fixed() && self.0.is_fixed()
    }

    fn len(&self) -> usize {
        self.0.len() + self.1.len()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty() && self.1.is_empty()
    }

    fn boxed_all(self) -> UiNodeVec {
        let mut a = self.0.boxed_all();
        a.extend(self.1.boxed_all());
        a
    }

    fn init_all(&mut self, ctx: &mut WidgetContext) {
        self.0.init_all(ctx);
        self.1.init_all(ctx);
    }

    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        self.0.deinit_all(ctx);
        self.1.deinit_all(ctx);
    }

    fn update_all<O: UiListObserver>(&mut self, ctx: &mut WidgetContext, observer: &mut O) {
        self.0.update_all(ctx, observer);
        self.1.update_all(ctx, &mut OffsetUiListObserver(self.0.len(), observer));
    }

    fn event_all<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
        self.0.event_all(ctx, args);
        self.1.event_all(ctx, args);
    }

    fn measure_all<C, D>(&self, ctx: &mut MeasureContext, mut pre_measure: C, mut pos_measure: D)
    where
        C: FnMut(&mut MeasureContext, &mut super::PreMeasureArgs),
        D: FnMut(&mut MeasureContext, super::PosMeasureArgs),
    {
        self.0.measure_all(ctx, &mut pre_measure, &mut pos_measure);
        let offset = self.0.len();
        self.1.measure_all(
            ctx,
            |ctx, args| {
                args.index += offset;
                pre_measure(ctx, args);
                args.index -= offset;
            },
            |ctx, mut args| {
                args.index += offset;
                pos_measure(ctx, args);
            },
        )
    }

    fn item_measure(&self, index: usize, ctx: &mut MeasureContext) -> PxSize {
        let a_len = self.0.len();
        if index < a_len {
            self.0.item_measure(index, ctx)
        } else {
            self.1.item_measure(index - a_len, ctx)
        }
    }

    fn layout_all<C, D>(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout, mut pre_layout: C, mut pos_layout: D)
    where
        C: FnMut(&mut LayoutContext, &mut WidgetLayout, &mut PreLayoutArgs),
        D: FnMut(&mut LayoutContext, &mut WidgetLayout, PosLayoutArgs),
    {
        self.0.layout_all(ctx, wl, &mut pre_layout, &mut pos_layout);
        let offset = self.0.len();
        self.1.layout_all(
            ctx,
            wl,
            |ctx, wl, args| {
                args.index += offset;
                pre_layout(ctx, wl, args);
                args.index -= offset;
            },
            |ctx, wl, mut args| {
                args.index += offset;
                pos_layout(ctx, wl, args);
            },
        );
    }

    fn item_layout(&mut self, index: usize, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        let a_len = self.0.len();
        if index < a_len {
            self.0.item_layout(index, ctx, wl)
        } else {
            self.1.item_layout(index - a_len, ctx, wl)
        }
    }

    fn info_all(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        self.0.info_all(ctx, info);
        self.1.info_all(ctx, info);
    }

    fn subscriptions_all(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
        self.0.subscriptions_all(ctx, subs);
        self.1.subscriptions_all(ctx, subs);
    }

    fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.0.render_all(ctx, frame);
        self.1.render_all(ctx, frame);
    }

    fn item_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.item_render(index, ctx, frame)
        } else {
            self.1.item_render(index - a_len, ctx, frame)
        }
    }

    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.0.render_update_all(ctx, update);
        self.1.render_update_all(ctx, update);
    }

    fn item_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.item_render_update(index, ctx, update)
        } else {
            self.1.item_render_update(index - a_len, ctx, update)
        }
    }

    fn try_item_id(&self, index: usize) -> Option<WidgetId> {
        let a_len = self.0.len();
        if index < a_len {
            self.0.try_item_id(index)
        } else {
            self.1.try_item_id(index - a_len)
        }
    }

    fn try_item_state(&self, index: usize) -> Option<&StateMap> {
        let a_len = self.0.len();
        if index < a_len {
            self.0.try_item_state(index)
        } else {
            self.1.try_item_state(index - a_len)
        }
    }

    fn try_item_state_mut(&mut self, index: usize) -> Option<&mut StateMap> {
        let a_len = self.0.len();
        if index < a_len {
            self.0.try_item_state_mut(index)
        } else {
            self.1.try_item_state_mut(index - a_len)
        }
    }

    fn try_item_bounds_info(&self, index: usize) -> Option<&WidgetBoundsInfo> {
        let a_len = self.0.len();
        if index < a_len {
            self.0.try_item_bounds_info(index)
        } else {
            self.1.try_item_bounds_info(index - a_len)
        }
    }

    fn try_item_border_info(&self, index: usize) -> Option<&WidgetBorderInfo> {
        let a_len = self.0.len();
        if index < a_len {
            self.0.try_item_border_info(index)
        } else {
            self.1.try_item_border_info(index - a_len)
        }
    }

    fn try_item_render_info(&self, index: usize) -> Option<&WidgetRenderInfo> {
        let a_len = self.0.len();
        if index < a_len {
            self.0.try_item_render_info(index)
        } else {
            self.1.try_item_render_info(index - a_len)
        }
    }

    fn render_node_filtered<F>(&self, mut filter: F, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        F: FnMut(super::UiNodeFilterArgs) -> bool,
    {
        self.0.render_node_filtered(&mut filter, ctx, frame);
        let offset = self.0.len();
        self.1.render_node_filtered(
            |mut a| {
                a.index += offset;
                filter(a)
            },
            ctx,
            frame,
        );
    }

    fn try_item_outer<F, R>(&mut self, index: usize, wl: &mut WidgetLayout, keep_previous: bool, transform: F) -> Option<R>
    where
        F: FnOnce(&mut WidgetLayoutTranslation, PosLayoutArgs) -> R,
    {
        let a_len = self.0.len();
        if index < a_len {
            self.0.try_item_outer(index, wl, keep_previous, transform)
        } else {
            self.1.try_item_outer(index - a_len, wl, keep_previous, transform)
        }
    }

    fn try_outer_all<F>(&mut self, wl: &mut WidgetLayout, keep_previous: bool, mut transform: F)
    where
        F: FnMut(&mut WidgetLayoutTranslation, PosLayoutArgs),
    {
        self.0.try_outer_all(wl, keep_previous, &mut transform);
        let offset = self.0.len();
        self.1.try_outer_all(wl, keep_previous, |wlt, mut args| {
            args.index += offset;
            transform(wlt, args);
        })
    }

    fn count_nodes<F>(&self, mut filter: F) -> usize
    where
        F: FnMut(super::UiNodeFilterArgs) -> bool,
    {
        let a_count = self.0.count_nodes(&mut filter);

        let offset = self.0.len();
        let b_count = self.1.count_nodes(|mut args| {
            args.index += offset;
            filter(args)
        });

        a_count + b_count
    }
}

impl<A: WidgetList, B: WidgetList> WidgetList for WidgetListChain<A, B> {
    fn count<F>(&self, mut filter: F) -> usize
    where
        F: FnMut(WidgetFilterArgs) -> bool,
        Self: Sized,
    {
        let a_count = self.0.count(&mut filter);

        let offset = self.0.len();
        let b_count = self.1.count(|mut args| {
            args.index += offset;
            filter(args)
        });

        a_count + b_count
    }

    fn boxed_widget_all(self) -> WidgetVec {
        let mut a = self.0.boxed_widget_all();
        a.extend(self.1.boxed_widget_all());
        a
    }

    fn render_filtered<F>(&self, mut filter: F, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        F: FnMut(WidgetFilterArgs) -> bool,
    {
        self.0.render_filtered(&mut filter, ctx, frame);
        let offset = self.0.len();
        self.1.render_filtered(
            |mut a| {
                a.index += offset;
                filter(a)
            },
            ctx,
            frame,
        );
    }

    fn item_id(&self, index: usize) -> WidgetId {
        let a_len = self.0.len();
        if index < a_len {
            self.0.item_id(index)
        } else {
            self.1.item_id(index - a_len)
        }
    }

    fn item_state(&self, index: usize) -> &StateMap {
        let a_len = self.0.len();
        if index < a_len {
            self.0.item_state(index)
        } else {
            self.1.item_state(index - a_len)
        }
    }

    fn item_state_mut(&mut self, index: usize) -> &mut StateMap {
        let a_len = self.0.len();
        if index < a_len {
            self.0.item_state_mut(index)
        } else {
            self.1.item_state_mut(index - a_len)
        }
    }

    fn item_bounds_info(&self, index: usize) -> &WidgetBoundsInfo {
        let a_len = self.0.len();
        if index < a_len {
            self.0.item_bounds_info(index)
        } else {
            self.1.item_bounds_info(index - a_len)
        }
    }

    fn item_border_info(&self, index: usize) -> &WidgetBorderInfo {
        let a_len = self.0.len();
        if index < a_len {
            self.0.item_border_info(index)
        } else {
            self.1.item_border_info(index - a_len)
        }
    }

    fn item_render_info(&self, index: usize) -> &WidgetRenderInfo {
        let a_len = self.0.len();
        if index < a_len {
            self.0.item_render_info(index)
        } else {
            self.1.item_render_info(index - a_len)
        }
    }

    fn item_outer<F, R>(&mut self, index: usize, wl: &mut WidgetLayout, keep_previous: bool, transform: F) -> R
    where
        F: FnOnce(&mut WidgetLayoutTranslation, PosLayoutArgs) -> R,
    {
        let a_len = self.0.len();
        if index < a_len {
            self.0.item_outer(index, wl, keep_previous, transform)
        } else {
            self.1.item_outer(index - a_len, wl, keep_previous, transform)
        }
    }

    fn outer_all<F>(&mut self, wl: &mut WidgetLayout, keep_previous: bool, mut transform: F)
    where
        F: FnMut(&mut WidgetLayoutTranslation, PosLayoutArgs),
    {
        self.0.outer_all(wl, keep_previous, &mut transform);
        let offset = self.0.len();
        self.1.outer_all(wl, keep_previous, |wlt, mut args| {
            args.index += offset;
            transform(wlt, args);
        })
    }
}

/// Two [`UiNodeList`] lists chained.
///
/// See [`UiNodeList::chain_nodes`] for more information.
pub struct UiNodeListChain<A: UiNodeList, B: UiNodeList>(pub(super) A, pub(super) B);

impl<A: UiNodeList, B: UiNodeList> UiNodeList for UiNodeListChain<A, B> {
    fn is_fixed(&self) -> bool {
        false
    }

    fn len(&self) -> usize {
        self.0.len() + self.1.len()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty() && self.1.is_empty()
    }

    fn boxed_all(self) -> UiNodeVec {
        let mut a = self.0.boxed_all();
        a.extend(self.1.boxed_all());
        a
    }

    fn init_all(&mut self, ctx: &mut WidgetContext) {
        self.0.init_all(ctx);
        self.1.init_all(ctx);
    }

    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        self.0.deinit_all(ctx);
        self.1.deinit_all(ctx);
    }

    fn update_all<O: UiListObserver>(&mut self, ctx: &mut WidgetContext, observer: &mut O) {
        self.0.update_all(ctx, observer);
        self.1.update_all(ctx, &mut OffsetUiListObserver(self.0.len(), observer));
    }

    fn event_all<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
        self.0.event_all(ctx, args);
        self.1.event_all(ctx, args);
    }

    fn measure_all<C, D>(&self, ctx: &mut MeasureContext, mut pre_measure: C, mut pos_measure: D)
    where
        C: FnMut(&mut MeasureContext, &mut super::PreMeasureArgs),
        D: FnMut(&mut MeasureContext, super::PosMeasureArgs),
    {
        self.0.measure_all(ctx, &mut pre_measure, &mut pos_measure);
        let offset = self.0.len();
        self.1.measure_all(
            ctx,
            |ctx, args| {
                args.index += offset;
                pre_measure(ctx, args);
                args.index -= offset;
            },
            |ctx, mut args| {
                args.index += offset;
                pos_measure(ctx, args);
            },
        )
    }

    fn item_measure(&self, index: usize, ctx: &mut MeasureContext) -> PxSize {
        let a_len = self.0.len();
        if index < a_len {
            self.0.item_measure(index, ctx)
        } else {
            self.1.item_measure(index - a_len, ctx)
        }
    }

    fn layout_all<C, D>(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout, mut pre_layout: C, mut pos_layout: D)
    where
        C: FnMut(&mut LayoutContext, &mut WidgetLayout, &mut PreLayoutArgs),
        D: FnMut(&mut LayoutContext, &mut WidgetLayout, PosLayoutArgs),
    {
        self.0.layout_all(ctx, wl, &mut pre_layout, &mut pos_layout);
        let offset = self.0.len();
        self.1.layout_all(
            ctx,
            wl,
            |ctx, wl, mut args| {
                args.index += offset;
                pre_layout(ctx, wl, args)
            },
            |ctx, wl, mut args| {
                args.index += offset;
                pos_layout(ctx, wl, args)
            },
        );
    }

    fn item_layout(&mut self, index: usize, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        let a_len = self.0.len();
        if index < a_len {
            self.0.item_layout(index, ctx, wl)
        } else {
            self.1.item_layout(index - a_len, ctx, wl)
        }
    }

    fn info_all(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        self.0.info_all(ctx, info);
        self.1.info_all(ctx, info);
    }

    fn subscriptions_all(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
        self.0.subscriptions_all(ctx, subs);
        self.1.subscriptions_all(ctx, subs);
    }

    fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.0.render_all(ctx, frame);
        self.1.render_all(ctx, frame);
    }

    fn item_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.item_render(index, ctx, frame)
        } else {
            self.1.item_render(index - a_len, ctx, frame)
        }
    }

    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.0.render_update_all(ctx, update);
        self.1.render_update_all(ctx, update);
    }

    fn item_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.item_render_update(index, ctx, update)
        } else {
            self.1.item_render_update(index - a_len, ctx, update)
        }
    }

    fn try_item_id(&self, index: usize) -> Option<WidgetId> {
        let a_len = self.0.len();
        if index < a_len {
            self.0.try_item_id(index)
        } else {
            self.1.try_item_id(index - a_len)
        }
    }

    fn try_item_state(&self, index: usize) -> Option<&StateMap> {
        let a_len = self.0.len();
        if index < a_len {
            self.0.try_item_state(index)
        } else {
            self.1.try_item_state(index - a_len)
        }
    }

    fn try_item_state_mut(&mut self, index: usize) -> Option<&mut StateMap> {
        let a_len = self.0.len();
        if index < a_len {
            self.0.try_item_state_mut(index)
        } else {
            self.1.try_item_state_mut(index - a_len)
        }
    }

    fn try_item_bounds_info(&self, index: usize) -> Option<&WidgetBoundsInfo> {
        let a_len = self.0.len();
        if index < a_len {
            self.0.try_item_bounds_info(index)
        } else {
            self.1.try_item_bounds_info(index - a_len)
        }
    }

    fn try_item_border_info(&self, index: usize) -> Option<&WidgetBorderInfo> {
        let a_len = self.0.len();
        if index < a_len {
            self.0.try_item_border_info(index)
        } else {
            self.1.try_item_border_info(index - a_len)
        }
    }

    fn try_item_render_info(&self, index: usize) -> Option<&WidgetRenderInfo> {
        let a_len = self.0.len();
        if index < a_len {
            self.0.try_item_render_info(index)
        } else {
            self.1.try_item_render_info(index - a_len)
        }
    }

    fn render_node_filtered<F>(&self, mut filter: F, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        F: FnMut(super::UiNodeFilterArgs) -> bool,
    {
        self.0.render_node_filtered(&mut filter, ctx, frame);
        let offset = self.0.len();
        self.1.render_node_filtered(
            |mut a| {
                a.index += offset;
                filter(a)
            },
            ctx,
            frame,
        );
    }

    fn try_item_outer<F, R>(&mut self, index: usize, wl: &mut WidgetLayout, keep_previous: bool, transform: F) -> Option<R>
    where
        F: FnOnce(&mut WidgetLayoutTranslation, PosLayoutArgs) -> R,
    {
        let a_len = self.0.len();
        if index < a_len {
            self.0.try_item_outer(index, wl, keep_previous, transform)
        } else {
            self.1.try_item_outer(index - a_len, wl, keep_previous, transform)
        }
    }

    fn try_outer_all<F>(&mut self, wl: &mut WidgetLayout, keep_previous: bool, mut transform: F)
    where
        F: FnMut(&mut WidgetLayoutTranslation, PosLayoutArgs),
    {
        self.0.try_outer_all(wl, keep_previous, &mut transform);
        let offset = self.0.len();
        self.1.try_outer_all(wl, keep_previous, |wlt, mut args| {
            args.index += offset;
            transform(wlt, args);
        })
    }

    fn count_nodes<F>(&self, mut filter: F) -> usize
    where
        F: FnMut(super::UiNodeFilterArgs) -> bool,
    {
        let a_count = self.0.count_nodes(&mut filter);

        let offset = self.0.len();
        let b_count = self.1.count_nodes(|mut args| {
            args.index += offset;
            filter(args)
        });

        a_count + b_count
    }
}
