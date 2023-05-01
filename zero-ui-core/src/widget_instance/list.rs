use std::{
    cmp::Ordering,
    mem, ops,
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc,
    },
};

use crate::{
    context_local, property,
    render::FrameValueKey,
    task::{self, ParallelIteratorExt},
    var::{IntoVar, Var},
};

use rayon::prelude::*;

use super::*;

/// Creates an [`UiNodeVec`] containing the arguments.
///  
/// Note that the items can be any type, `ui_vec!` automatically calls [`UiNode::boxed`] for each item.
///
/// # Examples
///
/// Create a vec containing a list of nodes/widgets:
///
/// ```
/// # use zero_ui_core::widget_instance::*;
/// # use zero_ui_core::widget_base::*;
/// # macro_rules! Text { ($($tt:tt)*) => { NilUiNode } }
/// let widgets = ui_vec![
///     Text!("Hello"),
///     Text!("World!")
/// ];
/// ```
///
/// Create a vec containing the node repeated **n** times:
///
/// ```
/// # use zero_ui_core::widget_instance::*;
/// # use zero_ui_core::widget_base::*;
/// # macro_rules! Text { ($($tt:tt)*) => { NilUiNode } }
/// let widgets = ui_vec![Text!(" . "); 10];
/// ```
///
/// Note that this is different from `vec![item; n]`, the node is not cloned, the expression is called **n** times to
/// generate the nodes.
#[macro_export]
macro_rules! ui_vec {
    () => { $crate::widget_instance::UiNodeVec::new() };
    ($node:expr; $n:expr) => {
        {
            let mut n: usize = $n;
            let mut vec = $crate::widget_instance::UiNodeVec::with_capacity(n);
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

// attribute to support `#[cfg(_)]` in items, Rust does not allow a match to `$(#[$meta:meta])* $node:expr`.
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
            result { $($r)* $crate::widget_instance::UiNode::boxed($node), }
        }
    };
    // match last node expr, no trailing comma
    (
        match { $node:expr }
        result { $($r:tt)* }
    ) => {
        $crate::ui_vec_items! {
            match { }
            result { $($r)* $crate::widget_instance::UiNode::boxed($node) }
        }
    };
    // finished
    (
        match { }
        result { $($r:tt)* }
    ) => {
        $crate::widget_instance::UiNodeVec(std::vec![
            $($r)*
        ])
    };
}

impl UiNodeList for Vec<BoxedUiNode> {
    fn with_node<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        f(&mut self[index])
    }

    fn for_each<F>(&mut self, mut f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode),
    {
        self.iter_mut().enumerate().for_each(|(i, n)| f(i, n))
    }

    fn par_each<F>(&mut self, f: F)
    where
        F: Fn(usize, &mut BoxedUiNode) + Send + Sync,
    {
        self.par_iter_mut().enumerate().with_ctx().for_each(|(i, n)| f(i, n));
    }

    fn par_fold_reduce<T, I, F, R>(&mut self, identity: I, fold: F, reduce: R) -> T
    where
        T: Send,
        I: Fn() -> T + Send + Sync,
        F: Fn(T, usize, &mut BoxedUiNode) -> T + Send + Sync,
        R: Fn(T, T) -> T + Send + Sync,
    {
        self.par_iter_mut()
            .enumerate()
            .with_ctx()
            .fold(&identity, move |a, (i, n)| fold(a, i, n))
            .reduce(&identity, reduce)
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn boxed(self) -> BoxedUiNodeList {
        Box::new(self)
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        vec.append(self)
    }
}

/// Vec of boxed UI nodes.
///
/// This is a thin wrapper around `Vec<BoxedUiNode>` that adds helper methods for pushing widgets without needing to box.
#[derive(Default)]
pub struct UiNodeVec(pub Vec<BoxedUiNode>);
impl UiNodeVec {
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
    pub fn push(&mut self, node: impl UiNode) {
        self.0.push(node.boxed())
    }

    /// Box and [`insert`] the node.
    ///
    /// [`insert`]: Vec::insert
    pub fn insert(&mut self, index: usize, node: impl UiNode) {
        self.0.insert(index, node.boxed())
    }
}
impl ops::Deref for UiNodeVec {
    type Target = Vec<BoxedUiNode>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ops::DerefMut for UiNodeVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl From<Vec<BoxedUiNode>> for UiNodeVec {
    fn from(vec: Vec<BoxedUiNode>) -> Self {
        Self(vec)
    }
}
impl From<UiNodeVec> for Vec<BoxedUiNode> {
    fn from(vec: UiNodeVec) -> Self {
        vec.0
    }
}
impl<U: UiNode> FromIterator<U> for UiNodeVec {
    fn from_iter<T: IntoIterator<Item = U>>(iter: T) -> Self {
        Self(Vec::from_iter(iter.into_iter().map(UiNode::boxed)))
    }
}
impl IntoIterator for UiNodeVec {
    type Item = BoxedUiNode;

    type IntoIter = std::vec::IntoIter<BoxedUiNode>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl UiNodeList for UiNodeVec {
    fn with_node<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        self.0.with_node(index, f)
    }

    fn for_each<F>(&mut self, f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode),
    {
        self.0.for_each(f)
    }

    fn par_each<F>(&mut self, f: F)
    where
        F: Fn(usize, &mut BoxedUiNode) + Send + Sync,
    {
        self.0.par_each(f)
    }

