use std::{
    cmp::Ordering,
    fmt, mem,
    ops::{self, ControlFlow},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering::Relaxed},
    },
};

use crate::{
    render::{FrameBuilder, FrameUpdate, FrameValueKey},
    update::{EventUpdate, UPDATES, WidgetUpdates},
    widget::{
        WIDGET, WidgetUpdateMode,
        base::{PARALLEL_VAR, Parallel},
        info::{WidgetInfo, WidgetInfoBuilder, WidgetLayout, WidgetMeasure},
    },
};
use parking_lot::Mutex;
use task::ParallelIteratorExt;
use zng_app_context::context_local;
use zng_layout::unit::{Factor, PxSize, PxTransform, PxVector};
use zng_state_map::StateId;
use zng_task::{self as task, rayon::prelude::*};
use zng_unique_id::static_id;
use zng_var::{animation::Transitionable, impl_from_and_into_var};

use super::*;

/// Creates an [`UiVec`] containing the arguments.
///  
/// Note that the items can be any type that converts to nodes, `ui_vec!` automatically calls [`IntoUiNode::into_node`] for each item.
///
/// # Examples
///
/// Create a vec containing a list of nodes/widgets:
///
/// ```
/// # use zng_app::widget::node::*;
/// # use zng_app::widget::base::*;
/// # macro_rules! Text { ($($tt:tt)*) => { UiNode::nil() } }
/// let widgets = ui_vec![Text!("Hello"), Text!("World!")];
/// ```
///
/// Create a vec containing the node repeated **n** times:
///
/// ```
/// # use zng_app::widget::node::*;
/// # use zng_app::widget::base::*;
/// # macro_rules! Text { ($($tt:tt)*) => { UiNode::nil() } }
/// let widgets = ui_vec![Text!(" . "); 10];
/// ```
///
/// Note that this is different from `vec![item; n]`, the node is not cloned, the expression is called **n** times to
/// generate the nodes.
#[macro_export]
macro_rules! ui_vec {
    () => { $crate::widget::node::UiVec::new() };
    ($node:expr; $n:expr) => {
        {
            let mut n: usize = $n;
            let mut vec = $crate::widget::node::UiVec::with_capacity(n);
            while n > 0 {
                vec.push($node);
                n -= 1;
            }
            vec
        }
    };
    ($($nodes:tt)+) => {
        $crate::ui_vec_items! {
            match { $($nodes)+ }
            result { }
        }
    };
}
#[doc(inline)]
pub use crate::ui_vec;

