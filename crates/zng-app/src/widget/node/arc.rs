use std::sync::{Arc, Weak};

use crate::{
    event::{Event, EventArgs},
    update::UPDATES,
    var::*,
    widget::{WidgetHandlesCtx, WidgetId, WidgetUpdateMode},
};

type SlotId = usize;

struct SlotData<U> {
    item: Mutex<U>,
    slots: Mutex<SlotsData<U>>,
}
struct SlotsData<U> {
    // id of the next slot created.
    next_slot: SlotId,

    // slot and context where the node is inited.
    owner: Option<(SlotId, WidgetId)>,
    // slot and context that has requested ownership.
    move_request: Option<(SlotId, WidgetId)>,

    // node instance that must replace the current in the active slot.
    replacement: Option<U>,
}
impl<U> SlotsData<U> {
    fn next_slot(&mut self) -> SlotId {
        let r = self.next_slot;
        self.next_slot = self.next_slot.wrapping_add(1);
        r
    }
}
impl<U> Default for SlotsData<U> {
    fn default() -> Self {
        Self {
            next_slot: Default::default(),
            owner: Default::default(),
            move_request: Default::default(),
            replacement: Default::default(),
        }
    }
}

/// A reference counted [`UiNode`].
///
/// Nodes can only be used in one place at a time, this `struct` allows the
/// creation of ***slots*** that are [`UiNode`] implementers that can ***exclusive take*** the
/// referenced node as its child.
///
/// When a slot takes the node it is deinited in the previous place and reinited in the slot place.
///
/// Slots hold a strong reference to the node when they have it as their child and a weak reference when they don't.
pub struct ArcNode<U: UiNode>(Arc<SlotData<U>>);
impl<U: UiNode> Clone for ArcNode<U> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<U: UiNode> ArcNode<U> {
    /// New node.
    pub fn new(node: U) -> Self {
        ArcNode(Arc::new(SlotData {
            item: Mutex::new(node),
            slots: Mutex::default(),
        }))
    }

    /// New node that contains a weak reference to itself.
    ///
    /// Note that the weak reference cannot be [upgraded](WeakNode::upgrade) during the call to `node`.
    pub fn new_cyclic(node: impl FnOnce(WeakNode<U>) -> U) -> Self {
        Self(Arc::new_cyclic(|wk| {
            let node = node(WeakNode(wk.clone()));
            SlotData {
                item: Mutex::new(node),
                slots: Mutex::default(),
            }
        }))
    }

    /// Creates a [`WeakNode<U>`] reference to this node.
    pub fn downgrade(&self) -> WeakNode<U> {
        WeakNode(Arc::downgrade(&self.0))
    }

    /// Replace the current node with the `new_node` in the current slot.
    ///
    /// The previous node is deinited and the `new_node` is inited.
    pub fn set(&self, new_node: U) {
        let mut slots = self.0.slots.lock();
        let slots = &mut *slots;
        if let Some((_, id)) = &slots.owner {
            // current node inited on a slot, signal it to replace.
            slots.replacement = Some(new_node);
            let _ = UPDATES.update(*id);
        } else {
            // node already not inited, just replace.
            *self.0.item.lock() = new_node;
        }
    }

    /// Create a slot node that takes ownership of this node when `var` updates to `true`.
    ///
    /// The slot node also takes ownership on init if the `var` is already `true`.
    pub fn take_when<I>(&self, var: I) -> TakeSlot<U, impls::TakeWhenVar<I::Var>>
    where
        I: IntoVar<bool>,
    {
        impls::TakeSlot {
            slot: self.0.slots.lock().next_slot(),
            rc: self.0.clone(),
            take: impls::TakeWhenVar { var: var.into_var() },
            delegate_init: |n| n.init(),
            delegate_deinit: |n| n.deinit(),
            wgt_handles: WidgetHandlesCtx::new(),
        }
    }

    /// Create a slot node that takes ownership of this node when `event` updates and `filter` returns `true`.
    ///
    /// The slot node also takes ownership on init if `take_on_init` is `true`.
    pub fn take_on<A, F>(&self, event: Event<A>, filter: F, take_on_init: bool) -> TakeSlot<U, impls::TakeOnEvent<A, F>>
    where
        A: EventArgs,
        F: FnMut(&A) -> bool + Send + 'static,
    {
        impls::TakeSlot {
            slot: self.0.slots.lock().next_slot(),
            rc: self.0.clone(),
            take: impls::TakeOnEvent {
                event,
                filter,
                take_on_init,
            },
            delegate_init: |n| n.init(),
            delegate_deinit: |n| n.deinit(),
            wgt_handles: WidgetHandlesCtx::new(),
        }
    }

