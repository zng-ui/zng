use super::units::{LayoutPoint, LayoutSize};
#[allow(unused)] // used in docs.
use super::UiNode;
use super::{
    context::{LayoutContext, LazyStateMap, WidgetContext},
    render::{FrameBuilder, FrameUpdate},
    Widget, WidgetId,
};

/// A mixed vector of [`Widget`] types.
pub type WidgetVec = Vec<Box<dyn Widget>>;

/// Creates a [`WidgetVec`](crate::WidgetVec) containing the arguments.
///
/// # Example
///
/// ```
/// # use zero_ui_core::{ui_vec, UiNode, Widget, WidgetId, NilUiNode};
/// # use zero_ui_core::widget_base::*;
/// # fn text(fake: &str) -> impl Widget { default_widget_new(NilUiNode, widget_id::ArgsImpl::new(WidgetId::new_unique()))  };
/// let widgets = ui_vec![
///     text("Hello"),
///     text("World!")
/// ];
/// ```
/// `ui_vec!` automatically boxes each widget.
#[macro_export]
macro_rules! ui_vec {
    () => { $crate::WidgetVec::new() };
    ($($node:expr),+ $(,)?) => {
        vec![
            $($crate::Widget::boxed_widget($node)),*
        ]
    };
}
#[doc(inline)]
pub use crate::ui_vec;

/// A generic view over a list of [`UiNode`] items.
pub trait UiNodeList: 'static {
    /// Number of items in the list.
    fn len(&self) -> usize;

    /// If the list is empty.
    fn is_empty(&self) -> bool;

    /// Boxes all items.
    fn boxed_all(self) -> Vec<Box<dyn UiNode>>;

    /// Creates a new list that consists of this list followed by the `other` list of nodes.
    fn chain_nodes<U>(self, other: U) -> UiNodeListChain<Self, U>
    where
        Self: Sized,
        U: UiNodeList,
    {
        UiNodeListChain(self, other)
    }

    /// Calls [`UiNode::init`] in all widgets in the list, sequentially.
    fn init_all(&mut self, ctx: &mut WidgetContext);

    /// Calls [`UiNode::deinit`] in all widgets in the list, sequentially.
    fn deinit_all(&mut self, ctx: &mut WidgetContext);

    /// Calls [`UiNode::update`] in all widgets in the list, sequentially.
    fn update_all(&mut self, ctx: &mut WidgetContext);

    /// Calls [`UiNode::update_hp`] in all widgets in the list, sequentially.
    fn update_hp_all(&mut self, ctx: &mut WidgetContext);

    /// Calls [`UiNode::measure`] in all widgets in the list, sequentially.
    ///
    /// # `available_size`
    ///
    /// The `available_size` parameter is a function that takes a widget index and the `ctx` and returns
    /// the available size for the widget.
    ///
    /// The index is zero-based, `0` is the first widget, `len() - 1` is the last.
    ///
    /// # `desired_size`
    ///
    /// The `desired_size` parameter is a function is called with the widget index, the widget measured size and the `ctx`.
    ///
    /// This is how you get the widget desired size.
    fn measure_all<A, D>(&mut self, available_size: A, desired_size: D, ctx: &mut LayoutContext)
    where
        A: FnMut(usize, &mut LayoutContext) -> LayoutSize,
        D: FnMut(usize, LayoutSize, &mut LayoutContext);

    /// Calls [`UiNode::arrange`] in all widgets in the list, sequentially.
    ///
    /// # `final_size`
    ///
    /// The `final size` parameter is a function that takes a widget index and the `ctx` and returns the
    /// final size the widget must use.
    fn arrange_all<F>(&mut self, final_size: F, ctx: &mut LayoutContext)
    where
        F: FnMut(usize, &mut LayoutContext) -> LayoutSize;

    /// Calls [`UiNode::render`] in all widgets in the list, sequentially. Uses a reference frame
    /// to offset each widget.
    ///
    /// # `origin`
    ///
    /// The `origin` parameter is a function that takes a widget index and returns the offset that must
    /// be used to render it.
    fn render_all<O>(&self, origin: O, frame: &mut FrameBuilder)
    where
        O: FnMut(usize) -> LayoutPoint;

    /// Calls [`UiNode::render_update`] in all widgets in the list, sequentially.
    fn render_update_all(&self, update: &mut FrameUpdate);
}

