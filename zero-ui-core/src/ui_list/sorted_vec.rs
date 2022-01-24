use std::{cell::RefCell, cmp, mem, ops::Deref, rc::Rc};

use crate::{
    context::{InfoContext, LayoutContext, RenderContext, WidgetContext, WithUpdates},
    event::EventUpdateArgs,
    render::{FrameBuilder, FrameUpdate},
    state::StateMap,
    units::{AvailableSize, PxPoint, PxRect, PxSize},
    widget_base::Visibility,
    widget_info::{UpdateSlot, WidgetInfoBuilder, WidgetOffset, WidgetSubscriptions},
    BoxedWidget, UiNode, UiNodeList, UiNodeVec, Widget, WidgetFilterArgs, WidgetId, WidgetList, WidgetVec, WidgetVecRef,
};

use super::SpatialIdGen;

/// A vector of boxed [`Widget`] items that remains sorted.
///
/// This type is a [`WidgetList`] that can be modified during runtime, and automatically remains sorted
/// by a custom sorting function.
///
/// The [`widget_vec!`] macro can be used to initialize a [`WidgetVec`] and then call [`WidgetVec::sorting`] to convert to
/// a sorting vector.
///
/// The sorting is done using the [`std::slice::sort_by`], insertion sorting is done using a binary search followed by a small linear search,
/// in both cases the sorting is *stable*, widgets with equal keys retain order of insertion.
/// 
/// [`std::slice::sort_by`]: https://doc.rust-lang.org/std/primitive.slice.html#method.sort_by
pub struct SortedWidgetVec {
    vec: Vec<BoxedWidget>,
    id: SpatialIdGen,

    sort: Box<dyn FnMut(&BoxedWidget, &BoxedWidget) -> cmp::Ordering>,
    ctrl: SortedWidgetVecRef,
}
impl SortedWidgetVec {
    /// New empty (default).
    #[inline]
    pub fn new(sort: impl FnMut(&BoxedWidget, &BoxedWidget) -> cmp::Ordering + 'static) -> Self {
        SortedWidgetVec {
            vec: vec![],
            id: SpatialIdGen::default(),
            sort: Box::new(sort),
            ctrl: SortedWidgetVecRef::new(),
        }
    }

    /// New empty with pre-allocated capacity.
    pub fn with_capacity(capacity: usize, sort: impl FnMut(&BoxedWidget, &BoxedWidget) -> cmp::Ordering + 'static) -> Self {
        SortedWidgetVec {
            vec: Vec::with_capacity(capacity),
            id: SpatialIdGen::default(),
            sort: Box::new(sort),
            ctrl: SortedWidgetVecRef::new(),
        }
    }

    /// New from a [`WidgetVec`].
    pub fn from_vec(widgets: WidgetVec, sort: impl FnMut(&BoxedWidget, &BoxedWidget) -> cmp::Ordering + 'static) -> Self {
        let mut self_ = SortedWidgetVec {
            vec: widgets.vec,
            id: widgets.id,
            sort: Box::new(sort),
            ctrl: SortedWidgetVecRef::new(),
        };
        self_.sort();
        self_
    }

    /// Returns a [`SortedWidgetVecRef`] that can be used to insert, resort and remove widgets from this vector
    /// after it is moved to a widget list property.
    pub fn reference(&self) -> SortedWidgetVecRef {
        self.ctrl.clone()
    }

    /// Insert the widget in its sorted position.
    ///
    /// The widget is inserted all other widgets that are less then or equal to as defined by the sorting function.
    ///
    /// Automatically calls [`Widget::boxed_widget`].
    pub fn insert<W: Widget>(&mut self, widget: W) {
        let widget = widget.boxed_widget();
        match self.vec.binary_search_by(|a| (self.sort)(a, &widget)) {
            Ok(i) => {
                // last leg linear search.
                for i in i + 1..self.vec.len() {
                    if (self.sort)(&self.vec[i], &widget) != cmp::Ordering::Equal {
                        self.vec.insert(i, widget);
                        return;
                    }
                }
                self.vec.push(widget);
            }
            Err(i) => self.vec.insert(i, widget),
        }
    }

    /// Returns a reference to the widget with the same `id`.
    pub fn get(&self, id: impl Into<WidgetId>) -> Option<&BoxedWidget> {
        let id = id.into();
        self.vec.iter().find(|w| w.id() == id)
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

    /// Shrinks the capacity of the vector as much as possible.
    ///
    /// See [`Vec::shrink_to_fit`]
    pub fn shrink_to_fit(&mut self) {
        self.vec.shrink_to_fit()
    }

    /// Shrinks the capacity of the vector with a lower bound.
    ///
    /// See [`Vec::shrink_to`]
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.vec.shrink_to(min_capacity)
    }

    fn sort(&mut self) {
        self.vec.sort_by(|a, b| (self.sort)(a, b));
    }

    /// Sort and returns `true` if any widget was moved.
    fn sort_check(&mut self) -> bool {
        let mut any = false;
        self.vec.sort_by(|a, b| {
            let r = (self.sort)(a, b);
            any |= r == cmp::Ordering::Greater;
            r
        });
        any
    }

