use std::{
    cell::{Cell, RefCell},
    fmt, mem,
    rc::{Rc, Weak},
};

use crate::{
    context::{InfoContext, LayoutContext, RenderContext, WidgetContext},
    event::{Event, EventUpdateArgs},
    render::{FrameBuilder, FrameUpdate},
    units::{AvailableSize, PxSize},
    widget_info::{UpdateSlot, WidgetInfoBuilder, WidgetLayout, WidgetSubscriptions},
    UiNode,
};

/// A reference counted [`UiNode`].
///
/// Nodes can only appear in one place of the UI tree at a time, this `struct` allows the
/// creation of ***slots*** that are [`UiNode`] implementers that can *exclusive take* the
/// referenced node as its child.
///
/// When a slot takes the node it is deinited in the previous UI tree place and reinited in the slot place.
///
/// Slots hold a strong reference to the node when they have it as their child and a weak reference when they don't.
pub struct RcNode<U: UiNode>(Rc<RcNodeData<U>>);
impl<U: UiNode> Clone for RcNode<U> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}
impl<U: UiNode> RcNode<U> {
    /// New rc node.
    ///
    /// The `node` is assumed to not be inited.
    pub fn new(node: U) -> Self {
        Self(Rc::new(RcNodeData::new(Some(node))))
    }

    /// New rc node that contains a weak reference to itself.
    ///
    /// **Note** the weak reference cannot be [upgraded](WeakNode::upgrade) during the call to `node`.
    pub fn new_cyclic(node: impl FnOnce(WeakNode<U>) -> U) -> Self {
        // Note: Rewrite this method with `Rc::new_cyclic` when
        // https://github.com/rust-lang/rust/issues/75861 stabilizes
        let r = Self(Rc::new(RcNodeData::new(None)));
        let n = node(r.downgrade());
        *r.0.node.borrow_mut() = Some(n);
        r
    }

    /// Creates an [`UiNode`] implementer that can *exclusive take* the referenced node as its child when
    /// signaled by `take_signal`.
    pub fn slot<S: RcNodeTakeSignal>(&self, take_signal: S) -> impl UiNode {
        SlotNode {
            update_slot: self.0.update_slot,
            slot_id: self.0.next_id(),
            take_signal,
            event_signal: false,
            state: if S::TAKE_ON_INIT {
                SlotNodeState::TakeOnInit(Rc::clone(&self.0))
            } else {
                SlotNodeState::Inactive(Rc::downgrade(&self.0))
            },
        }
    }

    /// Creates a new [`WeakNode`] that points to this node.
    #[inline]
    pub fn downgrade(&self) -> WeakNode<U> {
        WeakNode(Rc::downgrade(&self.0))
    }

    /// Replace the current node with the `new_node` in the current slot.
    ///
    /// The previous node is deinited and the `new_node` is inited.
    pub fn set(&self, new_node: U) {
        *self.0.new_node.borrow_mut() = Some(new_node);
    }
}

/// `Weak` version of [`RcNode`].
pub struct WeakNode<U: UiNode>(Weak<RcNodeData<U>>);
impl<U: UiNode> Clone for WeakNode<U> {
    fn clone(&self) -> Self {
        Self(Weak::clone(&self.0))
    }
}
impl<U: UiNode> WeakNode<U> {
    /// Attempts to upgrade to a [`RcNode`].
    pub fn upgrade(&self) -> Option<RcNode<U>> {
        if let Some(rc) = self.0.upgrade() {
            if rc.node.borrow().is_some() {
                return Some(RcNode(rc));
            }
        }
        None
    }
}

/// Signal an [`RcNode`] slot to take the referenced node as its child.
///
/// This trait is implemented for all `bool` variables, you can also use [`take_on_init`] to
/// be the first slot to take the widget, [`take_on`] to take when an event updates.
pub trait RcNodeTakeSignal: 'static {
    /// If slot node must take the node when it is created.
    const TAKE_ON_INIT: bool = false;

    /// Signal subscriptions, [`event_take`] and  [`update_take`] are only called if
    /// their update and event sources are registered here.
    ///
    /// [`update_take`]: Self::update_take
    /// [`event_take`]: Self::event_take
    fn subscribe(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        let _ = (ctx, subscriptions);
    }

    /// Returns `true` when the slot must take the node as its child.
    fn event_take<E: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &E) -> bool {
        let _ = (ctx, args);
        false
    }

    /// Returns `true` when the slot must take the node as its child.
    fn update_take(&mut self, ctx: &mut WidgetContext) -> bool {
        let _ = ctx;
        false
    }
}
impl<V> RcNodeTakeSignal for V
where
    V: crate::var::Var<bool>,
{
    fn subscribe(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        subscriptions.var(ctx, self);
    }

    /// Takes the widget when the var value is `true`.
    fn update_take(&mut self, ctx: &mut WidgetContext) -> bool {
        *self.get(ctx)
    }
}
/// An [`RcNodeTakeSignal`] that takes the widget every time the `event` updates and passes the filter.
pub fn take_on<E, F>(event: E, filter: F) -> impl RcNodeTakeSignal
where
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
{
    struct TakeOn<E, F>(E, F);
    impl<E, F> RcNodeTakeSignal for TakeOn<E, F>
    where
        E: Event,
        F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    {
        fn event_take<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) -> bool {
            self.0.update(args).map(|a| (self.1)(ctx, a)).unwrap_or_default()
        }
    }
    TakeOn(event, filter)
}
/// An [`RcNodeTakeSignal`] that takes the widget once on init.
pub fn take_on_init() -> impl RcNodeTakeSignal {
    struct TakeOnInit;
    impl RcNodeTakeSignal for TakeOnInit {
        const TAKE_ON_INIT: bool = true;
    }
    TakeOnInit
}