/// A generic view over a list of [`Widget`] UI nodes.
///
/// Layout widgets should use this to abstract the children list type if possible.
pub trait WidgetList: UiNodeList {
    /// Count widgets that pass filter using the widget state.
    fn count<F>(&self, mut filter: F) -> usize
    where
        F: FnMut(usize, &LazyStateMap) -> bool,
    {
        let mut count = 0;
        for i in 0..self.len() {
            if filter(i, self.widget_state(i)) {
                count += 1;
            }
        }
        count
    }

    /// Boxes all widgets and moved then to a [`WidgetVec`].
    fn boxed_widget_all(self) -> WidgetVec;

    /// Creates a new list that consists of this list followed by the `other` list.
    fn chain<U>(self, other: U) -> WidgetListChain<Self, U>
    where
        Self: Sized,
        U: WidgetList,
    {
        WidgetListChain(self, other)
    }

    /// Gets the id of the widget at the `index`.
    ///
    /// The index is zero-based.
    fn widget_id(&self, index: usize) -> WidgetId;

    /// Reference the state of the widget at the `index`.
    fn widget_state(&self, index: usize) -> &LazyStateMap;

    /// Exclusive reference the state of the widget at the `index`.
    fn widget_state_mut(&mut self, index: usize) -> &mut LazyStateMap;

    /// Gets the last arranged size of the widget at the `index`.
    fn widget_size(&self, index: usize) -> LayoutSize;

    /// Calls [`UiNode::render`] in all widgets in the list that have an origin, sequentially. Uses a reference frame
    /// to offset each widget.
    ///
    /// # `origin`
    ///
    /// The `origin` parameter is a function that takes a widget index and state and returns the offset that must
    /// be used to render it, if it must be rendered.
    fn render_filtered<O>(&self, origin: O, frame: &mut FrameBuilder)
    where
        O: FnMut(usize, &LazyStateMap) -> Option<LayoutPoint>;
}

/// Two [`WidgetList`] lists chained.
///
/// See [`WidgetList::chain`] for more information.
pub struct WidgetListChain<A: WidgetList, B: WidgetList>(A, B);

impl<A: WidgetList, B: WidgetList> UiNodeList for WidgetListChain<A, B> {
    fn len(&self) -> usize {
        self.0.len() + self.1.len()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty() && self.1.is_empty()
    }

    fn boxed_all(self) -> Vec<Box<dyn UiNode>> {
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

    fn update_all(&mut self, ctx: &mut WidgetContext) {
        self.0.update_all(ctx);
        self.1.update_all(ctx);
    }

    fn update_hp_all(&mut self, ctx: &mut WidgetContext) {
        self.0.update_hp_all(ctx);
        self.1.update_hp_all(ctx);
    }

    fn measure_all<AS, D>(&mut self, mut available_size: AS, mut desired_size: D, ctx: &mut LayoutContext)
    where
        AS: FnMut(usize, &mut LayoutContext) -> LayoutSize,
        D: FnMut(usize, LayoutSize, &mut LayoutContext),
    {
        self.0
            .measure_all(|i, c| available_size(i, c), |i, l, c| desired_size(i, l, c), ctx);
        let offset = self.0.len();
        self.1
            .measure_all(|i, c| available_size(i + offset, c), |i, l, c| desired_size(i + offset, l, c), ctx);
    }

    fn arrange_all<F>(&mut self, mut final_size: F, ctx: &mut LayoutContext)
    where
        F: FnMut(usize, &mut LayoutContext) -> LayoutSize,
    {
        self.0.arrange_all(|i, c| final_size(i, c), ctx);
        let offset = self.0.len();
        self.1.arrange_all(|i, c| final_size(i + offset, c), ctx);
    }

    fn render_all<O>(&self, mut origin: O, frame: &mut FrameBuilder)
    where
        O: FnMut(usize) -> LayoutPoint,
    {
        self.0.render_all(|i| origin(i), frame);
        let offset = self.0.len();
        self.1.render_all(|i| origin(i + offset), frame);
    }

    fn render_update_all(&self, update: &mut FrameUpdate) {
        self.0.render_update_all(update);
        self.1.render_update_all(update);
    }
}

impl<A: WidgetList, B: WidgetList> WidgetList for WidgetListChain<A, B> {
    fn boxed_widget_all(self) -> WidgetVec {
        let mut a = self.0.boxed_widget_all();
        a.extend(self.1.boxed_widget_all());
        a
    }

