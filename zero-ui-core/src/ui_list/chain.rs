use crate::{
    context::{InfoContext, LayoutContext, RenderContext, StateMap, WidgetContext},
    event::EventUpdateArgs,
    render::{FrameBuilder, FrameUpdate},
    ui_list::{
        ConfigContextArgs, FinalSizeArgs, LayoutContextConfig, OffsetUiListObserver, UiListObserver, UiNodeList, UiNodeVec,
        WidgetFilterArgs, WidgetList, WidgetVec,
    },
    units::PxSize,
    widget_info::{WidgetBorderInfo, WidgetInfoBuilder, WidgetLayout, WidgetLayoutInfo, WidgetRenderInfo, WidgetSubscriptions},
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

    fn layout_all<C, D>(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout, mut widget_ctx: C, mut final_size: D)
    where
        C: FnMut(&mut LayoutContext, ConfigContextArgs) -> LayoutContextConfig,
        D: FnMut(&mut LayoutContext, FinalSizeArgs),
    {
        self.0.layout_all(ctx, wl, &mut widget_ctx, &mut final_size);
        let offset = self.0.len();
        self.1.layout_all(
            ctx,
            wl,
            |ctx, mut args| {
                args.index += offset;
                widget_ctx(ctx, args)
            },
            |ctx, mut args| {
                args.index += offset;
                final_size(ctx, args)
            },
        );
    }

    fn widget_layout(&mut self, index: usize, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_layout(index, ctx, wl)
        } else {
            self.1.widget_layout(index - a_len, ctx, wl)
        }
    }

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

    fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.0.render_all(ctx, frame);
        self.1.render_all(ctx, frame);
    }

    fn widget_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_render(index, ctx, frame)
        } else {
            self.1.widget_render(index - a_len, ctx, frame)
        }
    }

    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.0.render_update_all(ctx, update);
        self.1.render_update_all(ctx, update);
    }

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

    fn widget_id(&self, index: usize) -> WidgetId {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_id(index)
        } else {
            self.1.widget_id(index - a_len)
        }
    }

    fn widget_state(&self, index: usize) -> &StateMap {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_state(index)
        } else {
            self.1.widget_state(index - a_len)
        }
    }

    fn widget_state_mut(&mut self, index: usize) -> &mut StateMap {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_state_mut(index)
        } else {
            self.1.widget_state_mut(index - a_len)
        }
    }

    fn widget_outer_info(&self, index: usize) -> &WidgetLayoutInfo {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_outer_info(index)
        } else {
            self.1.widget_outer_info(index - a_len)
        }
    }

    fn widget_inner_info(&self, index: usize) -> &WidgetLayoutInfo {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_inner_info(index)
        } else {
            self.1.widget_inner_info(index - a_len)
        }
    }

    fn widget_border_info(&self, index: usize) -> &WidgetBorderInfo {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_border_info(index)
        } else {
            self.1.widget_border_info(index - a_len)
        }
    }

    fn widget_render_info(&self, index: usize) -> &WidgetRenderInfo {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_render_info(index)
        } else {
            self.1.widget_render_info(index - a_len)
        }
    }

    fn widget_outer<F>(&mut self, index: usize, ctx: &mut LayoutContext, wl: &mut WidgetLayout, f: F)
    where
        F: FnOnce(&mut LayoutContext, FinalSizeArgs),
    {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_outer(index, ctx, wl, f);
        } else {
            self.1.widget_outer(index - a_len, ctx, wl, f);
        }
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

    fn layout_all<C, D>(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout, mut wgt_ctx: C, mut final_size: D)
    where
        C: FnMut(&mut LayoutContext, ConfigContextArgs) -> LayoutContextConfig,
        D: FnMut(&mut LayoutContext, FinalSizeArgs),
    {
        self.0.layout_all(ctx, wl, &mut wgt_ctx, &mut final_size);
        let offset = self.0.len();
        self.1.layout_all(
            ctx,
            wl,
            |ctx, mut args| {
                args.index += offset;
                wgt_ctx(ctx, args)
            },
            |ctx, mut args| {
                args.index += offset;
                final_size(ctx, args)
            },
        );
    }

    fn widget_layout(&mut self, index: usize, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_layout(index, ctx, wl)
        } else {
            self.1.widget_layout(index - a_len, ctx, wl)
        }
    }

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

    fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.0.render_all(ctx, frame);
        self.1.render_all(ctx, frame);
    }

    fn widget_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_render(index, ctx, frame)
        } else {
            self.1.widget_render(index - a_len, ctx, frame)
        }
    }

    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.0.render_update_all(ctx, update);
        self.1.render_update_all(ctx, update);
    }

    fn widget_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_render_update(index, ctx, update)
        } else {
            self.1.widget_render_update(index - a_len, ctx, update)
        }
    }
}