struct RcNodeData<U: UiNode> {
    next_id: Cell<u32>,
    owner_id: Cell<u32>,
    waiting_deinit: Cell<bool>,
    inited: Cell<bool>,
    node: RefCell<Option<U>>,
    new_node: RefCell<Option<U>>,
    update_slot: UpdateSlot,
}
impl<U: UiNode> RcNodeData<U> {
    pub fn new(node: Option<U>) -> Self {
        Self {
            next_id: Cell::new(1),
            owner_id: Cell::new(0),
            waiting_deinit: Cell::new(false),
            inited: Cell::new(false),
            node: RefCell::new(node),
            new_node: RefCell::new(None),
            update_slot: UpdateSlot::next(),
        }
    }

    pub fn next_id(&self) -> u32 {
        let id = self.next_id.get();
        self.next_id.set(id.wrapping_add(1));
        id
    }
}

enum SlotNodeState<U: UiNode> {
    TakeOnInit(Rc<RcNodeData<U>>),
    /// Slot is not the owner of the child node.
    Inactive(Weak<RcNodeData<U>>),
    /// Slot is the next owner of the child node, awaiting previous slot deinit.
    Activating(Rc<RcNodeData<U>>),
    /// Slot is the owner of the child node.
    Active(Rc<RcNodeData<U>>),
    /// Slot deinited itself when it was the owner of the child node.
    ActiveDeinited(Rc<RcNodeData<U>>),
    /// Tried to activate but the weak reference in `Inactive` is dead.
    Dropped,
}
impl<U: UiNode> fmt::Debug for SlotNodeState<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SlotNodeState::TakeOnInit(_) => write!(f, "TakeOnInit"),
            SlotNodeState::Inactive(wk) => {
                write!(f, "Inactive(can_upgrade: {})", wk.upgrade().is_some())
            }
            SlotNodeState::Activating(_) => write!(f, "Activating"),
            SlotNodeState::Active(_) => write!(f, "Active"),
            SlotNodeState::ActiveDeinited(_) => write!(f, "ActiveDeinited"),
            SlotNodeState::Dropped => write!(f, "Dropped"),
        }
    }
}

