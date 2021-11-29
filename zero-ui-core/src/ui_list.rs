use zero_ui_view_api::units::PxRect;

use super::units::{AvailableSize, PxPoint, PxSize};
#[allow(unused)] // used in docs.
use super::UiNode;
use super::{
    context::{LayoutContext, StateMap, WidgetContext},
    render::{FrameBuilder, FrameUpdate, SpatialFrameId},
    Widget, WidgetId,
};
use std::{
    cell::Cell,
    iter::FromIterator,
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
    /// # `final_rect`
    ///
    /// The `final_rect` parameter is a function that takes a widget index and the `ctx` and returns the
    /// final size and widget offset for the indexed widget.
    fn arrange_all<F>(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_rect: F)
    where
        F: FnMut(usize, &mut LayoutContext) -> PxRect;

    /// Calls [`UiNode::arrange`] in only the `index` widget.
    fn widget_arrange(&mut self, index: usize, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize);

    /// Calls [`UiNode::info`] in all widgets in the list, sequentially.
    fn info_all<O>(&self, origin: O, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder)
    where
        O: FnMut(usize) -> PxPoint;

    /// Calls [`UiNode::info`] in only the `index` widget.
    fn widget_info(&self, index: usize, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder);

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
        F: FnMut(usize, PxSize, &StateMap) -> bool,
    {
        let mut count = 0;
        for i in 0..self.len() {
            if filter(i, self.widget_size(i), self.widget_state(i)) {
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
    /// The `origin` parameter is a function that takes a widget index, size and state and returns the offset that must
    /// be used to render it, if it must be rendered.
    fn render_filtered<O>(&self, origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        O: FnMut(usize, PxSize, &StateMap) -> Option<PxPoint>;
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
pub struct WidgetVec {
    vec: Vec<BoxedWidget>,
    id: SpatialIdGen,
}
impl WidgetVec {
    /// New empty (default).
    #[inline]
    pub fn new() -> Self {
        WidgetVec {
            vec: vec![],
            id: SpatialIdGen::default(),
        }
    }

    /// New empty with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        WidgetVec {
            vec: Vec::with_capacity(capacity),
            id: SpatialIdGen::default(),
        }
    }

    /// Appends the widget, automatically calls [`Widget::boxed_widget`].
    pub fn push<W: Widget>(&mut self, widget: W) {
        self.vec.push(widget.boxed_widget());
    }
}
impl From<Vec<BoxedWidget>> for WidgetVec {
    fn from(vec: Vec<BoxedWidget>) -> Self {
        WidgetVec {
            vec,
            id: SpatialIdGen::default(),
        }
    }
}
impl From<WidgetVec> for Vec<BoxedWidget> {
    fn from(s: WidgetVec) -> Self {
        s.vec
    }
}
impl Default for WidgetVec {
    fn default() -> Self {
        Self::new()
    }
}
impl Deref for WidgetVec {
    type Target = Vec<BoxedWidget>;

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}
impl DerefMut for WidgetVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec
    }
}
impl<'a> IntoIterator for &'a WidgetVec {
    type Item = &'a BoxedWidget;

    type IntoIter = std::slice::Iter<'a, BoxedWidget>;

    fn into_iter(self) -> Self::IntoIter {
        self.vec.iter()
    }
}
impl<'a> IntoIterator for &'a mut WidgetVec {
    type Item = &'a mut BoxedWidget;

    type IntoIter = std::slice::IterMut<'a, BoxedWidget>;

    fn into_iter(self) -> Self::IntoIter {
        self.vec.iter_mut()
    }
}
impl IntoIterator for WidgetVec {
    type Item = BoxedWidget;

    type IntoIter = std::vec::IntoIter<BoxedWidget>;

    fn into_iter(self) -> Self::IntoIter {
        self.vec.into_iter()
    }
}
impl FromIterator<BoxedWidget> for WidgetVec {
    fn from_iter<T: IntoIterator<Item = BoxedWidget>>(iter: T) -> Self {
        Vec::from_iter(iter).into()
    }
}
impl UiNodeList for WidgetVec {
    fn len(&self) -> usize {
        self.vec.len()
    }

    fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    fn boxed_all(self) -> UiNodeVec {
        UiNodeVec {
            vec: self.vec.into_iter().map(|w| w.boxed()).collect(),
            id: SpatialIdGen::default(),
        }
    }

    fn init_all(&mut self, ctx: &mut WidgetContext) {
        for widget in &mut self.vec {
            widget.init(ctx);
        }
    }

    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        for widget in &mut self.vec {
            widget.deinit(ctx);
        }
    }

    fn update_all(&mut self, ctx: &mut WidgetContext) {
        for widget in &mut self.vec {
            widget.update(ctx);
        }
    }

    fn event_all<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
        for widget in &mut self.vec {
            widget.event(ctx, args);
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
        self.vec[index].measure(ctx, available_size)
    }

    fn arrange_all<F>(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, mut final_rect: F)
    where
        F: FnMut(usize, &mut LayoutContext) -> PxRect,
    {
        for (i, w) in self.iter_mut().enumerate() {
            let r = final_rect(i, ctx);
            widget_offset.with_offset(r.origin.to_vector(), |wo| w.arrange(ctx, wo, r.size))
        }
    }

    fn widget_arrange(&mut self, index: usize, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
        self.vec[index].arrange(ctx, widget_offset, final_size)
    }

    fn info_all<O>(&self, mut origin: O, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder)
    where
        O: FnMut(usize) -> PxPoint,
    {
        for (i, w) in self.iter().enumerate() {
            info.offset(origin(i).to_vector(), |info| w.info(ctx, info));
        }
    }

    fn widget_info(&self, index: usize, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder) {
        self.vec[index].info(ctx, info);
    }

    fn render_all<O>(&self, mut origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        O: FnMut(usize) -> PxPoint,
    {
        let id = self.id.get();
        for (i, w) in self.iter().enumerate() {
            let origin = origin(i);
            frame.push_reference_frame_item(id, i, origin, |frame| {
                w.render(ctx, frame);
            });
        }
    }

    fn widget_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.vec[index].render(ctx, frame);
    }

    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        for w in self.iter() {
            w.render_update(ctx, update);
        }
    }

    fn widget_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.vec[index].render_update(ctx, update);
    }
}
impl WidgetList for WidgetVec {
    fn boxed_widget_all(self) -> WidgetVec {
        self
    }