// macro to support `#[cfg(_)]` in items, Rust does not allow a match to `$(#[$meta:meta])* $node:expr`.
#[macro_export]
#[doc(hidden)]
macro_rules! ui_vec_items {
    // match attribute
    (
        match { #[$meta:meta] $($tt:tt)* }
        result { $($r:tt)* }
    ) => {
        $crate::ui_vec_items! {
            match { $($tt)* }
            result { $($r)* #[$meta] }
        }
    };
    // match node expr followed by comma
    (
        match { $node:expr, $($tt:tt)* }
        result { $($r:tt)* }
    ) => {
        $crate::ui_vec_items! {
            match { $($tt)* }
            result { $($r)* $crate::widget::node::IntoUiNode::into_node($node), }
        }
    };
    // match last node expr, no trailing comma
    (
        match { $node:expr }
        result { $($r:tt)* }
    ) => {
        $crate::ui_vec_items! {
            match { }
            result { $($r)* $crate::widget::node::IntoUiNode::into_node($node) }
        }
    };
    // finished
    (
        match { }
        result { $($r:tt)* }
    ) => {
        $crate::widget::node::UiVec::from(std::vec![
            $($r)*
        ])
    };
}

/// Vec of boxed UI nodes.
///
/// This is a thin wrapper around `Vec<UiNode>` that adds helper methods for pushing widgets without needing to box.
#[derive(Default)]
pub struct UiVec(Vec<UiNode>);
impl UiVec {
    /// New default.
    pub fn new() -> Self {
        Self::default()
    }

    /// New [`with_capacity`].
    ///
    /// [`with_capacity`]: Vec::with_capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /// Box and [`push`] the node.
    ///
    /// [`push`]: Vec::push
    pub fn push(&mut self, node: impl IntoUiNode) {
        self.0.push(node.into_node())
    }

    /// Box and [`insert`] the node.
    ///
    /// [`insert`]: Vec::insert
    pub fn insert(&mut self, index: usize, node: impl IntoUiNode) {
        self.0.insert(index, node.into_node())
    }

    /// Create a list chain node.
    ///
    /// See [`UiNode::chain`] for more details.
    pub fn chain(self, other: impl IntoUiNode) -> UiNode {
        UiNode::new(self).chain(other)
    }
}
impl ops::Deref for UiVec {
    type Target = Vec<UiNode>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ops::DerefMut for UiVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl From<Vec<UiNode>> for UiVec {
    fn from(vec: Vec<UiNode>) -> Self {
        Self(vec)
    }
}
impl From<UiVec> for Vec<UiNode> {
    fn from(vec: UiVec) -> Self {
        vec.0
    }
}
impl<U: IntoUiNode> FromIterator<U> for UiVec {
    fn from_iter<T: IntoIterator<Item = U>>(iter: T) -> Self {
        Self(Vec::from_iter(iter.into_iter().map(IntoUiNode::into_node)))
    }
}
impl IntoIterator for UiVec {
    type Item = UiNode;

    type IntoIter = std::vec::IntoIter<UiNode>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl IntoUiNode for Vec<UiNode> {
    fn into_node(self) -> UiNode {
        UiNode::new(UiVec(self))
    }
}
impl IntoUiNode for Box<[UiNode]> {
    fn into_node(self) -> UiNode {
        UiNode::new(UiVec(self.into()))
    }
}
impl UiNodeImpl for UiVec {
    fn children_len(&self) -> usize {
        self.len()
    }

    fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
        if index < self.len() {
            visitor(&mut self[index])
        }
    }

    fn is_list(&self) -> bool {
        true
    }

    fn for_each_child(&mut self, visitor: &mut dyn FnMut(usize, &mut UiNode)) {
        for (i, n) in self.0.iter_mut().enumerate() {
            visitor(i, n)
        }
    }

    fn try_for_each_child(
        &mut self,
        visitor: &mut dyn FnMut(usize, &mut UiNode) -> ControlFlow<BoxAnyVarValue>,
    ) -> ControlFlow<BoxAnyVarValue> {
        for (i, n) in self.0.iter_mut().enumerate() {
            visitor(i, n)?;
        }
        ControlFlow::Continue(())
    }

    fn par_each_child(&mut self, visitor: &(dyn Fn(usize, &mut UiNode) + Sync)) {
        if self.len() >= MIN_PARALLEL {
            self.par_iter_mut().enumerate().with_ctx().for_each(|(i, n)| visitor(i, n))
        } else {
            self.iter_mut().enumerate().for_each(|(i, n)| visitor(i, n))
        }
    }

    fn par_fold_reduce(
        &mut self,
        identity: BoxAnyVarValue,
        fold: &(dyn Fn(BoxAnyVarValue, usize, &mut UiNode) -> BoxAnyVarValue + Sync),
        reduce: &(dyn Fn(BoxAnyVarValue, BoxAnyVarValue) -> BoxAnyVarValue + Sync),
    ) -> BoxAnyVarValue {
        self.par_iter_mut()
            .enumerate()
            .with_ctx()
            .fold(|| identity.clone(), move |a, (i, n)| fold(a, i, n))
            .reduce(|| identity.clone(), reduce)
    }

    fn init(&mut self) {
        if (self as &mut dyn UiNodeImpl).parallelize_hint() && PARALLEL_VAR.get().contains(Parallel::INIT) {
            self.par_iter_mut().with_ctx().for_each(|n| n.init());
        } else {
            self.iter_mut().for_each(|n| n.init());
        }
    }

    fn deinit(&mut self) {
        if (self as &mut dyn UiNodeImpl).parallelize_hint() && PARALLEL_VAR.get().contains(Parallel::DEINIT) {
            self.par_iter_mut().with_ctx().for_each(|n| n.deinit());
        } else {
            self.iter_mut().for_each(|n| n.deinit());
        }
    }

    fn info(&mut self, info: &mut WidgetInfoBuilder) {
        if (self as &mut dyn UiNodeImpl).parallelize_hint() && PARALLEL_VAR.get().contains(Parallel::INFO) {
            let b = self
                .par_iter_mut()
                .with_ctx()
                .fold(
                    || info.parallel_split(),
                    |mut info, c| {
                        c.info(&mut info);
                        info
                    },
                )
                .reduce(
                    || info.parallel_split(),
                    |mut a, b| {
                        a.parallel_fold(b);
                        a
                    },
                );
            info.parallel_fold(b);
        } else {
            self.iter_mut().for_each(|n| n.info(info));
        }
    }

    fn event(&mut self, update: &EventUpdate) {
        if (self as &mut dyn UiNodeImpl).parallelize_hint() && PARALLEL_VAR.get().contains(Parallel::EVENT) {
            self.par_iter_mut().with_ctx().for_each(|n| n.event(update));
        } else {
            self.iter_mut().for_each(|n| n.event(update));
        }
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        if (self as &mut dyn UiNodeImpl).parallelize_hint() && PARALLEL_VAR.get().contains(Parallel::UPDATE) {
            self.par_iter_mut().with_ctx().for_each(|n| n.update(updates));
        } else {
            self.iter_mut().for_each(|n| n.update(updates));
        }
    }
    fn update_list(&mut self, updates: &WidgetUpdates, _: &mut dyn UiNodeListObserver) {
        self.update(updates);
    }

    fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
        if (self as &mut dyn UiNodeImpl).parallelize_hint() && PARALLEL_VAR.get().contains(Parallel::LAYOUT) {
            let (b, desired_size) = self
                .par_iter_mut()
                .with_ctx()
                .fold(
                    || (wm.parallel_split(), PxSize::zero()),
                    |(mut wm, desired_size), n| {
                        let n_ds = n.measure(&mut wm);
                        (wm, desired_size.max(n_ds))
                    },
                )
                .reduce(
                    || (wm.parallel_split(), PxSize::zero()),
                    |(mut wm, desired_size), (b_wm, b_ds)| {
                        wm.parallel_fold(b_wm);
                        (wm, desired_size.max(b_ds))
                    },
                );
            wm.parallel_fold(b);
            desired_size
        } else {
            let mut desired_size = PxSize::zero();
            self.iter_mut().for_each(|n| desired_size = desired_size.max(n.measure(wm)));
            desired_size
        }
    }

    fn measure_list(
        &mut self,
        wm: &mut WidgetMeasure,
        measure: &(dyn Fn(usize, &mut UiNode, &mut WidgetMeasure) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        if (self as &mut dyn UiNodeImpl).parallelize_hint() && PARALLEL_VAR.get().contains(Parallel::LAYOUT) {
            let (b, desired_size) = self
                .par_iter_mut()
                .enumerate()
                .with_ctx()
                .fold(
                    || (wm.parallel_split(), PxSize::zero()),
                    |(mut wm, desired_size), (i, n)| {
                        let n_ds = measure(i, n, &mut wm);
                        (wm, fold_size(desired_size, n_ds))
                    },
                )
                .reduce(
                    || (wm.parallel_split(), PxSize::zero()),
                    |(mut wm, desired_size), (b_wm, b_ds)| {
                        wm.parallel_fold(b_wm);
                        (wm, fold_size(desired_size, b_ds))
                    },
                );
            wm.parallel_fold(b);
            desired_size
        } else {
            let mut desired_size = PxSize::zero();
            self.iter_mut()
                .enumerate()
                .for_each(|(i, n)| desired_size = fold_size(desired_size, measure(i, n, wm)));
            desired_size
        }
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        if (self as &mut dyn UiNodeImpl).parallelize_hint() && PARALLEL_VAR.get().contains(Parallel::LAYOUT) {
            let (b, final_size) = self
                .par_iter_mut()
                .with_ctx()
                .fold(
                    || (wl.parallel_split(), PxSize::zero()),
                    |(mut wl, final_size), n| {
                        let n_ds = n.layout(&mut wl);
                        (wl, final_size.max(n_ds))
                    },
                )
                .reduce(
                    || (wl.parallel_split(), PxSize::zero()),
                    |(mut wl, desired_size), (b_wl, b_ds)| {
                        wl.parallel_fold(b_wl);
                        (wl, desired_size.max(b_ds))
                    },
                );
            wl.parallel_fold(b);
            final_size
        } else {
            let mut final_size = PxSize::zero();
            self.iter_mut().for_each(|n| final_size = final_size.max(n.layout(wl)));
            final_size
        }
    }

    fn layout_list(
        &mut self,
        wl: &mut WidgetLayout,
        layout: &(dyn Fn(usize, &mut UiNode, &mut WidgetLayout) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        if (self as &mut dyn UiNodeImpl).parallelize_hint() && PARALLEL_VAR.get().contains(Parallel::LAYOUT) {
            let (b, desired_size) = self
                .par_iter_mut()
                .enumerate()
                .with_ctx()
                .fold(
                    || (wl.parallel_split(), PxSize::zero()),
                    |(mut wl, desired_size), (i, n)| {
                        let n_ds = layout(i, n, &mut wl);
                        (wl, fold_size(desired_size, n_ds))
                    },
                )
                .reduce(
                    || (wl.parallel_split(), PxSize::zero()),
                    |(mut wl, desired_size), (b_wm, b_ds)| {
                        wl.parallel_fold(b_wm);
                        (wl, fold_size(desired_size, b_ds))
                    },
                );
            wl.parallel_fold(b);
            desired_size
        } else {
            let mut desired_size = PxSize::zero();
            self.iter_mut()
                .enumerate()
                .for_each(|(i, n)| desired_size = fold_size(desired_size, layout(i, n, wl)));
            desired_size
        }
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        if (self as &mut dyn UiNodeImpl).parallelize_hint() && PARALLEL_VAR.get().contains(Parallel::RENDER) {
            let mut par_start = 0;
            while frame.is_outer() && par_start < self.len() {
                // complete current widget first
                self[par_start].render(frame);
                par_start += 1;
            }
            let b = self[par_start..]
                .par_iter_mut()
                .with_ctx()
                .fold(
                    || frame.parallel_split(),
                    |mut frame, c| {
                        c.render(&mut frame);
                        frame
                    },
                )
                .reduce(
                    || frame.parallel_split(),
                    |mut a, b| {
                        a.parallel_fold(b);
                        a
                    },
                );
            frame.parallel_fold(b);
        } else {
            self.iter_mut().for_each(|n| n.render(frame));
        }
    }

    fn render_list(&mut self, frame: &mut FrameBuilder, render: &(dyn Fn(usize, &mut UiNode, &mut FrameBuilder) + Sync)) {
        if (self as &mut dyn UiNodeImpl).parallelize_hint() && PARALLEL_VAR.get().contains(Parallel::RENDER) {
            let mut par_start = 0;
            while frame.is_outer() && par_start < self.len() {
                // complete current widget first
                self[par_start].render(frame);
                par_start += 1;
            }
            let b = self[par_start..]
                .par_iter_mut()
                .enumerate()
                .with_ctx()
                .fold(
                    || frame.parallel_split(),
                    |mut frame, (i, c)| {
                        render(i, c, &mut frame);
                        frame
                    },
                )
                .reduce(
                    || frame.parallel_split(),
                    |mut a, b| {
                        a.parallel_fold(b);
                        a
                    },
                );
            frame.parallel_fold(b);
        } else {
            self.iter_mut().enumerate().for_each(|(i, n)| render(i, n, frame));
        }
    }

    fn render_update(&mut self, update: &mut FrameUpdate) {
        if (self as &mut dyn UiNodeImpl).parallelize_hint() && PARALLEL_VAR.get().contains(Parallel::RENDER) {
            let mut par_start = 0;
            while update.is_outer() && par_start < self.len() {
                // complete current widget first
                self[par_start].render_update(update);
                par_start += 1;
            }
            let b = self[par_start..]
                .par_iter_mut()
                .with_ctx()
                .fold(
                    || update.parallel_split(),
                    |mut update, c| {
                        c.render_update(&mut update);
                        update
                    },
                )
                .reduce(
                    || update.parallel_split(),
                    |mut a, b| {
                        a.parallel_fold(b);
                        a
                    },
                );
            update.parallel_fold(b);
        } else {
            self.iter_mut().for_each(|n| n.render_update(update));
        }
    }

    fn render_update_list(&mut self, update: &mut FrameUpdate, render_update: &(dyn Fn(usize, &mut UiNode, &mut FrameUpdate) + Sync)) {
        if (self as &mut dyn UiNodeImpl).parallelize_hint() && PARALLEL_VAR.get().contains(Parallel::RENDER) {
            let mut par_start = 0;
            while update.is_outer() && par_start < self.len() {
                // complete current widget first
                self[par_start].render_update(update);
                par_start += 1;
            }
            let b = self[par_start..]
                .par_iter_mut()
                .enumerate()
                .with_ctx()
                .fold(
                    || update.parallel_split(),
                    |mut update, (i, c)| {
                        render_update(i, c, &mut update);
                        update
                    },
                )
                .reduce(
                    || update.parallel_split(),
                    |mut a, b| {
                        a.parallel_fold(b);
                        a
                    },
                );
            update.parallel_fold(b);
        } else {
            self.iter_mut().enumerate().for_each(|(i, n)| render_update(i, n, update));
        }
    }

    fn as_widget(&mut self) -> Option<&mut dyn WidgetUiNodeImpl> {
        None
    }
}

/// List methods.
impl UiNode {
    /// Create a list node that has `self` followed by `other`.
    ///
    /// If `self` or `other` are already lists returns a list view that flattens the children when iterating.
    ///
    /// This method returns an optimized list view, it will reuse chain lists when possible,
    /// ignore nil and other tricks, if you need the inner lists to be a predictable arrangement use [`ChainList`] directly.
    pub fn chain(self, other: impl IntoUiNode) -> UiNode {
        self.chain_impl(other.into_node())
    }
    fn chain_impl(mut self, mut other: UiNode) -> UiNode {
        if self.is_nil() {
            return other;
        }
        if other.is_nil() {
            return self;
        }

        if let Some(chain) = self.downcast_mut::<ChainList>() {
            if let Some(other_too) = other.downcast_mut::<ChainList>() {
                chain.0.append(&mut other_too.0);
            } else {
                chain.0.push(other);
            }
            self
        } else {
            ChainList(ui_vec![self, other]).into_node()
        }
    }

    /// Create a sorting list view for this node into list.
    ///
    /// The list items are not moved, they are sorted only for layout and render, see [`SortingList`] for more details.
    ///
    /// If this node is already a sorting list just replaces the `sort`.
    pub fn sorting_by(mut self, sort: impl Fn(&mut UiNode, &mut UiNode) -> Ordering + Send + 'static) -> UiNode {
        if let Some(already) = self.downcast_mut::<SortingList>() {
            already.sort = Box::new(sort);
            already.invalidate_sort();
            self
        } else {
            SortingList::new(self, sort).into_node()
        }
    }
}

/// UI node list implementation that flattens child lists.
pub struct ChainList(pub UiVec);
impl ChainList {
    /// Append another list chain node.
    ///
    /// See [`UiNode::chain`] for more details.
    pub fn chain(self, other: impl IntoUiNode) -> UiNode {
        self.into_node().chain(other)
    }
}
impl UiNodeImpl for ChainList {
    fn children_len(&self) -> usize {
        let mut len = 0;
        for c in self.0.iter() {
            if c.is_list() {
                len += c.children_len();
            } else {
                len += 1;
            }
        }
        len
    }

    fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
        let mut offset = 0;
        for c in self.0.iter_mut() {
            let next_offset = offset + if c.is_list() { c.children_len() } else { 1 };
            if next_offset > index {
                c.with_child(index - offset, visitor);
                break;
            }
            offset = next_offset;
        }
    }

    fn is_list(&self) -> bool {
        true
    }

    fn for_each_child(&mut self, visitor: &mut dyn FnMut(usize, &mut UiNode)) {
        let mut offset = 0;
        for c in self.0.iter_mut() {
            if c.is_list() {
                c.for_each_child(|i, n| visitor(offset + i, n));
                offset += c.children_len();
            } else {
                visitor(offset, c);
                offset += 1;
            }
        }
    }

    fn try_for_each_child(
        &mut self,
        visitor: &mut dyn FnMut(usize, &mut UiNode) -> ControlFlow<BoxAnyVarValue>,
    ) -> ControlFlow<BoxAnyVarValue> {
        let mut offset = 0;
        for c in self.0.iter_mut() {
            if c.is_list() {
                let mut cf = ControlFlow::Continue(());
                c.for_each_child(|i, n| cf = visitor(offset + i, n));
                cf?;
                offset += c.children_len();
            } else {
                visitor(offset, c)?;
                offset += 1;
            }
        }
        ControlFlow::Continue(())
    }

    fn par_each_child(&mut self, visitor: &(dyn Fn(usize, &mut UiNode) + Sync)) {
        let mut offset = 0;
        for c in self.0.iter_mut() {
            if c.is_list() {
                c.par_each_child(|i, n| visitor(offset + i, n));
                offset += c.children_len();
            } else {
                visitor(offset, c);
                offset += 1;
            }
        }
    }

    fn par_fold_reduce(
        &mut self,
        identity: BoxAnyVarValue,
        fold: &(dyn Fn(BoxAnyVarValue, usize, &mut UiNode) -> BoxAnyVarValue + Sync),
        reduce: &(dyn Fn(BoxAnyVarValue, BoxAnyVarValue) -> BoxAnyVarValue + Sync),
    ) -> BoxAnyVarValue {
        let mut offset = 0;
        let mut accumulator = identity.clone();
        for c in self.0.iter_mut() {
            if c.is_list() {
                accumulator = c.0.par_fold_reduce(identity.clone(), &|acc, i, n| fold(acc, offset + i, n), reduce);
                offset += c.children_len();
            } else {
                accumulator = fold(accumulator, offset, c);
                offset += 1;
            }
        }
        accumulator
    }

    fn init(&mut self) {
        self.0.init();
    }

    fn deinit(&mut self) {
        self.0.deinit();
    }

    fn info(&mut self, info: &mut WidgetInfoBuilder) {
        self.0.info(info);
    }

    fn event(&mut self, update: &EventUpdate) {
        self.0.event(update);
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        self.0.update(updates);
    }

    fn update_list(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        if observer.is_reset_only() {
            if (self as &mut dyn UiNodeImpl).parallelize_hint() && PARALLEL_VAR.get().contains(Parallel::UPDATE) {
                let changed = self
                    .0
                    .par_iter_mut()
                    .with_ctx()
                    .map(|n| {
                        let mut changed = false;
                        n.update_list(updates, &mut changed);
                        changed
                    })
                    .reduce(|| false, |a, b| a || b);
                if changed {
                    observer.reset();
                }
            } else {
                let mut changed = false;
                for c in self.0.iter_mut() {
                    c.update_list(updates, &mut changed);
                }
                if changed {
                    observer.reset();
                }
            }
        } else {
            let mut offset = 0;
            for c in self.0.iter_mut() {
                if c.is_list() {
                    c.0.update_list(updates, &mut OffsetUiListObserver(offset, observer));
                    offset += c.children_len();
                } else {
                    c.update(updates);
                    offset += 1;
                }
            }
        }
    }

    fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
        self.0.measure(wm)
    }

    fn measure_list(
        &mut self,
        wm: &mut WidgetMeasure,
        measure: &(dyn Fn(usize, &mut UiNode, &mut WidgetMeasure) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        let mut offset = 0;
        let mut accumulator = PxSize::zero();
        for c in self.0.iter_mut() {
            if c.is_list() {
                let s = c.0.measure_list(wm, &|i, n, wm| measure(offset + i, n, wm), fold_size);
                accumulator = fold_size(accumulator, s);
                offset += c.children_len();
            } else {
                let s = measure(offset, c, wm);
                accumulator = fold_size(accumulator, s);
                offset += 1;
            }
        }
        accumulator
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        self.0.layout(wl)
    }

    fn layout_list(
        &mut self,
        wl: &mut WidgetLayout,
        layout: &(dyn Fn(usize, &mut UiNode, &mut WidgetLayout) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        let mut offset = 0;
        let mut accumulator = PxSize::zero();
        for c in self.0.iter_mut() {
            if c.is_list() {
                let s = c.0.layout_list(wl, &|i, n, wl| layout(offset + i, n, wl), fold_size);
                accumulator = fold_size(accumulator, s);
                offset += c.children_len();
            } else {
                let s = layout(offset, c, wl);
                accumulator = fold_size(accumulator, s);
                offset += 1;
            }
        }
        accumulator
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        self.0.render(frame);
    }

    fn render_list(&mut self, frame: &mut FrameBuilder, render: &(dyn Fn(usize, &mut UiNode, &mut FrameBuilder) + Sync)) {
        let mut offset = 0;
        for c in self.0.iter_mut() {
            if c.is_list() {
                c.0.render_list(frame, &|i, n, frame| render(offset + i, n, frame));
                offset += c.children_len();
            } else {
                render(offset, c, frame);
                offset += 1;
            }
        }
    }

    fn render_update(&mut self, update: &mut FrameUpdate) {
        self.0.render_update(update);
    }

    fn render_update_list(&mut self, update: &mut FrameUpdate, render_update: &(dyn Fn(usize, &mut UiNode, &mut FrameUpdate) + Sync)) {
        let mut offset = 0;
        for c in self.0.iter_mut() {
            if c.is_list() {
                c.0.render_update_list(update, &|i, n, update| render_update(offset + i, n, update));
                offset += c.children_len();
            } else {
                render_update(offset, c, update);
                offset += 1;
            }
        }
    }

    fn as_widget(&mut self) -> Option<&mut dyn WidgetUiNodeImpl> {
        None
    }
}

/// Represents the contextual parent [`SortingList`] during an update.
#[expect(non_camel_case_types)]
pub struct SORTING_LIST;
impl SORTING_LIST {
    /// If the current call has a parent sorting list.
    pub fn is_inside_list(&self) -> bool {
        !SORTING_LIST_PARENT.is_default()
    }

    /// Calls [`SortingList::invalidate_sort`] on the parent list.
    pub fn invalidate_sort(&self) {
        SORTING_LIST_PARENT.get().store(true, Relaxed)
    }

    fn with<R>(&self, action: impl FnOnce() -> R) -> (R, bool) {
        SORTING_LIST_PARENT.with_context(&mut Some(Arc::new(AtomicBool::new(false))), || {
            let r = action();
            (r, SORTING_LIST_PARENT.get().load(Relaxed))
        })
    }
}
context_local! {
    static SORTING_LIST_PARENT: AtomicBool = AtomicBool::new(false);
}

/// Represents a sorted view into an [`UiNode::is_list`].
///
/// The underlying list is not changed, a sorted index map is used to iterate the underlying list.
///
/// The sorting is lazy and gets invalidated on every init and every time there are changes observed in [`update_list`].
///
/// Methods `measure_list`, `layout_list`, `render`, `for_each_child` and `par_each_child` are the **only that iterate sorted**. Method
/// `with_child` uses the sort index. Method `update_list` notifies a reset if there is any change in the list or sorting.
/// Other methods delegate to the unsorted list.
///
/// [`update_list`]: UiNode::update_list
pub struct SortingList {
    list: UiNode,

    map: Vec<usize>,
    sort: Box<dyn Fn(&mut UiNode, &mut UiNode) -> Ordering + Send + 'static>,
}
impl SortingList {
    /// New from list and sort function.
    pub fn new(list: impl IntoUiNode, sort: impl Fn(&mut UiNode, &mut UiNode) -> Ordering + Send + 'static) -> Self {
        Self {
            list: list.into_node().into_list(),
            map: vec![],
            sort: Box::new(sort),
        }
    }

    fn update_map(&mut self) {
        let map = &mut self.map;
        let len = self.list.children_len();

        if len == 0 {
            map.clear();
        } else if map.len() != len {
            map.clear();
            map.extend(0..len);
            let mut taken_a = UiNode::nil();
            map.sort_by(|&a, &b| {
                self.list.with_child(a, |a| mem::swap(a, &mut taken_a));
                let result = self.list.with_child(b, |b| (self.sort)(&mut taken_a, b));
                self.list.with_child(a, |a| mem::swap(a, &mut taken_a));

                result
            })
        }
    }
    /// Mutable borrow the inner list.
    ///
    /// You must call [`invalidate_sort`] if any modification is done to the list.
    ///
    /// [`invalidate_sort`]: Self::invalidate_sort
    pub fn list(&mut self) -> &mut UiNode {
        &mut self.list
    }

    /// Invalidate the sort, the list will resort on the nest time the sorted positions are needed.
    ///
    /// Note that you can also invalidate sort from the inside using [`SORTING_LIST::invalidate_sort`].
    pub fn invalidate_sort(&mut self) {
        self.map.clear()
    }

    fn with_map<R>(&mut self, f: impl FnOnce(&[usize], &mut UiNode) -> R) -> R {
        self.update_map();

        let (r, resort) = SORTING_LIST.with(|| f(&self.map, &mut self.list));

        if resort {
            self.invalidate_sort();
        }

        r
    }

    /// Create a list chain node.
    ///
    /// See [`UiNode::chain`] for more details.
    pub fn chain(self, other: impl IntoUiNode) -> UiNode {
        self.into_node().chain(other)
    }
}
impl UiNodeImpl for SortingList {
    fn children_len(&self) -> usize {
        self.list.children_len()
    }

    fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
        self.with_map(|map, list| {
            if let Some(index) = map.get(index) {
                list.0.with_child(*index, visitor)
            }
        })
    }

    fn is_list(&self) -> bool {
        true
    }

    fn for_each_child(&mut self, visitor: &mut dyn FnMut(usize, &mut UiNode)) {
        self.with_map(|map, list| {
            for (i, &actual_i) in map.iter().enumerate() {
                list.with_child(actual_i, |n| visitor(i, n));
            }
        })
    }

    fn try_for_each_child(
        &mut self,
        visitor: &mut dyn FnMut(usize, &mut UiNode) -> ControlFlow<BoxAnyVarValue>,
    ) -> ControlFlow<BoxAnyVarValue> {
        self.with_map(|map, list| {
            for (i, &actual_i) in map.iter().enumerate() {
                let mut cf = ControlFlow::Continue(());
                list.with_child(actual_i, |n| cf = visitor(i, n));
                cf?;
            }
            ControlFlow::Continue(())
        })
    }

    fn par_each_child(&mut self, visitor: &(dyn Fn(usize, &mut UiNode) + Sync)) {
        self.for_each_child(&mut |i, n| visitor(i, n));
    }

    fn par_fold_reduce(
        &mut self,
        identity: BoxAnyVarValue,
        fold: &(dyn Fn(BoxAnyVarValue, usize, &mut UiNode) -> BoxAnyVarValue + Sync),
        _: &(dyn Fn(BoxAnyVarValue, BoxAnyVarValue) -> BoxAnyVarValue + Sync),
    ) -> BoxAnyVarValue {
        let mut acc = Some(identity);
        self.for_each_child(&mut |i, n| {
            acc = Some(fold(acc.take().unwrap(), i, n));
        });
        acc.unwrap()
    }

    fn init(&mut self) {
        let _ = SORTING_LIST.with(|| self.list.0.init());
        self.invalidate_sort();
    }

    fn deinit(&mut self) {
        let _ = SORTING_LIST.with(|| self.list.0.deinit());
        self.invalidate_sort();
    }

    fn info(&mut self, info: &mut WidgetInfoBuilder) {
        self.list.0.info(info);
    }

    fn event(&mut self, update: &EventUpdate) {
        self.list.0.event(update);
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        self.list.0.update(updates);
    }

    fn update_list(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        let mut changed = false;
        let (_, resort) = SORTING_LIST.with(|| self.list.0.update_list(updates, &mut changed));
        if changed || resort {
            self.invalidate_sort();
            observer.reset();
        }
    }

    fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
        self.list.0.measure(wm)
    }

    fn measure_list(
        &mut self,
        wm: &mut WidgetMeasure,
        measure: &(dyn Fn(usize, &mut UiNode, &mut WidgetMeasure) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        let mut acc = PxSize::zero();
        self.for_each_child(&mut |i, n| {
            let s = measure(i, n, wm);
            acc = fold_size(acc, s);
        });
        acc
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        self.list.0.layout(wl)
    }

    fn layout_list(
        &mut self,
        wl: &mut WidgetLayout,
        layout: &(dyn Fn(usize, &mut UiNode, &mut WidgetLayout) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        let mut acc = PxSize::zero();
        self.for_each_child(&mut |i, n| {
            let s = layout(i, n, wl);
            acc = fold_size(acc, s);
        });
        acc
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        self.for_each_child(&mut |_, n| n.render(frame));
    }

    fn render_list(&mut self, frame: &mut FrameBuilder, render: &(dyn Fn(usize, &mut UiNode, &mut FrameBuilder) + Sync)) {
        self.for_each_child(&mut |i, n| render(i, n, frame));
    }

    fn render_update(&mut self, update: &mut FrameUpdate) {
        self.list.0.render_update(update);
    }

    fn render_update_list(&mut self, update: &mut FrameUpdate, render_update: &(dyn Fn(usize, &mut UiNode, &mut FrameUpdate) + Sync)) {
        self.list.0.render_update_list(update, render_update);
    }

    fn as_widget(&mut self) -> Option<&mut dyn WidgetUiNodeImpl> {
        None
    }
}

/// Represents an [`UiNode::update_list`] observer that can be used to monitor widget insertion, removal and re-order.
///
/// All indexes are in the context of the previous changes, if you are maintaining a *mirror* vector simply using the
/// [`Vec::insert`] and [`Vec::remove`] commands in the same order as they are received should keep the vector in sync.
///
/// This trait is implemented for `()`, to not observe simply pass on a `&mut ()`.
///
/// This trait is implemented for [`bool`], if any change happens the flag is set to `true`.
pub trait UiNodeListObserver {
    /// Called when a node is inserted at `index`.
    fn inserted(&mut self, index: usize);
    /// Called when a node is removed from `index`.
    fn removed(&mut self, index: usize);
    /// Called when a node is removed from `removed_index` and re-inserted at `inserted_index`.
    fn moved(&mut self, removed_index: usize, inserted_index: usize);
    /// Called when large or unspecified changes happen to the list.
    fn reset(&mut self);

    /// Returns true if this observer does not use the item indexes.
    ///
    /// When true you can use [`reset`] to notify any changes.
    ///
    /// This flag can be used by list implementers to enable parallel processing in more contexts, for example, chain lists cannot
    /// parallelize because indexes of subsequent lists are dependent on indexes of previous lists, but if the observer only needs
    /// to known that some change happened the chain list can still parallelize.
    ///
    /// [`reset`]: Self::reset
    fn is_reset_only(&self) -> bool;
}
/// Does nothing.
impl UiNodeListObserver for () {
    fn is_reset_only(&self) -> bool {
        true
    }

    fn reset(&mut self) {}

    fn inserted(&mut self, _: usize) {}

    fn removed(&mut self, _: usize) {}

    fn moved(&mut self, _: usize, _: usize) {}
}
/// Sets to `true` for any change.
impl UiNodeListObserver for bool {
    fn is_reset_only(&self) -> bool {
        true
    }

    fn reset(&mut self) {
        *self = true;
    }

    fn inserted(&mut self, _: usize) {
        *self = true;
    }

    fn removed(&mut self, _: usize) {
        *self = true;
    }

    fn moved(&mut self, _: usize, _: usize) {
        *self = true;
    }
}

/// Represents an [`UiNodeListObserver`] that applies an offset to all indexes.
///
/// This type is useful for implementing node lists that are composed of other lists.
pub struct OffsetUiListObserver<'o>(pub usize, pub &'o mut dyn UiNodeListObserver);
impl UiNodeListObserver for OffsetUiListObserver<'_> {
    fn is_reset_only(&self) -> bool {
        self.1.is_reset_only()
    }

    fn reset(&mut self) {
        self.1.reset()
    }

    fn inserted(&mut self, index: usize) {
        self.1.inserted(index + self.0)
    }

    fn removed(&mut self, index: usize) {
        self.1.removed(index + self.0)
    }

    fn moved(&mut self, removed_index: usize, inserted_index: usize) {
        self.1.moved(removed_index + self.0, inserted_index + self.0)
    }
}

impl UiNodeListObserver for (&mut dyn UiNodeListObserver, &mut dyn UiNodeListObserver) {
    fn is_reset_only(&self) -> bool {
        self.0.is_reset_only() && self.1.is_reset_only()
    }

    fn reset(&mut self) {
        self.0.reset();
        self.1.reset();
    }

    fn inserted(&mut self, index: usize) {
        self.0.inserted(index);
        self.1.inserted(index);
    }

    fn removed(&mut self, index: usize) {
        self.0.removed(index);
        self.1.removed(index);
    }

    fn moved(&mut self, removed_index: usize, inserted_index: usize) {
        self.0.moved(removed_index, inserted_index);
        self.1.moved(removed_index, inserted_index);
    }
}

/// Represents an [`UiVec`] that can be modified using a connected sender.
pub struct EditableUiVec {
    vec: UiVec,
    ctrl: EditableUiVecRef,
}
impl Default for EditableUiVec {
    fn default() -> Self {
        Self {
            vec: ui_vec![],
            ctrl: EditableUiVecRef::new(true),
        }
    }
}
impl Drop for EditableUiVec {
    fn drop(&mut self) {
        self.ctrl.0.lock().alive = false;
    }
}
impl EditableUiVec {
    /// New default empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// New from an already allocated vec.
    pub fn from_vec(vec: impl Into<UiVec>) -> Self {
        let mut s = Self::new();
        s.vec = vec.into();
        s
    }

    /// Create a sender that can edit this list.
    pub fn reference(&self) -> EditableUiVecRef {
        self.ctrl.clone()
    }

    /// Create a list chain node.
    ///
    /// See [`UiNode::chain`] for more details.
    pub fn chain(self, other: impl IntoUiNode) -> UiNode {
        self.into_node().chain(other)
    }

    fn fulfill_requests(&mut self, observer: &mut dyn UiNodeListObserver) {
        if let Some(r) = self.ctrl.take_requests() {
            if r.clear {
                // if reset
                self.clear();
                observer.reset();

                for (i, mut wgt) in r.insert {
                    wgt.init();
                    WIDGET.update_info();
                    if i < self.len() {
                        self.insert(i, wgt);
                    } else {
                        self.push(wgt);
                    }
                }
                for mut wgt in r.push {
                    wgt.init();
                    WIDGET.update_info();
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

                        WIDGET.update_info();
                    }
                }
                for (id, to) in r.move_id {
                    if let Some(r) = self.vec.iter_mut().position(|n| n.as_widget().map(|mut w| w.id()) == Some(id)) {
                        let i = to(r, self.len());

                        if r != i {
                            let wgt = self.vec.remove(r);

                            if i < self.len() {
                                self.vec.insert(i, wgt);
                            } else {
                                self.vec.push(wgt);
                            }

                            WIDGET.update_info();
                        }
                    }
                }
            } else {
                let mut removed = false;
                for mut retain in r.retain {
                    let mut i = 0;
                    self.vec.retain_mut(|n| {
                        let r = retain(n);
                        if !r {
                            n.deinit();
                            removed = true;
                            observer.removed(i);
                        } else {
                            i += 1;
                        }
                        r
                    });
                }
                if removed {
                    WIDGET.update_info();
                }

                for (i, mut wgt) in r.insert {
                    wgt.init();
                    WIDGET.update_info();

                    if i < self.len() {
                        self.insert(i, wgt);
                        observer.inserted(i);
                    } else {
                        observer.inserted(self.len());
                        self.push(wgt);
                    }
                }

                for mut wgt in r.push {
                    wgt.init();
                    WIDGET.update_info();

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

                        WIDGET.update_info();
                    }
                }

                for (id, to) in r.move_id {
                    if let Some(r) = self.vec.iter_mut().position(|n| n.as_widget().map(|mut w| w.id()) == Some(id)) {
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

                            WIDGET.update_info();
                        }
                    }
                }
            }
        }
    }
}
impl ops::Deref for EditableUiVec {
    type Target = UiVec;

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}
impl ops::DerefMut for EditableUiVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec
    }
}
impl UiNodeImpl for EditableUiVec {
    fn children_len(&self) -> usize {
        self.vec.children_len()
    }

    fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
        self.vec.with_child(index, visitor);
    }

    fn is_list(&self) -> bool {
        true
    }

    fn for_each_child(&mut self, visitor: &mut dyn FnMut(usize, &mut UiNode)) {
        self.vec.for_each_child(visitor);
    }

    fn try_for_each_child(
        &mut self,
        visitor: &mut dyn FnMut(usize, &mut UiNode) -> ControlFlow<BoxAnyVarValue>,
    ) -> ControlFlow<BoxAnyVarValue> {
        self.vec.try_for_each_child(visitor)
    }

    fn par_each_child(&mut self, visitor: &(dyn Fn(usize, &mut UiNode) + Sync)) {
        self.vec.par_each_child(visitor);
    }

    fn par_fold_reduce(
        &mut self,
        identity: BoxAnyVarValue,
        fold: &(dyn Fn(BoxAnyVarValue, usize, &mut UiNode) -> BoxAnyVarValue + Sync),
        reduce: &(dyn Fn(BoxAnyVarValue, BoxAnyVarValue) -> BoxAnyVarValue + Sync),
    ) -> BoxAnyVarValue {
        self.vec.par_fold_reduce(identity, fold, reduce)
    }

    fn init(&mut self) {
        self.ctrl.0.lock().target = Some(WIDGET.id());
        self.vec.init();
    }

    fn deinit(&mut self) {
        self.ctrl.0.lock().target = None;
        self.vec.deinit();
    }

    fn info(&mut self, info: &mut WidgetInfoBuilder) {
        self.vec.info(info);
    }

    fn event(&mut self, update: &EventUpdate) {
        self.vec.event(update);
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        self.vec.update(updates);
        self.fulfill_requests(&mut ());
    }

    fn update_list(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        self.vec.update(updates);
        self.fulfill_requests(observer);
    }

    fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
        self.vec.measure(wm)
    }

    fn measure_list(
        &mut self,
        wm: &mut WidgetMeasure,
        measure: &(dyn Fn(usize, &mut UiNode, &mut WidgetMeasure) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        self.vec.measure_list(wm, measure, fold_size)
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        self.vec.layout(wl)
    }

    fn layout_list(
        &mut self,
        wl: &mut WidgetLayout,
        layout: &(dyn Fn(usize, &mut UiNode, &mut WidgetLayout) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        self.vec.layout_list(wl, layout, fold_size)
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        self.vec.render(frame);
    }

    fn render_list(&mut self, frame: &mut FrameBuilder, render: &(dyn Fn(usize, &mut UiNode, &mut FrameBuilder) + Sync)) {
        self.vec.render_list(frame, render);
    }

    fn render_update(&mut self, update: &mut FrameUpdate) {
        self.vec.render_update(update);
    }

    fn render_update_list(&mut self, update: &mut FrameUpdate, render_update: &(dyn Fn(usize, &mut UiNode, &mut FrameUpdate) + Sync)) {
        self.vec.render_update_list(update, render_update);
    }

    fn as_widget(&mut self) -> Option<&mut dyn WidgetUiNodeImpl> {
        None
    }
}

/// See [`EditableUiVecRef::move_to`] for more details
type NodeMoveToFn = fn(usize, usize) -> usize;

/// Represents a sender to an [`EditableUiVec`].
#[derive(Clone, Debug)]
pub struct EditableUiVecRef(Arc<Mutex<EditRequests>>);
struct EditRequests {
    target: Option<WidgetId>,
    insert: Vec<(usize, UiNode)>,
    push: Vec<UiNode>,
    retain: Vec<Box<dyn FnMut(&mut UiNode) -> bool + Send>>,
    move_index: Vec<(usize, usize)>,
    move_id: Vec<(WidgetId, NodeMoveToFn)>,
    clear: bool,

    alive: bool,
}
impl fmt::Debug for EditRequests {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EditRequests")
            .field("target", &self.target)
            .field("insert.len", &self.insert.len())
            .field("push.len", &self.push.len())
            .field("retain.len", &self.retain.len())
            .field("move_index", &self.move_index)
            .field("move_id", &self.move_id)
            .field("clear", &self.clear)
            .field("alive", &self.alive)
            .finish()
    }
}
impl EditableUiVecRef {
    fn new(alive: bool) -> Self {
        Self(Arc::new(Mutex::new(EditRequests {
            target: None,
            insert: vec![],
            push: vec![],
            retain: vec![],
            move_index: vec![],
            move_id: vec![],
            clear: false,
            alive,
        })))
    }

