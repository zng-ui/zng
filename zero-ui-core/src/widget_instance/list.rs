use std::{
    cell::{Cell, RefCell},
    cmp::Ordering,
    ops,
};

use crate::{
    context_value, property,
    var::{IntoVar, Var},
};

use super::*;

/// Creates a `Vec<BoxedUiNode>` containing the arguments.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::{ui_list, UiNode, Widget, WidgetId, NilUiNode};
/// # use zero_ui_core::widget_base::*;
/// # fn text(fake: &str) -> impl Widget { implicit_base::new(NilUiNode, WidgetId::new_unique())  };
/// # use text as foo;
/// # use text as bar;
/// let widgets = ui_list![
///     foo("Hello"),
///     bar("World!")
/// ];
/// ```
///
/// `ui_list!` automatically calls [`UiNode::boxed`] for each item.
#[macro_export]
macro_rules! ui_list {
    () => { std::vec::Vec::<$crate::widget_instance::BoxedUiNode>::new() };
    ($($node:expr),+ $(,)?) => {
        vec![
            $($crate::widget_instance::UiNode::boxed($node)),*
        ]
    };
}
#[doc(inline)]
pub use crate::ui_list;

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
    fn with_node<R, F>(&self, index: usize, f: F) -> R
    where
        F: FnOnce(&BoxedUiNode) -> R,
    {
        if index < self.0.len() {
            self.0.with_node(index, f)
        } else if index < self.1.len() {
            self.1.with_node(index, f)
        } else {
            assert_bounds(self.len(), index);
            unreachable!()
        }
    }

    fn with_node_mut<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        if index < self.0.len() {
            self.0.with_node_mut(index, f)
        } else if index < self.1.len() {
            self.1.with_node_mut(index, f)
        } else {
            assert_bounds(self.len(), index);
            unreachable!()
        }
    }

    fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(usize, &BoxedUiNode) -> bool,
    {
        let mut continue_iter = true;
        self.0.for_each(|i, n| {
            continue_iter = f(i, n);
            continue_iter
        });

        if continue_iter {
            let offset = self.0.len();
            self.1.for_each(move |i, n| f(i + offset, n))
        }
    }

    fn for_each_mut<F>(&mut self, f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode) -> bool,
    {
        let mut continue_iter = true;
        self.0.for_each_mut(|i, n| {
            continue_iter = f(i, n);
            continue_iter
        });

        if continue_iter {
            let offset = self.0.len();
            self.1.for_each_mut(move |i, n| f(i + offset, n))
        }
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

    fn init_all(&mut self, ctx: &mut WidgetContext) {
        self.0.init_all(ctx);
        self.1.init_all(ctx);
    }

    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        self.0.deinit_all(ctx);
        self.1.deinit_all(ctx);
    }

    fn event_all(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
        self.0.event_all(ctx, update);
        self.1.event_all(ctx, update);
    }

    fn update_all(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        self.0.update_all(ctx, updates, observer);
        self.1.update_all(ctx, updates, &mut OffsetUiListObserver(self.0.len(), observer));
    }
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
    S: Fn(&BoxedUiNode, &BoxedUiNode) -> Ordering + 'static,
{
    list: L,

    map: RefCell<Vec<usize>>,
    sort: S,
}
impl<L, S> SortingList<L, S>
where
    L: UiNodeList,
    S: Fn(&BoxedUiNode, &BoxedUiNode) -> Ordering + 'static,
{
    /// New from list and sort function.
    pub fn new(list: L, sort: S) -> Self {
        Self {
            list,
            map: RefCell::new(vec![]),
            sort,
        }
    }

    fn update_map(&self) {
        let mut map = self.map.borrow_mut();
        let len = self.list.len();

        if len == 0 {
            map.clear();
        } else if map.len() != len {
            map.clear();
            map.extend(0..len);
            map.sort_by(|&a, &b| self.list.with_node(a, |a| self.list.with_node(b, |b| (self.sort)(a, b))))
        }
    }

    /// Reference the inner list.
    pub fn list(&self) -> &L {
        &self.list
    }
}
impl<L, S> UiNodeList for SortingList<L, S>
where
    L: UiNodeList,
    S: Fn(&BoxedUiNode, &BoxedUiNode) -> Ordering + 'static,
{
    fn with_node<R, F>(&self, index: usize, f: F) -> R
    where
        F: FnOnce(&BoxedUiNode) -> R,
    {
        self.update_map();
        let index = self.map.borrow()[index];
        self.list.with_node(index, f)
    }

    fn with_node_mut<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        self.update_map();
        let index = self.map.borrow()[index];
        self.list.with_node_mut(index, f)
    }

    fn for_each<F>(&self, f: F)
    where
        F: FnMut(usize, &BoxedUiNode) -> bool,
    {
        self.update_map();
        for (index, map) in self.map.borrow().iter().enumerate() {
            if !self.list.with_node(*map, |n| f(index, n)) {
                break;
            }
        }
    }

    fn for_each_mut<F>(&mut self, f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode) -> bool,
    {
        self.update_map();
        for (index, map) in self.map.borrow().iter().enumerate() {
            if !self.list.with_node_mut(*map, |n| f(index, n)) {
                break;
            }
        }
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
        self.map.get_mut().clear();
    }

    fn init_all(&mut self, ctx: &mut WidgetContext) {
        self.map.get_mut().clear();
        self.list.init_all(ctx);
    }

    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        self.list.deinit_all(ctx);
    }

    fn event_all(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
        self.list.event_all(ctx, update);
    }

    fn update_all(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        let mut changed = false;
        self.list.update_all(ctx, updates, &mut (observer, &mut changed as _));
        if changed {
            self.map.get_mut().clear();
        }
    }
}

