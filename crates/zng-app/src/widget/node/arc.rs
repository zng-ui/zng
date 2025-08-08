use std::sync::{Arc, Weak};

use crate::{
    event::{Event, EventArgs},
    update::UPDATES,
    var::*,
    widget::{WidgetHandlesCtx, WidgetId, WidgetUpdateMode, node::IntoUiNode},
};

type SlotId = usize;

struct SlotData {
    item: Mutex<UiNode>,
    slots: Mutex<SlotsData>,
}
struct SlotsData {
    // id of the next slot created.
    next_slot: SlotId,

    // slot and context where the node is inited.
    owner: Option<(SlotId, WidgetId)>,
    // slot and context that has requested ownership.
    move_request: Option<(SlotId, WidgetId)>,

    // node instance that must replace the current in the active slot.
    replacement: Option<UiNode>,
}
impl SlotsData {
    fn next_slot(&mut self) -> SlotId {
        let r = self.next_slot;
        self.next_slot = self.next_slot.wrapping_add(1);
        r
    }
}
impl Default for SlotsData {
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
pub struct ArcNode(Arc<SlotData>);
impl Clone for ArcNode {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl ArcNode {
    /// New node.
    pub fn new(node: impl IntoUiNode) -> Self {
        Self::new_impl(node.into_node())
    }
    fn new_impl(node: UiNode) -> Self {
        ArcNode(Arc::new(SlotData {
            item: Mutex::new(node),
            slots: Mutex::default(),
        }))
    }

    /// New node that contains a weak reference to itself.
    ///
    /// Note that the weak reference cannot be [upgraded](WeakNode::upgrade) during the call to `node`.
    pub fn new_cyclic(node: impl FnOnce(WeakNode) -> UiNode) -> Self {
        Self(Arc::new_cyclic(|wk| {
            let node = node(WeakNode(wk.clone()));
            SlotData {
                item: Mutex::new(node),
                slots: Mutex::default(),
            }
        }))
    }

    /// Creates a [`WeakNode`] reference to this node.
    pub fn downgrade(&self) -> WeakNode {
        WeakNode(Arc::downgrade(&self.0))
    }

    /// Replace the current node with the `new_node` in the current slot.
    ///
    /// The previous node is deinited and the `new_node` is inited.
    pub fn set(&self, new_node: impl IntoUiNode) {
        self.set_impl(new_node.into_node())
    }
    fn set_impl(&self, new_node: UiNode) {
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
    pub fn take_when(&self, var: impl IntoVar<bool>) -> UiNode {
        self.take_when_impl(var.into_var())
    }
    fn take_when_impl(&self, var: Var<bool>) -> UiNode {
        impls::TakeSlot {
            slot: self.0.slots.lock().next_slot(),
            rc: self.0.clone(),
            take: impls::TakeWhenVar { var: var.into_var() },
            wgt_handles: WidgetHandlesCtx::new(),
        }
        .into_node()
    }

    /// Create a slot node that takes ownership of this node when `event` updates and `filter` returns `true`.
    ///
    /// The slot node also takes ownership on init if `take_on_init` is `true`.
    pub fn take_on<A, F>(&self, event: Event<A>, filter: F, take_on_init: bool) -> UiNode
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
            wgt_handles: WidgetHandlesCtx::new(),
        }
        .into_node()
    }

    /// Create a slot node that takes ownership of this node as soon as the node is inited.
    ///
    /// This is equivalent to `self.take_when(true)`.
    pub fn take_on_init(&self) -> UiNode {
        self.take_when(true)
    }

    /// Calls `f` in the context of the node, if it is a full widget.
    pub fn try_context<R>(&self, update_mode: WidgetUpdateMode, f: impl FnOnce() -> R) -> Option<R> {
        Some(self.0.item.try_lock()?.as_widget()?.with_context(update_mode, f))
    }
}

/// Weak reference to a [`ArcNode`].
pub struct WeakNode(Weak<SlotData>);
impl Clone for WeakNode {
    fn clone(&self) -> Self {
        Self(Weak::clone(&self.0))
    }
}
impl WeakNode {
    /// Attempts to upgrade to a [`ArcNode`].
    pub fn upgrade(&self) -> Option<ArcNode> {
        self.0.upgrade().map(ArcNode)
    }
}

use parking_lot::Mutex;

use super::UiNode;

mod impls {
    use std::sync::Arc;

    use zng_layout::unit::PxSize;
    use zng_var::Var;

    use crate::{
        event::{Event, EventArgs},
        render::{FrameBuilder, FrameUpdate},
        update::{EventUpdate, UPDATES, WidgetUpdates},
        widget::{
            WIDGET, WidgetHandlesCtx,
            info::{WidgetInfoBuilder, WidgetLayout, WidgetMeasure},
            node::{UiNode, UiNodeImpl},
        },
    };

    use super::{SlotData, SlotId};

    pub(super) trait TakeOn: Send + 'static {
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

