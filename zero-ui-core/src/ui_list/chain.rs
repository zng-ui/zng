use crate::{
    context::{InfoContext, LayoutContext, RenderContext, WidgetContext},
    event::EventUpdateArgs,
    render::{FrameBuilder, FrameUpdate},
    state::StateMap,
    ui_list::{
        AvailableSizeArgs, DesiredSizeArgs, FinalSizeArgs, OffsetUiListObserver, UiListObserver, UiNodeList, UiNodeVec, WidgetFilterArgs,
        WidgetList, WidgetVec,
    },
    units::{AvailableSize, PxSize},
    widget_base::Visibility,
    widget_info::{BoundsInfo, WidgetInfoBuilder, WidgetLayout, WidgetSubscriptions},
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

    #[inline]
    fn len(&self) -> usize {
        self.0.len() + self.1.len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.0.is_empty() && self.1.is_empty()
    }

    #[inline]
    fn boxed_all(self) -> UiNodeVec {
        let mut a = self.0.boxed_all();
        a.extend(self.1.boxed_all());
        a
    }

    #[inline(always)]
    fn init_all(&mut self, ctx: &mut WidgetContext) {
        self.0.init_all(ctx);
        self.1.init_all(ctx);
    }

    #[inline(always)]
    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        self.0.deinit_all(ctx);
        self.1.deinit_all(ctx);
    }

    #[inline(always)]
    fn update_all<O: UiListObserver>(&mut self, ctx: &mut WidgetContext, observer: &mut O) {
        self.0.update_all(ctx, observer);
        self.1.update_all(ctx, &mut OffsetUiListObserver(self.0.len(), observer));
    }

    #[inline(always)]
    fn event_all<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
        self.0.event_all(ctx, args);
        self.1.event_all(ctx, args);
    }

    #[inline(always)]
    fn measure_all<AS, D>(&mut self, ctx: &mut LayoutContext, mut available_size: AS, mut desired_size: D)
    where
        AS: FnMut(&mut LayoutContext, AvailableSizeArgs) -> AvailableSize,
        D: FnMut(&mut LayoutContext, DesiredSizeArgs),
    {
        self.0.measure_all(ctx, &mut available_size, &mut desired_size);
        let offset = self.0.len();
        self.1.measure_all(
            ctx,
            |ctx, mut args| {
                args.index += offset;
                available_size(ctx, args)
            },
            |ctx, mut args| {
                args.index += offset;
                desired_size(ctx, args)
            },
        );
    }

    #[inline]
    fn widget_measure(&mut self, index: usize, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_measure(index, ctx, available_size)
        } else {
            self.1.widget_measure(index - a_len, ctx, available_size)
        }
    }

    #[inline(always)]
    fn arrange_all<F>(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, mut final_size: F)
    where
        F: FnMut(&mut LayoutContext, &mut FinalSizeArgs) -> PxSize,
    {
        self.0.arrange_all(ctx, widget_layout, &mut final_size);
        let offset = self.0.len();
        self.1.arrange_all(ctx, widget_layout, |ctx, args| {
            args.index += offset;
            final_size(ctx, args)
        });
    }

    #[inline]
    fn widget_arrange(&mut self, index: usize, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_arrange(index, ctx, widget_layout, final_size)
        } else {
            self.1.widget_arrange(index - a_len, ctx, widget_layout, final_size)
        }
    }

    #[inline]
    fn info_all(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        self.0.info_all(ctx, info);
        self.1.info_all(ctx, info);
    }

    fn widget_info(&self, index: usize, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_info(index, ctx, info)
        } else {
            self.1.widget_info(index - a_len, ctx, info)
        }
    }

    #[inline]
    fn subscriptions_all(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        self.0.subscriptions_all(ctx, subscriptions);
        self.1.subscriptions_all(ctx, subscriptions);
    }

    fn widget_subscriptions(&self, index: usize, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_subscriptions(index, ctx, subscriptions);
        } else {
            self.1.widget_subscriptions(index - a_len, ctx, subscriptions);
        }
    }

    #[inline(always)]
    fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.0.render_all(ctx, frame);
        self.1.render_all(ctx, frame);
    }

    #[inline]
    fn widget_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_render(index, ctx, frame)
        } else {
            self.1.widget_render(index - a_len, ctx, frame)
        }
    }

    #[inline(always)]
    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.0.render_update_all(ctx, update);
        self.1.render_update_all(ctx, update);
    }

    #[inline]
    fn widget_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_render_update(index, ctx, update)
        } else {
            self.1.widget_render_update(index - a_len, ctx, update)
        }
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

    #[inline]
    fn boxed_widget_all(self) -> WidgetVec {
        let mut a = self.0.boxed_widget_all();
        a.extend(self.1.boxed_widget_all());
        a
    }

    #[inline(always)]
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

    #[inline]
    fn widget_id(&self, index: usize) -> WidgetId {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_id(index)
        } else {
            self.1.widget_id(index - a_len)
        }
    }

    #[inline]
    fn widget_state(&self, index: usize) -> &StateMap {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_state(index)
        } else {
            self.1.widget_state(index - a_len)
        }
    }

    #[inline]
    fn widget_state_mut(&mut self, index: usize) -> &mut StateMap {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_state_mut(index)
        } else {
            self.1.widget_state_mut(index - a_len)
        }
    }

    fn widget_outer_bounds(&self, index: usize) -> &BoundsInfo {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_outer_bounds(index)
        } else {
            self.1.widget_outer_bounds(index - a_len)
        }
    }

    fn widget_inner_bounds(&self, index: usize) -> &BoundsInfo {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_inner_bounds(index)
        } else {
            self.1.widget_inner_bounds(index - a_len)
        }
    }

    fn widget_visibility(&self, index: usize) -> Visibility {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_visibility(index)
        } else {
            self.1.widget_visibility(index - a_len)
        }
    }
}