/// Represents a [`z_index`] map that panel widgets can use to render in the configured order.
#[derive(Default)]
pub struct ZSort {
    map: RefCell<Vec<u64>>,
    naturally_sorted: Cell<bool>,
}
impl ZSort {
    /// New default.
    pub fn new() -> Self {
        Self::default()
    }

    fn sort(&self, list: &impl UiNodeList) {
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

        let len = list.len();
        assert!(len <= u32::MAX as usize);

        let mut prev_z = ZIndex::BACK;
        let mut need_map = false;
        let mut z_and_i = Vec::with_capacity(len);
        let mut has_non_default_zs = false;

        list.for_each(|i, node| {
            let z = ZIndex::get(node);
            z_and_i.push(((z.0 as u64) << 32) | i as u64);

            need_map |= z < prev_z;
            has_non_default_zs |= z != ZIndex::DEFAULT;
            prev_z = z;

            true
        });

        self.naturally_sorted.set(!need_map);

        if need_map {
            z_and_i.sort_unstable();

            for z in &mut z_and_i {
                *z &= u32::MAX as u64;
            }

            *self.map.borrow_mut() = z_and_i;
        } else {
            self.map.borrow_mut().clear();
        }
    }

    /// Init the `list` and invalidates the sort.
    pub fn init_all(&mut self, ctx: &mut WidgetContext, list: &mut impl UiNodeList) {
        self.map.get_mut().clear();
        let resort = ZIndexContext::with(ctx.path.widget_id(), || list.init_all(ctx));
        self.naturally_sorted.set(!resort);
    }

    /// Update the `list` and invalidates the sort if needed.
    pub fn update_all(
        &mut self,
        ctx: &mut WidgetContext,
        list: &mut impl UiNodeList,
        updates: &mut WidgetUpdates,
        observer: &mut dyn UiNodeListObserver,
    ) {
        let mut changed = false;

        let resort = ZIndexContext::with(ctx.path.widget_id(), || {
            list.update_all(ctx, updates, &mut (observer, &mut changed as _))
        });
        if resort || (changed && self.naturally_sorted.get()) {
            self.map.get_mut().clear();
            self.naturally_sorted.set(false);
            ctx.updates.render();
        }
    }

    /// Gets the `index` sorted in the `list`.
    pub fn map(&self, list: &impl UiNodeList, index: usize) -> usize {
        if self.naturally_sorted.get() {
            return index;
        }

        if self.map.borrow().len() != list.len() {
            self.sort(list);
        }

        self.map.borrow()[index] as usize
    }
}

static Z_INDEX_ID: StaticStateId<ZIndex> = StaticStateId::new_unique();

