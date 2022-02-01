use std::{
    cell::RefCell,
    cmp, mem,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use crate::{
    context::{InfoContext, LayoutContext, RenderContext, WidgetContext, WithUpdates},
    event::EventUpdateArgs,
    render::{FrameBuilder, FrameUpdate},
    state::StateMap,
    ui_list::{AvailableSizeArgs, DesiredSizeArgs, FinalSizeArgs, SortedWidgetVec, UiListObserver, WidgetFilterArgs, WidgetList},
    units::{AvailableSize, PxSize},
    widget_info::{UpdateSlot, WidgetInfoBuilder, WidgetLayout, WidgetLayoutInfo, WidgetRenderInfo, WidgetSubscriptions},
    BoxedUiNode, BoxedWidget, UiNode, UiNodeList, Widget, WidgetId,
};

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
    pub(super) ctrl: WidgetVecRef,
}
impl WidgetVec {
    /// New empty (default).
    #[inline]
    pub fn new() -> Self {
        WidgetVec {
            vec: vec![],
            ctrl: WidgetVecRef::new(),
        }
    }

    /// New empty with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        WidgetVec {
            vec: Vec::with_capacity(capacity),
            ctrl: WidgetVecRef::new(),
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

    fn fullfill_requests<O: UiListObserver>(&mut self, ctx: &mut WidgetContext, observer: &mut O) {
        if let Some(r) = self.ctrl.take_requests() {
            if r.clear {
                // if reset
                self.clear();
                observer.reseted();

                for (i, mut wgt) in r.insert {
                    wgt.init(ctx);
                    ctx.updates.info();
                    if i < self.len() {
                        self.insert(i, wgt);
                    } else {
                        self.push(wgt);
                    }
                }
                for mut wgt in r.push {
                    wgt.init(ctx);
                    ctx.updates.info();
                    self.push(wgt);
                }
                for (r, i) in r.move_index {
                    if r < self.len() {
                        let wgt = self.vec.remove(r);

                        if i < self.len() {
                            self.vec.insert(i, wgt);
                        } else {
                            self.vec.push(wgt);
                        }

                        ctx.updates.info();
                    }
                }
                for (id, to) in r.move_id {
                    if let Some(r) = self.vec.iter().position(|w| w.id() == id) {
                        let i = to(r, self.len());

                        if r != i {
                            let wgt = self.vec.remove(r);

                            if i < self.len() {
                                self.vec.insert(i, wgt);
                            } else {
                                self.vec.push(wgt);
                            }

                            ctx.updates.info();
                        }
                    }
                }
            } else {
                for id in r.remove {
                    if let Some(i) = self.vec.iter().position(|w| w.id() == id) {
                        let mut wgt = self.vec.remove(i);
                        wgt.deinit(ctx);
                        ctx.updates.info();

                        observer.removed(i);
                    }
                }

                for (i, mut wgt) in r.insert {
                    wgt.init(ctx);
                    ctx.updates.info();

                    if i < self.len() {
                        self.insert(i, wgt);
                        observer.inserted(i);
                    } else {
                        observer.inserted(self.len());
                        self.push(wgt);
                    }
                }

                for mut wgt in r.push {
                    wgt.init(ctx);
                    ctx.updates.info();

                    observer.inserted(self.len());
                    self.push(wgt);
                }

                for (r, i) in r.move_index {
                    if r < self.len() {
                        let wgt = self.vec.remove(r);

                        if i < self.len() {
                            self.vec.insert(i, wgt);

                            observer.moved(r, i);
                        } else {
                            let i = self.vec.len();

                            self.vec.push(wgt);

                            observer.moved(r, i);
                        }

                        ctx.updates.info();
                    }
                }

                for (id, to) in r.move_id {
                    if let Some(r) = self.vec.iter().position(|w| w.id() == id) {
                        let i = to(r, self.len());

                        if r != i {
                            let wgt = self.vec.remove(r);

                            if i < self.len() {
                                self.vec.insert(i, wgt);
                                observer.moved(r, i);
                            } else {
                                let i = self.vec.len();
                                self.vec.push(wgt);
                                observer.moved(r, i);
                            }

                            ctx.updates.info();
                        }
                    }
                }
            }
        }
    }
}
impl From<Vec<BoxedWidget>> for WidgetVec {
    fn from(vec: Vec<BoxedWidget>) -> Self {
        WidgetVec {
            vec,
            ctrl: WidgetVecRef::new(),
        }
    }
}
impl From<WidgetVec> for Vec<BoxedWidget> {
    fn from(mut s: WidgetVec) -> Self {
        mem::take(&mut s.vec)
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

    fn into_iter(mut self) -> Self::IntoIter {
        mem::take(&mut self.vec).into_iter()
    }
}
impl FromIterator<BoxedWidget> for WidgetVec {
    fn from_iter<T: IntoIterator<Item = BoxedWidget>>(iter: T) -> Self {
        Vec::from_iter(iter).into()
    }
}
impl UiNodeList for WidgetVec {
    fn is_fixed(&self) -> bool {
        false
    }