    pub(super) struct TakeWhenVar {
        pub(super) var: Var<bool>,
    }
    impl TakeOn for TakeWhenVar {
        fn take_on_init(&mut self) -> bool {
            WIDGET.sub_var(&self.var);
            self.var.get()
        }

        fn take_on_update(&mut self, _: &WidgetUpdates) -> bool {
            self.var.get_new().unwrap_or(false)
        }
    }

    pub(super) struct TakeOnEvent<A: EventArgs, F: FnMut(&A) -> bool + Send + 'static> {
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

    pub(super) struct TakeSlot<T: TakeOn> {
        pub(super) slot: SlotId,
        pub(super) rc: Arc<SlotData>,
        pub(super) take: T,

        pub(super) wgt_handles: WidgetHandlesCtx,
    }
    impl<T: TakeOn> TakeSlot<T> {
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
                WIDGET.with_handles(&mut self.wgt_handles, || self.rc.item.lock().deinit());
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
                    node.deinit();

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
                        node.deinit();
                    });
                    self.wgt_handles.clear();

                    WIDGET.with_handles(&mut self.wgt_handles, || {
                        new.init();
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
                    self.rc.item.lock().init();
                });
                WIDGET.update_info().layout().render();
            }
        }

        fn is_owner(&self) -> bool {
            self.rc.slots.lock().owner.as_ref().map(|(sl, _)| *sl == self.slot).unwrap_or(false)
        }

        fn delegate_owned<R>(&self, del: impl FnOnce(&UiNode) -> R) -> Option<R> {
            if self.is_owner() { Some(del(&*self.rc.item.lock())) } else { None }
        }
        fn delegate_owned_mut<R>(&mut self, del: impl FnOnce(&mut UiNode) -> R) -> Option<R> {
            if self.is_owner() {
                Some(del(&mut *self.rc.item.lock()))
            } else {
                None
            }
        }

        fn delegate_owned_mut_with_handles<R>(&mut self, del: impl FnOnce(&mut UiNode) -> R) -> Option<R> {
            if self.is_owner() {
                WIDGET.with_handles(&mut self.wgt_handles, || Some(del(&mut *self.rc.item.lock())))
            } else {
                None
            }
        }
    }

    impl<T: TakeOn> UiNodeImpl for TakeSlot<T> {
        fn children_len(&self) -> usize {
            self.delegate_owned(|n| n.0.children_len()).unwrap_or(0)
        }

        fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
            self.delegate_owned_mut(|n| n.0.with_child(index, visitor));
        }

        fn init(&mut self) {
            self.on_init();
        }

        fn deinit(&mut self) {
            self.on_deinit();
        }

        fn info(&mut self, info: &mut WidgetInfoBuilder) {
            self.delegate_owned_mut(|n| n.0.info(info));
        }

        fn event(&mut self, update: &EventUpdate) {
            self.delegate_owned_mut_with_handles(|n| n.0.event(update));
            self.on_event(update);
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            self.delegate_owned_mut_with_handles(|n| n.0.update(updates));
            self.on_update(updates);
        }

        fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
            self.delegate_owned_mut(|n| n.0.measure(wm)).unwrap_or_default()
        }

        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            self.delegate_owned_mut(|n| n.0.layout(wl)).unwrap_or_default()
        }

        fn render(&mut self, frame: &mut FrameBuilder) {
            self.delegate_owned_mut(|n| n.0.render(frame));
        }

        fn render_update(&mut self, update: &mut FrameUpdate) {
            self.delegate_owned_mut(|n| n.0.render_update(update));
        }

        fn is_list(&self) -> bool {
            self.delegate_owned(|n| n.is_list()).unwrap_or(false)
        }

        fn for_each_child(&mut self, visitor: &mut dyn FnMut(usize, &mut UiNode)) {
            self.delegate_owned_mut(|n| n.0.for_each_child(visitor));
        }

        fn par_each_child(&mut self, visitor: &(dyn Fn(usize, &mut UiNode) + Sync)) {
            self.delegate_owned_mut(|n| n.0.par_each_child(visitor));
        }

        fn par_fold_reduce(
            &mut self,
            identity: zng_var::BoxAnyVarValue,
            fold: &(dyn Fn(zng_var::BoxAnyVarValue, usize, &mut UiNode) -> zng_var::BoxAnyVarValue + Sync),
            reduce: &(dyn Fn(zng_var::BoxAnyVarValue, zng_var::BoxAnyVarValue) -> zng_var::BoxAnyVarValue + Sync),
        ) -> zng_var::BoxAnyVarValue {
            self.delegate_owned_mut(|n| n.0.par_fold_reduce(identity.clone(), fold, reduce))
                .unwrap_or(identity)
        }

        fn as_widget(&mut self) -> Option<&mut dyn crate::widget::node::WidgetUiNodeImpl> {
            todo!("!!: TODO")
        }
    }
}
