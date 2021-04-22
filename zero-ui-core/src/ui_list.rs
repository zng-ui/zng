use super::units::{LayoutPoint, LayoutSize};
#[allow(unused)] // used in docs.
use super::UiNode;
use super::{
    context::{LayoutContext, LazyStateMap, WidgetContext},
    render::{FrameBuilder, FrameUpdate},
    Widget, WidgetId,
};
use std::{
    any::Any,
    iter::FromIterator,
    mem,
    ops::{Deref, DerefMut},
};

/// A generic view over a list of [`UiNode`] items.
pub trait UiNodeList: 'static {
    /// Number of items in the list.
    fn len(&self) -> usize;

    /// If the list is empty.
    fn is_empty(&self) -> bool;

    /// Boxes all items.
    fn boxed_all(self) -> UiNodeVec;

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

/// A vector of boxed [`Widget`] items.
///
/// This type is a [`WidgetList`] that can be modified during runtime, the downside
/// is the dynamic dispatch.
///
/// The [widget_vec!] macro is provided to make initialization more convenient.
///
/// ```
/// # use zero_ui_core::{widget_vec, UiNode, Widget, WidgetId, NilUiNode};
/// # use zero_ui_core::widget_base::*;
/// # fn text(fake: &str) -> impl Widget { implicit_base::new(NilUiNode, WidgetId::new_unique())  };
/// # use text as foo;
/// # use text as bar;
/// let mut widgets = widget_vec![];
/// widgets.push(foo("Hello"));
/// widgets.push(bar("Dynamic!"));
///
/// for widget in widgets {
///     println!("{}", widget.size());
/// }
/// ```
#[derive(Default)]
pub struct WidgetVec(pub Vec<Box<dyn Widget>>);
impl WidgetVec {
    /// New empty (default).
    #[inline]
    pub fn new() -> WidgetVec {
        Self::default()
    }