    /// New reference to no list.
    ///
    /// [`alive`] is always false for the returned list.
    ///
    /// [`alive`]: Self::alive
    pub fn dummy() -> Self {
        Self::new(false)
    }

    /// Returns `true` if the [`EditableUiVec`] still exists.
    pub fn alive(&self) -> bool {
        self.0.lock().alive
    }

    /// Request an update for the insertion of the `widget`.
    ///
    /// The `index` is resolved after all [`remove`] requests, if it is out-of-bounds the widget is pushed.
    ///
    /// The `widget` will be inserted, inited and the info tree updated.
    ///
    /// [`remove`]: Self::remove
    pub fn insert(&self, index: usize, widget: impl IntoUiNode) {
        self.insert_impl(index, widget.into_node());
    }
    fn insert_impl(&self, index: usize, widget: UiNode) {
        let mut s = self.0.lock();
        if !s.alive {
            return;
        }
        s.insert.push((index, widget));
        UPDATES.update(s.target);
    }

    /// Request an update for the insertion of the `widget` at the end of the list.
    ///
    /// The widget will be pushed after all [`insert`] requests.
    ///
    /// The `widget` will be inserted, inited and the info tree updated.
    ///
    /// [`insert`]: Self::insert
    pub fn push(&self, widget: impl IntoUiNode) {
        self.push_impl(widget.into_node());
    }
    fn push_impl(&self, widget: UiNode) {
        let mut s = self.0.lock();
        if !s.alive {
            return;
        }
        s.push.push(widget);
        UPDATES.update(s.target);
    }

