use super::{SpatialIdGen, WidgetFilterArgs};
use crate::{
    context::{InfoContext, LayoutContext, RenderContext, StateMap, WidgetContext},
    event::EventUpdateArgs,
    node_vec,
    render::{FrameBuilder, FrameUpdate},
    units::{AvailableSize, PxPoint, PxRect, PxSize},
    widget_base::Visibility,
    widget_info::{WidgetInfoBuilder, WidgetOffset, WidgetSubscriptions},
    widget_vec, UiNode, UiNodeList, UiNodeVec, Widget, WidgetId, WidgetList, WidgetVec,
};

macro_rules! impl_tuples {
    ($($L:tt -> $LP:tt => $($n:tt),+;)+) => {$($crate::paste! {

        impl_tuples! { [<UiNodeList $L>] -> [<UiNodeList $LP>], [<WidgetList $L>] -> [<WidgetList $LP>] => $L => $($n = [<W $n>]),+ }

    })+};
    ($NodeList:ident -> $NodeListNext:ident, $WidgetList:ident -> $WidgetListNext:ident => $L:tt => $($n:tt = $W:ident),+) => {
        impl_tuples! { impl_node => $NodeList<UiNode> -> $NodeListNext => $L => $($n = $W),+ }
        impl_tuples! { impl_node => $WidgetList<Widget> -> $WidgetListNext => $L => $($n = $W),+ }

        impl<$($W: Widget),+> WidgetList for $WidgetList<$($W,)+> {
            #[inline]
            fn boxed_widget_all(self) -> WidgetVec {
                widget_vec![$(self.items.$n),+]
            }

            #[inline(always)]
            fn render_filtered<O>(&self, mut origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
            where
                O: FnMut(usize, WidgetFilterArgs) -> Option<PxPoint>,
            {
                let id = self.id.get();
                $(
                if let Some(o) = origin($n, WidgetFilterArgs::get(self, $n)) {
                    frame.push_reference_frame_item(id, $n, o, |frame| self.items.$n.render(ctx, frame));
                }
                )+
            }

            #[inline]
            fn widget_id(&self, index: usize) -> WidgetId {
                match index {
                    $($n => self.items.$n.id(),)+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }

            #[inline]
            fn widget_state(&self, index: usize) -> &StateMap {
                match index {
                    $($n => self.items.$n.state(),)+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }

            #[inline]
            fn widget_state_mut(&mut self, index: usize) -> &mut StateMap {
                match index {
                    $($n => self.items.$n.state_mut(),)+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }

            #[inline]
            fn widget_outer_bounds(&self, index: usize) -> PxRect {
                match index {
                    $($n => self.items.$n.outer_bounds(),)+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }

            #[inline]
            fn widget_inner_bounds(&self, index: usize) -> PxRect {
                match index {
                    $($n => self.items.$n.inner_bounds(),)+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }

            #[inline]
            fn widget_visibility(&self, index: usize) -> Visibility {
                match index {
                    $($n => self.items.$n.visibility(),)+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }
        }
    };

    (impl_node => $NodeList:ident <$Bound:ident> -> $NodeListNext:ident => $L:tt => $($n:tt = $W:ident),+) => {
        #[doc(hidden)]
        pub struct $NodeList<$($W: $Bound),+> {
            items: ($($W,)+),
            id: SpatialIdGen,
        }

        impl<$($W: $Bound),+> $NodeList<$($W,)+> {
            #[doc(hidden)]
            pub fn push<I: $Bound>(self, item: I) -> $NodeListNext<$($W),+ , I> {
                $NodeListNext {
                    items: (
                        $(self.items.$n,)+
                        item
                    ),
                    id: SpatialIdGen::default()
                }
            }
        }

        impl<$($W: $Bound),+> UiNodeList for $NodeList<$($W,)+> {
            #[inline]
            fn len(&self) -> usize {
                $L
            }

            #[inline]
            fn is_empty(&self) -> bool {
                false
            }

            #[inline]
            fn boxed_all(self) -> UiNodeVec {
                node_vec![
                    $(self.items.$n),+
                ]
            }

            #[inline(always)]
            fn init_all(&mut self, ctx: &mut WidgetContext) {
                $(self.items.$n.init(ctx);)+
            }

            #[inline(always)]
            fn deinit_all(&mut self, ctx: &mut WidgetContext) {
                $(self.items.$n.deinit(ctx);)+
            }

            #[inline(always)]
            fn update_all(&mut self, ctx: &mut WidgetContext) {
                $(self.items.$n.update(ctx);)+
            }

            #[inline(always)]
            fn event_all<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
                $(self.items.$n.event(ctx, args);)+
            }

            #[inline(always)]
            fn measure_all<A, D>(&mut self, ctx: &mut LayoutContext, mut available_size: A, mut desired_size: D)
            where
                A: FnMut(usize, &mut LayoutContext) -> AvailableSize,
                D: FnMut(usize, PxSize, &mut LayoutContext),
            {
                $(
                let av_sz = available_size($n, ctx);
                let r = self.items.$n.measure(ctx, av_sz);
                desired_size($n, r, ctx);
                )+
            }

            #[inline]
            fn widget_measure(&mut self, index: usize, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                match index {
                    $(
                        $n => self.items.$n.measure(ctx, available_size),
                    )+
                    _ => panic!("index {index} out of range for length {}", self.len()),
                }
            }

            #[inline(always)]
            fn arrange_all<F>(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, mut final_rect: F)
            where
                F: FnMut(usize, &mut LayoutContext) -> PxRect,
            {
                $(
                let r = final_rect($n, ctx);
                widget_offset.with_offset(r.origin.to_vector(), |wo| {
                    self.items.$n.arrange(ctx, wo, r.size);
                });

                )+
            }

            #[inline]
            fn widget_arrange(&mut self, index: usize, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
                match index {
                    $(
                        $n => self.items.$n.arrange(ctx, widget_offset, final_size),
                    )+
                    _ => panic!("index {index} out of range for length {}", self.len()),
                }
            }

            #[inline(always)]
            fn info_all(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                $(
                    self.items.$n.info(ctx, info);
                )+
            }

            #[inline]
            fn widget_info(&self, index: usize, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                match index {
                    $(
                        $n => self.items.$n.info(ctx, info),
                    )+
                    _ => panic!("index {index} out of range for length {}", self.len()),
                }
            }

            #[inline(always)]
            fn subscriptions_all(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                $(
                    self.items.$n.subscriptions(ctx, subscriptions);
                )+
            }

            #[inline]
            fn widget_subscriptions(&self, index: usize, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                match index {
                    $(
                        $n => self.items.$n.subscriptions(ctx, subscriptions),
                    )+
                    _ => panic!("index {index} out of range for length {}", self.len()),
                }
            }

            #[inline(always)]
            fn render_all<O>(&self, mut origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
            where
                O: FnMut(usize) -> PxPoint,
            {
                let id = self.id.get();
                $(
                let o = origin($n);
                frame.push_reference_frame_item(id, $n, o, |frame| self.items.$n.render(ctx, frame));
                )+
            }

            #[inline]
            fn widget_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                match index {
                    $(
                        $n => self.items.$n.render(ctx, frame),
                    )+
                    _ => panic!("index {index} out of range for length {}", self.len()),
                }
            }

            #[inline(always)]
            fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                $(self.items.$n.render_update(ctx, update);)+
            }

            #[inline]
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
    id: SpatialIdGen,
}
#[doc(hidden)]
#[allow(dead_code)]
pub struct WidgetList9<T0, T1, T2, T3, T4, T5, T6, T7, T8> {
    items: (T0, T1, T2, T3, T4, T5, T6, T7, T8),
    id: SpatialIdGen,
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
                    id: SpatialIdGen::default()
                }
            }
        }

        impl UiNodeList for $ident {
            #[inline]
            fn len(&self) -> usize {
                0
            }

            #[inline]
            fn is_empty(&self) -> bool {
                true
            }

            fn boxed_all(self) -> UiNodeVec {
                node_vec![]
            }

            #[inline]
            fn init_all(&mut self, _: &mut WidgetContext) {}

            #[inline]
            fn deinit_all(&mut self, _: &mut WidgetContext) {}

            #[inline]
            fn update_all(&mut self, _: &mut WidgetContext) {}

            #[inline]
            fn event_all<EU: EventUpdateArgs>(&mut self, _: &mut WidgetContext, _: &EU) {}

            #[inline]
            fn measure_all<A, D>(&mut self, _: &mut LayoutContext, _: A, _: D)
            where
                A: FnMut(usize, &mut LayoutContext) -> AvailableSize,
                D: FnMut(usize, PxSize, &mut LayoutContext),
            {
            }

            #[inline]
            fn widget_measure(&mut self, index: usize, _: &mut LayoutContext, _: AvailableSize) -> PxSize {
                panic!("index {index} out of range for length 0")
            }

            #[inline]
            fn arrange_all<F>(&mut self, _: &mut LayoutContext, _: &mut WidgetOffset, _: F)
            where
                F: FnMut(usize, &mut LayoutContext) -> PxRect,
            {
            }

            #[inline]
            fn widget_arrange(&mut self, index: usize, _: &mut LayoutContext, _: &mut WidgetOffset, _: PxSize) {
                panic!("index {index} out of range for length 0")
            }

            fn info_all(&self, _: &mut InfoContext, _: &mut WidgetInfoBuilder) {
            }

            #[inline]
            fn widget_info(&self, index: usize, _: &mut InfoContext, _: &mut WidgetInfoBuilder) {
                panic!("index {index} out of range for length 0")
            }

            fn subscriptions_all(&self, _: &mut InfoContext, _: &mut WidgetSubscriptions) {}

            #[inline]
            fn widget_subscriptions(&self, index: usize, _: &mut InfoContext, _: &mut WidgetSubscriptions) {
                panic!("index {index} out of range for length 0")
            }

            fn render_all<O>(&self, _: O, _: &mut RenderContext, _: &mut FrameBuilder)
            where
                O: FnMut(usize) -> PxPoint,
            {
            }

            #[inline]
            fn widget_render(&self, index: usize, _: &mut RenderContext, _: &mut FrameBuilder) {
                panic!("index {index} out of range for length 0")
            }

            #[inline]
            fn render_update_all(&self, _: &mut RenderContext, _: &mut FrameUpdate) {}

            #[inline]
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
    #[inline]
    fn boxed_widget_all(self) -> WidgetVec {
        widget_vec![]
    }

    fn render_filtered<O>(&self, _: O, _: &mut RenderContext, _: &mut FrameBuilder)
    where
        O: FnMut(usize, WidgetFilterArgs) -> Option<PxPoint>,
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

    fn widget_outer_bounds(&self, index: usize) -> PxRect {
        panic!("index {index} out of range for length 0")
    }

    fn widget_inner_bounds(&self, index: usize) -> PxRect {
        panic!("index {index} out of range for length 0")
    }

    fn widget_visibility(&self, index: usize) -> Visibility {
        panic!("index {index} out of range for length 0")
    }
}