    fn render_filtered<O>(&self, mut origin: O, frame: &mut FrameBuilder)
    where
        O: FnMut(usize, &LazyStateMap) -> Option<LayoutPoint>,
    {
        self.0.render_filtered(|i, s| origin(i, s), frame);
        let offset = self.0.len();
        self.1.render_filtered(|i, s| origin(i + offset, s), frame);
    }

    fn widget_id(&self, index: usize) -> WidgetId {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_id(index)
        } else {
            self.1.widget_id(index - a_len)
        }
    }

    fn widget_state(&self, index: usize) -> &LazyStateMap {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_state(index)
        } else {
            self.1.widget_state(index - a_len)
        }
    }

    fn widget_state_mut(&mut self, index: usize) -> &mut LazyStateMap {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_state_mut(index)
        } else {
            self.1.widget_state_mut(index - a_len)
        }
    }

    fn widget_size(&self, index: usize) -> LayoutSize {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_size(index)
        } else {
            self.1.widget_size(index - a_len)
        }
    }
}

/// Two [`UiNodeList`] lists chained.
///
/// See [`UiNodeList::chain_nodes`] for more information.
pub struct UiNodeListChain<A: UiNodeList, B: UiNodeList>(A, B);

impl<A: UiNodeList, B: UiNodeList> UiNodeList for UiNodeListChain<A, B> {
    fn len(&self) -> usize {
        self.0.len() + self.1.len()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty() && self.1.is_empty()
    }