    /// Request an update for the removal of the widget identified by `id`.
    ///
    /// The widget will be deinited, dropped and the info tree will update. Nothing happens
    /// if the widget is not found.
    pub fn remove(&self, id: impl Into<WidgetId>) {
        fn rmv_retain(id: WidgetId) -> impl FnMut(&mut UiNode) -> bool + Send + 'static {
            move |node| {
                match node.as_widget() {
                    Some(mut wgt) => wgt.id() != id,
                    None => true, // retain
                }
            }
        }
        self.retain(rmv_retain(id.into()))
    }

    /// Request a filtered mass removal of nodes in the list.
    ///
    /// Each node not retained will be deinited, dropped and the info tree will update if any was removed.
    ///
    /// Note that the `predicate` may be called on the same node multiple times or called in any order.
    pub fn retain(&self, predicate: impl FnMut(&mut UiNode) -> bool + Send + 'static) {
        let mut s = self.0.lock();
        if !s.alive {
            return;
        }
        s.retain.push(Box::new(predicate));
        UPDATES.update(s.target);
    }

    /// Request a widget remove and re-insert.
    ///
    /// If the `remove_index` is out of bounds nothing happens, if the `insert_index` is out-of-bounds
    /// the widget is pushed to the end of the vector, if `remove_index` and `insert_index` are equal nothing happens.
    ///
    /// Move requests happen after all other requests.
    pub fn move_index(&self, remove_index: usize, insert_index: usize) {
        if remove_index != insert_index {
            let mut s = self.0.lock();
            if !s.alive {
                return;
            }
            s.move_index.push((remove_index, insert_index));
            UPDATES.update(s.target);
        }
    }

    /// Request a widget move, the widget is searched by `id`, if found `get_move_to` id called with the index of the widget and length
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
    /// # fn demo(items: zng_app::widget::node::EditableUiVecRef) {
    /// items.move_id("my-widget", |i, _len| i.saturating_sub(1));
    /// # }
    /// ```
    ///
    /// And to move *down* stopping at the bottom:
    ///
    /// ```
    /// # fn demo(items: zng_app::widget::node::EditableUiVecRef) {
    /// items.move_id("my-widget", |i, _len| i.saturating_add(1));
    /// # }
    /// ```
    ///
    /// Note that if the returned index overflows the length the widget is
    /// pushed as the last item.
    ///
    /// The length can be used for implementing wrapping move *down*:
    ///
    /// ```
    /// # fn demo(items: zng_app::widget::node::EditableUiVecRef) {
    /// items.move_id("my-widget", |i, len| {
    ///     let next = i + 1;
    ///     if next < len { next } else { 0 }
    /// });
    /// # }
    /// ```
    pub fn move_id(&self, id: impl Into<WidgetId>, get_move_to: NodeMoveToFn) {
        let mut s = self.0.lock();
        if !s.alive {
            return;
        }
        s.move_id.push((id.into(), get_move_to));
        UPDATES.update(s.target);
    }

    /// Request a removal of all current widgets.
    ///
    /// All other requests will happen after the clear.
    pub fn clear(&self) {
        let mut s = self.0.lock();
        s.clear = true;
        UPDATES.update(s.target);
    }

    fn take_requests(&self) -> Option<EditRequests> {
        let mut s = self.0.lock();

        if s.clear
            || !s.insert.is_empty()
            || !s.push.is_empty()
            || !s.retain.is_empty()
            || !s.move_index.is_empty()
            || !s.move_id.is_empty()
        {
            let empty = EditRequests {
                target: s.target,
                alive: s.alive,

                insert: vec![],
                push: vec![],
                retain: vec![],
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

static_id! {
    static ref Z_INDEX_ID: StateId<ZIndex>;
}

/// Position of a widget inside an UI node list render operation.
///
/// When two widgets have the same index their logical position defines the render order.
///
/// # Examples
///
/// Create a Z-index that causes the widget to render in front of all siblings that don't set Z-index.
///
/// ```
/// # use zng_app::widget::node::ZIndex;
/// #
/// let highlight_z = ZIndex::DEFAULT + 1;
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Transitionable)]
pub struct ZIndex(u32);
impl ZIndex {
    /// Widget is rendered first causing all overlapping siblings to render on top of it.
    ///
    /// The value is `0`.
    pub const BACK: ZIndex = ZIndex(0);

    /// Z-index of widgets that don't set the index.
    ///
    /// The value is `u32::MAX / 2`.
    pub const DEFAULT: ZIndex = ZIndex(u32::MAX / 2);

    /// Widget is rendered after all siblings causing it to render on top.
    pub const FRONT: ZIndex = ZIndex(u32::MAX);

    /// Computes `other` above `self`, caps at [`FRONT`].
    ///
    /// This is the default ZIndex addition, equivalent to `self + other`.
    ///
    /// [`FRONT`]: Self::FRONT
    pub fn saturating_add(self, other: impl Into<Self>) -> Self {
        ZIndex(self.0.saturating_add(other.into().0))
    }

    /// Computes `other` below `self`, stops at [`BACK`].
    ///
    /// This is the default ZIndex subtraction, equivalent to `self - other`.
    ///
    /// [`BACK`]: Self::BACK
    pub fn saturating_sub(self, other: impl Into<Self>) -> Self {
        ZIndex(self.0.saturating_sub(other.into().0))
    }
}
impl Default for ZIndex {
    fn default() -> Self {
        ZIndex::DEFAULT
    }
}
impl<Z: Into<ZIndex>> ops::Add<Z> for ZIndex {
    type Output = Self;

    fn add(self, rhs: Z) -> Self::Output {
        self.saturating_add(rhs)
    }
}
impl<Z: Into<ZIndex>> ops::AddAssign<Z> for ZIndex {
    fn add_assign(&mut self, rhs: Z) {
        *self = *self + rhs;
    }
}
impl<Z: Into<ZIndex>> ops::Sub<Z> for ZIndex {
    type Output = Self;

    fn sub(self, rhs: Z) -> Self::Output {
        self.saturating_sub(rhs)
    }
}
impl<Z: Into<ZIndex>> ops::SubAssign<Z> for ZIndex {
    fn sub_assign(&mut self, rhs: Z) {
        *self = *self - rhs;
    }
}
impl ops::Mul<Factor> for ZIndex {
    type Output = Self;

    fn mul(self, rhs: Factor) -> Self::Output {
        ZIndex(self.0 * rhs)
    }
}
impl ops::Div<Factor> for ZIndex {
    type Output = Self;

    fn div(self, rhs: Factor) -> Self::Output {
        ZIndex(self.0 / rhs)
    }
}
impl ops::MulAssign<Factor> for ZIndex {
    fn mul_assign(&mut self, rhs: Factor) {
        self.0 *= rhs;
    }
}
impl ops::DivAssign<Factor> for ZIndex {
    fn div_assign(&mut self, rhs: Factor) {
        self.0 /= rhs;
    }
}
impl fmt::Debug for ZIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let z = *self;
        if f.alternate() {
            write!(f, "ZIndex::")?;
        }

        if z == Self::DEFAULT {
            write!(f, "DEFAULT")
        } else if z == Self::BACK {
            write!(f, "BACK")
        } else if z == Self::FRONT {
            write!(f, "FRONT")
        } else if z > Self::DEFAULT {
            if z > Self::FRONT - 10000 {
                write!(f, "FRONT-{}", Self::FRONT.0 - z.0)
            } else {
                write!(f, "DEFAULT+{}", z.0 - Self::DEFAULT.0)
            }
        } else if z < Self::BACK + 10000 {
            write!(f, "BACK+{}", z.0 - Self::BACK.0)
        } else {
            write!(f, "DEFAULT-{}", Self::DEFAULT.0 - z.0)
        }
    }
}
impl_from_and_into_var! {
    fn from(index: u32) -> ZIndex {
        ZIndex(index)
    }
    fn from(index: ZIndex) -> u32 {
        index.0
    }
    fn from(index: ZIndex) -> Option<ZIndex>;
}
#[derive(Default, Debug)]
struct ZIndexCtx {
    // used in `z_index` to validate that it will have an effect.
    panel_id: Option<WidgetId>,
    // set by `z_index` to signal a z-resort is needed.
    resort: AtomicBool,
}
context_local! {
    static Z_INDEX_CTX: ZIndexCtx = ZIndexCtx::default();
}
/// Access to widget z-index in a parent [`PanelList`].
#[expect(non_camel_case_types)]
pub struct Z_INDEX;
impl Z_INDEX {
    fn with(&self, panel_id: WidgetId, action: impl FnOnce()) -> bool {
        let ctx = ZIndexCtx {
            panel_id: Some(panel_id),
            resort: AtomicBool::new(false),
        };
        Z_INDEX_CTX.with_context(&mut Some(Arc::new(ctx)), || {
            action();
            Z_INDEX_CTX.get().resort.load(Relaxed)
        })
    }

