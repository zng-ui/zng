use std::{
    cell::RefCell,
    mem,
    rc::{Rc, Weak},
};

use crate::{
    app::AppEventSender,
    context::{InfoContext, LayoutContext, MeasureContext, RenderContext, WidgetContext, WidgetUpdates},
    event::{Event, EventArgs, EventUpdate},
    render::{FrameBuilder, FrameUpdate},
    units::PxSize,
    var::*,
    widget_info::{WidgetInfoBuilder, WidgetLayout},
    *,
};

type SlotId = usize;

struct NodeData<U> {
    node: RefCell<U>,
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
pub struct RcNode<U: UiNode>(Rc<NodeData<U>>);
impl<U: UiNode> Clone for RcNode<U> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<U: UiNode> RcNode<U> {
    /// New node.
    pub fn new(node: U) -> Self {
        RcNode(Rc::new(NodeData {
            node: RefCell::new(node),
            slots: RefCell::default(),
        }))
    }

    /// New rc node that contains a weak reference to itself.
    ///
    /// **Note** the weak reference cannot be [upgraded](WeakNode::upgrade) during the call to `node`.
    pub fn new_cyclic(node: impl FnOnce(WeakNode<U>) -> U) -> Self {
        Self(Rc::new_cyclic(|wk| {
            let node = node(WeakNode(wk.clone()));
            NodeData {
                node: RefCell::new(node),
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
            *self.0.node.borrow_mut() = new_node;
        }
    }

    /// Create a *slot* node that takes ownership of this node when `var` updates to `true`.
    ///
    /// The slot node also takes ownership on init if the `var`
    pub fn take_when(&self, var: impl IntoVar<bool>) -> impl UiNode {
        #[ui_node(struct TakeWhenNode<U: UiNode> {
            slot: SlotId,
            rc: Rc<NodeData<U>>,
            #[var] var: impl Var<bool>,
        })]
        impl TakeWhenNode {
            fn is_owner(&self) -> bool {
                self.rc
                    .slots
                    .borrow()
                    .owner
                    .as_ref()
                    .map(|(sl, _, _)| *sl == self.slot)
                    .unwrap_or(false)
            }

            #[UiNode]
            fn init(&mut self, ctx: &mut WidgetContext) {
                self.init_handles(ctx);

                if self.var.get() {
                    let mut is_owner = false;

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
                            is_owner = true;
                        }
                    }

                    if is_owner {
                        self.rc.node.borrow_mut().init(ctx);
                        ctx.updates.info_layout_and_render();
                    }
                }
            }

            #[UiNode]
            fn deinit(&mut self, ctx: &mut WidgetContext) {
                let mut is_owner = false;
                {
                    let mut slots = self.rc.slots.borrow_mut();
                    let slots = &mut *slots;
                    if let Some((slot, _, _)) = &slots.owner {
                        if *slot == self.slot {
                            slots.owner = None;
                            is_owner = true;
                        }
                    }
                }

                if is_owner {
                    self.rc.node.borrow_mut().deinit(ctx);
                }
            }

            #[UiNode]
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                if self.is_owner() {
                    self.rc.node.borrow().info(ctx, info);
                }
            }