/// Two [`UiNodeList`] lists chained.
///
/// See [`UiNodeList::chain_nodes`] for more information.
pub struct UiNodeListChain<A: UiNodeList, B: UiNodeList>(pub(super) A, pub(super) B);

impl<A: UiNodeList, B: UiNodeList> UiNodeList for UiNodeListChain<A, B> {
    #[inline]
    fn is_fixed(&self) -> bool {
        false
    }

    #[inline]
    fn len(&self) -> usize {
        self.0.len() + self.1.len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.0.is_empty() && self.1.is_empty()
    }

    #[inline]
    fn boxed_all(self) -> UiNodeVec {
        let mut a = self.0.boxed_all();
        a.extend(self.1.boxed_all());
        a
    }

    #[inline(always)]
    fn init_all(&mut self, ctx: &mut WidgetContext) {
        self.0.init_all(ctx);
        self.1.init_all(ctx);
    }

    #[inline(always)]
    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        self.0.deinit_all(ctx);
        self.1.deinit_all(ctx);
    }

    #[inline(always)]
    fn update_all<O: UiListObserver>(&mut self, ctx: &mut WidgetContext, observer: &mut O) {
        self.0.update_all(ctx, observer);
        self.1.update_all(ctx, &mut OffsetUiListObserver(self.0.len(), observer));
    }

    #[inline(always)]
    fn event_all<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
        self.0.event_all(ctx, args);
        self.1.event_all(ctx, args);
    }

    #[inline(always)]
    fn measure_all<AS, D>(&mut self, ctx: &mut LayoutContext, mut available_size: AS, mut desired_size: D)
    where
        AS: FnMut(&mut LayoutContext, AvailableSizeArgs) -> AvailableSize,
        D: FnMut(&mut LayoutContext, DesiredSizeArgs),
    {
        self.0.measure_all(ctx, &mut available_size, &mut desired_size);
        let offset = self.0.len();
        self.1.measure_all(
            ctx,
            |ctx, mut args| {
                args.index += offset;
                available_size(ctx, args)
            },
            |ctx, mut args| {
                args.index += offset;
                desired_size(ctx, args)
            },
        );
    }

    #[inline]
    fn widget_measure(&mut self, index: usize, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_measure(index, ctx, available_size)
        } else {
            self.1.widget_measure(index - a_len, ctx, available_size)
        }
    }

    #[inline(always)]
    fn arrange_all<F>(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, mut final_size: F)
    where
        F: FnMut(&mut LayoutContext, &mut FinalSizeArgs) -> PxSize,
    {
        self.0.arrange_all(ctx, widget_layout, &mut final_size);
        let offset = self.0.len();
        self.1.arrange_all(ctx, widget_layout, |ctx, args| {
            args.index += offset;
            final_size(ctx, args)
        });
    }

    #[inline]
    fn widget_arrange(&mut self, index: usize, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_arrange(index, ctx, widget_layout, final_size)
        } else {
            self.1.widget_arrange(index - a_len, ctx, widget_layout, final_size)
        }
    }

    #[inline]
    fn info_all(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        self.0.info_all(ctx, info);
        self.1.info_all(ctx, info);
    }

    fn widget_info(&self, index: usize, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_info(index, ctx, info)
        } else {
            self.1.widget_info(index - a_len, ctx, info)
        }
    }

    #[inline]
    fn subscriptions_all(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        self.0.subscriptions_all(ctx, subscriptions);
        self.1.subscriptions_all(ctx, subscriptions);
    }

    fn widget_subscriptions(&self, index: usize, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_subscriptions(index, ctx, subscriptions);
        } else {
            self.1.widget_subscriptions(index - a_len, ctx, subscriptions);
        }
    }

    #[inline(always)]
    fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.0.render_all(ctx, frame);
        self.1.render_all(ctx, frame);
    }

    #[inline]
    fn widget_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_render(index, ctx, frame)
        } else {
            self.1.widget_render(index - a_len, ctx, frame)
        }
    }

    #[inline(always)]
    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.0.render_update_all(ctx, update);
        self.1.render_update_all(ctx, update);
    }

    #[inline]
    fn widget_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_render_update(index, ctx, update)
        } else {
            self.1.widget_render_update(index - a_len, ctx, update)
        }
    }
}