    /// Create a slot node that takes ownership of this node as soon as the node is inited.
    ///
    /// This is equivalent to `self.take_when(true)`.
    pub fn take_on_init(&self) -> TakeSlot<U, impls::TakeWhenVar<zng_var::LocalVar<bool>>> {
        self.take_when(true)
    }

    /// Calls `f` in the context of the node, if it can be locked and is a full widget.
    pub fn try_context<R>(&self, update_mode: WidgetUpdateMode, f: impl FnOnce() -> R) -> Option<R> {
        self.0.item.try_lock()?.with_context(update_mode, f)
    }
}

/// Weak reference to a [`ArcNode<U>`].
pub struct WeakNode<U: UiNode>(Weak<SlotData<U>>);
impl<U: UiNode> Clone for WeakNode<U> {
    fn clone(&self) -> Self {
        Self(Weak::clone(&self.0))
    }
}
impl<U: UiNode> WeakNode<U> {
    /// Attempts to upgrade to a [`ArcNode<U>`].
    pub fn upgrade(&self) -> Option<ArcNode<U>> {
        self.0.upgrade().map(ArcNode)
    }
}

/// A reference counted [`UiNodeList`].
///
/// Nodes can only be used in one place at a time, this `struct` allows the
/// creation of ***slots*** that are [`UiNodeList`] implementers that can ***exclusive take*** the
/// referenced list as the children.
///
/// When a slot takes the list it is deinited in the previous place and reinited in the slot place.
///
/// Slots hold a strong reference to the list when they have it as their child and a weak reference when they don't.
pub struct ArcNodeList<L: UiNodeList>(Arc<SlotData<L>>);
impl<L: UiNodeList> Clone for ArcNodeList<L> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<L: UiNodeList> ArcNodeList<L> {
    /// New list.
    pub fn new(list: L) -> Self {
        ArcNodeList(Arc::new(SlotData {
            item: Mutex::new(list),
            slots: Mutex::default(),
        }))
    }

    /// New rc list that contains a weak reference to itself.
    ///
    /// Note that the weak reference cannot be [upgraded](WeakNodeList::upgrade) during the call to `list`.
    pub fn new_cyclic(list: impl FnOnce(WeakNodeList<L>) -> L) -> Self {
        Self(Arc::new_cyclic(|wk| {
            let list = list(WeakNodeList(wk.clone()));
            SlotData {
                item: Mutex::new(list),
                slots: Mutex::default(),
            }
        }))
    }

    /// Creates a [`WeakNodeList<L>`] reference to this list.
    pub fn downgrade(&self) -> WeakNodeList<L> {
        WeakNodeList(Arc::downgrade(&self.0))
    }

    /// Replace the current list with the `new_list` in the current slot.
    ///
    /// The previous list is deinited and the `new_list` is inited.
    pub fn set(&self, new_list: L) {
        let mut slots = self.0.slots.lock();
        let slots = &mut *slots;
        if let Some((_, id)) = &slots.owner {
            // current node inited on a slot, signal it to replace.
            slots.replacement = Some(new_list);
            UPDATES.update(*id);
        } else {
            // node already not inited, just replace.
            *self.0.item.lock() = new_list;
        }
    }

    /// Create a slot list that takes ownership of this list when `var` updates to `true`.
    ///
    /// The slot node also takes ownership on init if the `var` is already `true`.
    ///
    /// The return type implements [`UiNodeList`].
    pub fn take_when(&self, var: impl IntoVar<bool>) -> TakeSlot<L, impl TakeOn> {
        impls::TakeSlot {
            slot: self.0.slots.lock().next_slot(),
            rc: self.0.clone(),
            take: impls::TakeWhenVar { var: var.into_var() },
            delegate_init: |n| n.init_all(),
            delegate_deinit: |n| n.deinit_all(),
            wgt_handles: WidgetHandlesCtx::new(),
        }
    }

