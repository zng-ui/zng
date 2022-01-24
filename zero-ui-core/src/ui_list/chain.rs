use crate::{
    context::{InfoContext, LayoutContext, RenderContext, WidgetContext},
    event::EventUpdateArgs,
    render::{FrameBuilder, FrameUpdate},
    state::StateMap,
    units::{AvailableSize, PxPoint, PxRect, PxSize},
    widget_base::Visibility,
    widget_info::{WidgetInfoBuilder, WidgetOffset, WidgetSubscriptions},
    UiNodeList, UiNodeVec, WidgetFilterArgs, WidgetId, WidgetList, WidgetVec,
};

/// Two [`WidgetList`] lists chained.
///
/// See [`WidgetList::chain`] for more information.
pub struct WidgetListChain<A: WidgetList, B: WidgetList>(pub(super) A, pub(super) B);

impl<A: WidgetList, B: WidgetList> UiNodeList for WidgetListChain<A, B> {
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
    fn update_all(&mut self, ctx: &mut WidgetContext) {
        self.0.update_all(ctx);
        self.1.update_all(ctx);
    }

    #[inline(always)]
    fn event_all<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
        self.0.event_all(ctx, args);
        self.1.event_all(ctx, args);
    }

    #[inline(always)]
    fn measure_all<AS, D>(&mut self, ctx: &mut LayoutContext, mut available_size: AS, mut desired_size: D)
    where
        AS: FnMut(usize, &mut LayoutContext) -> AvailableSize,
        D: FnMut(usize, PxSize, &mut LayoutContext),
    {
        self.0
            .measure_all(ctx, |i, c| available_size(i, c), |i, l, c| desired_size(i, l, c));
        let offset = self.0.len();
        self.1
            .measure_all(ctx, |i, c| available_size(i + offset, c), |i, l, c| desired_size(i + offset, l, c));
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
    fn arrange_all<F>(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, mut final_rect: F)
    where
        F: FnMut(usize, &mut LayoutContext) -> PxRect,
    {
        self.0.arrange_all(ctx, widget_offset, |i, c| final_rect(i, c));
        let offset = self.0.len();
        self.1.arrange_all(ctx, widget_offset, |i, c| final_rect(i + offset, c));
    }

    #[inline]
    fn widget_arrange(&mut self, index: usize, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_arrange(index, ctx, widget_offset, final_size)
        } else {
            self.1.widget_arrange(index - a_len, ctx, widget_offset, final_size)
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
    fn render_all<O>(&self, mut origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        O: FnMut(usize) -> PxPoint,
    {
        self.0.render_all(&mut origin, ctx, frame);
        let offset = self.0.len();
        self.1.render_all(|i| origin(i + offset), ctx, frame);
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
    #[inline]
    fn boxed_widget_all(self) -> WidgetVec {
        let mut a = self.0.boxed_widget_all();
        a.extend(self.1.boxed_widget_all());
        a
    }

    #[inline(always)]
    fn render_filtered<O>(&self, mut origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        O: FnMut(usize, WidgetFilterArgs) -> Option<PxPoint>,
    {
        self.0.render_filtered(|i, a| origin(i, a), ctx, frame);
        let offset = self.0.len();
        self.1.render_filtered(|i, a| origin(i + offset, a), ctx, frame);
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

    fn widget_outer_bounds(&self, index: usize) -> PxRect {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_outer_bounds(index)
        } else {
            self.1.widget_outer_bounds(index - a_len)
        }
    }

    fn widget_inner_bounds(&self, index: usize) -> PxRect {
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
    fn update_all(&mut self, ctx: &mut WidgetContext) {
        self.0.update_all(ctx);
        self.1.update_all(ctx);
    }

    #[inline(always)]
    fn event_all<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
        self.0.event_all(ctx, args);
        self.1.event_all(ctx, args);
    }

    #[inline(always)]
    fn measure_all<AS, D>(&mut self, ctx: &mut LayoutContext, mut available_size: AS, mut desired_size: D)
    where
        AS: FnMut(usize, &mut LayoutContext) -> AvailableSize,
        D: FnMut(usize, PxSize, &mut LayoutContext),
    {
        self.0
            .measure_all(ctx, |i, c| available_size(i, c), |i, l, c| desired_size(i, l, c));
        let offset = self.0.len();
        self.1
            .measure_all(ctx, |i, c| available_size(i + offset, c), |i, l, c| desired_size(i + offset, l, c));
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
    fn arrange_all<F>(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, mut final_rect: F)
    where
        F: FnMut(usize, &mut LayoutContext) -> PxRect,
    {
        self.0.arrange_all(ctx, widget_offset, |i, c| final_rect(i, c));
        let offset = self.0.len();
        self.1.arrange_all(ctx, widget_offset, |i, c| final_rect(i + offset, c));
    }

    #[inline]
    fn widget_arrange(&mut self, index: usize, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_arrange(index, ctx, widget_offset, final_size)
        } else {
            self.1.widget_arrange(index - a_len, ctx, widget_offset, final_size)
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
    fn render_all<O>(&self, mut origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        O: FnMut(usize) -> PxPoint,
    {
        self.0.render_all(&mut origin, ctx, frame);
        let offset = self.0.len();
        self.1.render_all(|i| origin(i + offset), ctx, frame);
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
