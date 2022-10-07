use std::{cell::RefCell, cmp, mem, ops::Deref, rc::Rc};

use crate::{
    context::{
        state_map, InfoContext, LayoutContext, MeasureContext, RenderContext, StateMapMut, StateMapRef, WidgetContext, WidgetUpdates,
        WithUpdates,
    },
    event::EventUpdate,
    render::{FrameBuilder, FrameUpdate},
    ui_list::{PosLayoutArgs, PreLayoutArgs, UiListObserver, UiNodeList, UiNodeVec, WidgetFilterArgs, WidgetList, WidgetVec, WidgetVecRef},
    units::PxSize,
    widget_info::{WidgetBorderInfo, WidgetBoundsInfo, WidgetInfoBuilder, WidgetLayout, WidgetLayoutTranslation},
    BoxedWidget, UiNode, Widget, WidgetId,
};

use super::UiNodeFilterArgs;

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
/// [`widget_vec!`]: crate::ui_list::widget_vec
pub struct SortedWidgetVec {
    vec: Vec<BoxedWidget>,

    sort: Box<dyn FnMut(&BoxedWidget, &BoxedWidget) -> cmp::Ordering>,
    ctrl: SortedWidgetVecRef,
}
impl SortedWidgetVec {
    /// New empty (default).
    pub fn new(sort: impl FnMut(&BoxedWidget, &BoxedWidget) -> cmp::Ordering + 'static) -> Self {
        SortedWidgetVec {
            vec: vec![],
            sort: Box::new(sort),
            ctrl: SortedWidgetVecRef::new(),
        }
    }

    /// New empty with pre-allocated capacity.
    pub fn with_capacity(capacity: usize, sort: impl FnMut(&BoxedWidget, &BoxedWidget) -> cmp::Ordering + 'static) -> Self {
        SortedWidgetVec {
            vec: Vec::with_capacity(capacity),
            sort: Box::new(sort),
            ctrl: SortedWidgetVecRef::new(),
        }
    }