    fn par_fold_reduce<T, I, F, R>(&mut self, identity: I, fold: F, reduce: R) -> T
    where
        T: Send + 'static,
        I: Fn() -> T + Send + Sync,
        F: Fn(T, usize, &mut BoxedUiNode) -> T + Send + Sync,
        R: Fn(T, T) -> T + Send + Sync,
    {
        self.0.par_fold_reduce(identity, fold, reduce)
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn boxed(self) -> BoxedUiNodeList {
        self.0.boxed()
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        self.0.drain_into(vec)
    }
}

/// Adds the `chain` method for all [`UiNodeList`] implementers.
pub trait UiNodeListChain: UiNodeList {
    /// Creates a new [`UiNodeList`] that chains `self` and `other`.
    ///
    /// Special features of each inner list type is preserved.
    fn chain<B>(self, other: B) -> UiNodeListChainImpl<Self, B>
    where
        B: UiNodeList,
        Self: Sized;
}
impl<A: UiNodeList> UiNodeListChain for A {
    fn chain<B>(self, other: B) -> UiNodeListChainImpl<Self, B>
    where
        B: UiNodeList,
    {
        UiNodeListChainImpl(self, other)
    }
}

/// Implements [`UiNodeListChain`].
pub struct UiNodeListChainImpl<A: UiNodeList, B: UiNodeList>(pub A, pub B);
impl<A: UiNodeList, B: UiNodeList> UiNodeList for UiNodeListChainImpl<A, B> {
    fn with_node<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        assert_bounds(self.len(), index);

        if index < self.0.len() {
            self.0.with_node(index, f)
        } else {
            self.1.with_node(index - self.0.len(), f)
        }
    }

    fn for_each<F>(&mut self, mut f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode),
    {
        self.0.for_each(&mut f);
        let offset = self.0.len();
        self.1.for_each(move |i, n| f(i + offset, n))
    }

    fn par_each<F>(&mut self, f: F)
    where
        F: Fn(usize, &mut BoxedUiNode) + Send + Sync,
    {
        let offset = self.0.len();
        task::join(|| self.0.par_each(&f), || self.1.par_each(|i, n| f(i + offset, n)));
    }

    fn par_fold_reduce<T, I, F, R>(&mut self, identity: I, fold: F, reduce: R) -> T
    where
        T: Send + 'static,
        I: Fn() -> T + Send + Sync,
        F: Fn(T, usize, &mut BoxedUiNode) -> T + Send + Sync,
        R: Fn(T, T) -> T + Send + Sync,
    {
        let offset = self.0.len();
        let (a, b) = task::join(
            || self.0.par_fold_reduce(&identity, &fold, &reduce),
            || self.1.par_fold_reduce(&identity, |a, i, n| fold(a, i + offset, n), &reduce),
        );
        reduce(a, b)
    }

    fn len(&self) -> usize {
        self.0.len() + self.1.len()
    }

    fn boxed(self) -> BoxedUiNodeList {
        Box::new(self)
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        self.0.drain_into(vec);
        self.1.drain_into(vec);
    }

    fn init_all(&mut self) {
        if PARALLEL_VAR.get().contains(Parallel::INIT) {
            task::join(|| self.0.init_all(), || self.1.init_all());
        } else {
            self.0.init_all();
            self.1.init_all();
        }
    }

    fn deinit_all(&mut self) {
        if PARALLEL_VAR.get().contains(Parallel::DEINIT) {
            task::join(|| self.0.deinit_all(), || self.1.deinit_all());
        } else {
            self.0.deinit_all();
            self.1.deinit_all();
        }
    }

    fn info_all(&mut self, info: &mut WidgetInfoBuilder) {
        if PARALLEL_VAR.get().contains(Parallel::INFO) {
            let mut b = info.parallel_split();
            task::join(|| self.0.info_all(info), || self.1.info_all(&mut b));
            info.parallel_fold(b);
        } else {
            self.0.info_all(info);
            self.1.info_all(info);
        }
    }

    fn event_all(&mut self, update: &EventUpdate) {
        if PARALLEL_VAR.get().contains(Parallel::EVENT) {
            task::join(|| self.0.event_all(update), || self.1.event_all(update));
        } else {
            self.0.event_all(update);
            self.1.event_all(update);
        }
    }

    fn update_all(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        if observer.is_reset_only() && PARALLEL_VAR.get().contains(Parallel::UPDATE) {
            let (r0, r1) = task::join(
                || {
                    let mut r = false;
                    self.0.update_all(updates, &mut r);
                    r
                },
                || {
                    let mut r = false;
                    self.1.update_all(updates, &mut r);
                    r
                },
            );

            if r0 || r1 {
                observer.reset();
            }
        } else {
            self.0.update_all(updates, observer);
            self.1.update_all(updates, &mut OffsetUiListObserver(self.0.len(), observer));
        }
    }

    fn render_all(&mut self, frame: &mut FrameBuilder) {
        if PARALLEL_VAR.get().contains(Parallel::RENDER) {
            let mut b = frame.parallel_split();
            task::join(|| self.0.render_all(frame), || self.1.render_all(&mut b));
            frame.parallel_fold(b);
        } else {
            self.0.render_all(frame);
            self.1.render_all(frame);
        }
    }

    fn render_update_all(&mut self, update: &mut FrameUpdate) {
        if PARALLEL_VAR.get().contains(Parallel::RENDER) {
            let mut b = update.parallel_split();
            task::join(|| self.0.render_update_all(update), || self.1.render_update_all(&mut b));
            update.parallel_fold(b);
        } else {
            self.0.render_update_all(update);
            self.1.render_update_all(update);
        }
    }
}

/// Represents the contextual parent [`SortingList`] during a list.
pub struct SortingListParent {}
impl SortingListParent {
    /// If the current call has a parent list.
    pub fn is_inside_list() -> bool {
        !SORTING_LIST_PARENT.is_default()
    }

    /// Calls [`SortingList::invalidate_sort`] on the parent list.
    pub fn invalidate_sort() {
        SORTING_LIST_PARENT.get().store(true, Relaxed)
    }