    fn widget_id(&self, index: usize) -> WidgetId {
        self.vec[index].id()
    }

    fn widget_state(&self, index: usize) -> &StateMap {
        self.vec[index].state()
    }

    fn widget_state_mut(&mut self, index: usize) -> &mut StateMap {
        self.vec[index].state_mut()
    }

    fn widget_size(&self, index: usize) -> PxSize {
        self.vec[index].size()
    }

    fn render_filtered<O>(&self, mut origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        O: FnMut(usize, PxSize, &StateMap) -> Option<PxPoint>,
    {
        let id = self.id.get();
        for (i, w) in self.iter().enumerate() {
            if let Some(origin) = origin(i, w.size(), w.state()) {
                frame.push_reference_frame_item(id, i, origin, |frame| {
                    w.render(ctx, frame);
                });
            }
        }
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
pub struct UiNodeVec {
    vec: Vec<BoxedUiNode>,
    id: SpatialIdGen,
}
impl UiNodeVec {
    /// New empty (default).
    #[inline]
    pub fn new() -> Self {
        UiNodeVec {
            vec: vec![],
            id: SpatialIdGen::default(),
        }
    }

    /// New empty with pre-allocated capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        UiNodeVec {
            vec: Vec::with_capacity(capacity),
            id: SpatialIdGen::default(),
        }
    }

    /// Appends the node, automatically calls [`UiNode::boxed`].
    pub fn push<N: UiNode>(&mut self, node: N) {
        self.vec.push(node.boxed());
    }
}
impl Default for UiNodeVec {
    fn default() -> Self {
        Self::new()
    }
}
impl Deref for UiNodeVec {
    type Target = Vec<BoxedUiNode>;

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}
impl DerefMut for UiNodeVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec
    }
}
impl IntoIterator for UiNodeVec {
    type Item = BoxedUiNode;

    type IntoIter = std::vec::IntoIter<BoxedUiNode>;