#[derive(Default, Clone, Debug)]
struct ZIndexContext {
    // used in `z_index` to validate that it will have an effect.
    panel_id: Option<WidgetId>,
    // set by `z_index` to signal a z-resort is needed.
    resort: bool,
}
impl ZIndexContext {
    fn with(panel_id: WidgetId, action: impl FnOnce()) -> bool {
        let ctx = ZIndexContext {
            panel_id: Some(panel_id),
            resort: false,
        };
        Z_INDEX.with_context(&mut Some(ctx), || {
            action();
            Z_INDEX.get().resort
        })
    }
}
context_value! {
    static Z_INDEX: ZIndexContext = ZIndexContext::default();
}

/// Position of a widget inside a [`WidgetList`] render operation.
///
/// When two widgets have the same index their logical position defines the render order.
///
/// # Examples
///
/// Create a Z-index that causes the widget to render in front of all siblings that don't set Z-index.
///
/// ```
/// # use zero_ui_core::ui_list::ZIndex;
///
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

    /// Gets the index set on a widget.
    ///
    /// Returns `DEFAULT` if the node is not an widget.
    pub fn get(widget: &impl UiNode) -> ZIndex {
        widget
            .with_context(|ctx| ctx.widget_state.copy(&Z_INDEX_ID).unwrap_or_default())
            .unwrap_or_default()
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
impl<Z: Into<ZIndex>> ops::Sub<Z> for ZIndex {
    type Output = Self;

    fn sub(self, rhs: Z) -> Self::Output {
        self.saturating_sub(rhs)
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
/// see [`ZSorted`] for more details.
#[property(context, default(ZIndex::DEFAULT))]
pub fn z_index(child: impl UiNode, index: impl IntoVar<ZIndex>) -> impl UiNode {
    #[ui_node(struct ZIndexNode {
        child: impl UiNode,
        #[var] index: impl Var<ZIndex>,
        valid: bool,
    })]
    impl UiNode for ZIndexNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            Z_INDEX.with_mut(|z_ctx| {
                if z_ctx.panel_id != ctx.path.ancestors().next() || z_ctx.panel_id.is_none() {
                    tracing::error!(
                        "property `z_index` set for `{}` but it is not the direct child of a Z-sorting panel",
                        ctx.path.widget_id()
                    );
                    self.valid = false;
                } else {
                    self.valid = true;
                    self.init_handles(ctx);

                    let index = self.index.get();
                    if index != ZIndex::DEFAULT {
                        z_ctx.resort = true;
                        ctx.widget_state.set(&Z_INDEX_ID, self.index.get());
                    }
                }
            });
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.valid {
                if let Some(i) = self.index.get_new(ctx) {
                    Z_INDEX.with_mut(|z_ctx| {
                        debug_assert_eq!(z_ctx.panel_id, ctx.path.ancestors().next());
                        z_ctx.resort = true;
                    });
                    ctx.widget_state.set(&Z_INDEX_ID, i);
                }
            }

            self.child.update(ctx, updates);
        }
    }
    ZIndexNode {
        child,
        index: index.into_var(),
        valid: false,
    }
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
    /// Large changes made to the list.
    fn reseted(&mut self);
    /// Widget inserted at the `index`.
    fn inserted(&mut self, index: usize);
    /// Widget removed from the `index`.
    fn removed(&mut self, index: usize);
    /// Widget removed and re-inserted.
    fn moved(&mut self, removed_index: usize, inserted_index: usize);
}
/// Does nothing.
impl UiNodeListObserver for () {
    fn reseted(&mut self) {}

    fn inserted(&mut self, _: usize) {}

    fn removed(&mut self, _: usize) {}

    fn moved(&mut self, _: usize, _: usize) {}
}
/// Sets to `true` for any change.
impl UiNodeListObserver for bool {
    fn reseted(&mut self) {
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

/// Represents an [`UiListObserver`] that applies an offset to all indexes.
///
/// This type is useful for implementing [`UiNodeList`] that are composed of other lists.
pub struct OffsetUiListObserver<'o>(pub usize, pub &'o mut dyn UiNodeListObserver);
impl<'o> UiNodeListObserver for OffsetUiListObserver<'o> {
    fn reseted(&mut self) {
        self.1.reseted()
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
    fn reseted(&mut self) {
        self.0.reseted();
        self.1.reseted();
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