    fn with<R>(action: impl FnOnce() -> R) -> (R, bool) {
        SORTING_LIST_PARENT.with_context_value(AtomicBool::new(false), || {
            let r = action();
            (r, SORTING_LIST_PARENT.get().load(Relaxed))
        })
    }
}
context_local! {
    static SORTING_LIST_PARENT: AtomicBool = AtomicBool::new(false);
}

/// Represents a sorted view into an [`UiNodeList`] that is not changed.
///
/// Note that the `*_all` methods are not sorted, only the other accessors map to the sorted position of nodes. The sorting is lazy
/// and gets invalidated on every init and every time there are changes observed in [`update_all`].
///
/// [`update_all`]: UiNodeList
pub struct SortingList<L, S>
where
    L: UiNodeList,
    S: Fn(&BoxedUiNode, &BoxedUiNode) -> Ordering + Send + 'static,
{
    list: L,

    map: Vec<usize>,
    sort: S,
}
impl<L, S> SortingList<L, S>
where
    L: UiNodeList,
    S: Fn(&BoxedUiNode, &BoxedUiNode) -> Ordering + Send + 'static,
{
    /// New from list and sort function.
    pub fn new(list: L, sort: S) -> Self {
        Self { list, map: vec![], sort }
    }

    fn update_map(&mut self) {
        let map = &mut self.map;
        let len = self.list.len();

        if len == 0 {
            map.clear();
        } else if map.len() != len {
            map.clear();
            map.extend(0..len);
            let mut taken_a = NilUiNode.boxed();
            map.sort_by(|&a, &b| {
                self.list.with_node(a, |a| mem::swap(a, &mut taken_a));
                let result = self.list.with_node(b, |b| (self.sort)(&taken_a, b));

                self.list.with_node(a, |a| mem::swap(a, &mut taken_a));

                result
            })
        }
    }
    /// Mutable borrow the inner list.
    ///
    /// You must call [`invalidate_sort`] if any modification may have affected sort without changing the list length.
    ///
    /// [`invalidate_sort`]: Self::invalidate_sort
    pub fn list(&mut self) -> &mut L {
        &mut self.list
    }

    /// Invalidate the sort, the list will resort on the nest time the sorted positions are needed.
    ///
    /// Note that you can also invalidate sort from the inside using [`SortingListParent::invalidate_sort`].
    pub fn invalidate_sort(&mut self) {
        self.map.clear()
    }

    fn with_map<R>(&mut self, f: impl FnOnce(&[usize], &mut L) -> R) -> R {
        self.update_map();

        let (r, resort) = SortingListParent::with(|| f(&self.map, &mut self.list));

        if resort {
            self.invalidate_sort();
        }

        r
    }
}
impl<L, S> UiNodeList for SortingList<L, S>
where
    L: UiNodeList,
    S: Fn(&BoxedUiNode, &BoxedUiNode) -> Ordering + Send + 'static,
{
    fn with_node<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        self.with_map(|map, list| list.with_node(map[index], f))
    }

    fn for_each<F>(&mut self, mut f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode),
    {
        self.with_map(|map, list| {
            for (index, map) in map.iter().enumerate() {
                list.with_node(*map, |n| f(index, n))
            }
        });
    }

    fn par_each<F>(&mut self, f: F)
    where
        F: Fn(usize, &mut BoxedUiNode) + Send + Sync,
    {
        self.for_each(f)
    }

    fn par_fold_reduce<T, I, F, R>(&mut self, identity: I, fold: F, _: R) -> T
    where
        T: Send + 'static,
        I: Fn() -> T + Send + Sync,
        F: Fn(T, usize, &mut BoxedUiNode) -> T + Send + Sync,
        R: Fn(T, T) -> T + Send + Sync,
    {
        let mut r = Some(identity());
        self.for_each(|i, n| {
            r = Some(fold(r.take().unwrap(), i, n));
        });
        r.unwrap()
    }

    fn len(&self) -> usize {
        self.list.len()
    }

    fn boxed(self) -> BoxedUiNodeList {
        Box::new(self)
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        let start = vec.len();
        self.list.drain_into(vec);
        vec[start..].sort_by(&self.sort);
        self.map.clear();
    }

    fn init_all(&mut self) {
        let _ = SortingListParent::with(|| self.list.init_all());
        self.invalidate_sort();
    }

    fn deinit_all(&mut self) {
        let _ = SortingListParent::with(|| self.list.deinit_all());
        self.invalidate_sort();
    }

    fn info_all(&mut self, info: &mut WidgetInfoBuilder) {
        self.list.info_all(info);
    }

    fn event_all(&mut self, update: &EventUpdate) {
        self.list.event_all(update);
    }

    fn update_all(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        let mut changed = false;
        let (_, resort) = SortingListParent::with(|| self.list.update_all(updates, &mut (observer, &mut changed as _)));
        if changed || resort {
            self.invalidate_sort();
        }
    }

    fn render_all(&mut self, frame: &mut FrameBuilder) {
        self.for_each(|_, n| n.render(frame));
    }