    fn boxed_all(self) -> Vec<Box<dyn UiNode>> {
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

    fn update_all(&mut self, ctx: &mut WidgetContext) {
        self.0.update_all(ctx);
        self.1.update_all(ctx);
    }

    fn update_hp_all(&mut self, ctx: &mut WidgetContext) {
        self.0.update_hp_all(ctx);
        self.1.update_hp_all(ctx);
    }

    fn measure_all<AS, D>(&mut self, mut available_size: AS, mut desired_size: D, ctx: &mut LayoutContext)
    where
        AS: FnMut(usize, &mut LayoutContext) -> LayoutSize,
        D: FnMut(usize, LayoutSize, &mut LayoutContext),
    {
        self.0
            .measure_all(|i, c| available_size(i, c), |i, l, c| desired_size(i, l, c), ctx);
        let offset = self.0.len();
        self.1
            .measure_all(|i, c| available_size(i + offset, c), |i, l, c| desired_size(i + offset, l, c), ctx);
    }

    fn arrange_all<F>(&mut self, mut final_size: F, ctx: &mut LayoutContext)
    where
        F: FnMut(usize, &mut LayoutContext) -> LayoutSize,
    {
        self.0.arrange_all(|i, c| final_size(i, c), ctx);
        let offset = self.0.len();
        self.1.arrange_all(|i, c| final_size(i + offset, c), ctx);
    }

    fn render_all<O>(&self, mut origin: O, frame: &mut FrameBuilder)
    where
        O: FnMut(usize) -> LayoutPoint,
    {
        self.0.render_all(|i| origin(i), frame);
        let offset = self.0.len();
        self.1.render_all(|i| origin(i + offset), frame);
    }

    fn render_update_all(&self, update: &mut FrameUpdate) {
        self.0.render_update_all(update);
        self.1.render_update_all(update);
    }
}

macro_rules! impl_iter_node {
    () => {
        #[inline]
        fn init_all(&mut self, ctx: &mut WidgetContext) {
            for w in self {
                w.init(ctx);
            }
        }

        #[inline]
        fn deinit_all(&mut self, ctx: &mut WidgetContext) {
            for w in self {
                w.deinit(ctx);
            }
        }

        #[inline]
        fn update_all(&mut self, ctx: &mut WidgetContext) {
            for w in self {
                w.update(ctx);
            }
        }

        #[inline]
        fn update_hp_all(&mut self, ctx: &mut WidgetContext) {
            for w in self {
                w.update_hp(ctx);
            }
        }

        fn measure_all<A, D>(&mut self, mut available_size: A, mut desired_size: D, ctx: &mut LayoutContext)
        where
            A: FnMut(usize, &mut LayoutContext) -> LayoutSize,
            D: FnMut(usize, LayoutSize, &mut LayoutContext),
        {
            for (i, w) in self.iter_mut().enumerate() {
                let available_size = available_size(i, ctx);
                let r = w.measure(available_size, ctx);
                desired_size(i, r, ctx);
            }
        }

        fn arrange_all<F>(&mut self, mut final_size: F, ctx: &mut LayoutContext)
        where
            F: FnMut(usize, &mut LayoutContext) -> LayoutSize,
        {
            for (i, w) in self.iter_mut().enumerate() {
                let final_size = final_size(i, ctx);
                w.arrange(final_size, ctx);
            }
        }

        fn render_all<O>(&self, mut origin: O, frame: &mut FrameBuilder)
        where
            O: FnMut(usize) -> LayoutPoint,
        {
            for (i, w) in self.iter().enumerate() {
                let origin = origin(i);
                frame.push_reference_frame(origin, |frame| w.render(frame));
            }
        }

        #[inline]
        fn render_update_all(&self, update: &mut FrameUpdate) {
            for w in self {
                w.render_update(update);
            }
        }
    };
}

macro_rules! impl_iter {
    () => {
        fn render_filtered<O>(&self, mut origin: O, frame: &mut FrameBuilder)
        where
            O: FnMut(usize, &LazyStateMap) -> Option<LayoutPoint>,
        {
            for (i, w) in self.iter().enumerate() {
                if let Some(origin) = origin(i, w.state()) {
                    frame.push_reference_frame(origin, |frame| w.render(frame));
                }
            }
        }

        fn widget_id(&self, index: usize) -> WidgetId {
            self[index].id()
        }

        fn widget_state(&self, index: usize) -> &LazyStateMap {
            self[index].state()
        }

        fn widget_state_mut(&mut self, index: usize) -> &mut LazyStateMap {
            self[index].state_mut()
        }

        fn widget_size(&self, index: usize) -> LayoutSize {
            self[index].size()
        }
    };
}

impl<W: UiNode> UiNodeList for Vec<W> {
    #[inline]
    fn len(&self) -> usize {
        self.len()
    }
    #[inline]
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
    #[inline]
    fn boxed_all(self) -> Vec<Box<dyn UiNode>> {
        self.into_iter().map(|n| n.boxed()).collect()
    }

    impl_iter_node! {}
}

impl<W: Widget> WidgetList for Vec<W> {
    #[inline]
    fn boxed_widget_all(self) -> WidgetVec {
        self.into_iter().map(|w| w.boxed_widget()).collect()
    }