    /// Create a slot list that takes ownership of this list when `event` updates and `filter` returns `true`.
    ///
    /// The slot list also takes ownership on init if `take_on_init` is `true`.
    ///
    /// The return type implements [`UiNodeList`].
    pub fn take_on<A: EventArgs>(
        &self,
        event: Event<A>,
        filter: impl FnMut(&A) -> bool + Send + 'static,
        take_on_init: bool,
    ) -> TakeSlot<L, impl TakeOn> {
        impls::TakeSlot {
            slot: self.0.slots.lock().next_slot(),
            rc: self.0.clone(),
            take: impls::TakeOnEvent {
                event,
                filter,
                take_on_init,
            },
            delegate_init: |n| n.init_all(),
            delegate_deinit: |n| n.deinit_all(),
            wgt_handles: WidgetHandlesCtx::new(),
        }
    }

    /// Create a slot node list that takes ownership of this list as soon as the node is inited.
    ///
    /// This is equivalent to `self.take_when(true)`.
    pub fn take_on_init(&self) -> TakeSlot<L, impl TakeOn> {
        self.take_when(true)
    }

    /// Iterate over widget contexts.
    pub fn for_each_ctx(&self, update_mode: WidgetUpdateMode, mut f: impl FnMut(usize)) {
        self.0.item.lock().for_each(|i, n| {
            n.with_context(update_mode, || f(i));
        })
    }
}

/// Weak reference to a [`ArcNodeList<U>`].
pub struct WeakNodeList<L: UiNodeList>(Weak<SlotData<L>>);
impl<L: UiNodeList> Clone for WeakNodeList<L> {
    fn clone(&self) -> Self {
        Self(Weak::clone(&self.0))
    }
}
impl<L: UiNodeList> WeakNodeList<L> {
    /// Attempts to upgrade to a [`ArcNodeList<U>`].
    pub fn upgrade(&self) -> Option<ArcNodeList<L>> {
        self.0.upgrade().map(ArcNodeList)
    }
}

pub use impls::*;
use parking_lot::Mutex;

use super::{UiNode, UiNodeList};

mod impls {
    use std::sync::Arc;

    use zng_layout::unit::PxSize;
    use zng_var::Var;

    use crate::{
        event::{Event, EventArgs},
        render::{FrameBuilder, FrameUpdate},
        update::{EventUpdate, UPDATES, WidgetUpdates},
        widget::{
            WIDGET, WidgetHandlesCtx, WidgetUpdateMode,
            info::{WidgetInfoBuilder, WidgetLayout, WidgetMeasure},
            node::{BoxedUiNode, BoxedUiNodeList, UiNode, UiNodeList, UiNodeListObserver},
        },
    };

    use super::{SlotData, SlotId};

    #[doc(hidden)]
    pub trait TakeOn: Send + 'static {
        fn take_on_init(&mut self) -> bool {
            false
        }

        fn take_on_event(&mut self, update: &EventUpdate) -> bool {
            let _ = update;
            false
        }