    /// Appends the widget, automatically calls [`Widget::boxed_widget`].
    pub fn push<W: Widget>(&mut self, widget: W) {
        self.0.push(widget.boxed_widget());
    }
}
impl Deref for WidgetVec {
    type Target = Vec<Box<dyn Widget>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for WidgetVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<'a> IntoIterator for &'a WidgetVec {
    type Item = &'a Box<dyn Widget>;

    type IntoIter = std::slice::Iter<'a, Box<dyn Widget>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
impl<'a> IntoIterator for &'a mut WidgetVec {
    type Item = &'a mut Box<dyn Widget>;

    type IntoIter = std::slice::IterMut<'a, Box<dyn Widget>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}
impl IntoIterator for WidgetVec {
    type Item = Box<dyn Widget>;

    type IntoIter = std::vec::IntoIter<Box<dyn Widget>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl FromIterator<Box<dyn Widget>> for WidgetVec {
    fn from_iter<T: IntoIterator<Item = Box<dyn Widget>>>(iter: T) -> Self {
        WidgetVec(Vec::from_iter(iter))
    }
}
impl<U: UiNode> UiNodeList for Vec<U> {
    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn boxed_all(mut self) -> UiNodeVec {
        if let Some(done) = <dyn Any>::downcast_mut(&mut self) {
            UiNodeVec(mem::take(done))
        } else {
            UiNodeVec(self.into_iter().map(|u| u.boxed()).collect())
        }
    }

    fn init_all(&mut self, ctx: &mut WidgetContext) {
        for node in self {
            node.init(ctx);
        }
    }

    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        for node in self {
            node.deinit(ctx);
        }
    }

    fn update_all(&mut self, ctx: &mut WidgetContext) {
        for node in self {
            node.update(ctx);
        }
    }

    fn update_hp_all(&mut self, ctx: &mut WidgetContext) {
        for node in self {
            node.update_hp(ctx);
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

    fn render_update_all(&self, update: &mut FrameUpdate) {
        for w in self {
            w.render_update(update);
        }
    }
}
impl<W: Widget> WidgetList for Vec<W> {
    fn boxed_widget_all(mut self) -> WidgetVec {
        if let Some(done) = <dyn Any>::downcast_mut(&mut self) {
            WidgetVec(mem::take(done))
        } else {
            WidgetVec(self.into_iter().map(|w| w.boxed_widget()).collect())
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
}
impl UiNodeList for WidgetVec {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn boxed_all(self) -> UiNodeVec {
        self.0.boxed_all()
    }

    fn init_all(&mut self, ctx: &mut WidgetContext) {
        self.0.init_all(ctx)
    }

    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        self.0.deinit_all(ctx)
    }

    fn update_all(&mut self, ctx: &mut WidgetContext) {
        self.0.update_all(ctx)
    }

    fn update_hp_all(&mut self, ctx: &mut WidgetContext) {
        self.0.update_hp_all(ctx)
    }

    fn measure_all<A, D>(&mut self, available_size: A, desired_size: D, ctx: &mut LayoutContext)
    where
        A: FnMut(usize, &mut LayoutContext) -> LayoutSize,
        D: FnMut(usize, LayoutSize, &mut LayoutContext),
    {
        self.0.measure_all(available_size, desired_size, ctx)
    }

    fn arrange_all<F>(&mut self, final_size: F, ctx: &mut LayoutContext)
    where
        F: FnMut(usize, &mut LayoutContext) -> LayoutSize,
    {
        self.0.arrange_all(final_size, ctx)
    }

    fn render_all<O>(&self, origin: O, frame: &mut FrameBuilder)
    where
        O: FnMut(usize) -> LayoutPoint,
    {
        self.0.render_all(origin, frame)
    }

    fn render_update_all(&self, update: &mut FrameUpdate) {
        self.0.render_update_all(update)
    }
}
impl WidgetList for WidgetVec {
    fn boxed_widget_all(self) -> WidgetVec {
        self
    }

    fn widget_id(&self, index: usize) -> WidgetId {
        self.0.widget_id(index)
    }

    fn widget_state(&self, index: usize) -> &LazyStateMap {
        self.0.widget_state(index)
    }

    fn widget_state_mut(&mut self, index: usize) -> &mut LazyStateMap {
        self.0.widget_state_mut(index)
    }

    fn widget_size(&self, index: usize) -> LayoutSize {
        self.0.widget_size(index)
    }

    fn render_filtered<O>(&self, origin: O, frame: &mut FrameBuilder)
    where
        O: FnMut(usize, &LazyStateMap) -> Option<LayoutPoint>,
    {
        self.0.render_filtered(origin, frame)
    }
}

/// A vector of boxed [`UiNode`] items.
///
/// This type is a [`UiNodeList`] that can be modified during runtime, the downside
/// is the dynamic dispatch.
///
/// The [node_vec!] macro is provided to make initialization more convenient.
///
/// ```
/// # use zero_ui_core::{node_vec, UiNode, Widget, WidgetId, NilUiNode};
/// # use zero_ui_core::widget_base::*;
/// # fn text(fake: &str) -> impl UiNode { zero_ui_core::NilUiNode };
/// # use text as foo;
/// # use text as bar;
/// let mut nodes = node_vec![];
/// nodes.push(foo("Hello"));
/// nodes.push(bar("Dynamic!"));
/// ```
#[derive(Default)]
pub struct UiNodeVec(pub Vec<Box<dyn UiNode>>);
impl UiNodeVec {
    /// New empty (default).
    #[inline]
    pub fn new() -> UiNodeVec {
        Self::default()
    }

    /// Appends the node, automatically calls [`UiNode::boxed`].
    pub fn push<N: UiNode>(&mut self, node: N) {
        self.0.push(node.boxed());
    }
}
impl Deref for UiNodeVec {
    type Target = Vec<Box<dyn UiNode>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for UiNodeVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<'a> IntoIterator for &'a UiNodeVec {
    type Item = &'a Box<dyn UiNode>;

    type IntoIter = std::slice::Iter<'a, Box<dyn UiNode>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
impl<'a> IntoIterator for &'a mut UiNodeVec {
    type Item = &'a mut Box<dyn UiNode>;

    type IntoIter = std::slice::IterMut<'a, Box<dyn UiNode>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}
impl IntoIterator for UiNodeVec {
    type Item = Box<dyn UiNode>;

    type IntoIter = std::vec::IntoIter<Box<dyn UiNode>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl FromIterator<Box<dyn UiNode>> for UiNodeVec {
    fn from_iter<T: IntoIterator<Item = Box<dyn UiNode>>>(iter: T) -> Self {
        UiNodeVec(Vec::from_iter(iter))
    }
}
impl UiNodeList for UiNodeVec {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    fn boxed_all(self) -> UiNodeVec {
        self
    }
    fn init_all(&mut self, ctx: &mut WidgetContext) {
        self.0.init_all(ctx)
    }
    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        self.0.deinit_all(ctx)
    }
    fn update_all(&mut self, ctx: &mut WidgetContext) {
        self.0.update_all(ctx)
    }
    fn update_hp_all(&mut self, ctx: &mut WidgetContext) {
        self.0.update_hp_all(ctx)
    }

    fn measure_all<A, D>(&mut self, available_size: A, desired_size: D, ctx: &mut LayoutContext)
    where
        A: FnMut(usize, &mut LayoutContext) -> LayoutSize,
        D: FnMut(usize, LayoutSize, &mut LayoutContext),
    {
        self.0.measure_all(available_size, desired_size, ctx)
    }

    fn arrange_all<F>(&mut self, final_size: F, ctx: &mut LayoutContext)
    where
        F: FnMut(usize, &mut LayoutContext) -> LayoutSize,
    {
        self.0.arrange_all(final_size, ctx)
    }

    fn render_all<O>(&self, origin: O, frame: &mut FrameBuilder)
    where
        O: FnMut(usize) -> LayoutPoint,
    {
        self.0.render_all(origin, frame)
    }
    fn render_update_all(&self, update: &mut FrameUpdate) {
        self.0.render_update_all(update)
    }
}

/// Creates a [`WidgetVec`](crate::WidgetVec) containing the arguments.
///
/// # Example
///
/// ```
/// # use zero_ui_core::{widget_vec, UiNode, Widget, WidgetId, NilUiNode};
/// # use zero_ui_core::widget_base::*;
/// # fn text(fake: &str) -> impl Widget { implicit_base::new(NilUiNode, WidgetId::new_unique())  };
/// # use text as foo;
/// # use text as bar;
/// let widgets = widget_vec![
///     foo("Hello"),
///     bar("World!")
/// ];
/// ```
///
/// `widget_vec!` automatically calls [`Widget::boxed_widget`] for each item.
#[macro_export]
macro_rules! widget_vec {
    () => { $crate::WidgetVec::new() };
    ($($widget:expr),+ $(,)?) => {
        $crate::WidgetVec(vec![
            $($crate::Widget::boxed_widget($widget)),*
        ])
    };
}
#[doc(inline)]
pub use crate::widget_vec;

/// Creates a [`UiNodeVec`](crate::UiNodeVec) containing the arguments.
///
/// # Example
///
/// ```
/// # use zero_ui_core::{node_vec, UiNode, Widget, WidgetId, NilUiNode};
/// # use zero_ui_core::widget_base::*;
/// # fn text(fake: &str) -> impl Widget { implicit_base::new(NilUiNode, WidgetId::new_unique())  };
/// # use text as foo;
/// # use text as bar;
/// let widgets = node_vec![
///     foo("Hello"),
///     bar("World!")
/// ];
/// ```
///
/// `node_vec!` automatically calls [`UiNode::boxed`] for each item.
#[macro_export]
macro_rules! node_vec {
    () => { $crate::UiNodeVec::new() };
    ($($node:expr),+ $(,)?) => {
        $crate::UiNodeVec(vec![
            $($crate::UiNode::boxed($node)),*
        ])
    };
}
#[doc(inline)]
pub use crate::node_vec;

/// Initialize an optimized [`WidgetList`].
///
/// The list type is opaque (`impl WidgetList`), and it changes depending on if the build is release or debug.
/// In both cases the list cannot be modified and the only methods available are provided by [`WidgetList`].
///
/// This is the recommended way to declare the contents of layout panel.
///
/// # Example
///
/// ```todo
/// # use zero_ui_core::{widgets, UiNode, Widget, WidgetId, NilUiNode};
/// # use zero_ui_core::widget_base::*;
/// # fn text(fake: &str) -> impl Widget { implicit_base::new(NilUiNode, WidgetId::new_unique())  };
/// # use text as foo;
/// # use text as bar;
/// let items = widgets![
///     foo("Hello "),
///     bar("World!")
/// ];
/// ```
#[macro_export]
macro_rules! widgets {
    () => {
        $crate::opaque_widgets($crate::WidgetList0)
    };
    ($($widget:expr),+ $(,)?) => {
        $crate::__widgets!{ $($widget),+ }
    }
}
#[doc(inline)]
pub use crate::widgets;

/// Initialize an optimized [`UiNodeList`].
///
/// The list type is opaque (`impl UiNodeList`), and it changes depending on if the build is release or debug.
/// In both cases the list cannot be modified and the only methods available are provided by [`UiNodeList`].
///
/// This is the recommended way to declare the contents of a property that takes multiple [`UiNode`](crate::UiNode) implementers.
///
/// # Example
///
/// ```todo
/// # use zero_ui_core::{nodes, UiNode, Widget, WidgetId, NilUiNode};
/// # use zero_ui_core::widget_base::*;
/// # fn text(fake: &str) -> impl Widget { implicit_base::new(NilUiNode, WidgetId::new_unique())  };
/// # use text as foo;
/// # use text as bar;
/// let items = widgets![
///     foo("Hello "),
///     bar("World!")
/// ];
/// ```
#[macro_export]
macro_rules! nodes {
    () => {
        $crate::opaque_nodes($crate::UiNodeList0)
    };
    ($($node:expr),+ $(,)?) => {
        $crate::__nodes!{ $($node),+ }
    }
}
#[doc(inline)]
pub use crate::nodes;

#[cfg(debug_assertions)]
#[doc(hidden)]
#[macro_export]
macro_rules! __nodes {
    ($($node:expr),+ $(,)?) => {
        $crate::opaque_nodes($crate::node_vec![
            $($node),+
        ])
    };
}

#[cfg(debug_assertions)]
#[doc(hidden)]
#[macro_export]
macro_rules! __widgets {
    ($($widget:expr),+ $(,)?) => {
        $crate::opaque_widgets($crate::widget_vec![
            $($widget),+
        ])
    };
}

#[cfg(not(debug_assertions))]
#[doc(hidden)]
#[macro_export]
macro_rules! __nodes {
    ($n0:expr $(,)?) => {
        $crate::opaque_nodes($crate::UiNodeList1($n0))
    };
    ($n0:expr, $n1:expr $(,)?) => {
        $crate::opaque_nodes($crate::UiNodeList2($n0, $n1))
    };
    ($n0:expr, $n1:expr, $n2:expr $(,)?) => {
        $crate::opaque_nodes($crate::UiNodeList3($n0, $n1, $n2))
    };
    ($n0:expr, $n1:expr, $n2:expr, $n3:expr $(,)?) => {
        $crate::opaque_nodes($crate::UiNodeList4($n0, $n1, $n2, $n3))
    };
    ($n0:expr, $n1:expr, $n2:expr, $n3:expr, $n4:expr $(,)?) => {
        $crate::opaque_nodes($crate::UiNodeList5($n0, $n1, $n2, $n3, $n4))
    };
    ($n0:expr, $n1:expr, $n2:expr, $n3:expr, $n4:expr, $n5:expr $(,)?) => {
        $crate::opaque_nodes($crate::UiNodeList6($n0, $n1, $n2, $n3, $n4, $n5))
    };
    ($n0:expr, $n1:expr, $n2:expr, $n3:expr, $n4:expr, $n5:expr, $n6:expr $(,)?) => {
        $crate::opaque_nodes($crate::UiNodeList7($n0, $n1, $n2, $n3, $n4, $n5, $n6))
    };
    ($n0:expr, $n1:expr, $n2:expr, $n3:expr, $n4:expr, $n5:expr, $n6:expr, $n7:expr $(,)?) => {
        $crate::opaque_nodes($crate::UiNodeList8($n0, $n1, $n2, $n3, $n4, $n5, $n6, $n7))
    };
    ($n0:expr, $n1:expr, $n2:expr, $n3:expr, $n4:expr, $n5:expr, $n6:expr, $n7:expr, $($n_rest:expr),+ $(,)?) => {
        $crate::opaque_nodes({
            let n8 = $crate::UiNodeList8($n0, $n1, $n2, $n3, $n4, $n5, $n6, $n7);
            $crate::UiNodeList::chain_nodes(n8, $crate::__nodes!($($n_rest),+))
        })
    };
}

#[cfg(not(debug_assertions))]
#[doc(hidden)]
#[macro_export]
macro_rules! __widgets {
    ($w0:expr $(,)?) => {
        $crate::opaque_widgets($crate::WidgetList1($w0))
    };
    ($w0:expr, $w1:expr $(,)?) => {
        $crate::opaque_widgets($crate::WidgetList2($w0, $w1))
    };
    ($w0:expr, $w1:expr, $w2:expr $(,)?) => {
        $crate::opaque_widgets($crate::WidgetList3($w0, $w1, $w2))
    };
    ($w0:expr, $w1:expr, $w2:expr, $w3:expr $(,)?) => {
        $crate::opaque_widgets($crate::WidgetList4($w0, $w1, $w2, $w3))
    };
    ($w0:expr, $w1:expr, $w2:expr, $w3:expr, $w4:expr $(,)?) => {
        $crate::opaque_widgets($crate::WidgetList5($w0, $w1, $w2, $w3, $w4))
    };
    ($w0:expr, $w1:expr, $w2:expr, $w3:expr, $w4:expr, $w5:expr $(,)?) => {
        $crate::opaque_widgets($crate::WidgetList6($w0, $w1, $w2, $w3, $w4, $w5))
    };
    ($w0:expr, $w1:expr, $w2:expr, $w3:expr, $w4:expr, $w5:expr, $w6:expr $(,)?) => {
        $crate::opaque_widgets($crate::WidgetList7($w0, $w1, $w2, $w3, $w4, $w5, $w6))
    };
    ($w0:expr, $w1:expr, $w2:expr, $w3:expr, $w4:expr, $w5:expr, $w6:expr, $w7:expr $(,)?) => {
        $crate::opaque_widgets($crate::WidgetList8($w0, $w1, $w2, $w3, $w4, $w5, $w6, $w7))
    };
    ($w0:expr, $w1:expr, $w2:expr, $w3:expr, $w4:expr, $w5:expr, $w6:expr, $w7:expr, $($w_rest:expr),+ $(,)?) => {
        $crate::opaque_widgets({
            let w8 = $crate::WidgetList8($w0, $w1, $w2, $w3, $w4, $w5, $w6, $w7);
            $crate::WidgetList::chain(w8, $crate::__widgets!($($w_rest),+))
        })
    };
}

#[doc(hidden)]
pub fn opaque_widgets(widgets: impl WidgetList) -> impl WidgetList {
    widgets
}

#[doc(hidden)]
pub fn opaque_nodes(nodes: impl UiNodeList) -> impl UiNodeList {
    nodes
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

macro_rules! impl_tuples {
    ($($L:tt => $($n:tt),+;)+) => {$($crate::paste! {

        impl_tuples! { [<UiNodeList $L>], [<WidgetList $L>] => $L => $($n = [<W $n>]),+ }

    })+};
    ($NodeList:ident, $WidgetList:ident => $L:tt => $($n:tt = $W:ident),+) => {
        impl_tuples! { impl_node => $NodeList<UiNode> => $L => $($n = $W),+ }
        impl_tuples! { impl_node => $WidgetList<Widget> => $L => $($n = $W),+ }

        impl<$($W: Widget),+> WidgetList for $WidgetList<$($W,)+> {
            #[inline]
            fn boxed_widget_all(self) -> WidgetVec {
                widget_vec![$(self.$n),+]
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

    (impl_node => $NodeList:ident <$Bound:ident> => $L:tt => $($n:tt = $W:ident),+) => {
        #[doc(hidden)]
        pub struct $NodeList<$($W: $Bound),+>($(pub $W),+);

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
                    $(self.$n),+
                ]
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
}

macro_rules! empty_node_list {
    ($($ident:ident),+) => {$(
        #[doc(hidden)]
        pub struct $ident;

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
    )+}
}
empty_node_list! {
    UiNodeList0,
    WidgetList0
}
impl WidgetList for WidgetList0 {
    #[inline]
    fn boxed_widget_all(self) -> WidgetVec {
        widget_vec![]
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