    fn render_update_all(&mut self, update: &mut FrameUpdate) {
        self.list.render_update_all(update);
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
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

/// Access to widget z-index.
///
/// The z-index can be set using the [`z_index`] property.
///
/// [`z_index`]: fn@z_index
#[allow(non_camel_case_types)]
pub struct Z_INDEX;
impl Z_INDEX {
    fn with(&self, panel_id: WidgetId, action: impl FnOnce()) -> bool {
        let ctx = ZIndexCtx {
            panel_id: Some(panel_id),
            resort: AtomicBool::new(false),
        };
        Z_INDEX_CTX.with_context_value(ctx, || {
            action();
            Z_INDEX_CTX.get().resort.load(Relaxed)
        })
    }

    /// Gets the index set on the [`WIDGET`].
    ///
    /// Returns `DEFAULT` if the node is not an widget.
    pub fn get(&self) -> ZIndex {
        WIDGET.get_state(&Z_INDEX_ID).unwrap_or_default()
    }

    /// Gets the index set on the `widget`.
    ///
    /// Returns `DEFAULT` if the node is not an widget.
    pub fn get_wgt(&self, widget: &mut impl UiNode) -> ZIndex {
        widget.with_context(|| self.get()).unwrap_or_default()
    }
}

static Z_INDEX_ID: StaticStateId<ZIndex> = StaticStateId::new_unique();

/// Position of a widget inside an [`UiNodeList`] render operation.
///
/// When two widgets have the same index their logical position defines the render order.
///
/// # Examples
///
/// Create a Z-index that causes the widget to render in front of all siblings that don't set Z-index.
///
/// ```
/// # use zero_ui_core::widget_instance::ZIndex;
/// #
/// let highlight_z = ZIndex::DEFAULT + 1;
/// ```
///
/// See [`z_index`] for more details.
///
/// [`z_index`]: fn@z_index
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ZIndex(pub u32);
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
}

/// Defines the render order of a widget in a layout panel.
///
/// When set the widget will still update and layout according to their *logical* position in the list but
/// they will render according to the order defined by the [`ZIndex`] value.
///
/// Layout panels that support this property should mention it in their documentation, implementers
/// see [`PanelList`] for more details.
#[property(CONTEXT, default(ZIndex::DEFAULT))]
pub fn z_index(child: impl UiNode, index: impl IntoVar<ZIndex>) -> impl UiNode {
    let index = index.into_var();
    let mut valid = false;

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            let z_ctx = Z_INDEX_CTX.get();

            if z_ctx.panel_id != WIDGET.parent_id() || z_ctx.panel_id.is_none() {
                tracing::error!(
                    "property `z_index` set for `{}` but it is not the direct child of a Z-sorting panel",
                    WIDGET.id()
                );
                valid = false;
            } else {
                valid = true;
                WIDGET.sub_var(&index);

                if index.get() != ZIndex::DEFAULT {
                    z_ctx.resort.store(true, Relaxed);
                    WIDGET.set_state(&Z_INDEX_ID, index.get());
                }
            }
        }
        UiNodeOp::Update { .. } => {
            if valid {
                if let Some(i) = index.get_new() {
                    let z_ctx = Z_INDEX_CTX.get();
                    debug_assert_eq!(z_ctx.panel_id, WIDGET.parent_id());
                    z_ctx.resort.store(true, Relaxed);

                    WIDGET.set_state(&Z_INDEX_ID, i);
                }
            }
        }
        _ => {}
    })
}

/// Represents an [`UiNodeList::update_all`] observer that can be used to monitor widget insertion, removal and re-order.
///
/// All indexes are in the context of the previous changes, if you are maintaining a *mirror* vector simply using the
/// [`Vec::insert`] and [`Vec::remove`] commands in the same order as they are received should keep the vector in sync.
///
/// This trait is implemented for `()`, to **not** observe simply pass on a `&mut ()`.
///
/// This trait is implemented for [`bool`], if any change happens the flag is set to `true`.
pub trait UiNodeListObserver {
    /// If  this observer does not use the item indexes and any/all calls to observer methods can be replaced by a single
    /// or multiple calls to [`reset`].
    ///
    /// This flag can be used by list implementers to enable parallel processing in more contexts, for example, chain lists cannot
    /// parallelize because indexes of subsequent lists are dependent on indexed of previous lists, but if the observer only needs
    /// to known that some change happened the chain list can still parallelize.
    ///
    /// [`reset`]: Self::reset
    fn is_reset_only(&self) -> bool;

    /// Large changes made to the list.
    fn reset(&mut self);
    /// Widget inserted at the `index`.
    fn inserted(&mut self, index: usize);
    /// Widget removed from the `index`.
    fn removed(&mut self, index: usize);
    /// Widget removed and re-inserted.
    fn moved(&mut self, removed_index: usize, inserted_index: usize);
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
/// This type is useful for implementing [`UiNodeList`] that are composed of other lists.
pub struct OffsetUiListObserver<'o>(pub usize, pub &'o mut dyn UiNodeListObserver);
impl<'o> UiNodeListObserver for OffsetUiListObserver<'o> {
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

/// Represents an [`UiNodeList`] that can be modified using a connected sender.
pub struct EditableUiNodeList {
    vec: Vec<BoxedUiNode>,
    ctrl: EditableUiNodeListRef,
}
impl Default for EditableUiNodeList {
    fn default() -> Self {
        Self {
            vec: vec![],
            ctrl: EditableUiNodeListRef::new(),
        }
    }
}
impl Drop for EditableUiNodeList {
    fn drop(&mut self) {
        self.ctrl.0.lock().alive = false;
    }
}
impl EditableUiNodeList {
    /// New default empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// New from an already allocated vec.
    pub fn from_vec(vec: impl Into<Vec<BoxedUiNode>>) -> Self {
        let mut s = Self::new();
        s.vec = vec.into();
        s
    }

    /// Create a sender that can edit this list.
    pub fn reference(&self) -> EditableUiNodeListRef {
        self.ctrl.clone()
    }

    /// Take the list of pending retain requests, the widgets will not be removed on the next update.
    pub fn take_retain_requests(&mut self) -> Vec<Box<dyn FnMut(&mut BoxedUiNode) -> bool + Send>> {
        self.ctrl.take_retain_requests()
    }