    /// New from a [`WidgetVec`].
    pub fn from_vec(mut widgets: WidgetVec, sort: impl FnMut(&BoxedWidget, &BoxedWidget) -> cmp::Ordering + 'static) -> Self {
        let mut self_ = SortedWidgetVec {
            vec: mem::take(&mut widgets.vec),
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
    /// Automatically calls [`Widget::boxed_wgt`].
    pub fn insert<W: Widget>(&mut self, widget: W) {
        self.insert_impl(widget.boxed_wgt());
    }
    fn insert_impl(&mut self, widget: BoxedWidget) -> usize {
        match self.vec.binary_search_by(|a| (self.sort)(a, &widget)) {
            Ok(i) => {
                // last leg linear search.
                for i in i + 1..self.vec.len() {
                    if (self.sort)(&self.vec[i], &widget) != cmp::Ordering::Equal {
                        self.vec.insert(i, widget);
                        return i;
                    }
                }

                let i = self.vec.len();
                self.vec.push(widget);

                i
            }
            Err(i) => {
                self.vec.insert(i, widget);
                i
            }
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
    fn sort_id(&mut self, id: WidgetId) -> Option<(usize, usize)> {
        if let Some(i) = self.vec.iter().position(|w| w.id() == id) {
            if i > 0 {
                let a = &self.vec[i - 1];
                let b = &self.vec[i];

                if (self.sort)(a, b) == cmp::Ordering::Greater {
                    let new_i = self.sort_i(i);
                    return Some((i, new_i));
                }
            }

            if i + 1 < self.vec.len() {
                let a = &self.vec[i];
                let b = &self.vec[i + 1];

                if (self.sort)(a, b) == cmp::Ordering::Greater {
                    let new_i = self.sort_i(i);
                    return Some((i, new_i));
                }
            }
        }
        None
    }
    fn sort_i(&mut self, i: usize) -> usize {
        let widget = self.vec.remove(i);
        self.insert_impl(widget)
    }

    fn fullfill_requests<O: UiListObserver>(&mut self, ctx: &mut WidgetContext, observer: &mut O) {
        if let Some(r) = self.ctrl.take_requests() {
            if r.sort_all || r.clear {
                // if large change
                let mut any_change = false;

                if r.clear {
                    any_change |= !self.vec.is_empty();

                    self.vec.clear();
                }

                for id in r.remove {
                    if let Some(i) = self.vec.iter().position(|w| w.id() == id) {
                        let mut wgt = self.vec.remove(i);
                        wgt.deinit(ctx);
                        ctx.updates.info();
                        any_change = true;
                    }
                }

                for mut wgt in r.insert {
                    wgt.init(ctx);
                    self.insert_impl(wgt);
                    ctx.updates.info();
                    any_change = true;
                }

                if r.sort_all {
                    if self.sort_check() {
                        any_change = true;
                    }
                } else {
                    for id in r.sort {
                        if self.sort_id(id).is_some() {
                            ctx.updates.info();
                        }
                    }
                }

                if any_change {
                    observer.reseted();
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

                for mut wgt in r.insert {
                    wgt.init(ctx);
                    let i = self.insert_impl(wgt);
                    ctx.updates.info();
                    observer.inserted(i);
                }

                for id in r.sort {
                    if let Some((r, i)) = self.sort_id(id) {
                        ctx.updates.info();
                        observer.moved(r, i);
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
        self.ctrl.0.borrow_mut().target = Some(ctx.path.widget_id());
        for widget in &mut self.vec {
            widget.init(ctx);
        }
    }

    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        self.ctrl.0.borrow_mut().target = None;
        for widget in &mut self.vec {
            widget.deinit(ctx);
        }
    }

    fn update_all<O: UiListObserver>(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates, observer: &mut O) {
        self.fullfill_requests(ctx, observer);
        for widget in &mut self.vec {
            widget.update(ctx, updates);
        }
    }

    fn event_all(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
        for widget in &mut self.vec {
            widget.event(ctx, update);
        }
    }

    fn measure_all<C, D>(&self, ctx: &mut MeasureContext, mut pre_measure: C, mut pos_measure: D)
    where
        C: FnMut(&mut MeasureContext, &mut super::PreMeasureArgs),
        D: FnMut(&mut MeasureContext, super::PosMeasureArgs),
    {
        for (i, w) in self.vec.iter().enumerate() {
            super::default_widget_list_measure_all(i, w, ctx, &mut pre_measure, &mut pos_measure)
        }
    }

    fn item_measure(&self, index: usize, ctx: &mut MeasureContext) -> PxSize {
        self.vec[index].measure(ctx)
    }

    fn layout_all<C, D>(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout, mut pre_layout: C, mut pos_layout: D)
    where
        C: FnMut(&mut LayoutContext, &mut WidgetLayout, &mut PreLayoutArgs),
        D: FnMut(&mut LayoutContext, &mut WidgetLayout, PosLayoutArgs),
    {
        for (i, w) in self.vec.iter_mut().enumerate() {
            super::default_widget_list_layout_all(i, w, ctx, wl, &mut pre_layout, &mut pos_layout)
        }
    }

    fn item_layout(&mut self, index: usize, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        self.vec[index].layout(ctx, wl)
    }

    fn info_all(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        for widget in &self.vec {
            widget.info(ctx, info);
        }
    }

    fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        for w in self {
            w.render(ctx, frame);
        }
    }

    fn item_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.vec[index].render(ctx, frame);
    }

    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        for w in self.iter() {
            w.render_update(ctx, update);
        }
    }

    fn item_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.vec[index].render_update(ctx, update);
    }

    fn try_item_id(&self, index: usize) -> Option<WidgetId> {
        self.vec[index].try_id()
    }

    fn try_item_state(&self, index: usize) -> Option<StateMapRef<state_map::Widget>> {
        self.vec[index].try_state()
    }

    fn try_item_state_mut(&mut self, index: usize) -> Option<StateMapMut<state_map::Widget>> {
        self.vec[index].try_state_mut()
    }

    fn try_item_bounds_info(&self, index: usize) -> Option<&WidgetBoundsInfo> {
        self.vec[index].try_bounds_info()
    }

    fn try_item_border_info(&self, index: usize) -> Option<&WidgetBorderInfo> {
        self.vec[index].try_border_info()
    }

    fn render_node_filtered<F>(&self, mut filter: F, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        F: FnMut(UiNodeFilterArgs) -> bool,
    {
        for (i, w) in self.iter().enumerate() {
            if filter(UiNodeFilterArgs::new(i, w)) {
                w.render(ctx, frame);
            }
        }
    }

    fn try_item_outer<F, R>(&mut self, index: usize, wl: &mut WidgetLayout, keep_previous: bool, transform: F) -> Option<R>
    where
        F: FnOnce(&mut WidgetLayoutTranslation, PosLayoutArgs) -> R,
    {
        let w = &mut self.vec[index];
        if let Some(size) = w.try_bounds_info().map(|i| i.outer_size()) {
            wl.try_with_outer(w, keep_previous, |wlt, w| {
                transform(wlt, PosLayoutArgs::new(index, w.try_state_mut(), size))
            })
        } else {
            None
        }
    }

    fn try_outer_all<F>(&mut self, wl: &mut WidgetLayout, keep_previous: bool, mut transform: F)
    where
        F: FnMut(&mut WidgetLayoutTranslation, PosLayoutArgs),
    {
        for (i, w) in self.vec.iter_mut().enumerate() {
            if let Some(size) = w.try_bounds_info().map(|i| i.outer_size()) {
                wl.try_with_outer(w, keep_previous, |wlt, w| {
                    transform(wlt, PosLayoutArgs::new(i, w.try_state_mut(), size));
                });
            }
        }
    }

    fn count_nodes<F>(&self, mut filter: F) -> usize
    where
        F: FnMut(UiNodeFilterArgs) -> bool,
    {
        let mut count = 0;
        for (i, w) in self.iter().enumerate() {
            if filter(UiNodeFilterArgs::new(i, w)) {
                count += 1;
            }
        }
        count
    }
}
impl WidgetList for SortedWidgetVec {
    fn boxed_widget_all(mut self) -> WidgetVec {
        WidgetVec {
            vec: mem::take(&mut self.vec),
            ctrl: WidgetVecRef::new(),
        }
    }

    fn item_id(&self, index: usize) -> WidgetId {
        self.vec[index].id()
    }

    fn item_state(&self, index: usize) -> StateMapRef<state_map::Widget> {
        self.vec[index].state()
    }

    fn item_state_mut(&mut self, index: usize) -> StateMapMut<state_map::Widget> {
        self.vec[index].state_mut()
    }

    fn item_bounds_info(&self, index: usize) -> &WidgetBoundsInfo {
        self.vec[index].bounds_info()
    }

    fn item_border_info(&self, index: usize) -> &WidgetBorderInfo {
        self.vec[index].border_info()
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

    fn item_outer<F, R>(&mut self, index: usize, wl: &mut WidgetLayout, keep_previous: bool, transform: F) -> R
    where
        F: FnOnce(&mut WidgetLayoutTranslation, PosLayoutArgs) -> R,
    {
        let w = &mut self.vec[index];
        let size = w.bounds_info().outer_size();
        wl.with_outer(w, keep_previous, |wlt, w| {
            transform(wlt, PosLayoutArgs::new(index, Some(w.state_mut()), size))
        })
    }

    fn outer_all<F>(&mut self, wl: &mut WidgetLayout, keep_previous: bool, mut transform: F)
    where
        F: FnMut(&mut WidgetLayoutTranslation, PosLayoutArgs),
    {
        for (i, w) in self.vec.iter_mut().enumerate() {
            let size = w.bounds_info().outer_size();
            wl.with_outer(w, keep_previous, |wlt, w| {
                transform(wlt, PosLayoutArgs::new(i, Some(w.state_mut()), size));
            });
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
    target: Option<WidgetId>,

    insert: Vec<BoxedWidget>,
    remove: Vec<WidgetId>,
    sort: Vec<WidgetId>,
    sort_all: bool,
    clear: bool,

    alive: bool,
}
impl SortedWidgetVecRef {
    fn new() -> Self {
        Self(Rc::new(RefCell::new(SortedWidgetVecRequests {
            target: None,
            insert: vec![],
            remove: vec![],
            sort: vec![],
            sort_all: false,
            clear: false,
            alive: true,
        })))
    }

    /// Returns `true` if the [`SortedWidgetVec`] still exists.
    pub fn alive(&self) -> bool {
        self.0.borrow().alive
    }

    /// Request an update for the insertion of the `widget`.
    ///
    /// The `widget` will be initialized, inserted in its sorted place and the info tree updated.
    pub fn insert(&self, updates: &mut impl WithUpdates, widget: impl Widget) {
        updates.with_updates(|u| {
            let mut s = self.0.borrow_mut();
            s.insert.push(widget.boxed_wgt());
            u.update(s.target);
        })
    }

    /// Request an update for the removal of the widget identified by `id`.
    ///
    /// The widget will be deinitialized, dropped and the info tree will update.
    pub fn remove(&self, updates: &mut impl WithUpdates, id: impl Into<WidgetId>) {
        updates.with_updates(|u| {
            let mut s = self.0.borrow_mut();
            s.remove.push(id.into());
            u.update(s.target);
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

                    u.update(s.target);
                } else {
                    s.sort.push(id.into());

                    u.update(s.target);
                }
            }
        })
    }

    /// Request a removal of all current widgets.
    ///
    /// All other requests will happen after the clear.
    pub fn clear(&self, updates: &mut impl WithUpdates) {
        updates.with_updates(|u| {
            let mut s = self.0.borrow_mut();
            s.clear = true;
            u.update(s.target);
        })
    }

    fn take_requests(&self) -> Option<SortedWidgetVecRequests> {
        let mut s = self.0.borrow_mut();

        if s.clear || s.sort_all || !s.sort.is_empty() || !s.insert.is_empty() || !s.remove.is_empty() {
            let empty = SortedWidgetVecRequests {
                target: s.target,

                alive: s.alive,
                sort_all: false,
                clear: false,

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