struct SlotNode<S: RcNodeTakeSignal, U: UiNode> {
    slot_id: u32,
    take_signal: S,
    event_signal: bool,
    update_slot: UpdateSlot,
    state: SlotNodeState<U>,
}
impl<S: RcNodeTakeSignal, U: UiNode> UiNode for SlotNode<S, U> {
    fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        if let SlotNodeState::Active(rc) = &self.state {
            rc.node.borrow().as_ref().unwrap().info(ctx, info);
        }
    }

    fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        subscriptions.update(self.update_slot);
        self.take_signal.subscribe(ctx, subscriptions);

        if let SlotNodeState::Active(rc) = &self.state {
            rc.node.borrow().as_ref().unwrap().subscriptions(ctx, subscriptions);
        }
    }

    fn init(&mut self, ctx: &mut WidgetContext) {
        match &self.state {
            SlotNodeState::TakeOnInit(rc) => {
                if rc.inited.get() {
                    rc.waiting_deinit.set(true);
                    self.state = SlotNodeState::Activating(Rc::clone(rc));
                    ctx.updates.update(self.update_slot.mask()); // notify the other slot to deactivate.
                } else {
                    // node already free to take.
                    rc.node.borrow_mut().as_mut().unwrap().init(ctx);
                    rc.inited.set(true);
                    rc.owner_id.set(self.slot_id);
                    self.state = SlotNodeState::Active(Rc::clone(rc));
                }
            }
            SlotNodeState::Inactive(wk) => {
                if self.take_signal.update_take(ctx) {
                    if let Some(rc) = wk.upgrade() {
                        if rc.inited.get() {
                            rc.waiting_deinit.set(true);
                            self.state = SlotNodeState::Activating(rc);
                            ctx.updates.update(self.update_slot.mask()); // notify the other slot to deactivate.
                        } else {
                            // node already free to take.
                            rc.node.borrow_mut().as_mut().unwrap().init(ctx);
                            rc.inited.set(true);
                            rc.owner_id.set(self.slot_id);
                            self.state = SlotNodeState::Active(rc);
                        }
                    } else {
                        self.state = SlotNodeState::Dropped;
                    }
                }
            }
            SlotNodeState::ActiveDeinited(rc) => {
                if rc.owner_id.get() == self.slot_id {
                    // still the owner
                    assert!(!rc.inited.get());
                    assert!(!rc.waiting_deinit.get());

                    rc.node.borrow_mut().as_mut().unwrap().init(ctx);
                    rc.inited.set(true);

                    self.state = SlotNodeState::Active(Rc::clone(rc));
                } else {
                    // TODO check signal?
                }
            }
            SlotNodeState::Activating(_) => {
                panic!("`SlotNode` in `Activating` state on init")
            }
            SlotNodeState::Active(_) => {
                panic!("`SlotNode` in `Active` state on init")
            }
            SlotNodeState::Dropped => {}
        }
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if let SlotNodeState::Active(rc) = &self.state {
            assert!(rc.inited.take());

            rc.node.borrow_mut().as_mut().unwrap().deinit(ctx);
            rc.waiting_deinit.set(false); // just in case?

            self.state = SlotNodeState::ActiveDeinited(Rc::clone(rc));
        }
    }

    fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU)
    where
        Self: Sized,
    {
        if let SlotNodeState::Active(rc) = &self.state {
            // propagate event to active node.
            rc.node.borrow_mut().as_mut().unwrap().event(ctx, args);
        } else if let SlotNodeState::Inactive(_) = &self.state {
            // check event take_signal.
            if self.take_signal.event_take(ctx, args) {
                self.event_signal = true;
                ctx.updates.update(self.update_slot.mask());
            }
        }
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        match &self.state {
            SlotNodeState::Inactive(wk) => {
                // fulfill event take_signal or update take_signal:
                if mem::take(&mut self.event_signal) || self.take_signal.update_take(ctx) {
                    if let Some(rc) = wk.upgrade() {
                        if rc.inited.get() {
                            // node is inited in other slot.
                            rc.waiting_deinit.set(true);
                            self.state = SlotNodeState::Activating(rc);
                            ctx.updates.update(self.update_slot.mask()); // notify the other slot to deactivate.
                        } else {
                            // node already free to take.
                            rc.node.borrow_mut().as_mut().unwrap().init(ctx);
                            rc.inited.set(true);
                            rc.owner_id.set(self.slot_id);
                            self.state = SlotNodeState::Active(rc);
                            ctx.updates.info_layout_and_render();
                        }
                    } else {
                        self.state = SlotNodeState::Dropped
                    }
                }
            }
            SlotNodeState::Activating(rc) => {
                if !rc.inited.get() {
                    // node now free to take.
                    rc.node.borrow_mut().as_mut().unwrap().init(ctx);
                    rc.inited.set(true);
                    self.state = SlotNodeState::Active(Rc::clone(rc));
                    ctx.updates.info_layout_and_render();
                }
            }
            SlotNodeState::Active(rc) => {
                if rc.waiting_deinit.take() {
                    // moving to other slot, must deinit here.
                    if rc.inited.take() {
                        rc.node.borrow_mut().as_mut().unwrap().deinit(ctx);
                    }
                    ctx.updates.update(self.update_slot.mask()); // notify the other slot to activate.
                    self.state = SlotNodeState::Inactive(Rc::downgrade(rc));
                    ctx.updates.info_layout_and_render();
                } else if let Some(mut new) = rc.new_node.borrow_mut().take() {
                    let mut old = rc.node.borrow_mut().take();
                    if rc.inited.take() {
                        old.deinit(ctx);
                    }
                    new.init(ctx);
                    *rc.node.borrow_mut() = Some(new);
                    rc.inited.set(true);
                    ctx.updates.info_layout_and_render();
                } else {
                    rc.node.borrow_mut().as_mut().unwrap().update(ctx);
                }
            }
            SlotNodeState::ActiveDeinited(_) => {
                panic!("`SlotNode` in `ActiveDeinited` state on update")
            }
            SlotNodeState::TakeOnInit(_) => {
                panic!("`SlotNode` in `TakeOnInit` state on update")
            }
            SlotNodeState::Dropped => {}
        }
    }

    fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        if let SlotNodeState::Active(rc) = &self.state {
            rc.node.borrow_mut().as_mut().unwrap().measure(ctx, available_size)
        } else {
            PxSize::zero()
        }
    }

    fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
        if let SlotNodeState::Active(rc) = &self.state {
            rc.node.borrow_mut().as_mut().unwrap().arrange(ctx, widget_layout, final_size);
        }
    }

    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        if let SlotNodeState::Active(rc) = &self.state {
            rc.node.borrow().as_ref().unwrap().render(ctx, frame);
        }
    }

    fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        if let SlotNodeState::Active(rc) = &self.state {
            rc.node.borrow().as_ref().unwrap().render_update(ctx, update);
        }
    }
}