    /// Gets the index set on the [`WIDGET`].
    ///
    /// Returns `DEFAULT` if the node is not a widget.
    pub fn get(&self) -> ZIndex {
        WIDGET.get_state(*Z_INDEX_ID).unwrap_or_default()
    }

    /// Gets the index set on the `widget`.
    ///
    /// Returns `DEFAULT` if the node is not a widget.
    pub fn get_wgt(&self, widget: &mut UiNode) -> ZIndex {
        match widget.as_widget() {
            Some(mut w) => w.with_context(WidgetUpdateMode::Ignore, || self.get()),
            None => ZIndex::DEFAULT,
        }
    }

    /// Try set the z-index in the current [`WIDGET`].
    ///
    /// Returns if z-index can be set on the widget, this is only `true` if the current [`WIDGET`]
    /// is a direct child of a panel widget that supports z-index.
    ///
    /// This must be called on node init and update only, always returns `false` if called during
    /// other node operations.
    pub fn set(&self, index: ZIndex) -> bool {
        let z_ctx = Z_INDEX_CTX.get();
        let valid = z_ctx.panel_id == WIDGET.parent_id() && z_ctx.panel_id.is_some();
        if valid {
            z_ctx.resort.store(true, Relaxed);
            WIDGET.set_state(*Z_INDEX_ID, index);
        }
        valid
    }
}

/// First and last child widget in a [`PanelList`].
#[derive(Debug, Clone)]
pub struct PanelListRange {
    // none is empty
    range: Option<(WidgetId, WidgetId)>,
    version: u8,
}
impl PanelListRange {
    /// Gets the panel children if it may have changed since `last_version`.
    ///
    /// The [`PanelList`] requests an update for each child after info rebuild if it has changed,
    /// the item properties can used this method on update to react.
    pub fn update(
        parent: &WidgetInfo,
        panel_id: impl Into<StateId<Self>>,
        last_version: &mut Option<u8>,
    ) -> Option<crate::widget::info::iter::Children> {
        let range = parent.meta().get_clone(panel_id);
        if let Some(Self { range, version }) = range {
            let version = Some(version);
            if *last_version != version {
                *last_version = version;

                if let Some((s, e)) = range {
                    let tree = parent.tree();
                    if let Some(s) = tree.get(s)
                        && let Some(e) = tree.get(e)
                    {
                        let parent = Some(parent);
                        if s.parent().as_ref() == parent && e.parent().as_ref() == parent {
                            return Some(crate::widget::info::iter::Children::new_range(s, e));
                        }
                    }
                }
            }
        }
        None
    }

    /// Gets the panel children if the `parent` contains the `panel_id`.
    pub fn get(parent: &WidgetInfo, panel_id: impl Into<StateId<Self>>) -> Option<crate::widget::info::iter::Children> {
        let range = parent.meta().get_clone(panel_id);
        if let Some(Self { range: Some((s, e)), .. }) = range {
            let tree = parent.tree();
            if let Some(s) = tree.get(s)
                && let Some(e) = tree.get(e)
            {
                let parent = Some(parent);
                if s.parent().as_ref() == parent && e.parent().as_ref() == parent {
                    return Some(crate::widget::info::iter::Children::new_range(s, e));
                }
            }
        }
        None
    }
}

/// Represents the final UI list in a panel layout node.
///
/// Panel widgets should wrap their children node on this type to support Z-index sorting and to easily track associated
/// item data.
///
/// By default the associated item data is a [`DefaultPanelListData`] that represents the offset of each item inside the panel,
/// but it can be any type that implements [`PanelListData`]. The panel list default render implementation uses this data
/// to position the children widgets. Note that you must [`commit_data`] changes to this data at the end of a layout pass.
///
/// Panel widgets can also mark the list using [`track_info_range`] to implement getter properties such as `is_odd` or
/// `is_even`.
///
/// [`track_info_range`]: Self::track_info_range
/// [`commit_data`]: Self::commit_data
pub struct PanelList<D = DefaultPanelListData>
where
    D: PanelListData,
{
    list: UiNode,
    data: Vec<Mutex<D>>, // Mutex to implement `par_each_mut`.

    offset_key: FrameValueKey<PxTransform>,
    info_id: Option<(StateId<PanelListRange>, u8, bool)>,

    z_map: Vec<u64>,
    z_naturally_sorted: bool,
}
impl PanelList<DefaultPanelListData> {
    /// New from `list` and default data.
    pub fn new(list: impl IntoUiNode) -> Self {
        Self::new_custom(list)
    }
}

