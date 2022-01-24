use std::{
    cmp,
    ops::{Deref, DerefMut}, rc::Rc, cell::RefCell, mem,
};

use crate::{
    context::{InfoContext, LayoutContext, RenderContext, WidgetContext, WithUpdates},
    event::EventUpdateArgs,
    render::{FrameBuilder, FrameUpdate},
    state::StateMap,
    units::{AvailableSize, PxPoint, PxRect, PxSize},
    widget_base::Visibility,
    widget_info::{WidgetInfoBuilder, WidgetOffset, WidgetSubscriptions, UpdateSlot},
    BoxedUiNode, BoxedWidget, SortedWidgetVec, UiNode, UiNodeList, Widget, WidgetFilterArgs, WidgetId, WidgetList,
};

use super::SpatialIdGen;

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
///     println!("{:?}", widget.inner_bounds());
/// }
/// ```
pub struct WidgetVec {
    pub(super) vec: Vec<BoxedWidget>,
    pub(super) id: SpatialIdGen,

    pub(super) ctrl: WidgetVecRef,
}
impl WidgetVec {
    /// New empty (default).
    #[inline]
    pub fn new() -> Self {
        WidgetVec {
            vec: vec![],
            id: SpatialIdGen::default(),
            ctrl: WidgetVecRef::new()
        }
    }

