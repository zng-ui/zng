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
    /// Every update `take_signal` is probed, when it returns `true` the movable widget is [deinitialized](UiNode::deinit)
    /// in its previous initialized slot and then [reinitialized](UiNode::init) in this slot.
    ///
    /// The `take_signal` does not need to constantly return `true`, the last slot to signal is the new
    /// owner of the movable node.
    ///
    /// See also [`slot!`](mod@slot).
    pub fn slot(&self, take_signal: impl UiMovableTakeSignal) -> impl UiNode {
        UiMovableSlotNode {
            id: {
                let id = self.inner.next_slot_id.get();
                self.inner.next_slot_id.set(id.wrapping_add(1));
                id
            },
            taking: false,
            take_signal,
            inner: self.inner.clone(),
        }
    }
}

/// Signals an [`UiMovable`] slot that it should take the widget as its exclusive child.
///
/// This trait is implemented for all `bool` variables, you can also use [`take_on_init`] to
/// be the first slot to take the widget, [`take_on`] to take when an event updates or [`take_if`]
/// to use a custom delegate to signal.
pub trait UiMovableTakeSignal: 'static {
    /// Returns `true` when the slot must take the movable widget as its exclusive child.
    fn take_if(&mut self, ctx: &mut WidgetContext) -> bool;
}
impl<V> UiMovableTakeSignal for V
where
    V: VarObj<bool>,
{
    /// Takes the widget when the var value is `true`.
    fn take_if(&mut self, ctx: &mut WidgetContext) -> bool {
        *self.get(ctx.vars)
    }
}

/// An [`UiMovableTakeSignal`] that takes the widget when `custom` returns `true`.
pub fn take_if<F: FnMut(&mut WidgetContext) -> bool + 'static>(custom: F) -> impl UiMovableTakeSignal {
    struct TakeIf<F>(F);
    impl<F: FnMut(&mut WidgetContext) -> bool + 'static> UiMovableTakeSignal for TakeIf<F> {
        fn take_if(&mut self, ctx: &mut WidgetContext) -> bool {
            (self.0)(ctx)
        }
    }
    TakeIf(custom)
}

/// An [`UiMovableTakeSignal`] that takes the widget every time the `event` updates.
pub fn take_on<E>(event: EventListener<E>) -> impl UiMovableTakeSignal {
    struct TakeOn<E: 'static>(EventListener<E>);
    impl<E> UiMovableTakeSignal for TakeOn<E> {
        fn take_if(&mut self, ctx: &mut WidgetContext) -> bool {
            self.0.has_updates(ctx.events)
        }
    }
    TakeOn(event)
}

/// An [`UiMovableTakeSignal`] that takes the widget once on init.
pub fn take_on_init() -> impl UiMovableTakeSignal {
    struct TakeOnInit(bool);
    impl UiMovableTakeSignal for TakeOnInit {
        fn take_if(&mut self, _: &mut WidgetContext) -> bool {
            std::mem::take(&mut self.0)
        }
    }
    TakeOnInit(true)
}

/// An [`UiMovable`] slot widget.
///
/// ## `slot()`
///
/// If you only want to create a slot as an widget there is a [`slot`](fn@slot) shortcut function.
#[widget($crate::widgets::slot)]
pub mod slot {
    use super::*;

    properties! {
        /// The [`UiMovable`] reference.
        #[allowed_in_when = false]
        movable(UiMovable<impl UiNode>);

        /// A closure that returns `true` when this slot should **take** the `movable`.
        ///
        /// This property accepts any `bool` variable, you can also use [`take_on_init`] to
        /// be the first slot to take the widget, [`take_on`] to take when an event listener updates or [`take_if`]
        /// to use a custom delegate to signal.
        ///
        /// See [`UiMovable::slot`] for more details.
        #[allowed_in_when = false]
        take_signal(impl UiMovableTakeSignal);
    }

    fn new_child(movable: UiMovable<impl UiNode>, take_signal: impl UiMovableTakeSignal) -> impl UiNode {
        movable.slot(take_signal)
    }
}

/// An [`UiMovable`] slot widget.
///
/// # `slot!`
///
/// This function is just a shortcut for [`slot!`](mod@slot).
pub fn slot(movable: UiMovable<impl UiNode>, take_signal: impl UiMovableTakeSignal) -> impl Widget {
    slot!(movable; take_signal)
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
    take_signal: T,
    inner: Rc<UiMovableInner<N>>,
}

impl<T, N> UiMovableSlotNode<T, N>
where
    T: UiMovableTakeSignal,
    N: UiNode,
{
    fn maybe_take(&mut self, ctx: &mut WidgetContext) {
        if self.taking || self.take_signal.take_if(ctx) {
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
    T: UiMovableTakeSignal,
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