    fn fullfill_requests(&mut self, observer: &mut dyn UiNodeListObserver) {
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
                    if let Some(r) = self
                        .vec
                        .iter_mut()
                        .position(|w| w.with_context(|| WIDGET.id() == id).unwrap_or(false))
                    {
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
                    if let Some(r) = self
                        .vec
                        .iter_mut()
                        .position(|w| w.with_context(|| WIDGET.id() == id).unwrap_or(false))
                    {
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
impl ops::Deref for EditableUiNodeList {
    type Target = Vec<BoxedUiNode>;

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}
impl ops::DerefMut for EditableUiNodeList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec
    }
}
impl UiNodeList for EditableUiNodeList {
    fn with_node<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        self.vec.with_node(index, f)
    }

    fn for_each<F>(&mut self, f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode),
    {
        self.vec.for_each(f)
    }

    fn par_each<F>(&mut self, f: F)
    where
        F: Fn(usize, &mut BoxedUiNode) + Send + Sync,
    {
        self.vec.par_each(f)
    }

    fn par_fold_reduce<T, I, F, R>(&mut self, identity: I, fold: F, reduce: R) -> T
    where
        T: Send + 'static,
        I: Fn() -> T + Send + Sync,
        F: Fn(T, usize, &mut BoxedUiNode) -> T + Send + Sync,
        R: Fn(T, T) -> T + Send + Sync,
    {
        self.vec.par_fold_reduce(identity, fold, reduce)
    }

    fn len(&self) -> usize {
        self.vec.len()
    }

    fn boxed(self) -> BoxedUiNodeList {
        Box::new(self)
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        vec.append(&mut self.vec)
    }

    fn init_all(&mut self) {
        self.ctrl.0.lock().target = Some(WIDGET.id());
        self.vec.init_all();
    }

    fn deinit_all(&mut self) {
        self.ctrl.0.lock().target = None;
        self.vec.deinit_all();
    }

    fn info_all(&mut self, info: &mut WidgetInfoBuilder) {
        self.vec.info_all(info);
    }

    fn event_all(&mut self, update: &EventUpdate) {
        self.vec.event_all(update)
    }

    fn update_all(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        self.fullfill_requests(observer);
        self.vec.update_all(updates, observer);
    }

    fn measure_each<F, S>(&mut self, wm: &mut WidgetMeasure, measure: F, fold_size: S) -> PxSize
    where
        F: Fn(usize, &mut BoxedUiNode, &mut WidgetMeasure) -> PxSize + Send + Sync,
        S: Fn(PxSize, PxSize) -> PxSize + Send + Sync,
    {
        self.vec.measure_each(wm, measure, fold_size)
    }

    fn layout_each<F, S>(&mut self, wl: &mut WidgetLayout, layout: F, fold_size: S) -> PxSize
    where
        F: Fn(usize, &mut BoxedUiNode, &mut WidgetLayout) -> PxSize + Send + Sync,
        S: Fn(PxSize, PxSize) -> PxSize + Send + Sync,
    {
        self.vec.layout_each(wl, layout, fold_size)
    }

    fn render_all(&mut self, frame: &mut FrameBuilder) {
        self.vec.render_all(frame)
    }

    fn render_update_all(&mut self, update: &mut FrameUpdate) {
        self.vec.render_update_all(update)
    }
}

/// See [`EditableUiNodeListRef::move_to`] for more details
type NodeMoveToFn = fn(usize, usize) -> usize;

/// Represents a sender to an [`EditableUiNodeList`].
#[derive(Clone, Debug)]
pub struct EditableUiNodeListRef(Arc<Mutex<EditRequests>>);
struct EditRequests {
    target: Option<WidgetId>,
    insert: Vec<(usize, BoxedUiNode)>,
    push: Vec<BoxedUiNode>,
    retain: Vec<Box<dyn FnMut(&mut BoxedUiNode) -> bool + Send>>,
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
impl EditableUiNodeListRef {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(EditRequests {
            target: None,
            insert: vec![],
            push: vec![],
            retain: vec![],
            move_index: vec![],
            move_id: vec![],
            clear: false,
            alive: true,
        })))
    }

    /// Returns `true` if the [`EditableUiNodeList`] still exists.
    pub fn alive(&self) -> bool {
        self.0.lock().alive
    }

    /// Request an update for the insertion of the `widget`.
    ///
    /// The `index` is resolved after all [`remove`] requests, if it is out-of-bounds the widget is pushed.
    ///
    /// The `widget` will inserted, inited and the info tree updated.
    ///
    /// [`remove`]: Self::remove
    pub fn insert(&self, index: usize, widget: impl UiNode) {
        let mut s = self.0.lock();
        s.insert.push((index, widget.boxed()));
        UPDATES.update(s.target);
    }

    /// Request an update for the insertion of the `widget` at the end of the list.
    ///
    /// The widget will be pushed after all [`insert`] requests.
    ///
    /// The `widget` will be inserted, inited and the info tree updated.
    ///
    /// [`insert`]: Self::insert
    pub fn push(&self, widget: impl UiNode) {
        let mut s = self.0.lock();
        s.push.push(widget.boxed());
        UPDATES.update(s.target);
    }

