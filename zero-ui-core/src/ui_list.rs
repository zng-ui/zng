use super::units::{AvailableSize, PxPoint, PxSize};
#[allow(unused)] // used in docs.
use super::UiNode;
use super::{
    context::{LayoutContext, StateMap, WidgetContext},
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

    /// Calls [`UiNode::event`] in all widgets in the list, sequentially.
    fn event_all<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU);

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
    fn measure_all<A, D>(&mut self, ctx: &mut LayoutContext, available_size: A, desired_size: D)
    where
        A: FnMut(usize, &mut LayoutContext) -> AvailableSize,
        D: FnMut(usize, PxSize, &mut LayoutContext);

    /// Calls [`UiNode::measure`] in only the `index` widget.
    fn widget_measure(&mut self, index: usize, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize;

    /// Calls [`UiNode::arrange`] in all widgets in the list, sequentially.
    ///
    /// # `final_size`
    ///
    /// The `final size` parameter is a function that takes a widget index and the `ctx` and returns the
    /// final size the widget must use.
    fn arrange_all<F>(&mut self, ctx: &mut LayoutContext, final_size: F)
    where
        F: FnMut(usize, &mut LayoutContext) -> PxSize;

    /// Calls [`UiNode::arrange`] in only the `index` widget.
    fn widget_arrange(&mut self, index: usize, ctx: &mut LayoutContext, final_size: PxSize);

    /// Calls [`UiNode::render`] in all widgets in the list, sequentially. Uses a reference frame
    /// to offset each widget.
    ///
    /// # `origin`
    ///
    /// The `origin` parameter is a function that takes a widget index and returns the offset that must
    /// be used to render it.
    fn render_all<O>(&self, origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        O: FnMut(usize) -> PxPoint;

    /// Calls [`UiNode::render`] in only the `index` widget.
    fn widget_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder);

    /// Calls [`UiNode::render_update`] in all widgets in the list, sequentially.
    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate);

    /// Calls [`UiNode::render_update`] in only the `index` widget.
    fn widget_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate);
}

/// A generic view over a list of [`Widget`] UI nodes.
///
/// Layout widgets should use this to abstract the children list type if possible.
pub trait WidgetList: UiNodeList {
    /// Count widgets that pass filter using the widget state.
    fn count<F>(&self, mut filter: F) -> usize
    where
        F: FnMut(usize, &StateMap) -> bool,
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
    fn widget_state(&self, index: usize) -> &StateMap;

    /// Exclusive reference the state of the widget at the `index`.
    fn widget_state_mut(&mut self, index: usize) -> &mut StateMap;

    /// Gets the last arranged size of the widget at the `index`.
    fn widget_size(&self, index: usize) -> PxSize;

