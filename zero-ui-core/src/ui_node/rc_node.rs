use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::{
    app::AppEventSender,
    event::{Event, EventArgs},
    var::*,
    *,
};

type SlotId = usize;

struct SlotData<U> {
    item: RefCell<U>,
    slots: RefCell<SlotsData<U>>,
}
struct SlotsData<U> {
    // id of the next slot created.
    next_slot: SlotId,

    // slot and context where the node is inited.
    owner: Option<(SlotId, WidgetId, AppEventSender)>,
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
/// Nodes can only appear in one place of the UI tree at a time, this `struct` allows the
/// creation of ***slots*** that are [`UiNode`] implementers that can *exclusive take* the
/// referenced node as its child.
///
/// When a slot takes the node it is deinited in the previous UI tree place and reinited in the slot place.
///
/// Slots hold a strong reference to the node when they have it as their child and a weak reference when they don't.
pub struct RcNode<U: UiNode>(Rc<SlotData<U>>);
impl<U: UiNode> Clone for RcNode<U> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<U: UiNode> RcNode<U> {
    /// New node.
    pub fn new(node: U) -> Self {
        RcNode(Rc::new(SlotData {
            item: RefCell::new(node),
            slots: RefCell::default(),
        }))
    }

    /// New rc node that contains a weak reference to itself.
    ///
    /// **Note** the weak reference cannot be [upgraded](WeakNode::upgrade) during the call to `node`.
    pub fn new_cyclic(node: impl FnOnce(WeakNode<U>) -> U) -> Self {
        Self(Rc::new_cyclic(|wk| {
            let node = node(WeakNode(wk.clone()));
            SlotData {
                item: RefCell::new(node),
                slots: RefCell::default(),
            }
        }))
    }

    /// Creates a [`WeakNode<U>`] reference to this node.
    pub fn downgrade(&self) -> WeakNode<U> {
        WeakNode(Rc::downgrade(&self.0))
    }

    /// Replace the current node with the `new_node` in the current slot.
    ///
    /// The previous node is deinited and the `new_node` is inited.
    pub fn set(&self, new_node: U) {
        let mut slots = self.0.slots.borrow_mut();
        let slots = &mut *slots;
        if let Some((_, id, u)) = &slots.owner {
            // current node inited on a slot, signal it to replace.
            slots.replacement = Some(new_node);
            let _ = u.send_update(vec![*id]);
        } else {
            // node already not inited, just replace.
            *self.0.item.borrow_mut() = new_node;
        }
    }

    /// Create a *slot* node that takes ownership of this node when `var` updates to `true`.
    ///
    /// The slot node also takes ownership on init if the `var` is already `true`.
    pub fn take_when(&self, var: impl IntoVar<bool>) -> impl UiNode {
        impls::TakeOnSlot {
            slot: self.0.slots.borrow_mut().next_slot(),
            rc: self.0.clone(),
            take: impls::TakeOnVar { var: var.into_var() },
            delegate_init: |n, ctx| n.init(ctx),
            delegate_deinit: |n, ctx| n.deinit(ctx),
        }
    }

    /// Create a *slot* node that takes ownership of this node when `event` updates and `filter` returns `true`.
    ///
    /// The slot node also takes ownership on init if `take_on_init` is `true`.
    pub fn take_on<A: EventArgs>(&self, event: Event<A>, filter: impl FnMut(&A) -> bool + 'static, take_on_init: bool) -> impl UiNode {
        impls::TakeOnSlot {
            slot: self.0.slots.borrow_mut().next_slot(),
            rc: self.0.clone(),
            take: impls::TakeOnEvent {
                event,
                filter,
                take_on_init,
            },
            delegate_init: |n, ctx| n.init(ctx),
            delegate_deinit: |n, ctx| n.deinit(ctx),
        }
    }

