use super::WidgetFilterArgs;
use crate::{
    context::{InfoContext, LayoutContext, RenderContext, StateMap, WidgetContext},
    event::EventUpdateArgs,
    node_vec,
    render::{FrameBuilder, FrameUpdate},
    ui_list::{ConfigContextArgs, FinalSizeArgs, LayoutContextConfig, UiListObserver, UiNodeList, UiNodeVec, WidgetList, WidgetVec},
    units::PxSize,
    widget_info::{WidgetBorderInfo, WidgetInfoBuilder, WidgetLayout, WidgetLayoutInfo, WidgetRenderInfo, WidgetSubscriptions},
    widget_vec, UiNode, Widget, WidgetId,
};

macro_rules! impl_tuples {
    ($($L:tt -> $LP:tt => $($n:tt),+;)+) => {$($crate::paste! {

        impl_tuples! { [<UiNodeList $L>] -> [<UiNodeList $LP>], [<WidgetList $L>] -> [<WidgetList $LP>] => $L => $($n = [<W $n>]),+ }

    })+};
    ($NodeList:ident -> $NodeListNext:ident, $WidgetList:ident -> $WidgetListNext:ident => $L:tt => $($n:tt = $W:ident),+) => {
        impl_tuples! { impl_node {
            list: $NodeList,
            bound: UiNode,
            next_list: $NodeListNext,
            len: $L,
            items { $($n = $W),+ }

            layout {
                fn layout_all<C, D>(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout, mut wgt_ctx: C, mut final_size: D)
                where
                    C: FnMut(&mut LayoutContext, ConfigContextArgs) -> LayoutContextConfig,
                    D: FnMut(&mut LayoutContext, FinalSizeArgs)
                {$(
                    let cfg = wgt_ctx(ctx, ConfigContextArgs {
                        index: $n,
                        state: None,
                    });

                    let size = cfg.with(ctx, |ctx| {
                        self.items.$n.layout(ctx, wl)
                    });

                    final_size(ctx, FinalSizeArgs {
                        index: $n,
                        state: None,
                        size,
                        transform: None
                    });
                )+}
            }
        } }
        impl_tuples! { impl_node {
            list: $WidgetList,
            bound: Widget,
            next_list: $WidgetListNext,
            len: $L,
            items { $($n = $W),+ }

            layout {
                fn layout_all<C, D>(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout, mut wgt_ctx: C, mut final_size: D)
                where
                    C: FnMut(&mut LayoutContext, ConfigContextArgs) -> LayoutContextConfig,
                    D: FnMut(&mut LayoutContext, FinalSizeArgs)
                {$(
                    let cfg = wgt_ctx(ctx, ConfigContextArgs {
                        index: $n,
                        state: Some(self.items.$n.state_mut())
                    });

                    let size = cfg.with(ctx, |ctx| {
                        self.items.$n.layout(ctx, wl)
                    });

                    wl.with_outer(ctx, &mut self.items.$n, |ctx, w, t| {
                        final_size(ctx, FinalSizeArgs {
                            index: $n,
                            state: Some(w.state_mut()),
                            size,
                            transform: Some(t)
                        })
                    });
                )+}
            }
        } }

        impl<$($W: Widget),+> WidgetList for $WidgetList<$($W,)+> {

            fn boxed_widget_all(self) -> WidgetVec {
                widget_vec![$(self.items.$n),+]
            }

            fn count<F>(&self, mut filter: F) -> usize
            where
                F: FnMut(WidgetFilterArgs) -> bool,
            {
                let mut count = 0;
                $(
                if filter(WidgetFilterArgs::new($n, &self.items.$n)) {
                    count += 1;
                }
                )+
                count
            }

            fn render_filtered<F>(&self, mut filter: F, ctx: &mut RenderContext, frame: &mut FrameBuilder)
            where
                F: FnMut(WidgetFilterArgs) -> bool,
            {
                $(
                if filter(WidgetFilterArgs::new($n, &self.items.$n)) {
                    self.items.$n.render(ctx, frame);
                }
                )+
            }

            fn widget_id(&self, index: usize) -> WidgetId {
                match index {
                    $($n => self.items.$n.id(),)+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }

            fn widget_state(&self, index: usize) -> &StateMap {
                match index {
                    $($n => self.items.$n.state(),)+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }

            fn widget_state_mut(&mut self, index: usize) -> &mut StateMap {
                match index {
                    $($n => self.items.$n.state_mut(),)+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }

            fn widget_outer_info(&self, index: usize) -> &WidgetLayoutInfo {
                match index {
                    $($n => self.items.$n.outer_info(),)+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }

            fn widget_inner_info(&self, index: usize) -> &WidgetLayoutInfo {
                match index {
                    $($n => self.items.$n.inner_info(),)+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }

            fn widget_border_info(&self, index: usize) -> &WidgetBorderInfo {
                match index {
                    $($n => self.items.$n.border_info(),)+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }

            fn widget_render_info(&self, index: usize) -> &WidgetRenderInfo {
                match index {
                    $($n => self.items.$n.render_info(),)+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }

            fn widget_outer<F>(&mut self, index: usize, ctx: &mut LayoutContext, wl: &mut WidgetLayout, f: F)
            where
            F: FnOnce(&mut LayoutContext, FinalSizeArgs)
            {
                match index {
                    $($n => {
                        let w = &mut self.items.$n;
                        wl.with_outer(ctx, w, |ctx, w, t| {
                            let size = w.outer_info().size();
                            f(ctx, FinalSizeArgs {
                                index: $n,
                                state: Some(w.state_mut()),
                                size,
                                transform: Some(t)
                            })
                        })
                    })+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }
        }
    };

    (impl_node {
        list: $NodeList:ident,
        bound: $Bound:ident,
        next_list: $NodeListNext:ident,
        len: $L:tt,
        items { $($n:tt = $W:ident),+ }
        layout {
            $($layout_all:tt)+
        }
    }) => {
        #[doc(hidden)]
        pub struct $NodeList<$($W: $Bound),+> {
            items: ($($W,)+),
        }

        impl<$($W: $Bound),+> $NodeList<$($W,)+> {
            #[doc(hidden)]
            pub fn push<I: $Bound>(self, item: I) -> $NodeListNext<$($W),+ , I> {
                $NodeListNext {
                    items: (
                        $(self.items.$n,)+
                        item
                    ),
                }
            }
        }

        impl<$($W: $Bound),+> UiNodeList for $NodeList<$($W,)+> {
            fn is_fixed(&self) -> bool {
                true
            }

            fn len(&self) -> usize {
                $L
            }

            fn is_empty(&self) -> bool {
                false
            }

            fn boxed_all(self) -> UiNodeVec {
                node_vec![
                    $(self.items.$n),+
                ]
            }

            fn init_all(&mut self, ctx: &mut WidgetContext) {
                $(self.items.$n.init(ctx);)+
            }


            fn deinit_all(&mut self, ctx: &mut WidgetContext) {
                $(self.items.$n.deinit(ctx);)+
            }

            fn update_all<O: UiListObserver>(&mut self, ctx: &mut WidgetContext, _: &mut O) {
                $(self.items.$n.update(ctx);)+
            }

            fn event_all<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
                $(self.items.$n.event(ctx, args);)+
            }

            $($layout_all)+

            fn widget_layout(&mut self, index: usize, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                match index {
                    $(
                        $n => self.items.$n.layout(ctx, wl),
                    )+
                    _ => panic!("index {index} out of range for length {}", self.len()),
                }
            }

            fn info_all(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                $(
                    self.items.$n.info(ctx, info);
                )+
            }

            fn widget_info(&self, index: usize, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                match index {
                    $(
                        $n => self.items.$n.info(ctx, info),
                    )+
                    _ => panic!("index {index} out of range for length {}", self.len()),
                }
            }

            fn subscriptions_all(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                $(
                    self.items.$n.subscriptions(ctx, subscriptions);
                )+
            }

            fn widget_subscriptions(&self, index: usize, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                match index {
                    $(
                        $n => self.items.$n.subscriptions(ctx, subscriptions),
                    )+
                    _ => panic!("index {index} out of range for length {}", self.len()),
                }
            }

            fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                $(
                    self.items.$n.render(ctx, frame);
                )+
            }

            fn widget_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                match index {
                    $(
                        $n => self.items.$n.render(ctx, frame),
                    )+
                    _ => panic!("index {index} out of range for length {}", self.len()),
                }
            }

            fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                $(self.items.$n.render_update(ctx, update);)+
            }

            fn widget_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                match index {
                    $(
                        $n => self.items.$n.render_update(ctx, update),
                    )+
                    _ => panic!("index {index} out of range for length {}", self.len()),
                }
            }
        }
    };
}
impl_tuples! {
    1 -> 2 => 0;
    2 -> 3 => 0, 1;
    3 -> 4 => 0, 1, 2;
    4 -> 5 => 0, 1, 2, 3;
    5 -> 6 => 0, 1, 2, 3, 4;
    6 -> 7 => 0, 1, 2, 3, 4, 5;
    7 -> 8 => 0, 1, 2, 3, 4, 5, 6;
    8 -> 9 => 0, 1, 2, 3, 4, 5, 6, 7;
}

// we need this types due to limitation in macro_rules.
#[doc(hidden)]
#[allow(dead_code)]
pub struct UiNodeList9<T0, T1, T2, T3, T4, T5, T6, T7, T8> {
    items: (T0, T1, T2, T3, T4, T5, T6, T7, T8),
}
#[doc(hidden)]
#[allow(dead_code)]
pub struct WidgetList9<T0, T1, T2, T3, T4, T5, T6, T7, T8> {
    items: (T0, T1, T2, T3, T4, T5, T6, T7, T8),
}

macro_rules! empty_node_list {
    ($($ident:ident -> $ident_one:ident<$bounds:ident>),+) => {$(
        #[doc(hidden)]
        pub struct $ident;

        impl $ident {
            #[doc(hidden)]
            pub fn push<N: $bounds>(self, node: N) -> $ident_one<N> {
                $ident_one {
                    items: (node,),
                }
            }
        }

        impl UiNodeList for $ident {
            fn is_fixed(&self) -> bool {
                true
            }

            fn len(&self) -> usize {
                0
            }

            fn is_empty(&self) -> bool {
                true
            }

            fn boxed_all(self) -> UiNodeVec {
                node_vec![]
            }

            fn init_all(&mut self, _: &mut WidgetContext) {}

            fn deinit_all(&mut self, _: &mut WidgetContext) {}

            fn update_all<O: UiListObserver>(&mut self, _: &mut WidgetContext, _: &mut O) {}

            fn event_all<EU: EventUpdateArgs>(&mut self, _: &mut WidgetContext, _: &EU) {}

            fn layout_all<C, D>(&mut self, _: &mut LayoutContext, _: &mut WidgetLayout, _: C, _: D)
            where
                C: FnMut(&mut LayoutContext, ConfigContextArgs) -> LayoutContextConfig,
                D: FnMut(&mut LayoutContext, FinalSizeArgs)
                {}

            fn widget_layout(&mut self, index: usize, _: &mut LayoutContext, _: &mut WidgetLayout) -> PxSize {
                panic!("index {index} out of range for length 0")
            }

            fn info_all(&self, _: &mut InfoContext, _: &mut WidgetInfoBuilder) {
            }

            fn widget_info(&self, index: usize, _: &mut InfoContext, _: &mut WidgetInfoBuilder) {
                panic!("index {index} out of range for length 0")
            }

            fn subscriptions_all(&self, _: &mut InfoContext, _: &mut WidgetSubscriptions) {}

            fn widget_subscriptions(&self, index: usize, _: &mut InfoContext, _: &mut WidgetSubscriptions) {
                panic!("index {index} out of range for length 0")
            }

            fn render_all(&self, _: &mut RenderContext, _: &mut FrameBuilder) {
            }

            fn widget_render(&self, index: usize, _: &mut RenderContext, _: &mut FrameBuilder) {
                panic!("index {index} out of range for length 0")
            }

            fn render_update_all(&self, _: &mut RenderContext, _: &mut FrameUpdate) {}

            fn widget_render_update(&self, index: usize, _: &mut RenderContext, _: &mut FrameUpdate) {
                panic!("index {index} out of range for length 0")
            }
        }
    )+}
}
empty_node_list! {
    UiNodeList0 -> UiNodeList1<UiNode>,
    WidgetList0 -> WidgetList1<Widget>
}
impl WidgetList for WidgetList0 {
    fn count<F>(&self, _: F) -> usize
    where
        F: FnMut(WidgetFilterArgs) -> bool,
    {
        0
    }

    fn boxed_widget_all(self) -> WidgetVec {
        widget_vec![]
    }

    fn render_filtered<F>(&self, _: F, _: &mut RenderContext, _: &mut FrameBuilder)
    where
        F: FnMut(WidgetFilterArgs) -> bool,
    {
    }

    fn widget_id(&self, index: usize) -> WidgetId {
        panic!("index {index} out of range for length 0")
    }

    fn widget_state(&self, index: usize) -> &StateMap {
        panic!("index {index} out of range for length 0")
    }

    fn widget_state_mut(&mut self, index: usize) -> &mut StateMap {
        panic!("index {index} out of range for length 0")
    }

    fn widget_outer_info(&self, index: usize) -> &WidgetLayoutInfo {
        panic!("index {index} out of range for length 0")
    }

    fn widget_inner_info(&self, index: usize) -> &WidgetLayoutInfo {
        panic!("index {index} out of range for length 0")
    }

    fn widget_border_info(&self, index: usize) -> &WidgetBorderInfo {
        panic!("index {index} out of range for length 0")
    }

    fn widget_render_info(&self, index: usize) -> &WidgetRenderInfo {
        panic!("index {index} out of range for length 0")
    }

    fn widget_outer<F>(&mut self, index: usize, _: &mut LayoutContext, _: &mut WidgetLayout, _: F)
    where
        F: FnOnce(&mut LayoutContext, FinalSizeArgs),
    {
        panic!("index {index} out of range for length 0")
    }
}