    fn into_iter(self) -> Self::IntoIter {
        self.vec.into_iter()
    }
}
impl FromIterator<BoxedUiNode> for UiNodeVec {
    fn from_iter<T: IntoIterator<Item = BoxedUiNode>>(iter: T) -> Self {
        UiNodeVec {
            vec: Vec::from_iter(iter),
            id: SpatialIdGen::default(),
        }
    }
}
impl From<Vec<BoxedUiNode>> for UiNodeVec {
    fn from(vec: Vec<BoxedUiNode>) -> Self {
        UiNodeVec {
            vec,
            id: SpatialIdGen::default(),
        }
    }
}
impl From<UiNodeVec> for Vec<BoxedUiNode> {
    fn from(s: UiNodeVec) -> Self {
        s.vec
    }
}
impl UiNodeList for UiNodeVec {
    fn len(&self) -> usize {
        self.vec.len()
    }
    fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }
    fn boxed_all(self) -> UiNodeVec {
        self
    }
    fn init_all(&mut self, ctx: &mut WidgetContext) {
        for node in self.iter_mut() {
            node.init(ctx);
        }
    }
    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        for node in self.iter_mut() {
            node.deinit(ctx);
        }
    }
    fn update_all(&mut self, ctx: &mut WidgetContext) {
        for node in self.iter_mut() {
            node.update(ctx);
        }
    }
    fn event_all<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
        for node in self.iter_mut() {
            node.event(ctx, args);
        }
    }

    fn measure_all<A, D>(&mut self, ctx: &mut LayoutContext, mut available_size: A, mut desired_size: D)
    where
        A: FnMut(usize, &mut LayoutContext) -> AvailableSize,
        D: FnMut(usize, PxSize, &mut LayoutContext),
    {
        for (i, node) in self.iter_mut().enumerate() {
            let av = available_size(i, ctx);
            let d = node.measure(ctx, av);
            desired_size(i, d, ctx);
        }
    }

    fn widget_measure(&mut self, index: usize, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        self.vec[index].measure(ctx, available_size)
    }

    fn arrange_all<F>(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, mut final_rect: F)
    where
        F: FnMut(usize, &mut LayoutContext) -> PxRect,
    {
        for (i, node) in self.iter_mut().enumerate() {
            let r = final_rect(i, ctx);
            widget_offset.with_offset(r.origin.to_vector(), |wo| node.arrange(ctx, wo, r.size));
        }
    }

    fn widget_arrange(&mut self, index: usize, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
        self.vec[index].arrange(ctx, widget_offset, final_size);
    }

    fn info_all<O>(&self, mut origin: O, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder)
    where
        O: FnMut(usize) -> PxPoint,
    {
        for (i, w) in self.iter().enumerate() {
            info.offset(origin(i).to_vector(), |info| w.info(ctx, info));
        }
    }

    fn widget_info(&self, index: usize, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder) {
        self.vec[index].info(ctx, info);
    }

    fn render_all<O>(&self, mut origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        O: FnMut(usize) -> PxPoint,
    {
        let id = self.id.get();
        for (i, w) in self.iter().enumerate() {
            let origin = origin(i);
            frame.push_reference_frame_item(id, i, origin, |frame| {
                w.render(ctx, frame);
            });
        }
    }

    fn widget_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.vec[index].render(ctx, frame);
    }

    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        for w in self.iter() {
            w.render_update(ctx, update)
        }
    }

    fn widget_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.vec[index].render_update(ctx, update);
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
        $crate::WidgetVec::from(vec![
            $($crate::Widget::boxed_widget($widget)),*
        ])
    };
}
#[doc(inline)]
pub use crate::widget_vec;
use crate::{
    context::RenderContext,
    event::EventUpdateArgs,
    widget_info::{WidgetInfoBuilder, WidgetOffset},
    BoxedUiNode, BoxedWidget,
};

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
        $crate::UiNodeVec::from(vec![
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
    ($w0:expr, $w1:expr, $w2:expr, $w3:expr, $w4:expr, $w5:expr, $w6:expr, $w7:expr, $($w_rest:expr),+ $(,)?) => {
        $crate::opaque_nodes({
            let w8 = $crate::__nodes!($w0, $w1, $w2, $w3, $w4, $w5, $w6, $w7);
            $crate::UiNodeList::chain_nodes(w8, $crate::__nodes!($($w_rest),+))
        })
    };
    ($($tt:tt)*) => {
        $crate::opaque_nodes($crate::static_list!($crate::UiNodeList0; $($tt)*))
    };
}

#[cfg(not(debug_assertions))]
#[doc(hidden)]
#[macro_export]
macro_rules! __widgets {
    ($w0:expr, $w1:expr, $w2:expr, $w3:expr, $w4:expr, $w5:expr, $w6:expr, $w7:expr, $($w_rest:expr),+ $(,)?) => {
        $crate::opaque_widgets({
            let w8 = $crate::__widgets!($w0, $w1, $w2, $w3, $w4, $w5, $w6, $w7);
            $crate::WidgetList::chain(w8, $crate::__widgets!($($w_rest),+))
        })
    };
    ($($tt:tt)*) => {
        $crate::opaque_widgets($crate::static_list!($crate::WidgetList0; $($tt)*))
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
    fn info_all<O>(&self, mut origin: O, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder)
    where
        O: FnMut(usize) -> PxPoint,
    {
        self.0.info_all(|i| origin(i), ctx, info);
        let offset = self.0.len();
        self.1.info_all(|i| origin(i + offset), ctx, info);
    }

    fn widget_info(&self, index: usize, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_info(index, ctx, info)
        } else {
            self.1.widget_info(index - a_len, ctx, info)
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
        O: FnMut(usize, PxSize, &StateMap) -> Option<PxPoint>,
    {
        self.0.render_filtered(|i, b, s| origin(i, b, s), ctx, frame);
        let offset = self.0.len();
        self.1.render_filtered(|i, b, s| origin(i + offset, b, s), ctx, frame);
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
    fn info_all<O>(&self, mut origin: O, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder)
    where
        O: FnMut(usize) -> PxPoint,
    {
        self.0.info_all(|i| origin(i), ctx, info);
        let offset = self.0.len();
        self.1.info_all(|i| origin(i + offset), ctx, info);
    }

    fn widget_info(&self, index: usize, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder) {
        let a_len = self.0.len();
        if index < a_len {
            self.0.widget_info(index, ctx, info)
        } else {
            self.1.widget_info(index - a_len, ctx, info)
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
                O: FnMut(usize, PxSize, &StateMap) -> Option<PxPoint>,
            {
                let id = self.id.get();
                $(
                if let Some(o) = origin($n, self.items.$n.size(), self.items.$n.state()) {
                    frame.push_reference_frame_item(id, $n, o, |frame| self.items.$n.render(ctx, frame));
                }
                )+
            }

            #[inline]
            fn widget_id(&self, index: usize) -> WidgetId {
                match index {
                    $($n => self.items.$n.id(),)+
                    _ => panic!("index {} out of range for length {}", index, self.len())
                }
            }

            #[inline]
            fn widget_state(&self, index: usize) -> &StateMap {
                match index {
                    $($n => self.items.$n.state(),)+
                    _ => panic!("index {} out of range for length {}", index, self.len())
                }
            }

            #[inline]
            fn widget_state_mut(&mut self, index: usize) -> &mut StateMap {
                match index {
                    $($n => self.items.$n.state_mut(),)+
                    _ => panic!("index {} out of range for length {}", index, self.len())
                }
            }

            #[inline]
            fn widget_size(&self, index: usize) -> PxSize {
                match index {
                    $($n => self.items.$n.size(),)+
                    _ => panic!("index {} out of range for length {}", index, self.len())
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
                    _ => panic!("index {} out of range for length {}", index, self.len()),
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
                    _ => panic!("index {} out of range for length {}", index, self.len()),
                }
            }

            #[inline(always)]
            fn info_all<O>(&self, mut origin: O, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder)
            where
                O: FnMut(usize) -> PxPoint,
            {
                $(
                info.offset(origin($n).to_vector(), |info| self.items.$n.info(ctx, info));
                )+
            }

            #[inline]
            fn widget_info(&self, index: usize, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder) {
                match index {
                    $(
                        $n => self.items.$n.info(ctx, info),
                    )+
                    _ => panic!("index {} out of range for length {}", index, self.len()),
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
                    _ => panic!("index {} out of range for length {}", index, self.len()),
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
                    _ => panic!("index {} out of range for length {}", index, self.len()),
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
                panic!("index {} out of range for length 0", index)
            }

            #[inline]
            fn arrange_all<F>(&mut self, _: &mut LayoutContext, _: &mut WidgetOffset, _: F)
            where
                F: FnMut(usize, &mut LayoutContext) -> PxRect,
            {
            }

            #[inline]
            fn widget_arrange(&mut self, index: usize, _: &mut LayoutContext, _: &mut WidgetOffset, _: PxSize) {
                panic!("index {} out of range for length 0", index)
            }

            fn info_all<O>(&self, _: O, _: &mut RenderContext, _: &mut WidgetInfoBuilder)
            where
                O: FnMut(usize) -> PxPoint,
            {
            }

            #[inline]
            fn widget_info(&self, index: usize, _: &mut RenderContext, _: &mut WidgetInfoBuilder) {
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
        O: FnMut(usize, PxSize, &StateMap) -> Option<PxPoint>,
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

#[derive(Default)]
struct SpatialIdGen(Cell<Option<SpatialFrameId>>);
impl SpatialIdGen {
    pub fn get(&self) -> SpatialFrameId {
        if let Some(id) = self.0.get() {
            id
        } else {
            let id = SpatialFrameId::new_unique();
            self.0.set(Some(id));
            id
        }
    }
}