    /// Create a *slot* node that takes ownership of this node as soon as the node is inited.
    ///
    /// This is equivalent to `self.take_when(true)`
    pub fn take_on_init(&self) -> impl UiNode {
        self.take_when(true)
    }
}

/// `Weak` reference to a [`RcNode<U>`].
pub struct WeakNode<U: UiNode>(Weak<SlotData<U>>);
impl<U: UiNode> Clone for WeakNode<U> {
    fn clone(&self) -> Self {
        Self(Weak::clone(&self.0))
    }
}
impl<U: UiNode> WeakNode<U> {
    /// Attempts to upgrade to a [`RcNode<U>`].
    pub fn upgrade(&self) -> Option<RcNode<U>> {
        self.0.upgrade().map(RcNode)
    }
}

/// A reference counted [`UiNodeList`].
///
/// Nodes can only appear in one place of the UI tree at a time, this `struct` allows the
/// creation of ***slots*** that are [`UiNodeList`] implementers that can *exclusive take* the
/// referenced list as the children.
///
/// When a slot takes the list it is deinited in the previous UI tree place and reinited in the slot place.
///
/// Slots hold a strong reference to the list when they have it as their child and a weak reference when they don't.
pub struct RcNodeList<L: UiNodeList>(Rc<SlotData<L>>);
impl<L: UiNodeList> Clone for RcNodeList<L> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<L: UiNodeList> RcNodeList<L> {
    /// New list.
    pub fn new(list: L) -> Self {
        RcNodeList(Rc::new(SlotData {
            item: RefCell::new(list),
            slots: RefCell::default(),
        }))
    }

    /// New rc list that contains a weak reference to itself.
    ///
    /// **Note** the weak reference cannot be [upgraded](WeakNodeList::upgrade) during the call to `list`.
    pub fn new_cyclic(list: impl FnOnce(WeakNodeList<L>) -> L) -> Self {
        Self(Rc::new_cyclic(|wk| {
            let list = list(WeakNodeList(wk.clone()));
            SlotData {
                item: RefCell::new(list),
                slots: RefCell::default(),
            }
        }))
    }

    /// Creates a [`WeakNodeList<L>`] reference to this list.
    pub fn downgrade(&self) -> WeakNodeList<L> {
        WeakNodeList(Rc::downgrade(&self.0))
    }

    /// Replace the current list with the `new_list` in the current slot.
    ///
    /// The previous list is deinited and the `new_list` is inited.
    pub fn set(&self, new_list: L) {
        let mut slots = self.0.slots.borrow_mut();
        let slots = &mut *slots;
        if let Some((_, id, u)) = &slots.owner {
            // current node inited on a slot, signal it to replace.
            slots.replacement = Some(new_list);
            let _ = u.send_update(vec![*id]);
        } else {
            // node already not inited, just replace.
            *self.0.item.borrow_mut() = new_list;
        }
    }

    /// Create a *slot* node that takes ownership of this node when `var` updates to `true`.
    ///
    /// The slot node also takes ownership on init if the `var` is already `true`.
    pub fn take_when(&self, var: impl IntoVar<bool>) -> impl UiNodeList {
        impls::TakeOnSlot {
            slot: self.0.slots.borrow_mut().next_slot(),
            rc: self.0.clone(),
            take: impls::TakeOnVar { var: var.into_var() },
            delegate_init: |n, ctx| n.init_all(ctx),
            delegate_deinit: |n, ctx| n.deinit_all(ctx),
        }
    }

    /// Create a *slot* node that takes ownership of this node when `event` updates and `filter` returns `true`.
    ///
    /// The slot node also takes ownership on init if `take_on_init` is `true`.
    pub fn take_on<A: EventArgs>(&self, event: Event<A>, filter: impl FnMut(&A) -> bool + 'static, take_on_init: bool) -> impl UiNodeList {
        impls::TakeOnSlot {
            slot: self.0.slots.borrow_mut().next_slot(),
            rc: self.0.clone(),
            take: impls::TakeOnEvent {
                event,
                filter,
                take_on_init,
            },
            delegate_init: |n, ctx| n.init_all(ctx),
            delegate_deinit: |n, ctx| n.deinit_all(ctx),
        }
    }

    /// Create a *slot* node list that takes ownership of this list as soon as the node is inited.
    ///
    /// This is equivalent to `self.take_when(true)`
    pub fn take_on_init(&self) -> impl UiNodeList {
        self.take_when(true)
    }
}

/// `Weak` reference to a [`RcNodeList<U>`].
pub struct WeakNodeList<L: UiNodeList>(Weak<SlotData<L>>);
impl<L: UiNodeList> Clone for WeakNodeList<L> {
    fn clone(&self) -> Self {
        Self(Weak::clone(&self.0))
    }
}
impl<L: UiNodeList> WeakNodeList<L> {
    /// Attempts to upgrade to a [`RcNodeList<U>`].
    pub fn upgrade(&self) -> Option<RcNodeList<L>> {
        self.0.upgrade().map(RcNodeList)
    }
}

mod impls {
    use std::rc::Rc;