    /// remove and reinsert the widget if its sorting is invalid.
    fn sort_id(&mut self, id: WidgetId) -> bool {
        if let Some(i) = self.vec.iter().position(|w| w.id() == id) {
            if i > 0 {
                let a = &self.vec[i - 1];
                let b = &self.vec[i];

                if (self.sort)(a, b) == cmp::Ordering::Greater {
                    self.sort_i(i);
                    return true;
                }
            }

            if i + 1 < self.vec.len() {
                let a = &self.vec[i];
                let b = &self.vec[i + 1];

                if (self.sort)(a, b) == cmp::Ordering::Greater {
                    self.sort_i(i);
                    return true;
                }
            }
        }
        false
    }
    fn sort_i(&mut self, i: usize) {
        let widget = self.vec.remove(i);
        self.insert(widget);
    }

    fn fullfill_requests(&mut self, ctx: &mut WidgetContext) {
        if let Some(r) = self.ctrl.take_requests() {
            for id in r.remove {
                if let Some(mut wgt) = self.remove(id) {
                    wgt.deinit(ctx);
                    ctx.updates.info();
                }
            }

            for mut wgt in r.insert {
                wgt.init(ctx);
                self.insert(wgt);
                ctx.updates.info();
            }

            if r.sort_all {
                if self.sort_check() {
                    ctx.updates.info();
                }
            } else {
                for id in r.sort {
                    if self.sort_id(id) {
                        ctx.updates.info();
                    }
                }
            }
        }
    }
}
impl From<SortedWidgetVec> for WidgetVec {
    fn from(self_: SortedWidgetVec) -> Self {
        self_.boxed_widget_all()
    }
}
impl Deref for SortedWidgetVec {
    type Target = Vec<BoxedWidget>;

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}
impl<'a> IntoIterator for &'a SortedWidgetVec {
    type Item = &'a BoxedWidget;

    type IntoIter = std::slice::Iter<'a, BoxedWidget>;

    fn into_iter(self) -> Self::IntoIter {
        self.vec.iter()
    }
}
impl IntoIterator for SortedWidgetVec {
    type Item = BoxedWidget;

    type IntoIter = std::vec::IntoIter<BoxedWidget>;

    fn into_iter(mut self) -> Self::IntoIter {
        mem::take(&mut self.vec).into_iter()
    }
}
impl UiNodeList for SortedWidgetVec {
    fn len(&self) -> usize {
        self.vec.len()
    }

    fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    fn boxed_all(mut self) -> UiNodeVec {
        UiNodeVec {
            vec: mem::take(&mut self.vec).into_iter().map(|w| w.boxed()).collect(),
            id: mem::take(&mut self.id),
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
        for (i, w) in self.vec.iter_mut().enumerate() {
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
        for (i, w) in self.vec.iter_mut().enumerate() {
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
impl WidgetList for SortedWidgetVec {
    fn boxed_widget_all(mut self) -> WidgetVec {
        WidgetVec {
            vec: mem::take(&mut self.vec),
            id: mem::take(&mut self.id),
            ctrl: WidgetVecRef::new(),
        }
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
impl Drop for SortedWidgetVec {
    fn drop(&mut self) {
        self.ctrl.0.borrow_mut().alive = false;
    }
}

/// Represents a [`SortedWidgetVec`] controller that can be used to insert, resort or remove widgets
/// after the vector is placed in a widget list property.
#[derive(Clone)]
pub struct SortedWidgetVecRef(Rc<RefCell<SortedWidgetVecRequests>>);
struct SortedWidgetVecRequests {
    update_slot: UpdateSlot,
    insert: Vec<BoxedWidget>,
    remove: Vec<WidgetId>,
    sort: Vec<WidgetId>,
    sort_all: bool,

    alive: bool,
}
impl SortedWidgetVecRef {
    fn new() -> Self {
        Self(Rc::new(RefCell::new(SortedWidgetVecRequests {
            update_slot: UpdateSlot::next(),
            insert: vec![],
            remove: vec![],
            sort: vec![],
            sort_all: false,
            alive: true,
        })))
    }

    /// Returns `true` if the [`SortedWidgetVec`] still exists.
    pub fn alive(&self) -> bool {
        self.0.borrow().alive
    }

    /// Request an update for the insertion of the `widget`.
    ///
    /// The `widget` will be initialized, inserted in its sorted place and the info tree and subscriptions updated.
    pub fn insert(&self, updates: &mut impl WithUpdates, widget: impl Widget) {
        updates.with_updates(|u| {
            let mut s = self.0.borrow_mut();
            s.insert.push(widget.boxed_widget());
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

    /// Request an update for the resort of the widget identified by `id`.
    ///
    /// The list will be resorted and the info tree updated.
    pub fn sort(&self, updates: &mut impl WithUpdates, id: impl Into<WidgetId>) {
        updates.with_updates(|u| {
            let mut s = self.0.borrow_mut();

            if !s.sort_all {
                if s.sort.len() > 20 {
                    s.sort.clear();
                    s.sort_all = true;

                    u.update(s.update_slot.mask());
                } else {
                    s.sort.push(id.into());

                    u.update(s.update_slot.mask());
                }
            }
        })
    }

    fn update_slot(&self) -> UpdateSlot {
        self.0.borrow().update_slot
    }

    fn take_requests(&self) -> Option<SortedWidgetVecRequests> {
        let mut s = self.0.borrow_mut();

        if s.sort_all || !s.sort.is_empty() || !s.insert.is_empty() || !s.remove.is_empty() {
            let empty = SortedWidgetVecRequests {
                update_slot: s.update_slot,
                alive: s.alive,
                sort_all: false,

                insert: vec![],
                remove: vec![],
                sort: Vec::with_capacity(s.sort.len()),
            };
            Some(mem::replace(&mut *s, empty))
        } else {
            None
        }
    }
}