impl<D> PanelList<D>
where
    D: PanelListData,
{
    /// New from `list` and custom data type.
    pub fn new_custom(list: impl IntoUiNode) -> Self {
        Self::new_custom_impl(list.into_node())
    }
    fn new_custom_impl(list: UiNode) -> Self {
        let list = list.into_list();
        Self {
            data: {
                let mut d = vec![];
                d.resize_with(list.children_len(), Default::default);
                d
            },
            list,
            offset_key: FrameValueKey::new_unique(),
            info_id: None,
            z_map: vec![],
            z_naturally_sorted: false,
        }
    }

    /// Enable tracking the first and last child in the parent widget info.
    ///
    /// The info is set in the `info_id`, it can be used to identify the children widgets
    /// that are the panel children as the info tree may track extra widgets as children
    /// when they are set by other properties, like background.
    pub fn track_info_range(mut self, info_id: impl Into<StateId<PanelListRange>>) -> Self {
        self.info_id = Some((info_id.into(), 0, true));
        self
    }

    /// Into list and associated data.
    pub fn into_parts(self) -> (UiNode, Vec<Mutex<D>>, FrameValueKey<PxTransform>, Option<StateId<PanelListRange>>) {
        (self.list, self.data, self.offset_key, self.info_id.map(|t| t.0))
    }

    /// New from list and associated data.
    ///
    /// # Panics
    ///
    /// Panics if the `list` and `data` don't have the same length.
    pub fn from_parts(
        list: UiNode,
        data: Vec<Mutex<D>>,
        offset_key: FrameValueKey<PxTransform>,
        info_id: Option<StateId<PanelListRange>>,
    ) -> Self {
        assert!(list.is_list());
        assert_eq!(list.children_len(), data.len());
        Self {
            list,
            data,
            offset_key,
            info_id: info_id.map(|i| (i, 0, true)),
            z_map: vec![],
            z_naturally_sorted: false,
        }
    }

    /// Gets the ID set on the parent widget info if [`track_info_range`] was enabled.
    ///
    /// [`track_info_range`]: Self::track_info_range
    pub fn info_id(&self) -> Option<StateId<PanelListRange>> {
        self.info_id.as_ref().map(|t| t.0)
    }

    /// Call `visitor` with a exclusive reference to the child node and associated data identified by `index`.
    ///
    /// Panics if the `index` is out of bounds.
    pub fn with_child<R>(&mut self, index: usize, visitor: impl FnOnce(&mut UiNode, &mut D) -> R) -> R {
        let data = self.data[index].get_mut();
        self.list.with_child(index, |u| visitor(u, data))
    }

    /// Call `visitor` for each child node of `self`, one at a time.
    ///
    /// The closure parameters are the child index, the child and the associated data.
    pub fn for_each_child(&mut self, mut visitor: impl FnMut(usize, &mut UiNode, &mut D)) {
        let data = &mut self.data;
        self.list.for_each_child(|i, u| visitor(i, u, data[i].get_mut()));
    }

    /// Call `visitor` for each child node of `self`, one at a time, with control flow.
    ///
    /// The closure parameters are the child index, the child and the associated data.
    pub fn try_for_each_child<B: zng_var::VarValue>(
        &mut self,
        visitor: &mut dyn FnMut(usize, &mut UiNode, &mut D) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        let data = &mut self.data;
        self.list.try_for_each_child(|i, u| visitor(i, u, data[i].get_mut()))
    }

    /// Calls `visitor` for each child node in parallel.
    ///
    /// The closure parameters are the child index, the child and the associated data.
    pub fn par_each_child(&mut self, visitor: impl Fn(usize, &mut UiNode, &mut D) + Sync)
    where
        D: Sync,
    {
        let data = &self.data;
        self.list.par_each_child(|i, u| {
            visitor(
                i,
                u,
                &mut *data[i].try_lock().expect("par_each_child called visitor twice on same index"),
            )
        });
    }

    /// Calls `fold` for each child node with associated data in parallel, with fold accumulators produced by cloning
    /// `identity`, then merges the folded results using `reduce` to produce the final value also in parallel.
    ///
    /// The `reduce` call is [associative], the order is preserved in the result.
    ///
    /// [associative]: https://en.wikipedia.org/wiki/Associative_property
    pub fn par_fold_reduce<T>(
        &mut self,
        identity: T,
        fold: impl Fn(T, usize, &mut UiNode, &mut D) -> T + Sync,
        reduce: impl Fn(T, T) -> T + Send + Sync,
    ) -> T
    where
        T: zng_var::VarValue,
    {
        let data = &self.data;
        self.list.par_fold_reduce(
            identity,
            |acc, i, n| {
                fold(
                    acc,
                    i,
                    n,
                    &mut *data[i].try_lock().expect("par_fold_reduce called visitor twice on same index"),
                )
            },
            reduce,
        )
    }

    /// Call `measure` for each node with associated data and combines the final size using `fold_size`.
    pub fn measure_list(
        &mut self,
        wm: &mut WidgetMeasure,
        measure: impl Fn(usize, &mut UiNode, &mut D, &mut WidgetMeasure) -> PxSize + Sync,
        fold_size: impl Fn(PxSize, PxSize) -> PxSize + Sync,
    ) -> PxSize {
        let data = &self.data;
        self.list.measure_list(
            wm,
            |i, n, wm| {
                measure(
                    i,
                    n,
                    &mut *data[i].try_lock().expect("measure_list called visitor twice on same index"),
                    wm,
                )
            },
            fold_size,
        )
    }

    /// Call `layout` for each node with associated data and combines the final size using `fold_size`.
    pub fn layout_list(
        &mut self,
        wl: &mut WidgetLayout,
        layout: impl Fn(usize, &mut UiNode, &mut D, &mut WidgetLayout) -> PxSize + Sync,
        fold_size: impl Fn(PxSize, PxSize) -> PxSize + Sync,
    ) -> PxSize {
        let data = &self.data;
        self.list.layout_list(
            wl,
            |i, n, wl| {
                layout(
                    i,
                    n,
                    &mut *data[i].try_lock().expect("layout_list called visitor twice on same index"),
                    wl,
                )
            },
            fold_size,
        )
    }

    /// Call `render` for each node with associated data.
    ///
    /// Note that the [`PanelListData`] child offset and reference frame are already pushed when `render` is called.
    pub fn render_list(&mut self, frame: &mut FrameBuilder, render: impl Fn(usize, &mut UiNode, &mut D, &mut FrameBuilder) + Sync) {
        let offset_key = self.offset_key;

        if self.z_naturally_sorted {
            let data = &self.data;
            self.list.render_list(frame, |i, child, frame| {
                let mut data = data[i].try_lock().expect("render_list called visitor twice on same index");
                let offset = data.child_offset();
                if data.define_reference_frame() {
                    frame.push_reference_frame(
                        (offset_key, i as u32).into(),
                        offset_key.bind_child(i as u32, offset.into(), false),
                        true,
                        true,
                        |frame| render(i, child, &mut *data, frame),
                    );
                } else {
                    frame.push_child(offset, |frame| render(i, child, &mut *data, frame));
                }
            });
        } else {
            self.for_each_z_sorted(|i, child, data| {
                let offset = data.child_offset();
                if data.define_reference_frame() {
                    frame.push_reference_frame(
                        (offset_key, i as u32).into(),
                        offset_key.bind_child(i as u32, offset.into(), false),
                        true,
                        true,
                        |frame| render(i, child, data, frame),
                    );
                } else {
                    frame.push_child(offset, |frame| render(i, child, data, frame));
                }
            });
        }
    }

    /// Call `render_update` for each node with associated data.
    ///
    /// Note that the [`PanelListData`] child offset and reference frame are already pushed when `render_update` is called.
    pub fn render_update_list(
        &mut self,
        update: &mut FrameUpdate,
        render_update: impl Fn(usize, &mut UiNode, &mut D, &mut FrameUpdate) + Sync,
    ) {
        let offset_key = self.offset_key;
        let data = &self.data;
        self.list.render_update_list(update, |i, n, update| {
            let mut data = data[i].try_lock().expect("render_update_list called visitor twice on same index");

            let offset = data.child_offset();
            if data.define_reference_frame() {
                update.with_transform(offset_key.update_child(i as u32, offset.into(), false), true, |update| {
                    render_update(i, n, &mut *data, update);
                });
            } else {
                update.with_child(offset, |update| {
                    render_update(i, n, &mut *data, update);
                });
            }
        });
    }

    /// Iterate over the list in the Z order.
    pub fn for_each_z_sorted(&mut self, mut visitor: impl FnMut(usize, &mut UiNode, &mut D)) {
        if self.z_naturally_sorted {
            self.for_each_child(visitor)
        } else {
            if self.z_map.len() != self.list.children_len() {
                self.z_sort();
            }

            if self.z_naturally_sorted {
                self.for_each_child(visitor);
            } else {
                for &index in self.z_map.iter() {
                    let index = index as usize;
                    let data = self.data[index].get_mut();
                    self.list.with_child(index, |node| visitor(index, node, data));
                }
            }
        }
    }

    fn z_sort(&mut self) {
        // We pack *z* and *i* as u32s in one u64 then create the sorted lookup table if
        // observed `[I].Z < [I-1].Z`, also records if any `Z != DEFAULT`:
        //
        // Advantages:
        //
        // - Makes `sort_unstable` stable.
        // - Only one alloc needed, just mask out Z after sorting.
        //
        // Disadvantages:
        //
        // - Only supports u32::MAX widgets.
        // - Uses 64-bit indexes in 32-bit builds.

        let len = self.list.children_len();
        assert!(len <= u32::MAX as usize);

        let mut prev_z = ZIndex::BACK;
        let mut need_map = false;
        let mut z_and_i = Vec::with_capacity(len);
        let mut has_non_default_zs = false;

        self.list.for_each_child(|i, node| {
            let z = Z_INDEX.get_wgt(node);
            z_and_i.push(((z.0 as u64) << 32) | i as u64);

            need_map |= z < prev_z;
            has_non_default_zs |= z != ZIndex::DEFAULT;
            prev_z = z;
        });

        self.z_naturally_sorted = !need_map;

        if need_map {
            z_and_i.sort_unstable();

            for z in &mut z_and_i {
                *z &= u32::MAX as u64;
            }

            self.z_map = z_and_i;
        } else {
            self.z_map.clear();
        }
    }

    /// Gets the `index` sorted in the `list`.
    pub fn z_map(&mut self, index: usize) -> usize {
        if self.z_naturally_sorted {
            return index;
        }

        if self.z_map.len() != self.list.children_len() {
            self.z_sort();
        }

        if self.z_naturally_sorted {
            return index;
        }

        self.z_map[index] as usize
    }

    /// Reference the associated data.
    pub fn data(&mut self, index: usize) -> &mut D {
        self.data[index].get_mut()
    }

    /// Calls [`commit`] for each child data, aggregate changes.
    ///
    /// This must be called after the last update to the children data in a layout pass. Note that
    /// you can call [`commit`] directly in a `for_each` iteration if that iteration is the
    /// last in the layout pass.
    ///
    /// [`commit`]: PanelListData::commit
    pub fn commit_data(&mut self) -> PanelListDataChanges {
        let mut changes = PanelListDataChanges::empty();
        for data in self.data.iter_mut() {
            changes |= data.get_mut().commit();
        }
        changes
    }

    /// Key used to define reference frames for each item.
    ///
    /// The default implementation of `render_all` uses this key and the item index.
    pub fn offset_key(&self) -> FrameValueKey<PxTransform> {
        self.offset_key
    }
}
impl<D> UiNodeImpl for PanelList<D>
where
    D: PanelListData,
{
    fn children_len(&self) -> usize {
        self.list.0.children_len()
    }

    fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
        self.list.0.with_child(index, visitor)
    }

    fn is_list(&self) -> bool {
        true
    }

    fn for_each_child(&mut self, visitor: &mut dyn FnMut(usize, &mut UiNode)) {
        self.list.0.for_each_child(visitor);
    }

    fn try_for_each_child(
        &mut self,
        visitor: &mut dyn FnMut(usize, &mut UiNode) -> ControlFlow<BoxAnyVarValue>,
    ) -> ControlFlow<BoxAnyVarValue> {
        self.list.0.try_for_each_child(visitor)
    }

    fn par_each_child(&mut self, visitor: &(dyn Fn(usize, &mut UiNode) + Sync)) {
        self.list.0.par_each_child(visitor);
    }

    fn par_fold_reduce(
        &mut self,
        identity: BoxAnyVarValue,
        fold: &(dyn Fn(BoxAnyVarValue, usize, &mut UiNode) -> BoxAnyVarValue + Sync),
        reduce: &(dyn Fn(BoxAnyVarValue, BoxAnyVarValue) -> BoxAnyVarValue + Sync),
    ) -> BoxAnyVarValue {
        self.list.0.par_fold_reduce(identity, fold, reduce)
    }

    fn init(&mut self) {
        self.z_map.clear();
        let resort = Z_INDEX.with(WIDGET.id(), || self.list.0.init());
        self.z_naturally_sorted = !resort;
        self.data.resize_with(self.list.0.children_len(), Default::default);
    }

    fn deinit(&mut self) {
        self.list.deinit();
    }

    fn info(&mut self, info: &mut WidgetInfoBuilder) {
        let len = self.list.0.children_len();
        if len == 0 {
            return;
        }

        self.list.0.info(info);

        if let Some((id, version, pump_update)) = &mut self.info_id {
            let start = self.list.with_child(0, |c| c.as_widget().map(|mut w| w.id()));
            let end = self.list.with_child(len - 1, |c| c.as_widget().map(|mut w| w.id()));
            let range = match (start, end) {
                (Some(s), Some(e)) => Some((s, e)),
                _ => None,
            };
            info.set_meta(*id, PanelListRange { range, version: *version });

            if mem::take(pump_update) {
                self.list.for_each_child(|_, c| {
                    if let Some(mut w) = c.as_widget() {
                        w.with_context(WidgetUpdateMode::Bubble, || WIDGET.update());
                    }
                });
            }
        }
    }

    fn event(&mut self, update: &EventUpdate) {
        self.list.event(update);
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        self.update_list(updates, &mut ());
    }

    fn update_list(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        let mut observer = PanelObserver {
            changed: false,
            data: &mut self.data,
            observer,
        };
        let resort = Z_INDEX.with(WIDGET.id(), || self.list.update_list(updates, &mut observer));
        let observer_changed = observer.changed;
        if resort || (observer.changed && self.z_naturally_sorted) {
            self.z_map.clear();
            self.z_naturally_sorted = false;
            WIDGET.render();
        }
        self.data.resize_with(self.list.children_len(), Default::default);

        if observer_changed && let Some((_, v, u)) = &mut self.info_id {
            if !*u {
                *v = v.wrapping_add(1);
                *u = true;
            }
            WIDGET.info();
        }
    }

    fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
        self.list.measure(wm)
    }

    fn measure_list(
        &mut self,
        wm: &mut WidgetMeasure,
        measure: &(dyn Fn(usize, &mut UiNode, &mut WidgetMeasure) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        self.list.measure_list(wm, measure, fold_size)
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        self.list.layout(wl)
    }

    fn layout_list(
        &mut self,
        wl: &mut WidgetLayout,
        layout: &(dyn Fn(usize, &mut UiNode, &mut WidgetLayout) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        self.list.layout_list(wl, layout, fold_size)
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        self.render_list(frame, |_, n, _, frame| n.render(frame));
    }

    fn render_update(&mut self, update: &mut FrameUpdate) {
        self.render_update_list(update, |_, n, _, update| n.render_update(update));
    }

    fn as_widget(&mut self) -> Option<&mut dyn WidgetUiNodeImpl> {
        self.list.0.as_widget()
    }
}

bitflags::bitflags! {
    /// Identifies changes in [`PanelListData`] since last layout.
    #[must_use = "|= with other item changes, call request_render"]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    pub struct PanelListDataChanges: u8 {
        /// The [`PanelListData::child_offset`] changed since last layout.
        const CHILD_OFFSET = 0b01;
        /// The [`PanelListData::define_reference_frame`] changed since last layout.
        const DEFINE_REFERENCE_FRAME = 0b10;
    }
}
impl PanelListDataChanges {
    /// Request render or render update if there are any changes.
    pub fn request_render(self) {
        if self.contains(Self::DEFINE_REFERENCE_FRAME) {
            WIDGET.render();
        } else if self.contains(Self::CHILD_OFFSET) {
            WIDGET.render_update();
        }
    }
}