    /// Calls [`UiNode::render`] in all widgets in the list that have an origin, sequentially. Uses a reference frame
    /// to offset each widget.
    ///
    /// # `origin`
    ///
    /// The `origin` parameter is a function that takes a widget index and state and returns the offset that must
    /// be used to render it, if it must be rendered.
    fn render_filtered<O>(&self, origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        O: FnMut(usize, &StateMap) -> Option<PxPoint>;
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
///     println!("{:?}", widget.size());
/// }
/// ```
#[derive(Default)]
pub struct WidgetVec(pub Vec<BoxedWidget>);
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
    type Target = Vec<BoxedWidget>;

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
    type Item = &'a BoxedWidget;

    type IntoIter = std::slice::Iter<'a, BoxedWidget>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
impl<'a> IntoIterator for &'a mut WidgetVec {
    type Item = &'a mut BoxedWidget;

    type IntoIter = std::slice::IterMut<'a, BoxedWidget>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}
impl IntoIterator for WidgetVec {
    type Item = BoxedWidget;

    type IntoIter = std::vec::IntoIter<BoxedWidget>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl FromIterator<BoxedWidget> for WidgetVec {
    fn from_iter<T: IntoIterator<Item = BoxedWidget>>(iter: T) -> Self {
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

    fn event_all<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
        for node in self {
            node.event(ctx, args);
        }
    }

    fn measure_all<A, D>(&mut self, ctx: &mut LayoutContext, mut available_size: A, mut desired_size: D)
    where
        A: FnMut(usize, &mut LayoutContext) -> AvailableSize,
        D: FnMut(usize, PxSize, &mut LayoutContext),
    {
        for (i, w) in self.iter_mut().enumerate() {
            let available_size = available_size(i, ctx);
            let r = w.measure(ctx, available_size);
            desired_size(i, r, ctx);
        }
    }

    fn widget_measure(&mut self, index: usize, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        self[index].measure(ctx, available_size)
    }

    fn arrange_all<F>(&mut self, ctx: &mut LayoutContext, mut final_size: F)
    where
        F: FnMut(usize, &mut LayoutContext) -> PxSize,
    {
        for (i, w) in self.iter_mut().enumerate() {
            let final_size = final_size(i, ctx);
            w.arrange(ctx, final_size);
        }
    }

    fn widget_arrange(&mut self, index: usize, ctx: &mut LayoutContext, final_size: PxSize) {
        self[index].arrange(ctx, final_size)
    }

    fn render_all<O>(&self, mut origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        O: FnMut(usize) -> PxPoint,
    {
        for (i, w) in self.iter().enumerate() {
            let origin = origin(i);
            frame.push_reference_frame(origin, |frame| w.render(ctx, frame));
        }
    }

    fn widget_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self[index].render(ctx, frame)
    }

    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        for w in self {
            w.render_update(ctx, update);
        }
    }

    fn widget_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self[index].render_update(ctx, update)
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

    fn widget_state(&self, index: usize) -> &StateMap {
        self[index].state()
    }

    fn widget_state_mut(&mut self, index: usize) -> &mut StateMap {
        self[index].state_mut()
    }

    fn widget_size(&self, index: usize) -> PxSize {
        self[index].size()
    }

    fn render_filtered<O>(&self, mut origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        O: FnMut(usize, &StateMap) -> Option<PxPoint>,
    {
        for (i, w) in self.iter().enumerate() {
            if let Some(origin) = origin(i, w.state()) {
                frame.push_reference_frame(origin, |frame| w.render(ctx, frame));
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

    fn event_all<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
        self.0.event_all(ctx, args);
    }

    fn measure_all<A, D>(&mut self, ctx: &mut LayoutContext, available_size: A, desired_size: D)
    where
        A: FnMut(usize, &mut LayoutContext) -> AvailableSize,
        D: FnMut(usize, PxSize, &mut LayoutContext),
    {
        self.0.measure_all(ctx, available_size, desired_size)
    }

    fn widget_measure(&mut self, index: usize, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        self.0.widget_measure(index, ctx, available_size)
    }

    fn arrange_all<F>(&mut self, ctx: &mut LayoutContext, final_size: F)
    where
        F: FnMut(usize, &mut LayoutContext) -> PxSize,
    {
        self.0.arrange_all(ctx, final_size)
    }

    fn widget_arrange(&mut self, index: usize, ctx: &mut LayoutContext, final_size: PxSize) {
        self.0.widget_arrange(index, ctx, final_size)
    }

    fn render_all<O>(&self, origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        O: FnMut(usize) -> PxPoint,
    {
        self.0.render_all(origin, ctx, frame)
    }

    fn widget_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.0.widget_render(index, ctx, frame)
    }

    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.0.render_update_all(ctx, update)
    }

    fn widget_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.0.widget_render_update(index, ctx, update)
    }
}
impl WidgetList for WidgetVec {
    fn boxed_widget_all(self) -> WidgetVec {
        self
    }

    fn widget_id(&self, index: usize) -> WidgetId {
        self.0.widget_id(index)
    }

    fn widget_state(&self, index: usize) -> &StateMap {
        self.0.widget_state(index)
    }

    fn widget_state_mut(&mut self, index: usize) -> &mut StateMap {
        self.0.widget_state_mut(index)
    }

    fn widget_size(&self, index: usize) -> PxSize {
        self.0.widget_size(index)
    }

    fn render_filtered<O>(&self, origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        O: FnMut(usize, &StateMap) -> Option<PxPoint>,
    {
        self.0.render_filtered(origin, ctx, frame)
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
pub struct UiNodeVec(pub Vec<BoxedUiNode>);
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
    type Target = Vec<BoxedUiNode>;

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
    type Item = &'a BoxedUiNode;

    type IntoIter = std::slice::Iter<'a, BoxedUiNode>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
impl<'a> IntoIterator for &'a mut UiNodeVec {
    type Item = &'a mut BoxedUiNode;

    type IntoIter = std::slice::IterMut<'a, BoxedUiNode>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}
impl IntoIterator for UiNodeVec {
    type Item = BoxedUiNode;

    type IntoIter = std::vec::IntoIter<BoxedUiNode>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl FromIterator<BoxedUiNode> for UiNodeVec {
    fn from_iter<T: IntoIterator<Item = BoxedUiNode>>(iter: T) -> Self {
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
    fn event_all<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
        self.0.event_all(ctx, args)
    }

    fn measure_all<A, D>(&mut self, ctx: &mut LayoutContext, available_size: A, desired_size: D)
    where
        A: FnMut(usize, &mut LayoutContext) -> AvailableSize,
        D: FnMut(usize, PxSize, &mut LayoutContext),
    {
        self.0.measure_all(ctx, available_size, desired_size)
    }

    fn widget_measure(&mut self, index: usize, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        self.0.widget_measure(index, ctx, available_size)
    }

    fn arrange_all<F>(&mut self, ctx: &mut LayoutContext, final_size: F)
    where
        F: FnMut(usize, &mut LayoutContext) -> PxSize,
    {
        self.0.arrange_all(ctx, final_size)
    }

    fn widget_arrange(&mut self, index: usize, ctx: &mut LayoutContext, final_size: PxSize) {
        self.0.widget_arrange(index, ctx, final_size)
    }

    fn render_all<O>(&self, origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        O: FnMut(usize) -> PxPoint,
    {
        self.0.render_all(origin, ctx, frame)
    }

    fn widget_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.0.widget_render(index, ctx, frame)
    }

    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.0.render_update_all(ctx, update)
    }

    fn widget_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.0.widget_render_update(index, ctx, update)
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
use crate::{context::RenderContext, event::EventUpdateArgs, BoxedUiNode, BoxedWidget};

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
    fn arrange_all<F>(&mut self, ctx: &mut LayoutContext, mut final_size: F)
    where
        F: FnMut(usize, &mut LayoutContext) -> PxSize,
    {
        self.0.arrange_all(ctx, |i, c| final_size(i, c));
        let offset = self.0.len();
        self.1.arrange_all(ctx, |i, c| final_size(i + offset, c));
    }

    #[inline]
    fn widget_arrange(&mut self, index: usize, ctx: &mut LayoutContext, final_size: PxSize) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_arrange(index, ctx, final_size)
        } else {
            self.1.widget_arrange(index - a_len, ctx, final_size)
        }
    }

    #[inline(always)]
    fn render_all<O>(&self, mut origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        O: FnMut(usize) -> PxPoint,
    {
        self.0.render_all(|i| origin(i), ctx, frame);
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
        O: FnMut(usize, &StateMap) -> Option<PxPoint>,
    {
        self.0.render_filtered(|i, s| origin(i, s), ctx, frame);
        let offset = self.0.len();
        self.1.render_filtered(|i, s| origin(i + offset, s), ctx, frame);
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

    #[inline]
    fn widget_size(&self, index: usize) -> PxSize {
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
    fn arrange_all<F>(&mut self, ctx: &mut LayoutContext, mut final_size: F)
    where
        F: FnMut(usize, &mut LayoutContext) -> PxSize,
    {
        self.0.arrange_all(ctx, |i, c| final_size(i, c));
        let offset = self.0.len();
        self.1.arrange_all(ctx, |i, c| final_size(i + offset, c));
    }

    #[inline]
    fn widget_arrange(&mut self, index: usize, ctx: &mut LayoutContext, final_size: PxSize) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_arrange(index, ctx, final_size)
        } else {
            self.1.widget_arrange(index - a_len, ctx, final_size)
        }
    }

    #[inline(always)]
    fn render_all<O>(&self, mut origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        O: FnMut(usize) -> PxPoint,
    {
        self.0.render_all(|i| origin(i), ctx, frame);
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

            #[inline(always)]
            fn render_filtered<O>(&self, mut origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
            where
                O: FnMut(usize, &StateMap) -> Option<PxPoint>,
            {
                $(
                if let Some(o) = origin($n, self.$n.state()) {
                    frame.push_reference_frame(o, |frame| self.$n.render(ctx, frame));
                }
                )+
            }

            #[inline]
            fn widget_id(&self, index: usize) -> WidgetId {
                match index {
                    $($n => self.$n.id(),)+
                    _ => panic!("index {} out of range for length {}", index, self.len())
                }
            }

            #[inline]
            fn widget_state(&self, index: usize) -> &StateMap {
                match index {
                    $($n => self.$n.state(),)+
                    _ => panic!("index {} out of range for length {}", index, self.len())
                }
            }

            #[inline]
            fn widget_state_mut(&mut self, index: usize) -> &mut StateMap {
                match index {
                    $($n => self.$n.state_mut(),)+
                    _ => panic!("index {} out of range for length {}", index, self.len())
                }
            }

            #[inline]
            fn widget_size(&self, index: usize) -> PxSize {
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

            #[inline(always)]
            fn init_all(&mut self, ctx: &mut WidgetContext) {
                $(self.$n.init(ctx);)+
            }

            #[inline(always)]
            fn deinit_all(&mut self, ctx: &mut WidgetContext) {
                $(self.$n.deinit(ctx);)+
            }

            #[inline(always)]
            fn update_all(&mut self, ctx: &mut WidgetContext) {
                $(self.$n.update(ctx);)+
            }

            #[inline(always)]
            fn event_all<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
                $(self.$n.event(ctx, args);)+
            }

            #[inline(always)]
            fn measure_all<A, D>(&mut self, ctx: &mut LayoutContext, mut available_size: A, mut desired_size: D)
            where
                A: FnMut(usize, &mut LayoutContext) -> AvailableSize,
                D: FnMut(usize, PxSize, &mut LayoutContext),
            {
                $(
                let av_sz = available_size($n, ctx);
                let r = self.$n.measure(ctx, av_sz);
                desired_size($n, r, ctx);
                )+
            }

            #[inline]
            fn widget_measure(&mut self, index: usize, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                match index {
                    $(
                        $n => self.$n.measure(ctx, available_size),
                    )+
                    _ => panic!("index {} out of range for length {}", index, self.len()),
                }
            }

            #[inline(always)]
            fn arrange_all<F>(&mut self, ctx: &mut LayoutContext, mut final_size: F)
            where
                F: FnMut(usize, &mut LayoutContext) -> PxSize,
            {
                $(
                let fi_sz = final_size($n, ctx);
                self.$n.arrange(ctx, fi_sz);
                )+
            }

            #[inline]
            fn widget_arrange(&mut self, index: usize, ctx: &mut LayoutContext, final_size: PxSize) {
                match index {
                    $(
                        $n => self.$n.arrange(ctx, final_size),
                    )+
                    _ => panic!("index {} out of range for length {}", index, self.len()),
                }
            }

            #[inline(always)]
            fn render_all<O>(&self, mut origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
            where
                O: FnMut(usize) -> PxPoint,
            {
                $(
                let o = origin($n);
                frame.push_reference_frame(o, |frame| self.$n.render(ctx, frame));
                )+
            }

            #[inline]
            fn widget_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                match index {
                    $(
                        $n => self.$n.render(ctx, frame),
                    )+
                    _ => panic!("index {} out of range for length {}", index, self.len()),
                }
            }

            #[inline(always)]
            fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                $(self.$n.render_update(ctx, update);)+
            }

            #[inline]
            fn widget_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                match index {
                    $(
                        $n => self.$n.render_update(ctx, update),
                    )+
                    _ => panic!("index {} out of range for length {}", index, self.len()),
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
                panic!("index {} out of range for length 0", index)
            }

            #[inline]
            fn arrange_all<F>(&mut self, _: &mut LayoutContext, _: F)
            where
                F: FnMut(usize, &mut LayoutContext) -> PxSize,
            {
            }

            #[inline]
            fn widget_arrange(&mut self, index: usize, _: &mut LayoutContext, _: PxSize) {
                panic!("index {} out of range for length 0", index)
            }

            fn render_all<O>(&self, _: O, _: &mut RenderContext, _: &mut FrameBuilder)
            where
                O: FnMut(usize) -> PxPoint,
            {
            }

            #[inline]
            fn widget_render(&self, index: usize, _: &mut RenderContext, _: &mut FrameBuilder) {
                panic!("index {} out of range for length 0", index)
            }

            #[inline]
            fn render_update_all(&self, _: &mut RenderContext, _: &mut FrameUpdate) {}

            #[inline]
            fn widget_render_update(&self, index: usize, _: &mut RenderContext, _: &mut FrameUpdate) {
                panic!("index {} out of range for length 0", index)
            }
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

    fn render_filtered<O>(&self, _: O, _: &mut RenderContext, _: &mut FrameBuilder)
    where
        O: FnMut(usize, &StateMap) -> Option<PxPoint>,
    {
    }

    fn widget_id(&self, index: usize) -> WidgetId {
        panic!("index {} out of range for length 0", index)
    }

    fn widget_state(&self, index: usize) -> &StateMap {
        panic!("index {} out of range for length 0", index)
    }

    fn widget_state_mut(&mut self, index: usize) -> &mut StateMap {
        panic!("index {} out of range for length 0", index)
    }

    fn widget_size(&self, index: usize) -> PxSize {
        panic!("index {} out of range for length 0", index)
    }
}