    use crate::{
        context::*,
        event::{Event, EventArgs, EventUpdate},
        render::{FrameBuilder, FrameUpdate},
        units::PxSize,
        var::*,
        widget_info::{WidgetInfoBuilder, WidgetLayout},
        *,
    };

    use super::{SlotData, SlotId};

    pub(super) trait TakeOn: 'static {
        fn take_on_init(&mut self, ctx: &mut WidgetContext) -> bool {
            let _ = ctx;
            false
        }

        fn take_on_event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) -> bool {
            let _ = (ctx, update);
            false
        }

        fn take_on_update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) -> bool {
            let _ = (ctx, updates);
            false
        }
    }

    pub(super) struct TakeOnVar<V: Var<bool>> {
        pub(super) var: V,
    }
    impl<V: Var<bool>> TakeOn for TakeOnVar<V> {
        fn take_on_init(&mut self, ctx: &mut WidgetContext) -> bool {
            ctx.sub_var(&self.var);
            self.var.get()
        }

        fn take_on_update(&mut self, ctx: &mut WidgetContext, _: &mut WidgetUpdates) -> bool {
            self.var.get_new(ctx).unwrap_or(false)
        }
    }

    pub(super) struct TakeOnEvent<A: EventArgs, F: FnMut(&A) -> bool + 'static> {
        pub(super) event: Event<A>,
        pub(super) filter: F,
        pub(super) take_on_init: bool,
    }
    impl<A: EventArgs, F: FnMut(&A) -> bool + 'static> TakeOn for TakeOnEvent<A, F> {
        fn take_on_init(&mut self, ctx: &mut WidgetContext) -> bool {
            ctx.sub_event(&self.event);
            self.take_on_init
        }

        fn take_on_event(&mut self, _: &mut WidgetContext, update: &mut EventUpdate) -> bool {
            if let Some(args) = self.event.on(update) {
                (self.filter)(args)
            } else {
                false
            }
        }
    }

    pub(super) struct TakeOnSlot<U, T: TakeOn> {
        pub(super) slot: SlotId,
        pub(super) rc: Rc<SlotData<U>>,
        pub(super) take: T,

        pub(super) delegate_init: fn(&mut U, &mut WidgetContext),
        pub(super) delegate_deinit: fn(&mut U, &mut WidgetContext),
    }
    impl<U, T: TakeOn> TakeOnSlot<U, T> {
        fn on_init(&mut self, ctx: &mut WidgetContext) {
            if self.take.take_on_init(ctx) {
                self.take(ctx);
            }
        }

        fn on_deinit(&mut self, ctx: &mut WidgetContext) {
            let mut was_owner = false;
            {
                let mut slots = self.rc.slots.borrow_mut();
                let slots = &mut *slots;
                if let Some((slot, _, _)) = &slots.owner {
                    if *slot == self.slot {
                        slots.owner = None;
                        was_owner = true;
                    }
                }
            }

            if was_owner {
                (self.delegate_deinit)(&mut *self.rc.item.borrow_mut(), ctx)
            }
        }

        fn on_event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            if !self.is_owner() && self.take.take_on_event(ctx, update) {
                // request ownership.
                self.take(ctx);
            }
        }

        fn on_update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.is_owner() {
                let mut slots = self.rc.slots.borrow_mut();
                if let Some((_, id)) = slots.move_request {
                    // deinit to move to other slot.

                    let replacement = slots.replacement.take();
                    slots.owner = None;

                    drop(slots);

                    let mut node = self.rc.item.borrow_mut();
                    (self.delegate_deinit)(&mut node, ctx);

                    ctx.updates.info_layout_and_render();

                    if let Some(new) = replacement {
                        *node = new;
                    }

                    ctx.updates.update(id);
                } else if let Some(mut new) = slots.replacement.take() {
                    // apply replacement.

                    drop(slots);

                    let mut node = self.rc.item.borrow_mut();
                    (self.delegate_deinit)(&mut node, ctx);
                    (self.delegate_init)(&mut new, ctx);
                    *node = new;

                    ctx.updates.info_layout_and_render();
                }
            } else if self.take.take_on_update(ctx, updates) {
                // request ownership.
                self.take(ctx);
            } else {
                let mut slots = self.rc.slots.borrow_mut();
                if let Some((slot, _)) = &slots.move_request {
                    if *slot == self.slot && slots.owner.is_none() {
                        slots.move_request = None;
                        // requested move in prev update, now can take ownership.
                        drop(slots);
                        self.take(ctx);
                    }
                }
            }
        }

        fn take(&mut self, ctx: &mut WidgetContext) {
            {
                let mut slots = self.rc.slots.borrow_mut();
                let slots = &mut *slots;
                if let Some((_, id, _)) = &slots.owner {
                    // currently inited in another slot, signal it to deinit.
                    slots.move_request = Some((self.slot, ctx.path.widget_id()));
                    ctx.updates.update(*id);
                } else {
                    // no current owner, take ownership immediately.
                    slots.owner = Some((self.slot, ctx.path.widget_id(), ctx.updates.sender()));
                }
            }

            if self.is_owner() {
                (self.delegate_init)(&mut *self.rc.item.borrow_mut(), ctx)
            }
        }

        fn is_owner(&self) -> bool {
            self.rc
                .slots
                .borrow()
                .owner
                .as_ref()
                .map(|(sl, _, _)| *sl == self.slot)
                .unwrap_or(false)
        }

        fn delegate_owned<R>(&self, del: impl FnOnce(&U) -> R) -> Option<R> {
            if self.is_owner() {
                Some(del(&*self.rc.item.borrow()))
            } else {
                None
            }
        }
        fn delegate_owned_mut<R>(&mut self, del: impl FnOnce(&mut U) -> R) -> Option<R> {
            if self.is_owner() {
                Some(del(&mut *self.rc.item.borrow_mut()))
            } else {
                None
            }
        }
    }

    impl<U: UiNode, T: TakeOn> UiNode for TakeOnSlot<U, T> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.on_init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.on_deinit(ctx);
        }

        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            self.delegate_owned(|n| n.info(ctx, info));
        }

        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            self.on_event(ctx, update);
            self.delegate_owned_mut(|n| n.event(ctx, update));
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.on_update(ctx, updates);
            self.delegate_owned_mut(|n| n.update(ctx, updates));
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            self.delegate_owned(|n| n.measure(ctx)).unwrap_or_default()
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            self.delegate_owned_mut(|n| n.layout(ctx, wl)).unwrap_or_default()
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.delegate_owned(|n| n.render(ctx, frame));
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            self.delegate_owned(|n| n.render_update(ctx, update));
        }

        fn is_widget(&self) -> bool {
            self.delegate_owned(UiNode::is_widget).unwrap_or(false)
        }

        fn try_id(&self) -> Option<WidgetId> {
            self.delegate_owned(UiNode::try_id).unwrap_or(None)
        }

        fn try_state(&self) -> Option<StateMapRef<state_map::Widget>> {
            todo!("ref!")
        }

        fn try_state_mut(&mut self) -> Option<StateMapMut<state_map::Widget>> {
            todo!("ref!")
        }

        fn try_bounds_info(&self) -> Option<&widget_info::WidgetBoundsInfo> {
            todo!("ref!")
        }

        fn try_border_info(&self) -> Option<&widget_info::WidgetBorderInfo> {
            todo!("ref!")
        }
    }

    impl<U: UiNodeList, T: TakeOn> UiNodeList for TakeOnSlot<U, T> {
        fn is_fixed(&self) -> bool {
            self.delegate_owned(UiNodeList::is_fixed).unwrap_or(false)
        }

        fn len(&self) -> usize {
            self.delegate_owned(UiNodeList::len).unwrap_or(0)
        }

        fn is_empty(&self) -> bool {
            self.delegate_owned(UiNodeList::is_empty).unwrap_or(true)
        }

        fn boxed_all(self) -> ui_list::UiNodeVec {
            todo!("actual boxing needed")
        }

        fn info_all(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            self.delegate_owned(|l| l.info_all(ctx, info));
        }

        fn init_all(&mut self, ctx: &mut WidgetContext) {
            self.on_init(ctx)
        }

        fn deinit_all(&mut self, ctx: &mut WidgetContext) {
            self.on_deinit(ctx)
        }

        fn update_all<O: ui_list::UiListObserver>(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates, observer: &mut O) {
            self.on_update(ctx, updates);
            self.delegate_owned_mut(|l| l.update_all(ctx, updates, observer));
        }

        fn event_all(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            self.on_event(ctx, update);
            self.delegate_owned_mut(|l| l.event_all(ctx, update));
        }

        fn measure_all<C, D>(&self, ctx: &mut MeasureContext, pre_measure: C, pos_measure: D)
        where
            C: FnMut(&mut MeasureContext, &mut ui_list::PreMeasureArgs),
            D: FnMut(&mut MeasureContext, ui_list::PosMeasureArgs),
        {
            self.delegate_owned(|l| l.measure_all(ctx, pre_measure, pos_measure));
        }

        fn item_measure(&self, index: usize, ctx: &mut MeasureContext) -> PxSize {
            self.delegate_owned(|l| l.item_measure(index, ctx)).unwrap_or_default()
        }

        fn layout_all<C, D>(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout, pre_layout: C, pos_layout: D)
        where
            C: FnMut(&mut LayoutContext, &mut WidgetLayout, &mut ui_list::PreLayoutArgs),
            D: FnMut(&mut LayoutContext, &mut WidgetLayout, ui_list::PosLayoutArgs),
        {
            self.delegate_owned_mut(|l| l.layout_all(ctx, wl, pre_layout, pos_layout));
        }

        fn item_layout(&mut self, index: usize, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            self.delegate_owned_mut(|l| l.item_layout(index, ctx, wl)).unwrap_or_default()
        }

        fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.delegate_owned(|l| l.render_all(ctx, frame));
        }

        fn item_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.delegate_owned(|l| l.item_render(index, ctx, frame));
        }

        fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            self.delegate_owned(|l| l.render_update_all(ctx, update));
        }

        fn item_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            self.delegate_owned(|l| l.item_render_update(index, ctx, update));
        }

        fn try_item_id(&self, index: usize) -> Option<WidgetId> {
            self.delegate_owned(|l| l.try_item_id(index)).unwrap_or(None)
        }

        fn try_item_state(&self, index: usize) -> Option<StateMapRef<state_map::Widget>> {
            todo!("ref!")
        }

        fn try_item_state_mut(&mut self, index: usize) -> Option<StateMapMut<state_map::Widget>> {
            todo!("ref!")
        }

        fn try_item_bounds_info(&self, index: usize) -> Option<&widget_info::WidgetBoundsInfo> {
            todo!("ref!")
        }

        fn try_item_border_info(&self, index: usize) -> Option<&widget_info::WidgetBorderInfo> {
            todo!("ref!")
        }

        fn render_node_filtered<F>(&self, filter: F, ctx: &mut RenderContext, frame: &mut FrameBuilder)
        where
            F: FnMut(ui_list::UiNodeFilterArgs) -> bool,
        {
            self.delegate_owned(|l| l.render_node_filtered(filter, ctx, frame));
        }

        fn try_item_outer<F, R>(&mut self, index: usize, wl: &mut WidgetLayout, keep_previous: bool, transform: F) -> Option<R>
        where
            F: FnOnce(&mut widget_info::WidgetLayoutTranslation, ui_list::PosLayoutArgs) -> R,
        {
            self.delegate_owned_mut(|l| l.try_item_outer(index, wl, keep_previous, transform))
                .unwrap_or(None)
        }

        fn try_outer_all<F>(&mut self, wl: &mut WidgetLayout, keep_previous: bool, transform: F)
        where
            F: FnMut(&mut widget_info::WidgetLayoutTranslation, ui_list::PosLayoutArgs),
        {
            self.delegate_owned_mut(|l| l.try_outer_all(wl, keep_previous, transform));
        }

        fn count_nodes<F>(&self, filter: F) -> usize
        where
            F: FnMut(ui_list::UiNodeFilterArgs) -> bool,
        {
            self.delegate_owned(|l| l.count_nodes(filter)).unwrap_or(0)
        }
    }
}