    /// New empty with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        WidgetVec {
            vec: Vec::with_capacity(capacity),
            id: SpatialIdGen::default(),
            ctrl: WidgetVecRef::new()
        }
    }

    /// Returns a [`WidgetVecRef`] that can be used to insert, resort and remove widgets from this vector
    /// after it is moved to a widget list property.
    pub fn reference(&self) -> WidgetVecRef {
        self.ctrl.clone()
    }

    /// Appends the widget, automatically calls [`Widget::boxed_widget`].
    pub fn push<W: Widget>(&mut self, widget: W) {
        self.vec.push(widget.boxed_widget());
    }

    /// Appends the widget, automatically calls [`Widget::boxed_widget`].
    pub fn insert<W: Widget>(&mut self, index: usize, widget: W) {
        self.vec.insert(index, widget.boxed_widget());
    }

    /// Returns a reference to the widget with the same `id`.
    pub fn get(&self, id: impl Into<WidgetId>) -> Option<&BoxedWidget> {
        let id = id.into();
        self.vec.iter().find(|w| w.id() == id)
    }

    /// Returns a mutable reference to the widget with the same `id`.
    pub fn get_mut(&mut self, id: impl Into<WidgetId>) -> Option<&mut BoxedWidget> {
        let id = id.into();
        self.vec.iter_mut().find(|w| w.id() == id)
    }

    /// Removes and returns the widget, without affecting the order of widgets.
    pub fn remove(&mut self, id: impl Into<WidgetId>) -> Option<BoxedWidget> {
        let id = id.into();
        if let Some(i) = self.vec.iter().position(|w| w.id() == id) {
            Some(self.vec.remove(i))
        } else {
            None
        }
    }

    /// Convert `self` to a [`SortedWidgetVec`].
    ///
    /// See [`SortedWidgetVec::from_vec`] for more details.
    pub fn sorting(self, sort: impl FnMut(&BoxedWidget, &BoxedWidget) -> cmp::Ordering + 'static) -> SortedWidgetVec {
        SortedWidgetVec::from_vec(self, sort)
    }

    fn fullfill_requests(&mut self, ctx: &mut WidgetContext) {
        if let Some(r) = self.ctrl.take_requests() {
            for id in r.remove {
                if let Some(mut wgt) = self.remove(id) {
                    wgt.deinit(ctx);
                    ctx.updates.info();
                }
            }

            for (i, mut wgt) in r.insert {
                wgt.init(ctx);
                if i < self.len() {
                    self.insert(i, wgt);
                } else {
                    self.push(wgt);
                }
                ctx.updates.info();
            }

            for mut wgt in r.push {
                wgt.init(ctx);
                self.push(wgt);
                ctx.updates.info();
            }
        }
    }
}
impl From<Vec<BoxedWidget>> for WidgetVec {
    fn from(vec: Vec<BoxedWidget>) -> Self {
        WidgetVec {
            vec,
            id: SpatialIdGen::default(),
            ctrl: WidgetVecRef::new()
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
            id: self.id,
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
        self.fullfill_requests(ctx);
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

    fn info_all(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        for widget in &self.vec {
            widget.info(ctx, info);
        }
    }

    fn widget_info(&self, index: usize, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        self.vec[index].info(ctx, info);
    }

    fn subscriptions_all(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        subscriptions.update(self.ctrl.update_slot());
        for widget in &self.vec {
            widget.subscriptions(ctx, subscriptions);
        }
    }

    fn widget_subscriptions(&self, index: usize, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        self.vec[index].subscriptions(ctx, subscriptions);
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

    fn widget_outer_bounds(&self, index: usize) -> PxRect {
        self.vec[index].outer_bounds()
    }

    fn widget_inner_bounds(&self, index: usize) -> PxRect {
        self.vec[index].inner_bounds()
    }

    fn widget_visibility(&self, index: usize) -> Visibility {
        self.vec[index].visibility()
    }

    fn render_filtered<O>(&self, mut origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        O: FnMut(usize, WidgetFilterArgs) -> Option<PxPoint>,
    {
        let id = self.id.get();
        for (i, w) in self.iter().enumerate() {
            if let Some(origin) = origin(i, WidgetFilterArgs::get(self, i)) {
                frame.push_reference_frame_item(id, i, origin, |frame| {
                    w.render(ctx, frame);
                });
            }
        }
    }
}
/// Represents a [`WidgetVec`] controller that can be used to insert, push or remove widgets
/// after the vector is placed in a widget list property.
#[derive(Clone)]
pub struct WidgetVecRef(Rc<RefCell<WidgetVecRequests>>);
struct WidgetVecRequests {
    update_slot: UpdateSlot,
    insert: Vec<(usize, BoxedWidget)>,
    push: Vec<BoxedWidget>,
    remove: Vec<WidgetId>,

    alive: bool,
}
impl WidgetVecRef {
    pub(super) fn new() -> Self {
        Self(Rc::new(RefCell::new(WidgetVecRequests {
            update_slot: UpdateSlot::next(),
            insert: vec![],
            push: vec![],
            remove: vec![],
            alive: true,
        })))
    }

    /// Returns `true` if the [`SortedWidgetVec`] still exists.
    pub fn alive(&self) -> bool {
        self.0.borrow().alive
    }

    /// Request an update for the insertion of the `widget`.
    /// 
    /// The `index` is resolved after all [`remove`] requests, if it overflows the length the widget is pushed.
    ///
    /// The `widget` will be initialized, inserted and the info tree and subscriptions updated.
    /// 
    /// [`remove`]: Self::remove
    pub fn insert(&self, updates: &mut impl WithUpdates, index: usize, widget: impl Widget) {
        updates.with_updates(|u| {
            let mut s = self.0.borrow_mut();
            s.insert.push((index, widget.boxed_widget()));
            u.update(s.update_slot.mask());
        })
    }

    /// Request an update for the insertion of the `widget` at the end of the list.
    /// 
    /// The widget will be pushed after all [`insert`] requests.
    ///
    /// The `widget` will be initialized, inserted and the info tree and subscriptions updated.
    /// 
    /// [`insert`]: Self::insert
    pub fn push(&self, updates: &mut impl WithUpdates, widget: impl Widget) {
        updates.with_updates(|u| {
            let mut s = self.0.borrow_mut();
            s.push.push(widget.boxed_widget());
            u.update(s.update_slot.mask());
        })
    }

    /// Request an update for the removal of the widget identified by `id`.
    ///
    /// The widget will be deinitialized, dropped and the info tree and subscriptions will update.
    pub fn remove(&self, updates: &mut impl WithUpdates, id: impl Into<WidgetId>) {
        updates.with_updates(|u| {
            let mut s = self.0.borrow_mut();
            s.remove.push(id.into());
            u.update(s.update_slot.mask());
        })
    }

    fn update_slot(&self) -> UpdateSlot {
        self.0.borrow().update_slot
    }

    fn take_requests(&self) -> Option<WidgetVecRequests> {
        let mut s = self.0.borrow_mut();

        if !s.insert.is_empty() || !s.remove.is_empty() {
            let empty = WidgetVecRequests {
                update_slot: s.update_slot,
                alive: s.alive,

                insert: vec![],
                push: vec![],
                remove: vec![],
            };
            Some(mem::replace(&mut *s, empty))
        } else {
            None
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
    pub(super) vec: Vec<BoxedUiNode>,
    pub(super) id: SpatialIdGen,
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

    /// Insert the node, automatically calls [`UiNode::boxed`].
    pub fn insert<N: UiNode>(&mut self, index: usize, node: N) {
        self.vec.insert(index, node.boxed())
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

    fn info_all(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        for w in &self.vec {
            w.info(ctx, info);
        }
    }

    fn widget_info(&self, index: usize, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        self.vec[index].info(ctx, info);
    }

    fn subscriptions_all(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        for w in &self.vec {
            w.subscriptions(ctx, subscriptions);
        }
    }

    fn widget_subscriptions(&self, index: usize, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        self.vec[index].subscriptions(ctx, subscriptions);
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