        fn take_on_update(&mut self, updates: &WidgetUpdates) -> bool {
            let _ = updates;
            false
        }
    }

    #[doc(hidden)]
    pub struct TakeWhenVar<V: Var<bool>> {
        pub(super) var: V,
    }
    impl<V: Var<bool>> TakeOn for TakeWhenVar<V> {
        fn take_on_init(&mut self) -> bool {
            WIDGET.sub_var(&self.var);
            self.var.get()
        }

        fn take_on_update(&mut self, _: &WidgetUpdates) -> bool {
            self.var.get_new().unwrap_or(false)
        }
    }

    #[doc(hidden)]
    pub struct TakeOnEvent<A: EventArgs, F: FnMut(&A) -> bool + Send + 'static> {
        pub(super) event: Event<A>,
        pub(super) filter: F,
        pub(super) take_on_init: bool,
    }
    impl<A: EventArgs, F: FnMut(&A) -> bool + Send + Send + 'static> TakeOn for TakeOnEvent<A, F> {
        fn take_on_init(&mut self) -> bool {
            WIDGET.sub_event(&self.event);
            self.take_on_init
        }

        fn take_on_event(&mut self, update: &EventUpdate) -> bool {
            if let Some(args) = self.event.on(update) {
                (self.filter)(args)
            } else {
                false
            }
        }
    }

    #[doc(hidden)]
    pub struct TakeSlot<U, T: TakeOn> {
        pub(super) slot: SlotId,
        pub(super) rc: Arc<SlotData<U>>,
        pub(super) take: T,

        pub(super) delegate_init: fn(&mut U),
        pub(super) delegate_deinit: fn(&mut U),
        pub(super) wgt_handles: WidgetHandlesCtx,
    }
    impl<U, T: TakeOn> TakeSlot<U, T> {
        fn on_init(&mut self) {
            if self.take.take_on_init() {
                self.take();
            }
        }

        fn on_deinit(&mut self) {
            let mut was_owner = false;
            {
                let mut slots = self.rc.slots.lock();
                let slots = &mut *slots;
                if let Some((slot, _)) = &slots.owner {
                    if *slot == self.slot {
                        slots.owner = None;
                        was_owner = true;
                    }
                }
            }

            if was_owner {
                WIDGET.with_handles(&mut self.wgt_handles, || (self.delegate_deinit)(&mut *self.rc.item.lock()));
            }

            self.wgt_handles.clear();
        }

        fn on_event(&mut self, update: &EventUpdate) {
            if !self.is_owner() && self.take.take_on_event(update) {
                // request ownership.
                self.take();
            }
        }

        fn on_update(&mut self, updates: &WidgetUpdates) {
            if self.is_owner() {
                let mut slots = self.rc.slots.lock();
                if let Some((_, id)) = slots.move_request {
                    // deinit to move to other slot.

                    let replacement = slots.replacement.take();
                    slots.owner = None;

                    drop(slots);

                    let mut node = self.rc.item.lock();
                    (self.delegate_deinit)(&mut node);

                    WIDGET.update_info().layout().render();

                    if let Some(new) = replacement {
                        *node = new;
                    }

                    UPDATES.update(id);
                } else if let Some(mut new) = slots.replacement.take() {
                    // apply replacement.

                    drop(slots);

                    let mut node = self.rc.item.lock();
                    WIDGET.with_handles(&mut self.wgt_handles, || {
                        (self.delegate_deinit)(&mut node);
                    });
                    self.wgt_handles.clear();

                    WIDGET.with_handles(&mut self.wgt_handles, || {
                        (self.delegate_init)(&mut new);
                    });
                    *node = new;

                    WIDGET.update_info().layout().render();
                }
            } else if self.take.take_on_update(updates) {
                // request ownership.
                self.take();
            } else {
                let mut slots = self.rc.slots.lock();
                if let Some((slot, _)) = &slots.move_request {
                    if *slot == self.slot && slots.owner.is_none() {
                        slots.move_request = None;
                        // requested move in prev update, now can take ownership.
                        drop(slots);
                        self.take();
                    }
                }
            }
        }

        fn take(&mut self) {
            {
                let mut slots = self.rc.slots.lock();
                let slots = &mut *slots;
                if let Some((sl, id)) = &slots.owner {
                    if *sl != self.slot {
                        // currently inited in another slot, signal it to deinit.
                        slots.move_request = Some((self.slot, WIDGET.id()));
                        UPDATES.update(*id);
                    }
                } else {
                    // no current owner, take ownership immediately.
                    slots.owner = Some((self.slot, WIDGET.id()));
                }
            }

            if self.is_owner() {
                WIDGET.with_handles(&mut self.wgt_handles, || {
                    (self.delegate_init)(&mut *self.rc.item.lock());
                });
                WIDGET.update_info().layout().render();
            }
        }

        fn is_owner(&self) -> bool {
            self.rc.slots.lock().owner.as_ref().map(|(sl, _)| *sl == self.slot).unwrap_or(false)
        }

        fn delegate_owned<R>(&self, del: impl FnOnce(&U) -> R) -> Option<R> {
            if self.is_owner() { Some(del(&*self.rc.item.lock())) } else { None }
        }
        fn delegate_owned_mut<R>(&mut self, del: impl FnOnce(&mut U) -> R) -> Option<R> {
            if self.is_owner() {
                Some(del(&mut *self.rc.item.lock()))
            } else {
                None
            }
        }

        fn delegate_owned_mut_with_handles<R>(&mut self, del: impl FnOnce(&mut U) -> R) -> Option<R> {
            if self.is_owner() {
                WIDGET.with_handles(&mut self.wgt_handles, || Some(del(&mut *self.rc.item.lock())))
            } else {
                None
            }
        }
    }

    impl<U: UiNode, T: TakeOn> UiNode for TakeSlot<U, T> {
        fn init(&mut self) {
            self.on_init();
        }

        fn deinit(&mut self) {
            self.on_deinit();
        }

        fn info(&mut self, info: &mut WidgetInfoBuilder) {
            self.delegate_owned_mut(|n| n.info(info));
        }

        fn event(&mut self, update: &EventUpdate) {
            self.delegate_owned_mut_with_handles(|n| n.event(update));
            self.on_event(update);
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            self.delegate_owned_mut_with_handles(|n| n.update(updates));
            self.on_update(updates);
        }

        fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
            self.delegate_owned_mut(|n| n.measure(wm)).unwrap_or_default()
        }

        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            self.delegate_owned_mut(|n| n.layout(wl)).unwrap_or_default()
        }

        fn render(&mut self, frame: &mut FrameBuilder) {
            self.delegate_owned_mut(|n| n.render(frame));
        }

        fn render_update(&mut self, update: &mut FrameUpdate) {
            self.delegate_owned_mut(|n| n.render_update(update));
        }

        fn is_widget(&self) -> bool {
            self.delegate_owned(UiNode::is_widget).unwrap_or(false)
        }

        fn with_context<R, F>(&mut self, update_mode: WidgetUpdateMode, f: F) -> Option<R>
        where
            F: FnOnce() -> R,
        {
            self.delegate_owned_mut(|n| n.with_context(update_mode, f)).flatten()
        }
    }

    impl<U: UiNodeList, T: TakeOn> UiNodeList for TakeSlot<U, T> {
        fn with_node<R, F>(&mut self, index: usize, f: F) -> R
        where
            F: FnOnce(&mut BoxedUiNode) -> R,
        {
            self.delegate_owned_mut(move |l| l.with_node(index, f))
                .unwrap_or_else(|| panic!("index `{index}` is >= len `0`"))
        }

        fn for_each<F>(&mut self, f: F)
        where
            F: FnMut(usize, &mut BoxedUiNode),
        {
            self.delegate_owned_mut(|l| l.for_each(f));
        }

        fn par_each<F>(&mut self, f: F)
        where
            F: Fn(usize, &mut BoxedUiNode) + Send + Sync,
        {
            self.delegate_owned_mut(|l| l.par_each(f));
        }

        fn par_fold_reduce<TF, I, F, R>(&mut self, identity: I, fold: F, reduce: R) -> TF
        where
            TF: Send + 'static,
            I: Fn() -> TF + Send + Sync,
            F: Fn(TF, usize, &mut BoxedUiNode) -> TF + Send + Sync,
            R: Fn(TF, TF) -> TF + Send + Sync,
        {
            self.delegate_owned_mut(|l| l.par_fold_reduce(&identity, fold, reduce))
                .unwrap_or_else(identity)
        }

        fn len(&self) -> usize {
            self.delegate_owned(UiNodeList::len).unwrap_or(0)
        }

        fn boxed(self) -> BoxedUiNodeList {
            Box::new(self)
        }

        fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
            self.delegate_owned_mut(|l| l.drain_into(vec));
        }

        fn init_all(&mut self) {
            self.on_init();
            // delegation done in the handler
        }

        fn deinit_all(&mut self) {
            self.on_deinit();
            // delegation done in the handler
        }

        fn info_all(&mut self, info: &mut WidgetInfoBuilder) {
            self.delegate_owned_mut_with_handles(|l| l.info_all(info));
        }

        fn event_all(&mut self, update: &EventUpdate) {
            self.delegate_owned_mut_with_handles(|l| l.event_all(update));
            self.on_event(update);
        }

        fn update_all(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
            let _ = observer;
            self.delegate_owned_mut_with_handles(|l| l.update_all(updates, observer));
            self.on_update(updates);
        }

        fn render_all(&mut self, frame: &mut FrameBuilder) {
            self.delegate_owned_mut(|l| l.render_all(frame));
        }

        fn render_update_all(&mut self, update: &mut FrameUpdate) {
            self.delegate_owned_mut(|l| l.render_update_all(update));
        }
    }
}