    fn len(&self) -> usize {
        self.vec.len()
    }

    fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    fn boxed_all(mut self) -> UiNodeVec {
        UiNodeVec {
            vec: mem::take(&mut self.vec).into_iter().map(|w| w.boxed()).collect(),
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

    fn update_all<O: UiListObserver>(&mut self, ctx: &mut WidgetContext, observer: &mut O) {
        self.fullfill_requests(ctx, observer);
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
        A: FnMut(&mut LayoutContext, AvailableSizeArgs) -> AvailableSize,
        D: FnMut(&mut LayoutContext, DesiredSizeArgs),
    {
        for (i, w) in self.iter_mut().enumerate() {
            let available_size = available_size(
                ctx,
                AvailableSizeArgs {
                    index: i,
                    state: Some(w.state_mut()),
                },
            );

            let ds = w.measure(ctx, available_size);

            desired_size(
                ctx,
                DesiredSizeArgs {
                    index: i,
                    state: Some(w.state_mut()),
                    desired_size: ds,
                },
            );
        }
    }

    fn widget_measure(&mut self, index: usize, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        self.vec[index].measure(ctx, available_size)
    }

    fn arrange_all<F>(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, mut final_size: F)
    where
        F: FnMut(&mut LayoutContext, &mut FinalSizeArgs) -> PxSize,
    {
        for (i, w) in self.iter_mut().enumerate() {
            FinalSizeArgs::impl_widget(ctx, widget_layout, i, w, &mut final_size);
        }
    }

    fn widget_arrange(&mut self, index: usize, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
        self.vec[index].arrange(ctx, widget_layout, final_size)
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

    fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        for w in self {
            w.render(ctx, frame);
        }
    }

    fn widget_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.vec[index].render(ctx, frame);
    }

    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        for w in self {
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

    fn widget_outer_info(&self, index: usize) -> &WidgetLayoutInfo {
        self.vec[index].outer_info()
    }

    fn widget_inner_info(&self, index: usize) -> &WidgetLayoutInfo {
        self.vec[index].inner_info()
    }

    fn widget_render_info(&self, index: usize) -> &WidgetRenderInfo {
        self.vec[index].render_info()
    }

    fn render_filtered<F>(&self, mut filter: F, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        F: FnMut(WidgetFilterArgs) -> bool,
    {
        for (i, w) in self.iter().enumerate() {
            if filter(WidgetFilterArgs::new(i, w)) {
                w.render(ctx, frame);
            }
        }
    }

    // default implementation uses indexing, this is faster.
    fn count<F>(&self, mut filter: F) -> usize
    where
        F: FnMut(WidgetFilterArgs) -> bool,
        Self: Sized,
    {
        let mut count = 0;
        for (i, w) in self.iter().enumerate() {
            if filter(WidgetFilterArgs::new(i, w)) {
                count += 1;
            }
        }
        count
    }
}
impl Drop for WidgetVec {
    fn drop(&mut self) {
        self.ctrl.0.borrow_mut().alive = false;
    }
}

/// See [`WidgetVecRef::move_to`] for more details
type WidgetMoveToFn = fn(usize, usize) -> usize;

/// Represents a [`WidgetVec`] controller that can be used to insert, push or remove widgets
/// after the vector is placed in a widget list property.
#[derive(Clone)]
pub struct WidgetVecRef(Rc<RefCell<WidgetVecRequests>>);
struct WidgetVecRequests {
    update_slot: UpdateSlot,
    insert: Vec<(usize, BoxedWidget)>,
    push: Vec<BoxedWidget>,
    remove: Vec<WidgetId>,
    move_index: Vec<(usize, usize)>,
    move_id: Vec<(WidgetId, WidgetMoveToFn)>,
    clear: bool,

    alive: bool,
}
impl WidgetVecRef {
    pub(super) fn new() -> Self {
        Self(Rc::new(RefCell::new(WidgetVecRequests {
            update_slot: UpdateSlot::next(),
            insert: vec![],
            push: vec![],
            remove: vec![],
            move_index: vec![],
            move_id: vec![],
            clear: false,
            alive: true,
        })))
    }

    /// Returns `true` if the [`WidgetVec`] still exists.
    pub fn alive(&self) -> bool {
        self.0.borrow().alive
    }

    /// Request an update for the insertion of the `widget`.
    ///
    /// The `index` is resolved after all [`remove`] requests, if it is out-of-bounds the widget is pushed.
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
    /// The widget will be deinitialized, dropped and the info tree and subscriptions will update, nothing happens
    /// if the widget is not found.
    pub fn remove(&self, updates: &mut impl WithUpdates, id: impl Into<WidgetId>) {
        updates.with_updates(|u| {
            let mut s = self.0.borrow_mut();
            s.remove.push(id.into());
            u.update(s.update_slot.mask());
        })
    }

    /// Request an widget remove and re-insert.
    ///
    /// If the `remove_index` is out of bounds nothing happens, if the `insert_index` is out-of-bounds
    /// the widget is pushed to the end of the vector, if `remove_index` and `insert_index` are equal nothing happens.
    ///
    /// Move requests happen after all other requests.
    pub fn move_index(&self, updates: &mut impl WithUpdates, remove_index: usize, insert_index: usize) {
        if remove_index != insert_index {
            updates.with_updates(|u| {
                let mut s = self.0.borrow_mut();
                s.move_index.push((remove_index, insert_index));
                u.update(s.update_slot.mask());
            })
        }
    }

    /// Request an widget move, the widget is searched by `id`, if found `get_move_to` id called with the index of the widget and length
    /// of the vector, it must return the index the widget is inserted after it is removed.
    ///
    /// If the widget is not found nothing happens, if the returned index is the same nothing happens, if the returned index
    /// is out-of-bounds the widget if pushed to the end of the vector.
    ///
    /// Move requests happen after all other requests.
    ///
    /// # Examples
    ///
    /// If the widget vectors is layout as a vertical stack to move the widget *up* by one stopping at the top:
    ///
    /// ```
    /// # fn demo(ctx: &mut zero_ui_core::context::WidgetContext, items: zero_ui_core::ui_list::WidgetVecRef) {
    /// items.move_id(ctx.updates, "my-widget", |i, _len| i.saturating_sub(1));
    /// # }
    /// ```
    ///
    /// And to move *down* stopping at the bottom:
    ///
    /// ```
    /// # fn demo(ctx: &mut zero_ui_core::context::WidgetContext, items: zero_ui_core::ui_list::WidgetVecRef) {
    /// items.move_id(ctx.updates, "my-widget", |i, _len| i.saturating_add(1));
    /// # }
    /// ```
    ///
    /// Note that if the returned index overflows the length the widget is
    /// pushed as the last item.
    ///
    /// The length can be used for implementing wrapping move *down*:
    ///
    /// ```
    /// # fn demo(ctx: &mut zero_ui_core::context::WidgetContext, items: zero_ui_core::ui_list::WidgetVecRef) {
    /// items.move_id(ctx.updates, "my-widget", |i, len| {
    ///     let next = i.saturating_add(1);
    ///     if next < len { next } else { 0 }
    /// });
    /// # }
    /// ```
    pub fn move_id(&self, updates: &mut impl WithUpdates, id: impl Into<WidgetId>, get_move_to: WidgetMoveToFn) {
        updates.with_updates(|u| {
            let mut s = self.0.borrow_mut();
            s.move_id.push((id.into(), get_move_to));
            u.update(s.update_slot.mask());
        })
    }

    /// Request a removal of all current widgets.
    ///
    /// All other requests will happen after the clear.
    pub fn clear(&self, updates: &mut impl WithUpdates) {
        updates.with_updates(|u| {
            let mut s = self.0.borrow_mut();
            s.clear = true;
            u.update(s.update_slot.mask());
        })
    }

    fn update_slot(&self) -> UpdateSlot {
        self.0.borrow().update_slot
    }

    fn take_requests(&self) -> Option<WidgetVecRequests> {
        let mut s = self.0.borrow_mut();

        if s.clear
            || !s.insert.is_empty()
            || !s.push.is_empty()
            || !s.remove.is_empty()
            || !s.move_index.is_empty()
            || !s.move_id.is_empty()
        {
            let empty = WidgetVecRequests {
                update_slot: s.update_slot,
                alive: s.alive,

                insert: vec![],
                push: vec![],
                remove: vec![],
                move_index: vec![],
                move_id: vec![],
                clear: false,
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
}
impl UiNodeVec {
    /// New empty (default).
    #[inline]
    pub fn new() -> Self {
        UiNodeVec { vec: vec![] }
    }

    /// New empty with pre-allocated capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        UiNodeVec {
            vec: Vec::with_capacity(capacity),
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
impl<'a> IntoIterator for &'a UiNodeVec {
    type Item = &'a BoxedUiNode;

    type IntoIter = std::slice::Iter<'a, BoxedUiNode>;

    fn into_iter(self) -> Self::IntoIter {
        self.vec.iter()
    }
}
impl FromIterator<BoxedUiNode> for UiNodeVec {
    fn from_iter<T: IntoIterator<Item = BoxedUiNode>>(iter: T) -> Self {
        UiNodeVec { vec: Vec::from_iter(iter) }
    }
}
impl From<Vec<BoxedUiNode>> for UiNodeVec {
    fn from(vec: Vec<BoxedUiNode>) -> Self {
        UiNodeVec { vec }
    }
}
impl From<UiNodeVec> for Vec<BoxedUiNode> {
    fn from(s: UiNodeVec) -> Self {
        s.vec
    }
}
impl UiNodeList for UiNodeVec {
    fn is_fixed(&self) -> bool {
        false
    }
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
    fn update_all<O: UiListObserver>(&mut self, ctx: &mut WidgetContext, _: &mut O) {
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
        A: FnMut(&mut LayoutContext, AvailableSizeArgs) -> AvailableSize,
        D: FnMut(&mut LayoutContext, DesiredSizeArgs),
    {
        for (i, node) in self.iter_mut().enumerate() {
            let av = available_size(ctx, AvailableSizeArgs { index: i, state: None });

            let d = node.measure(ctx, av);

            desired_size(
                ctx,
                DesiredSizeArgs {
                    index: i,
                    state: None,
                    desired_size: d,
                },
            );
        }
    }

    fn widget_measure(&mut self, index: usize, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        self.vec[index].measure(ctx, available_size)
    }

    fn arrange_all<F>(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, mut final_size: F)
    where
        F: FnMut(&mut LayoutContext, &mut FinalSizeArgs) -> PxSize,
    {
        for (i, node) in self.iter_mut().enumerate() {
            FinalSizeArgs::impl_node(ctx, widget_layout, i, node, &mut final_size);
        }
    }

    fn widget_arrange(&mut self, index: usize, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
        self.vec[index].arrange(ctx, widget_layout, final_size);
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

    fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        for w in self {
            w.render(ctx, frame);
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

/// Creates a [`WidgetVec`] containing the arguments.
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
///
/// [`WidgetVec`]: crate::ui_list::WidgetVec
#[macro_export]
macro_rules! widget_vec {
    () => { $crate::ui_list::WidgetVec::new() };
    ($($widget:expr),+ $(,)?) => {
        $crate::ui_list::WidgetVec::from(vec![
            $($crate::Widget::boxed_widget($widget)),*
        ])
    };
}
#[doc(inline)]
pub use crate::widget_vec;

/// Creates a [`UiNodeVec`] containing the arguments.
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
///
/// [`UiNodeVec`]: crate::ui_list::UiNodeVec
#[macro_export]
macro_rules! node_vec {
    () => { $crate::ui_list::UiNodeVec::new() };
    ($($node:expr),+ $(,)?) => {
        $crate::ui_list::UiNodeVec::from(vec![
            $($crate::UiNode::boxed($node)),*
        ])
    };
}
#[doc(inline)]
pub use crate::node_vec;
