use crate::{
    context::{state_map, InfoContext, LayoutContext, MeasureContext, RenderContext, StateMapMut, StateMapRef, WidgetContext},
    event::EventUpdateArgs,
    node_vec,
    render::{FrameBuilder, FrameUpdate},
    ui_list::{
        PosLayoutArgs, PosMeasureArgs, PreLayoutArgs, PreMeasureArgs, UiListObserver, UiNodeFilterArgs, UiNodeList, UiNodeVec,
        WidgetFilterArgs, WidgetList, WidgetVec,
    },
    units::PxSize,
    widget_info::{WidgetBorderInfo, WidgetBoundsInfo, WidgetInfoBuilder, WidgetLayout, WidgetLayoutTranslation, WidgetSubscriptions},
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
                fn measure_all<C, D>(&self, ctx: &mut MeasureContext, mut pre_measure: C, mut pos_measure: D)
                where
                    C: FnMut(&mut MeasureContext, &mut PreMeasureArgs),
                    D: FnMut(&mut MeasureContext, PosMeasureArgs)
                {$(
                    super::default_ui_node_list_measure_all($n, &self.items.$n, ctx, &mut pre_measure, &mut pos_measure);
                )+}

                fn layout_all<C, D>(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout, mut pre_layout: C, mut pos_layout: D)
                where
                    C: FnMut(&mut LayoutContext, &mut WidgetLayout, &mut PreLayoutArgs),
                    D: FnMut(&mut LayoutContext, &mut WidgetLayout, PosLayoutArgs)
                {$(
                    super::default_ui_node_list_layout_all($n, &mut self.items.$n, ctx, wl, &mut pre_layout, &mut pos_layout);
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
                fn measure_all<C, D>(&self, ctx: &mut MeasureContext, mut pre_measure: C, mut pos_measure: D)
                where
                    C: FnMut(&mut MeasureContext, &mut PreMeasureArgs),
                    D: FnMut(&mut MeasureContext, PosMeasureArgs)
                {$(
                    super::default_widget_list_measure_all($n, &self.items.$n, ctx, &mut pre_measure, &mut pos_measure);
                )+}

                fn layout_all<C, D>(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout, mut pre_layout: C, mut pos_layout: D)
                where
                    C: FnMut(&mut LayoutContext, &mut WidgetLayout, &mut PreLayoutArgs),
                    D: FnMut(&mut LayoutContext, &mut WidgetLayout, PosLayoutArgs)
                {$(
                    super::default_widget_list_layout_all($n, &mut self.items.$n, ctx, wl, &mut pre_layout, &mut pos_layout);
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

            fn item_id(&self, index: usize) -> WidgetId {
                match index {
                    $($n => self.items.$n.id(),)+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }

            fn item_state(&self, index: usize) -> StateMapRef<state_map::Widget> {
                match index {
                    $($n => self.items.$n.state(),)+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }

            fn item_state_mut(&mut self, index: usize) -> StateMapMut<state_map::Widget> {
                match index {
                    $($n => self.items.$n.state_mut(),)+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }

            fn item_bounds_info(&self, index: usize) -> &WidgetBoundsInfo {
                match index {
                    $($n => self.items.$n.bounds_info(),)+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }

            fn item_border_info(&self, index: usize) -> &WidgetBorderInfo {
                match index {
                    $($n => self.items.$n.border_info(),)+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }

            fn item_outer<F, R>(&mut self, index: usize, wl: &mut WidgetLayout, keep_previous: bool, transform: F) -> R
            where
                F: FnOnce(&mut WidgetLayoutTranslation, PosLayoutArgs) -> R,
            {
                match index {
                    $($n => {
                        let w = &mut self.items.$n;
                        let size = w.bounds_info().outer_size();
                        wl.with_outer(w, keep_previous, |wlt, w| {
                            transform(wlt, PosLayoutArgs::new($n, Some(w.state_mut()), size))
                        })
                    })+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }

            fn outer_all<F>(&mut self, wl: &mut WidgetLayout, keep_previous: bool, mut transform: F)
            where
                F: FnMut(&mut WidgetLayoutTranslation, PosLayoutArgs),
            {
                $(
                    let w = &mut self.items.$n;
                    let size = w.bounds_info().outer_size();
                    wl.with_outer(w, keep_previous, |wlt, w| {
                        transform(wlt, PosLayoutArgs::new($n, Some(w.state_mut()), size));
                    });
                )*
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

            fn item_measure(&self, index: usize, ctx: &mut MeasureContext) -> PxSize {
                match index {
                    $(
                        $n => self.items.$n.measure(ctx),
                    )+
                    _ => panic!("index {index} out of range for length {}", self.len()),
                }
            }

            fn item_layout(&mut self, index: usize, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
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

            fn subscriptions_all(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
                $(
                    self.items.$n.subscriptions(ctx, subs);
                )+
            }

            fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                $(
                    self.items.$n.render(ctx, frame);
                )+
            }

            fn item_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
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

            fn item_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                match index {
                    $(
                        $n => self.items.$n.render_update(ctx, update),
                    )+
                    _ => panic!("index {index} out of range for length {}", self.len()),
                }
            }

            fn try_item_id(&self, index: usize) -> Option<WidgetId> {
                match index {
                    $(
                        $n => self.items.$n.try_id(),
                    )+
                    _ => panic!("index {index} out of range for length {}", self.len()),
                }
            }

            fn try_item_state(&self, index: usize) -> Option<StateMapRef<state_map::Widget>> {
                match index {
                    $(
                        $n => self.items.$n.try_state(),
                    )+
                    _ => panic!("index {index} out of range for length {}", self.len()),
                }
            }

            fn try_item_state_mut(&mut self, index: usize) -> Option<StateMapMut<state_map::Widget>> {
                match index {
                    $(
                        $n => self.items.$n.try_state_mut(),
                    )+
                    _ => panic!("index {index} out of range for length {}", self.len()),
                }
            }

            fn try_item_bounds_info(&self, index: usize) -> Option<&WidgetBoundsInfo> {
                match index {
                    $(
                        $n => self.items.$n.try_bounds_info(),
                    )+
                    _ => panic!("index {index} out of range for length {}", self.len()),
                }
            }

            fn try_item_border_info(&self, index: usize) -> Option<&WidgetBorderInfo> {
                match index {
                    $(
                        $n => self.items.$n.try_border_info(),
                    )+
                    _ => panic!("index {index} out of range for length {}", self.len()),
                }
            }

            fn render_node_filtered<F>(&self, mut filter: F, ctx: &mut RenderContext, frame: &mut FrameBuilder)
            where
                F: FnMut(super::UiNodeFilterArgs) -> bool,
            {
                $(
                    if filter(UiNodeFilterArgs::new($n, &self.items.$n)) {
                        self.items.$n.render(ctx, frame);
                    }
                )+
            }

            fn try_item_outer<F, R>(&mut self, index: usize, wl: &mut WidgetLayout, keep_previous: bool, transform: F) -> Option<R>
            where
                F: FnOnce(&mut WidgetLayoutTranslation, PosLayoutArgs) -> R,
            {
                match index {
                    $($n => {
                        let w = &mut self.items.$n;
                        if let Some(size) = w.try_bounds_info().map(|i|i.outer_size()) {
                            wl.try_with_outer(w, keep_previous, |wlt, w| {
                                transform(wlt, PosLayoutArgs::new($n, w.try_state_mut(), size))
                            })
                        } else {
                            None
                        }
                    })+
                    _ => panic!("index {index} out of range for length {}", self.len())
                }
            }

            fn try_outer_all<F>(&mut self, wl: &mut WidgetLayout, keep_previous: bool, mut transform: F)
            where
                F: FnMut(&mut WidgetLayoutTranslation, PosLayoutArgs),
            {
                $(
                    let w = &mut self.items.$n;
                    if let Some(size) = w.try_bounds_info().map(|i|i.outer_size()) {
                        wl.try_with_outer(w, keep_previous, |wlt, w| {
                            transform(wlt, PosLayoutArgs::new($n, w.try_state_mut(), size));
                        });
                    }
                )*
            }

            fn count_nodes<F>(&self, mut filter: F) -> usize
            where
                F: FnMut(super::UiNodeFilterArgs) -> bool,
            {
                let mut count = 0;
                $(
                if filter(UiNodeFilterArgs::new($n, &self.items.$n)) {
                    count += 1;
                }
                )+
                count
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

            fn measure_all<C, D>(&self, _: &mut MeasureContext, _: C, _: D)
            where
                C: FnMut(&mut MeasureContext, &mut PreMeasureArgs),
                D: FnMut(&mut MeasureContext, PosMeasureArgs)
                {}

            fn layout_all<C, D>(&mut self, _: &mut LayoutContext, _: &mut WidgetLayout, _: C, _: D)
            where
                C: FnMut(&mut LayoutContext, &mut WidgetLayout, &mut PreLayoutArgs),
                D: FnMut(&mut LayoutContext, &mut WidgetLayout, PosLayoutArgs)
                {}

            fn item_measure(&self, index: usize, _: &mut MeasureContext) -> PxSize {
                panic!("index {index} out of range for length 0")
            }
            fn item_layout(&mut self, index: usize, _: &mut LayoutContext, _: &mut WidgetLayout) -> PxSize {
                panic!("index {index} out of range for length 0")
            }

            fn info_all(&self, _: &mut InfoContext, _: &mut WidgetInfoBuilder) {
            }

            fn subscriptions_all(&self, _: &mut InfoContext, _: &mut WidgetSubscriptions) {}

            fn render_all(&self, _: &mut RenderContext, _: &mut FrameBuilder) {
            }

            fn item_render(&self, index: usize, _: &mut RenderContext, _: &mut FrameBuilder) {
                panic!("index {index} out of range for length 0")
            }

            fn render_update_all(&self, _: &mut RenderContext, _: &mut FrameUpdate) {}

            fn item_render_update(&self, index: usize, _: &mut RenderContext, _: &mut FrameUpdate) {
                panic!("index {index} out of range for length 0")
            }

            fn try_item_id(&self, index: usize) -> Option<WidgetId> {
                panic!("index {index} out of range for length 0")
            }

            fn try_item_state(&self, index: usize) -> Option<StateMapRef<state_map::Widget>> {
                panic!("index {index} out of range for length 0")
            }

            fn try_item_state_mut(&mut self, index: usize) -> Option<StateMapMut<state_map::Widget>> {
                panic!("index {index} out of range for length 0")
            }

            fn try_item_bounds_info(&self, index: usize) -> Option<&WidgetBoundsInfo> {
                panic!("index {index} out of range for length 0")
            }

            fn try_item_border_info(&self, index: usize) -> Option<&WidgetBorderInfo> {
                panic!("index {index} out of range for length 0")
            }

            fn render_node_filtered<F>(&self, _: F, _: &mut RenderContext, _: &mut FrameBuilder)
            where
                F: FnMut(super::UiNodeFilterArgs) -> bool,
            {
            }

            fn try_item_outer<F, R>(&mut self, index: usize, _: &mut WidgetLayout, _: bool, _: F) -> Option<R>
            where
                F: FnOnce(&mut WidgetLayoutTranslation, PosLayoutArgs) -> R,
            {
                panic!("index {index} out of range for length 0")
            }

            fn try_outer_all<F>(&mut self, _: &mut WidgetLayout, _: bool, _: F)
            where
                F: FnMut(&mut WidgetLayoutTranslation, PosLayoutArgs),
            {
            }

            fn count_nodes<F>(&self, _: F) -> usize
            where
                F: FnMut(super::UiNodeFilterArgs) -> bool,
            {
                0
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

    fn item_id(&self, index: usize) -> WidgetId {
        panic!("index {index} out of range for length 0")
    }

    fn item_state(&self, index: usize) -> StateMapRef<state_map::Widget> {
        panic!("index {index} out of range for length 0")
    }

    fn item_state_mut(&mut self, index: usize) -> StateMapMut<state_map::Widget> {
        panic!("index {index} out of range for length 0")
    }

    fn item_bounds_info(&self, index: usize) -> &WidgetBoundsInfo {
        panic!("index {index} out of range for length 0")
    }

    fn item_border_info(&self, index: usize) -> &WidgetBorderInfo {
        panic!("index {index} out of range for length 0")
    }

    fn item_outer<F, R>(&mut self, index: usize, _: &mut WidgetLayout, _: bool, _: F) -> R
    where
        F: FnOnce(&mut WidgetLayoutTranslation, PosLayoutArgs) -> R,
    {
        panic!("index {index} out of range for length 0")
    }

    fn outer_all<F>(&mut self, _: &mut WidgetLayout, _: bool, _: F)
    where
        F: FnMut(&mut WidgetLayoutTranslation, PosLayoutArgs),
    {
    }
}
