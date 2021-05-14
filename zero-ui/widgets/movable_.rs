use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use zero_ui_core::event::EventListener;

use crate::prelude::new_widget::*;

/// Represents a [`UiNode`] that can be moved to another widget parent or window.
///
/// *Slots* can be created to mark the places the movable node is inserted, these slots then
/// can **take** the movable node, causing it to be [deinitialized](UiNode::deinit) in the previous slot
/// and [reinitialized](UiNode::init) in the new slot.
///
/// This `struct` is cheap to clone.
pub struct UiMovable<N: UiNode> {
    inner: Rc<UiMovableInner<N>>,
}
impl<N: UiNode> Clone for UiMovable<N> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}
impl<N: UiNode> UiMovable<N> {
    /// New [`UiMovable`] with a `node` that is not placed anywhere.
    ///
    /// The `node` is assumed to not be [initialized](UiNode::init).
    pub fn new(node: N) -> Self {
        UiMovable {
            inner: Rc::new(UiMovableInner {
                next_slot_id: Cell::new(1),
                active_slot_id: Cell::new(0),
                waiting_deinit: Cell::new(false),
                inited: Cell::new(false),
                node: RefCell::new(node),
            }),
        }
    }

    /// Create an [`UiNode`] that **takes** the movable when `take_if` returns `true` in a [`update`](UiNode::update).
    ///
    /// Every update `take_if` is called, when it returns `true` the movable widget is [deinitialized](UiNode::deinit)
    /// in its previous initialized slot and then [reinitialized](UiNode::init) in this slot.
    ///
    /// The `take_if` closure does not need to constantly return `true`, the last slot to `take_if` is the new
    /// owner of the movable node.
    pub fn slot(&self, take_if: impl FnMut(&mut WidgetContext) -> bool + 'static) -> impl UiNode {
        UiMovableSlotNode {
            id: {
                let id = self.inner.next_slot_id.get();
                self.inner.next_slot_id.set(id.wrapping_add(1));
                id
            },
            taking: false,
            take_if,
            inner: self.inner.clone(),
        }
    }

    /// Create a [`UiNode`] that **takes** the movable node when `event` updates.
    pub fn slot_event<E>(&self, event: EventListener<E>) -> impl UiNode {
        self.slot(move |ctx| event.has_updates(ctx.events))
    }

    /// Create an [`UiNode`] that **takes** the movable node when `var` is `true`.
    ///
    /// The `var` does not need to stay `true`, the slot takes the movable node when the `var` first signal `true`
    /// and retains the movable node even if `var` changes to `false`.
    pub fn slot_var(&self, var: impl VarObj<bool>) -> impl UiNode {
        self.slot(move |ctx| *var.get(ctx.vars))
    }

    /// Create an [`UiNode`] that **takes** the movable node once on init.
    #[inline]
    pub fn slot_take(&self) -> impl UiNode {
        let mut take = true;
        self.slot(move |_| std::mem::take(&mut take))
    }
}

struct UiMovableInner<N> {
    next_slot_id: Cell<u32>,
    active_slot_id: Cell<u32>,
    waiting_deinit: Cell<bool>,
    inited: Cell<bool>,
    node: RefCell<N>,
}

struct UiMovableSlotNode<T, N> {
    id: u32,
    taking: bool,
    take_if: T,
    inner: Rc<UiMovableInner<N>>,
}

impl<T, N> UiMovableSlotNode<T, N>
where
    T: FnMut(&mut WidgetContext) -> bool + 'static,
    N: UiNode,
{
    fn maybe_take(&mut self, ctx: &mut WidgetContext) {
        if self.taking || (self.take_if)(ctx) {
            if self.inner.inited.get() {
                self.taking = true;
                self.inner.waiting_deinit.set(true);
                ctx.updates.update();
            } else {
                self.inner.node.borrow_mut().init(ctx);
                self.inner.active_slot_id.set(self.id);
                self.taking = false;
                ctx.updates.layout();
            }
        }
    }
}
impl<T, N> UiNode for UiMovableSlotNode<T, N>
where
    T: FnMut(&mut WidgetContext) -> bool + 'static,
    N: UiNode,
{
    #[inline]
    fn init(&mut self, ctx: &mut WidgetContext) {
        if self.inner.active_slot_id.get() == self.id && !self.inner.inited.get() {
            self.inner.node.borrow_mut().init(ctx);
            self.inner.inited.set(true);
        } else {
            self.maybe_take(ctx);
        }
    }

    #[inline]
    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if self.inner.active_slot_id.get() == self.id && self.inner.inited.get() {
            self.inner.node.borrow_mut().deinit(ctx);
            self.inner.inited.set(true);
        }
    }

    #[inline]
    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.inner.active_slot_id.get() == self.id {
            if self.inner.waiting_deinit.get() {
                self.inner.node.borrow_mut().deinit(ctx);
                self.inner.waiting_deinit.set(false);
                self.inner.inited.set(false);
            } else {
                self.inner.node.borrow_mut().update(ctx);
            }
        } else {
            self.maybe_take(ctx);
        }
    }

    #[inline]
    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        if self.inner.active_slot_id.get() == self.id {
            self.inner.node.borrow_mut().update_hp(ctx)
        }
    }

    #[inline]
    fn measure(&mut self, ctx: &mut LayoutContext, available_size: LayoutSize) -> LayoutSize {
        if self.inner.active_slot_id.get() == self.id {
            self.inner.node.borrow_mut().measure(ctx, available_size)
        } else {
            LayoutSize::zero()
        }
    }

    #[inline]
    fn arrange(&mut self, ctx: &mut LayoutContext, final_size: LayoutSize) {
        if self.inner.active_slot_id.get() == self.id {
            self.inner.node.borrow_mut().arrange(ctx, final_size)
        }
    }

    #[inline]
    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        if self.inner.active_slot_id.get() == self.id {
            self.inner.node.borrow().render(ctx, frame);
        }
    }

    #[inline]
    fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        if self.inner.active_slot_id.get() == self.id {
            self.inner.node.borrow().render_update(ctx, update)
        }
    }
}