    impl_iter! {}
}

macro_rules! impl_arrays {
    ( $($L:tt),+ $(,)?) => {$(
        impl<W: UiNode> UiNodeList for [W; $L] {
            fn len(&self) -> usize {
                $L
            }

            fn is_empty(&self) -> bool {
                $L == 0
            }

            fn boxed_all(self) -> Vec<Box<dyn UiNode>> {
                arrayvec::ArrayVec::from(self).into_iter().map(|w| w.boxed()).collect()
            }

            impl_iter_node! {}
        }

        impl<W: Widget> WidgetList for [W; $L] {
            fn boxed_widget_all(self) -> WidgetVec {
                arrayvec::ArrayVec::from(self).into_iter().map(|w| w.boxed_widget()).collect()
            }

            impl_iter! {}
        }
    )+};
}
impl_arrays! {
    0,
    1,
    2,
    3,
    4,
    5,
    6,
    7,
    8,
    9,
    10,
    11,
    12,
    13,
    14,
    15,
    16,
    17,
    18,
    19,
    20,
    21,
    22,
    23,
    24,
    25,
    26,
    27,
    28,
    29,
    30,
    31,
    32,
}

macro_rules! impl_tuples {
    ($($L:tt => $($n:tt),+;)+) => {$($crate::paste! {

        impl_tuples! { $L => $($n = [<W $n>]),+ }

    })+};
    ($L:tt => $($n:tt = $W:ident),+) => {
        impl<$($W: UiNode),+> UiNodeList for ($($W,)+) {
            #[inline]
            fn len(&self) -> usize {
                $L
            }

            #[inline]
            fn is_empty(&self) -> bool {
                false
            }

            #[inline]
            fn boxed_all(self) -> Vec<Box<dyn UiNode>> {
                vec![$(self.$n.boxed()),+]
            }

            #[inline]
            fn init_all(&mut self, ctx: &mut WidgetContext) {
                $(self.$n.init(ctx);)+
            }

            #[inline]
            fn deinit_all(&mut self, ctx: &mut WidgetContext) {
                $(self.$n.deinit(ctx);)+
            }

            #[inline]
            fn update_all(&mut self, ctx: &mut WidgetContext) {
                $(self.$n.update(ctx);)+
            }

            #[inline]
            fn update_hp_all(&mut self, ctx: &mut WidgetContext) {
                $(self.$n.update_hp(ctx);)+
            }

            fn measure_all<A, D>(&mut self, mut available_size: A, mut desired_size: D, ctx: &mut LayoutContext)
            where
                A: FnMut(usize, &mut LayoutContext) -> LayoutSize,
                D: FnMut(usize, LayoutSize, &mut LayoutContext),
            {
                $(
                let av_sz = available_size($n, ctx);
                let r = self.$n.measure(av_sz, ctx);
                desired_size($n, r, ctx);
                )+
            }

            fn arrange_all<F>(&mut self, mut final_size: F, ctx: &mut LayoutContext)
            where
                F: FnMut(usize, &mut LayoutContext) -> LayoutSize,
            {
                $(
                let fi_sz = final_size($n, ctx);
                self.$n.arrange(fi_sz, ctx);
                )+
            }

            fn render_all<O>(&self, mut origin: O, frame: &mut FrameBuilder)
            where
                O: FnMut(usize) -> LayoutPoint,
            {
                $(
                let o = origin($n);
                frame.push_reference_frame(o, |frame| self.$n.render(frame));
                )+
            }

            #[inline]
            fn render_update_all(&self, update: &mut FrameUpdate) {
                $(self.$n.render_update(update);)+
            }
        }

        impl<$($W: Widget),+> WidgetList for ($($W,)+) {
            #[inline]
            fn boxed_widget_all(self) -> WidgetVec {
                ui_vec![$(self.$n.boxed_widget()),+]
            }

            fn render_filtered<O>(&self, mut origin: O, frame: &mut FrameBuilder)
            where
                O: FnMut(usize, &LazyStateMap) -> Option<LayoutPoint>,
            {
                $(
                if let Some(o) = origin($n, self.$n.state()) {
                    frame.push_reference_frame(o, |frame| self.$n.render(frame));
                }
                )+
            }

            fn widget_id(&self, index: usize) -> WidgetId {
                match index {
                    $($n => self.$n.id(),)+
                    _ => panic!("index {} out of range for length {}", index, self.len())
                }
            }

            fn widget_state(&self, index: usize) -> &LazyStateMap {
                match index {
                    $($n => self.$n.state(),)+
                    _ => panic!("index {} out of range for length {}", index, self.len())
                }
            }

            fn widget_state_mut(&mut self, index: usize) -> &mut LazyStateMap {
                match index {
                    $($n => self.$n.state_mut(),)+
                    _ => panic!("index {} out of range for length {}", index, self.len())
                }
            }

            fn widget_size(&self, index: usize) -> LayoutSize {
                match index {
                    $($n => self.$n.size(),)+
                    _ => panic!("index {} out of range for length {}", index, self.len())
                }
            }
        }
    };
}
impl_tuples! {
    1 => 0;
    2 => 0, 1;
    3 => 0, 1, 2;
    4 => 0, 1, 2, 3;
    5 => 0, 1, 2, 3, 4;
    6 => 0, 1, 2, 3, 4, 5;
    7 => 0, 1, 2, 3, 4, 5, 6;
    8 => 0, 1, 2, 3, 4, 5, 6, 7;

    9 => 0, 1, 2, 3, 4, 5, 6, 7, 8;
    10 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9;
    11 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10;
    12 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11;
    13 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12;
    14 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13;
    15 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14;
    16 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15;

    17 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16;
    18 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17;
    18 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18;
    20 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19;
    21 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20;
    22 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21;
    23 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22;
    24 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23;

    25 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24;
    26 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25;
    27 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26;
    28 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27;
    29 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28;
    30 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29;
    31 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30;
    32 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31;
}

impl UiNodeList for () {
    #[inline]
    fn len(&self) -> usize {
        0
    }