/// Default [`PanelList`] associated data.
#[derive(Clone, Debug, Default)]
pub struct DefaultPanelListData {
    /// Child offset to be used in the default `render_all` and `render_update_all` implementations.
    pub child_offset: PxVector,
    /// If a new reference frame should be created for the item during render.
    pub define_reference_frame: bool,

    prev_child_offset: PxVector,
    prev_define_reference_frame: bool,
}
impl PanelListData for DefaultPanelListData {
    fn child_offset(&self) -> PxVector {
        self.child_offset
    }

    fn define_reference_frame(&self) -> bool {
        self.define_reference_frame
    }

    fn commit(&mut self) -> PanelListDataChanges {
        let mut changes = PanelListDataChanges::empty();
        if self.define_reference_frame != self.prev_define_reference_frame {
            changes |= PanelListDataChanges::DEFINE_REFERENCE_FRAME;
        }
        if self.child_offset != self.prev_child_offset {
            changes |= PanelListDataChanges::CHILD_OFFSET;
        }
        self.prev_define_reference_frame = self.define_reference_frame;
        self.prev_child_offset = self.child_offset;
        changes
    }
}

/// Represents an item's associated data in a [`PanelList`].
pub trait PanelListData: Default + Send + Any {
    /// Gets the child offset to be used in the default `render_all` and `render_update_all` implementations.
    fn child_offset(&self) -> PxVector;

    /// If a new reference frame should be created for the item during render.
    fn define_reference_frame(&self) -> bool;

    /// Commit `child_offset` and `define_reference_frame` changes.
    ///
    /// Returns flags that indicate what values changed.
    fn commit(&mut self) -> PanelListDataChanges;
}
impl PanelListData for () {
    fn child_offset(&self) -> PxVector {
        PxVector::zero()
    }

    fn define_reference_frame(&self) -> bool {
        false
    }

    fn commit(&mut self) -> PanelListDataChanges {
        PanelListDataChanges::empty()
    }
}

struct PanelObserver<'d, D>
where
    D: PanelListData,
{
    changed: bool,
    data: &'d mut Vec<Mutex<D>>,
    observer: &'d mut dyn UiNodeListObserver,
}
impl<D> UiNodeListObserver for PanelObserver<'_, D>
where
    D: PanelListData,
{
    fn is_reset_only(&self) -> bool {
        false
    }

    fn reset(&mut self) {
        self.changed = true;
        self.data.clear();
        self.observer.reset();
    }

    fn inserted(&mut self, index: usize) {
        self.changed = true;
        self.data.insert(index, Default::default());
        self.observer.inserted(index);
    }

    fn removed(&mut self, index: usize) {
        self.changed = true;
        self.data.remove(index);
        self.observer.removed(index);
    }

    fn moved(&mut self, removed_index: usize, inserted_index: usize) {
        self.changed = true;
        let item = self.data.remove(removed_index);
        self.data.insert(inserted_index, item);
        self.observer.moved(removed_index, inserted_index);
    }
}