            #[UiNode]
            fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
                if self.is_owner() {
                    self.rc.node.borrow_mut().event(ctx, update);
                }
            }

            #[UiNode]
            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                if self.is_owner() {
                    let mut slots = self.rc.slots.borrow_mut();
                    if let Some((_, id)) = slots.move_request {
                        // deinit to move to other slot.

                        let replacement = slots.replacement.take();
                        slots.owner = None;

                        drop(slots);

                        let mut node = self.rc.node.borrow_mut();
                        node.deinit(ctx);
                        ctx.updates.info_layout_and_render();

                        if let Some(new) = replacement {
                            *node = new;
                        }

                        ctx.updates.update(id);
                    } else if let Some(mut new) = slots.replacement.take() {
                        // apply replacement.

                        drop(slots);

                        let mut node = self.rc.node.borrow_mut();
                        node.deinit(ctx);
                        new.init(ctx);
                        *node = new;

                        ctx.updates.info_layout_and_render();
                    } else {
                        drop(slots);

                        // normal update.
                        self.rc.node.borrow_mut().update(ctx, updates);
                    }
                } else if let Some(true) = self.var.get_new(ctx) {
                    // request ownership.
                    self.init(ctx);
                } else {
                    let mut slots = self.rc.slots.borrow_mut();
                    if let Some((slot, _)) = &slots.move_request {
                        if *slot == self.slot && slots.owner.is_none() {
                            slots.move_request = None;
                            // requested move in prev update, now can take ownership.
                            drop(slots);
                            self.init(ctx);
                        }
                    }
                }
            }

            #[UiNode]
            fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
                if self.is_owner() {
                    self.rc.node.borrow().measure(ctx)
                } else {
                    PxSize::zero()
                }
            }

            #[UiNode]
            fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                if self.is_owner() {
                    self.rc.node.borrow_mut().layout(ctx, wl)
                } else {
                    PxSize::zero()
                }
            }

            #[UiNode]
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                if self.is_owner() {
                    self.rc.node.borrow().render(ctx, frame);
                }
            }

            #[UiNode]
            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                if self.is_owner() {
                    self.rc.node.borrow().render_update(ctx, update);
                }
            }
        }
        TakeWhenNode {
            slot: self.0.slots.borrow_mut().next_slot(),
            rc: self.0.clone(),
            var: var.into_var(),
        }
    }

    /// Create a *slot* node that takes ownership of this node when `event` updates and `filter` returns `true`.
    ///
    /// The slot node also takes ownership on init if `take_on_init` is `true`.
    pub fn take_on<A: EventArgs>(&self, event: Event<A>, filter: impl FnMut(&A) -> bool + 'static, take_on_init: bool) -> impl UiNode {
        #[ui_node(struct TakeOnNode<U: UiNode, A: EventArgs> {
            slot: SlotId,
            rc: Rc<NodeData<U>>,
            #[event] event: Event<A>,
            filter: impl FnMut(&A) -> bool + 'static,
            take_on_init: bool,
        })]
        impl TakeOnNode {
            fn is_owner(&self) -> bool {
                self.rc
                    .slots
                    .borrow()
                    .owner
                    .as_ref()
                    .map(|(sl, _, _)| *sl == self.slot)
                    .unwrap_or(false)
            }

            #[UiNode]
            fn init(&mut self, ctx: &mut WidgetContext) {
                self.init_handles(ctx);

                if self.take_on_init {
                    let mut is_owner = false;

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
                            is_owner = true;
                        }
                    }

                    if is_owner {
                        self.rc.node.borrow_mut().init(ctx);
                        ctx.updates.info_layout_and_render();
                    }
                }
            }

            #[UiNode]
            fn deinit(&mut self, ctx: &mut WidgetContext) {
                let mut is_owner = false;
                {
                    let mut slots = self.rc.slots.borrow_mut();
                    let slots = &mut *slots;
                    if let Some((slot, _, _)) = &slots.owner {
                        if *slot == self.slot {
                            slots.owner = None;
                            is_owner = true;
                        }
                    }
                }

                if is_owner {
                    self.rc.node.borrow_mut().deinit(ctx);
                }
            }

            #[UiNode]
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                if self.is_owner() {
                    self.rc.node.borrow().info(ctx, info);
                }
            }

            #[UiNode]
            fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
                if self.is_owner() {
                    self.rc.node.borrow_mut().event(ctx, update);
                } else if let Some(args) = self.event.on(update) {
                    if (self.filter)(args) {
                        // request ownership.
                        let tk_on = mem::replace(&mut self.take_on_init, true);
                        self.init(ctx);
                        self.take_on_init = tk_on;
                    }
                }
            }

            #[UiNode]
            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                if self.is_owner() {
                    let mut slots = self.rc.slots.borrow_mut();
                    if let Some((_, id)) = slots.move_request {
                        // deinit to move to other slot.

                        let replacement = slots.replacement.take();
                        slots.owner = None;

                        drop(slots);

                        let mut node = self.rc.node.borrow_mut();
                        node.deinit(ctx);
                        ctx.updates.info_layout_and_render();

                        if let Some(new) = replacement {
                            *node = new;
                        }

                        ctx.updates.update(id);
                    } else if let Some(mut new) = slots.replacement.take() {
                        // apply replacement.

                        drop(slots);

                        let mut node = self.rc.node.borrow_mut();
                        node.deinit(ctx);
                        new.init(ctx);
                        *node = new;

                        ctx.updates.info_layout_and_render();
                    } else {
                        drop(slots);

                        // normal update.
                        self.rc.node.borrow_mut().update(ctx, updates);
                    }
                } else {
                    let mut slots = self.rc.slots.borrow_mut();
                    if let Some((slot, _)) = &slots.move_request {
                        if *slot == self.slot && slots.owner.is_none() {
                            slots.move_request = None;
                            // requested move in prev update, now can take ownership.
                            drop(slots);

                            let tk_on = mem::replace(&mut self.take_on_init, true);
                            self.init(ctx);
                            self.take_on_init = tk_on;
                        }
                    }
                }
            }

            #[UiNode]
            fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
                if self.is_owner() {
                    self.rc.node.borrow().measure(ctx)
                } else {
                    PxSize::zero()
                }
            }

            #[UiNode]
            fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                if self.is_owner() {
                    self.rc.node.borrow_mut().layout(ctx, wl)
                } else {
                    PxSize::zero()
                }
            }

            #[UiNode]
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                if self.is_owner() {
                    self.rc.node.borrow().render(ctx, frame);
                }
            }

            #[UiNode]
            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                if self.is_owner() {
                    self.rc.node.borrow().render_update(ctx, update);
                }
            }
        }
        TakeOnNode {
            slot: self.0.slots.borrow_mut().next_slot(),
            rc: self.0.clone(),
            event,
            filter,
            take_on_init,
        }
    }
}

/// `Weak` reference to a [`RcNode<U>`].
pub struct WeakNode<U: UiNode>(Weak<NodeData<U>>);
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