    /// Request an update for the removal of the widget identified by `id`.
    ///
    /// The widget will be deinitialized, dropped and the info tree will update, nothing happens
    /// if the widget is not found.
    pub fn remove(&self, id: impl Into<WidgetId>) {
        fn remove_impl(id: WidgetId) -> impl FnMut(&mut BoxedUiNode) -> bool + Send + 'static {
            move |node| node.with_context(|| WIDGET.id() != id).unwrap_or(true)
        }
        self.retain(remove_impl(id.into()))
    }

    /// Request a filtered mass removal of nodes in the list.
    ///
    /// Each node not retained will be deinitialized, dropped and the info tree will update if any was removed.
    ///
    /// Note that the `predicate` may be called on the same node multiple times or called in any order.
    pub fn retain(&self, predicate: impl FnMut(&mut BoxedUiNode) -> bool + Send + 'static) {
        let mut s = self.0.lock();
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
    /// # fn demo(items: zero_ui_core::widget_instance::EditableUiNodeListRef) {
    /// items.move_id("my-widget", |i, _len| i.saturating_sub(1));
    /// # }
    /// ```
    ///
    /// And to move *down* stopping at the bottom:
    ///
    /// ```
    /// # fn demo(items: zero_ui_core::widget_instance::EditableUiNodeListRef) {
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
    /// # fn demo(items: zero_ui_core::widget_instance::EditableUiNodeListRef) {
    /// items.move_id("my-widget", |i, len| {
    ///     let next = i.saturating_add(1);
    ///     if next < len { next } else { 0 }
    /// });
    /// # }
    /// ```
    pub fn move_id(&self, id: impl Into<WidgetId>, get_move_to: NodeMoveToFn) {
        let mut s = self.0.lock();
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

    fn take_retain_requests(&self) -> Vec<Box<dyn FnMut(&mut BoxedUiNode) -> bool + Send>> {
        mem::take(&mut self.0.lock().retain)
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

fn many_list_index(lists: &Vec<BoxedUiNodeList>, index: usize) -> (usize, usize) {
    let mut offset = 0;

    for (li, list) in lists.iter().enumerate() {
        let i = index - offset;
        let len = list.len();

        if i < len {
            return (li, i);
        }

        offset += len;
    }

    panic!(
        "'index out of bounds: the len is {} but the index is {}",
        UiNodeList::len(lists),
        index
    );
}

impl UiNodeList for Vec<BoxedUiNodeList> {
    fn with_node<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        let (l, i) = many_list_index(self, index);
        self[l].with_node(i, f)
    }

    fn for_each<F>(&mut self, mut f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode),
    {
        let mut offset = 0;
        for list in self {
            list.for_each(|i, n| f(i + offset, n));
            offset += list.len();
        }
    }

    fn par_each<F>(&mut self, f: F)
    where
        F: Fn(usize, &mut BoxedUiNode) + Send + Sync,
    {
        task::scope(|s| {
            let f = &f;
            let mut offset = 0;
            for list in self {
                let len = list.len();
                s.spawn(move |_| {
                    list.par_each(move |i, n| f(i + offset, n));
                });
                offset += len;
            }
        });
    }

    fn par_fold_reduce<T, I, F, R>(&mut self, identity: I, fold: F, reduce: R) -> T
    where
        T: Send + 'static,
        I: Fn() -> T + Send + Sync,
        F: Fn(T, usize, &mut BoxedUiNode) -> T + Send + Sync,
        R: Fn(T, T) -> T + Send + Sync,
    {
        let mut offset = 0;
        let mut r = Some(identity());
        for list in self.chunks_mut(2) {
            let b = if list.len() == 2 {
                let mut pair = list.iter_mut();
                let a = pair.next().unwrap();
                let b = pair.next().unwrap();
                let offset_b = offset + a.len();

                let (a, b) = task::join(
                    || a.par_fold_reduce(&identity, |a, i, n| fold(a, i + offset, n), &reduce),
                    || b.par_fold_reduce(&identity, |a, i, n| fold(a, i + offset_b, n), &reduce),
                );

                reduce(a, b)
            } else {
                list[0].par_fold_reduce(&identity, |a, i, n| fold(a, i + offset, n), &reduce)
            };

            let a = r.take().unwrap();
            r = Some(reduce(a, b));

            offset += list.iter().map(|l| l.len()).sum::<usize>();
        }
        r.unwrap()
    }

    fn len(&self) -> usize {
        self.iter().map(|l| l.len()).sum()
    }

    fn boxed(self) -> BoxedUiNodeList {
        Box::new(self)
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        for mut list in self.drain(..) {
            list.drain_into(vec);
        }
    }

    fn is_empty(&self) -> bool {
        self.iter().all(|l| l.is_empty())
    }

    fn init_all(&mut self) {
        if self.len() > 1 && PARALLEL_VAR.get().contains(Parallel::INIT) {
            self.par_iter_mut().with_ctx().for_each(|l| l.init_all());
        } else {
            for l in self {
                l.init_all();
            }
        }
    }

    fn deinit_all(&mut self) {
        if self.len() > 1 && PARALLEL_VAR.get().contains(Parallel::DEINIT) {
            self.par_iter_mut().with_ctx().for_each(|l| l.deinit_all());
        } else {
            for list in self {
                list.deinit_all();
            }
        }
    }

    fn update_all(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        if self.len() > 1 && observer.is_reset_only() && PARALLEL_VAR.get().contains(Parallel::UPDATE) {
            let r = self
                .par_iter_mut()
                .with_ctx()
                .map(|l| {
                    let mut r = false;
                    l.update_all(updates, &mut r);
                    r
                })
                .any(std::convert::identity);
            if r {
                observer.reset();
            }
        } else {
            let mut offset = 0;
            for list in self {
                list.update_all(updates, &mut OffsetUiListObserver(offset, observer));
                offset += list.len();
            }
        }
    }

    fn info_all(&mut self, info: &mut WidgetInfoBuilder) {
        if self.len() > 1 && PARALLEL_VAR.get().contains(Parallel::INFO) {
            let b = self
                .par_iter_mut()
                .with_ctx()
                .fold(
                    || info.parallel_split(),
                    |mut info, list| {
                        list.info_all(&mut info);
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
            for list in self {
                list.info_all(info);
            }
        }
    }

    fn event_all(&mut self, update: &EventUpdate) {
        if self.len() > 1 && PARALLEL_VAR.get().contains(Parallel::EVENT) {
            self.par_iter_mut().with_ctx().for_each(|l| l.event_all(update));
        } else {
            for list in self {
                list.event_all(update);
            }
        }
    }

    // `measure_each` and `layout_each` can use the default impl because they are just
    // helpers, not like the `*_all` methods that must be called to support features
    // of the various list types.

    fn render_all(&mut self, frame: &mut FrameBuilder) {
        if self.len() > 1 && PARALLEL_VAR.get().contains(Parallel::RENDER) {
            let b = self
                .par_iter_mut()
                .with_ctx()
                .fold(
                    || frame.parallel_split(),
                    |mut frame, list| {
                        list.render_all(&mut frame);
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
            for list in self {
                list.render_all(frame);
            }
        }
    }

    fn render_update_all(&mut self, update: &mut FrameUpdate) {
        if self.len() > 1 && PARALLEL_VAR.get().contains(Parallel::RENDER) {
            let b = self
                .par_iter_mut()
                .with_ctx()
                .fold(
                    || update.parallel_split(),
                    |mut update, list| {
                        list.render_update_all(&mut update);
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
            for list in self {
                list.render_update_all(update);
            }
        }
    }
}

/// Represents the final [`UiNodeList`] in a panel layout node.
///
/// Panel widgets should wrap their children list on this type to support [`z_index`] sorting list and easily track
/// item data. By default the item data is a [`PxVector`] that represents the offset of each item inside the panel,
/// but it can be any type that implements [`PanelListData`].
///
/// [`z_index`]: fn@z_index
pub struct PanelList<D = DefaultPanelListData>
where
    D: PanelListData,
{
    list: BoxedUiNodeList,
    data: Vec<Mutex<D>>, // Mutex to implement `par_each_mut`.

    offset_key: FrameValueKey<PxTransform>,

    z_map: Vec<u64>,
    z_naturally_sorted: bool,
}
impl PanelList<DefaultPanelListData> {
    /// New from `list` and default data.
    pub fn new(list: impl UiNodeList) -> Self {
        Self::new_custom(list)
    }
}

impl<D> PanelList<D>
where
    D: PanelListData,
{
    /// New from `list` and custom data type.
    pub fn new_custom(list: impl UiNodeList) -> Self {
        Self {
            data: {
                let mut d = vec![];
                d.resize_with(list.len(), Default::default);
                d
            },
            list: list.boxed(),
            offset_key: FrameValueKey::new_unique(),
            z_map: vec![],
            z_naturally_sorted: false,
        }
    }

    /// Into list and associated data.
    pub fn into_parts(self) -> (BoxedUiNodeList, Vec<Mutex<D>>, FrameValueKey<PxTransform>) {
        (self.list, self.data, self.offset_key)
    }

    /// New from list and associated data.
    ///
    /// # Panics
    ///
    /// Panics if the `list` and `data` don't have the same length.
    pub fn from_parts(list: BoxedUiNodeList, data: Vec<Mutex<D>>, offset_key: FrameValueKey<PxTransform>) -> Self {
        assert_eq!(list.len(), data.len());
        Self {
            list,
            data,
            offset_key,
            z_map: vec![],
            z_naturally_sorted: false,
        }
    }

    /// Visit the specific node, panic if `index` is out of bounds.
    pub fn with_node<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode, &mut D) -> R,
    {
        let data = self.data[index].get_mut();
        self.list.with_node(index, move |n| f(n, data))
    }

    /// Calls `f` for each node in the list with the index and associated data.
    pub fn for_each<F>(&mut self, mut f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode, &mut D),
    {
        let data = &mut self.data;
        self.list.for_each(move |i, n| f(i, n, data[i].get_mut()))
    }

    /// Calls `f` for each node in the list with the index and associated data in parallel.
    pub fn par_each<F>(&mut self, f: F)
    where
        F: Fn(usize, &mut BoxedUiNode, &mut D) + Send + Sync,
        D: Sync,
    {
        let data = &self.data;
        self.list.par_each(|i, n| {
            f(
                i,
                n,
                &mut data[i].try_lock().unwrap_or_else(|| panic!("data for `{i}` is already locked")),
            )
        })
    }

    /// Calls `fold` for each node in the list with the index and associated data in parallel.
    ///
    /// This method behaves the same as [`UiNodeList::par_fold_reduce`], with the added data.
    pub fn par_fold_reduce<T, I, F, R>(&mut self, identity: I, fold: F, reduce: R) -> T
    where
        T: Send + 'static,
        I: Fn() -> T + Send + Sync,
        F: Fn(T, usize, &mut BoxedUiNode, &mut D) -> T + Send + Sync,
        R: Fn(T, T) -> T + Send + Sync,
    {
        let data = &self.data;
        self.list.par_fold_reduce(
            identity,
            |a, i, n| {
                fold(
                    a,
                    i,
                    n,
                    &mut data[i].try_lock().unwrap_or_else(|| panic!("data for `{i}` is already locked")),
                )
            },
            reduce,
        )
    }

    /// Call `measure` for each node and combines the final size using `fold_size`.
    ///
    /// The call to `measure` can be parallel if [`Parallel::LAYOUT`] is enabled, the inputs are the child index, node, data and the [`WidgetMeasure`].
    pub fn measure_each<F, S>(&mut self, wm: &mut WidgetMeasure, measure: F, fold_size: S) -> PxSize
    where
        F: Fn(usize, &mut BoxedUiNode, &mut D, &mut WidgetMeasure) -> PxSize + Send + Sync,
        S: Fn(PxSize, PxSize) -> PxSize + Send + Sync,
    {
        let data = &self.data;
        self.list.measure_each(
            wm,
            |i, n, wm| {
                measure(
                    i,
                    n,
                    &mut data[i].try_lock().unwrap_or_else(|| panic!("data for `{i}` is already locked")),
                    wm,
                )
            },
            fold_size,
        )
    }

    /// Call `layout` for each node and combines the final size using `fold_size`.
    ///
    /// The call to `layout` can be parallel if [`Parallel::LAYOUT`] is enabled, the inputs are the child index, node, data and the [`WidgetLayout`].
    pub fn layout_each<F, S>(&mut self, wl: &mut WidgetLayout, layout: F, fold_size: S) -> PxSize
    where
        F: Fn(usize, &mut BoxedUiNode, &mut D, &mut WidgetLayout) -> PxSize + Send + Sync,
        S: Fn(PxSize, PxSize) -> PxSize + Send + Sync,
    {
        let data = &self.data;
        self.list.layout_each(
            wl,
            |i, n, wl| {
                layout(
                    i,
                    n,
                    &mut data[i].try_lock().unwrap_or_else(|| panic!("data for `{i}` is already locked")),
                    wl,
                )
            },
            fold_size,
        )
    }

    /// Iterate over the list in the Z order.
    pub fn for_each_z_sorted(&mut self, mut f: impl FnMut(usize, &mut BoxedUiNode, &mut D)) {
        if self.z_naturally_sorted {
            self.for_each(f)
        } else {
            if self.z_map.len() != self.list.len() {
                self.z_sort();
            }

            if self.z_naturally_sorted {
                self.for_each(f);
            } else {
                for &index in self.z_map.iter() {
                    let index = index as usize;
                    let data = self.data[index].get_mut();
                    self.list.with_node(index, |node| f(index, node, data));
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

        let len = self.list.len();
        assert!(len <= u32::MAX as usize);

        let mut prev_z = ZIndex::BACK;
        let mut need_map = false;
        let mut z_and_i = Vec::with_capacity(len);
        let mut has_non_default_zs = false;

        self.list.for_each(|i, node| {
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

        if self.z_map.len() != self.list.len() {
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

    /// Key used to define reference frames for each item.
    ///
    /// The default implementation of `render_all` uses this key and the item index.
    pub fn offset_key(&self) -> FrameValueKey<PxTransform> {
        self.offset_key
    }
}
impl<D> UiNodeList for PanelList<D>
where
    D: PanelListData,
{
    fn with_node<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        self.list.with_node(index, f)
    }

    fn for_each<F>(&mut self, f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode),
    {
        self.list.for_each(f)
    }

    fn par_each<F>(&mut self, f: F)
    where
        F: Fn(usize, &mut BoxedUiNode) + Send + Sync,
    {
        self.list.par_each(f)
    }

    fn par_fold_reduce<T, I, F, R>(&mut self, identity: I, fold: F, reduce: R) -> T
    where
        T: Send + 'static,
        I: Fn() -> T + Send + Sync,
        F: Fn(T, usize, &mut BoxedUiNode) -> T + Send + Sync,
        R: Fn(T, T) -> T + Send + Sync,
    {
        self.list.par_fold_reduce(identity, fold, reduce)
    }

    fn len(&self) -> usize {
        self.list.len()
    }

    fn boxed(self) -> BoxedUiNodeList {
        Box::new(self)
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        self.list.drain_into(vec);
        self.data.clear();
        self.z_map.clear();
        self.z_naturally_sorted = true;
    }

    fn init_all(&mut self) {
        self.z_map.clear();
        let resort = Z_INDEX.with(WIDGET.id(), || self.list.init_all());
        self.z_naturally_sorted = !resort;
        self.data.resize_with(self.list.len(), Default::default);
    }

    fn deinit_all(&mut self) {
        self.list.deinit_all();
    }

    fn info_all(&mut self, info: &mut WidgetInfoBuilder) {
        self.list.info_all(info);
    }

    fn event_all(&mut self, update: &EventUpdate) {
        self.list.event_all(update);
    }

    fn update_all(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        let mut observer = PanelObserver {
            changed: false,
            data: &mut self.data,
            observer,
        };
        let resort = Z_INDEX.with(WIDGET.id(), || self.list.update_all(updates, &mut observer));
        if resort || (observer.changed && self.z_naturally_sorted) {
            self.z_map.clear();
            self.z_naturally_sorted = false;
            WIDGET.render();
        }
        self.data.resize_with(self.list.len(), Default::default);
    }

    fn render_all(&mut self, frame: &mut FrameBuilder) {
        let offset_key = self.offset_key;
        if self.z_naturally_sorted && self.len() > 1 && PARALLEL_VAR.get().contains(Parallel::RENDER) {
            let b = self.par_fold_reduce(
                || frame.parallel_split(),
                |mut frame, i, child, data| {
                    let offset = data.child_offset();
                    if data.define_reference_frame() {
                        frame.push_reference_frame(
                            (offset_key, i as u32).into(),
                            offset_key.bind_child(i as u32, offset.into(), false),
                            true,
                            true,
                            |frame| {
                                child.render(frame);
                            },
                        );
                    } else {
                        frame.push_child(offset, |frame| {
                            child.render(frame);
                        });
                    }
                    frame
                },
                |mut a, b| {
                    a.parallel_fold(b);
                    a
                },
            );
            frame.parallel_fold(b);
        } else {
            self.for_each_z_sorted(|i, child, data| {
                let offset = data.child_offset();
                if data.define_reference_frame() {
                    frame.push_reference_frame(
                        (offset_key, i as u32).into(),
                        offset_key.bind_child(i as u32, offset.into(), false),
                        true,
                        true,
                        |frame| {
                            child.render(frame);
                        },
                    );
                } else {
                    frame.push_child(offset, |frame| {
                        child.render(frame);
                    });
                }
            });
        }
    }

    fn render_update_all(&mut self, update: &mut FrameUpdate) {
        let offset_key = self.offset_key;

        if self.len() > 1 && PARALLEL_VAR.get().contains(Parallel::RENDER) {
            let b = self.par_fold_reduce(
                || update.parallel_split(),
                |mut update, i, child, data| {
                    let offset = data.child_offset();
                    if data.define_reference_frame() {
                        update.with_transform(offset_key.update_child(i as u32, offset.into(), false), true, |update| {
                            child.render_update(update);
                        });
                    } else {
                        update.with_child(offset, |update| {
                            child.render_update(update);
                        });
                    }
                    update
                },
                |mut a, b| {
                    a.parallel_fold(b);
                    a
                },
            );
            update.parallel_fold(b);
        } else {
            self.for_each(|i, child, data| {
                let offset = data.child_offset();
                if data.define_reference_frame() {
                    update.with_transform(offset_key.update_child(i as u32, offset.into(), false), true, |update| {
                        child.render_update(update);
                    });
                } else {
                    update.with_child(offset, |update| {
                        child.render_update(update);
                    });
                }
            });
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
}
impl PanelListData for DefaultPanelListData {
    fn child_offset(&self) -> PxVector {
        self.child_offset
    }

    fn define_reference_frame(&self) -> bool {
        self.define_reference_frame
    }
}

/// Represents associated data in a [`PanelList`].
pub trait PanelListData: Default + Send + Any {
    /// Gets the child offset to be used in the default `render_all` and `render_update_all` implementations.
    fn child_offset(&self) -> PxVector;

    /// If a new reference frame should be created for the item during render.
    fn define_reference_frame(&self) -> bool;
}
impl PanelListData for PxVector {
    fn child_offset(&self) -> PxVector {
        *self
    }

    fn define_reference_frame(&self) -> bool {
        true
    }
}
impl PanelListData for () {
    fn child_offset(&self) -> PxVector {
        PxVector::zero()
    }

    fn define_reference_frame(&self) -> bool {
        false
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
impl<'d, D> UiNodeListObserver for PanelObserver<'d, D>
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