    #[inline]
    fn is_empty(&self) -> bool {
        true
    }

    fn boxed_all(self) -> Vec<Box<dyn UiNode>> {
        vec![]
    }

    #[inline]
    fn init_all(&mut self, _: &mut WidgetContext) {}

    #[inline]
    fn deinit_all(&mut self, _: &mut WidgetContext) {}

    #[inline]
    fn update_all(&mut self, _: &mut WidgetContext) {}

    #[inline]
    fn update_hp_all(&mut self, _: &mut WidgetContext) {}

    fn measure_all<A, D>(&mut self, _: A, _: D, _: &mut LayoutContext)
    where
        A: FnMut(usize, &mut LayoutContext) -> LayoutSize,
        D: FnMut(usize, LayoutSize, &mut LayoutContext),
    {
    }

    fn arrange_all<F>(&mut self, _: F, _: &mut LayoutContext)
    where
        F: FnMut(usize, &mut LayoutContext) -> LayoutSize,
    {
    }

    fn render_all<O>(&self, _: O, _: &mut FrameBuilder)
    where
        O: FnMut(usize) -> LayoutPoint,
    {
    }

    #[inline]
    fn render_update_all(&self, _: &mut FrameUpdate) {}
}

impl WidgetList for () {
    #[inline]
    fn boxed_widget_all(self) -> WidgetVec {
        ui_vec![]
    }

    fn render_filtered<O>(&self, _: O, _: &mut FrameBuilder)
    where
        O: FnMut(usize, &LazyStateMap) -> Option<LayoutPoint>,
    {
    }

    fn widget_id(&self, index: usize) -> WidgetId {
        panic!("index {} out of range for length 0", index)
    }

    fn widget_state(&self, index: usize) -> &LazyStateMap {
        panic!("index {} out of range for length 0", index)
    }

    fn widget_state_mut(&mut self, index: usize) -> &mut LazyStateMap {
        panic!("index {} out of range for length 0", index)
    }

    fn widget_size(&self, index: usize) -> LayoutSize {
        panic!("index {} out of range for length 0", index)
    }
}
