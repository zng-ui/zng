use std::{
    cell::{Cell, RefCell},
    fmt,
    rc::{Rc, Weak},
};

use crate::context::*;
use crate::impl_ui_node;
use crate::render::{FrameBuilder, FrameUpdate};
use crate::units::*;

unique_id! {
    /// Unique id of a widget.
    ///
    /// # Details
    /// Underlying value is a `NonZeroU64` generated using a relaxed global atomic `fetch_add`,
    /// so IDs are unique for the process duration, but order is not guaranteed.
    ///
    /// Panics if you somehow reach `u64::max_value()` calls to `new`.
    pub struct WidgetId;
}

/// An Ui tree node.
pub trait UiNode: 'static {
    /// Called every time the node is plugged in an Ui tree.
    fn init(&mut self, ctx: &mut WidgetContext);

    /// Called every time the node is unplugged from an Ui tree.
    fn deinit(&mut self, ctx: &mut WidgetContext);

    /// Called every time a low pressure event update happens.
    ///
    /// # Event Pressure
    /// See [`update_hp`](UiNode::update_hp) for more information about event pressure rate.
    fn update(&mut self, ctx: &mut WidgetContext);

    /// Called every time a high pressure event update happens.
    ///
    /// # Event Pressure
    /// Some events occur a lot more times then others, for performance reasons this
    /// event source may choose to be propagated in this high-pressure lane.
    ///
    /// Event sources that are high pressure mention this in their documentation.
    fn update_hp(&mut self, ctx: &mut WidgetContext);

    /// Called every time a layout update is needed.
    ///
    /// # Arguments
    /// * `available_size`: The total available size for the node. Can contain positive infinity to
    /// indicate the parent will accommodate [any size](crate::is_layout_any_size). Finite values are pixel aligned.
    /// * `ctx`: Measure context.
    ///
    /// # Return
    /// Return the nodes desired size. Must not contain infinity or NaN. Must be pixel aligned.
    fn measure(&mut self, ctx: &mut LayoutContext, available_size: LayoutSize) -> LayoutSize;

    /// Called every time a layout update is needed, after [`measure`](UiNode::measure).
    ///
    /// # Arguments
    /// * `final_size`: The size the parent node reserved for the node. Must reposition its contents
    /// to fit this size. The value does not contain infinity or NaNs and is pixel aligned.
    /// TODO args docs.
    fn arrange(&mut self, ctx: &mut LayoutContext, final_size: LayoutSize);

    /// Called every time a new frame must be rendered.
    ///
    /// # Arguments
    /// * `frame`: Contains the next frame draw instructions.
    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder);

    /// Called every time a frame can be updated without fully rebuilding.
    ///
    /// # Arguments
    /// * `update`: Contains the frame value updates.
    fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate);

    /// Box this node, unless it is already `Box<dyn UiNode>`.
    fn boxed(self) -> Box<dyn UiNode>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}
#[impl_ui_node(delegate = self.as_ref(), delegate_mut = self.as_mut())]
impl UiNode for Box<dyn UiNode> {
    fn boxed(self) -> Box<dyn UiNode> {
        self
    }
}

macro_rules! declare_widget_test_calls {
    ($(
        $method:ident
    ),+) => {$(paste::paste! {
        #[doc = "<span class='stab portability' title='This is supported on `any(test, doc, feature=\"pub_test\")` only'><code>any(test, doc, feature=\"pub_test\")</code></span>"]
        #[doc = "Run [`UiNode::" $method "`] using the [`TestWidgetContext`]."]
        #[cfg(any(test, doc, feature = "pub_test"))]
        fn [<test_ $method>](&mut self, ctx: &mut TestWidgetContext) {
            // `self` already creates an `widget_context`, we assume, so this
            // call is for a dummy parent of `self`.
            ctx.widget_context(|ctx| {
                self.$method(ctx);
            });
        }
    })+};
}

/// Represents an widget [`UiNode`].
pub trait Widget: UiNode {
    /// Id of the widget.
    fn id(&self) -> WidgetId;

    /// Reference the widget lazy state.
    fn state(&self) -> &StateMap;
    /// Exclusive borrow the widget lazy state.
    fn state_mut(&mut self) -> &mut StateMap;

    /// Last arranged size.
    fn size(&self) -> LayoutSize;

    /// Box this widget node, unless it is already `Box<dyn Widget>`.
    fn boxed_widget(self) -> Box<dyn Widget>
    where
        Self: Sized,
    {
        Box::new(self)
    }

    declare_widget_test_calls! {
        init, deinit, update, update_hp
    }

    /// <span class='stab portability' title='This is supported on `any(test, doc, feature="pub_test")` only'><code>any(test, doc, feature="pub_test")</code></span>
    /// Run [`UiNode::measure`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "pub_test"))]
    fn test_measure(&mut self, ctx: &mut TestWidgetContext, available_size: LayoutSize) -> LayoutSize {
        ctx.layout_context(14.0, 14.0, self.size(), PixelGrid::new(1.0), |ctx| {
            self.measure(ctx, available_size)
        })
    }
    /// <span class='stab portability' title='This is supported on `any(test, doc, feature="pub_test")` only'><code>any(test, doc, feature="pub_test")</code></span>
    /// Run [`UiNode::arrange`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "pub_test"))]
    fn test_arrange(&mut self, ctx: &mut TestWidgetContext, final_size: LayoutSize) {
        ctx.layout_context(14.0, 14.0, self.size(), PixelGrid::new(1.0), |ctx| self.arrange(ctx, final_size))
    }

    // TODO don't require user to init frame?

    /// <span class='stab portability' title='This is supported on `any(test, doc, feature="pub_test")` only'><code>any(test, doc, feature="pub_test")</code></span>
    /// Run [`UiNode::render`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "pub_test"))]
    fn test_render(&self, ctx: &mut TestWidgetContext, frame: &mut FrameBuilder) {
        ctx.render_context(|ctx| self.render(ctx, frame));
    }

    /// <span class='stab portability' title='This is supported on `any(test, doc, feature="pub_test")` only'><code>any(test, doc, feature="pub_test")</code></span>
    /// Run [`UiNode::render_update`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "pub_test"))]
    fn test_render_update(&self, ctx: &mut TestWidgetContext, update: &mut FrameUpdate) {
        ctx.render_context(|ctx| self.render_update(ctx, update));
    }
}

#[impl_ui_node(delegate = self.as_ref(), delegate_mut = self.as_mut())]
impl UiNode for Box<dyn Widget> {}
impl Widget for Box<dyn Widget> {
    #[inline]
    fn id(&self) -> WidgetId {
        self.as_ref().id()
    }
    #[inline]
    fn state(&self) -> &StateMap {
        self.as_ref().state()
    }
    #[inline]
    fn state_mut(&mut self) -> &mut StateMap {
        self.as_mut().state_mut()
    }
    #[inline]
    fn size(&self) -> LayoutSize {
        self.as_ref().size()
    }
    #[inline]
    fn boxed_widget(self) -> Box<dyn Widget> {
        self
    }
}

/// A UI node that does not contain any other node, does not take any space and renders nothing.
pub struct NilUiNode;
#[impl_ui_node(none)]
impl UiNode for NilUiNode {
    fn measure(&mut self, _: &mut LayoutContext, _: LayoutSize) -> LayoutSize {
        LayoutSize::zero()
    }
}

/// A UI node that does not contain any other node, fills the available space, but renders nothing.
pub struct FillUiNode;
#[impl_ui_node(none)]
impl UiNode for FillUiNode {}

// Used by #[impl_ui_node] to validate custom delegation.
#[doc(hidden)]
pub mod impl_ui_node_util {
    use crate::{
        context::{LayoutContext, RenderContext, WidgetContext},
        render::{FrameBuilder, FrameUpdate},
        units::LayoutSize,
        UiNode, UiNodeList,
    };

    #[inline]
    pub fn delegate(d: &(impl UiNode + ?Sized)) -> &(impl UiNode + ?Sized) {
        d
    }
    #[inline]
    pub fn delegate_mut(d: &mut (impl UiNode + ?Sized)) -> &mut (impl UiNode + ?Sized) {
        d
    }

    #[inline]
    pub fn delegate_list(d: &(impl UiNodeList + ?Sized)) -> &(impl UiNodeList + ?Sized) {
        d
    }
    #[inline]
    pub fn delegate_list_mut(d: &mut (impl UiNodeList + ?Sized)) -> &mut (impl UiNodeList + ?Sized) {
        d
    }

    #[inline]
    pub fn delegate_iter<'a>(d: impl IntoIterator<Item = &'a impl UiNode>) -> impl IterImpl {
        d
    }
    #[inline]
    pub fn delegate_iter_mut<'a>(d: impl IntoIterator<Item = &'a mut impl UiNode>) -> impl IterMutImpl {
        d
    }

    pub trait IterMutImpl {
        fn init_all(self, ctx: &mut WidgetContext);
        fn deinit_all(self, ctx: &mut WidgetContext);
        fn update_all(self, ctx: &mut WidgetContext);
        fn update_hp_all(self, ctx: &mut WidgetContext);
        fn measure_all(self, ctx: &mut LayoutContext, available_size: LayoutSize) -> LayoutSize;
        fn arrange_all(self, ctx: &mut LayoutContext, final_size: LayoutSize);
    }
    pub trait IterImpl {
        fn render_all(self, ctx: &mut RenderContext, frame: &mut FrameBuilder);
        fn render_update_all(self, ctx: &mut RenderContext, update: &mut FrameUpdate);
    }

    impl<'u, U: UiNode, I: IntoIterator<Item = &'u mut U>> IterMutImpl for I {
        fn init_all(self, ctx: &mut WidgetContext) {
            for child in self {
                child.init(ctx);
            }
        }

        fn deinit_all(self, ctx: &mut WidgetContext) {
            for child in self {
                child.deinit(ctx);
            }
        }

        fn update_all(self, ctx: &mut WidgetContext) {
            for child in self {
                child.update(ctx);
            }
        }

        fn update_hp_all(self, ctx: &mut WidgetContext) {
            for child in self {
                child.update_hp(ctx);
            }
        }

        fn measure_all(self, ctx: &mut LayoutContext, available_size: LayoutSize) -> LayoutSize {
            let mut size = LayoutSize::zero();
            for child in self {
                size = child.measure(ctx, available_size).max(size);
            }
            size
        }

        fn arrange_all(self, ctx: &mut LayoutContext, final_size: LayoutSize) {
            for child in self {
                child.arrange(ctx, final_size);
            }
        }
    }

    impl<'u, U: UiNode, I: IntoIterator<Item = &'u U>> IterImpl for I {
        fn render_all(self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            for child in self {
                child.render(ctx, frame);
            }
        }

        fn render_update_all(self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            for child in self {
                child.render_update(ctx, update);
            }
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
    /// **Node** the weak reference cannot be updated during the call to `node`
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
    pub fn slot(&self, take_signal: impl RcNodeTakeSignal) -> impl UiNode {
        SlotNode {
            slot_id: self.0.next_id(),
            take_signal,
            state: SlotNodeState::Inactive(Rc::downgrade(&self.0)),
        }
    }

    /// Creates a new [`WeakNode`] that points to this node.
    #[inline]
    pub fn downgrade(&self) -> WeakNode<U> {
        WeakNode(Rc::downgrade(&self.0))
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
/// be the first slot to take the widget, [`take_on`] to take when an event updates or [`take_if`]
/// to use a custom delegate to signal.
pub trait RcNodeTakeSignal: 'static {
    /// Returns `true` when the slot must take the node as its child.
    fn take(&mut self, ctx: &mut WidgetContext) -> bool;
}
impl<V> RcNodeTakeSignal for V
where
    V: crate::var::VarObj<bool>,
{
    /// Takes the widget when the var value is `true`.
    fn take(&mut self, ctx: &mut WidgetContext) -> bool {
        *self.get(ctx.vars)
    }
}
/// An [`RcNodeTakeSignal`] that takes the widget when `custom` returns `true`.
pub fn take_if<F: FnMut(&mut WidgetContext) -> bool + 'static>(custom: F) -> impl RcNodeTakeSignal {
    struct TakeIf<F>(F);
    impl<F: FnMut(&mut WidgetContext) -> bool + 'static> RcNodeTakeSignal for TakeIf<F> {
        fn take(&mut self, ctx: &mut WidgetContext) -> bool {
            (self.0)(ctx)
        }
    }
    TakeIf(custom)
}
/// An [`RcNodeTakeSignal`] that takes the widget every time the `event` updates.
pub fn take_on<E>(event: crate::event::EventListener<E>) -> impl RcNodeTakeSignal {
    struct TakeOn<E: 'static>(crate::event::EventListener<E>);
    impl<E> RcNodeTakeSignal for TakeOn<E> {
        fn take(&mut self, ctx: &mut WidgetContext) -> bool {
            self.0.has_updates(ctx.events)
        }
    }
    TakeOn(event)
}
/// An [`RcNodeTakeSignal`] that takes the widget once on init.
pub fn take_on_init() -> impl RcNodeTakeSignal {
    struct TakeOnInit(bool);
    impl RcNodeTakeSignal for TakeOnInit {
        fn take(&mut self, _: &mut WidgetContext) -> bool {
            std::mem::take(&mut self.0)
        }
    }
    TakeOnInit(true)
}

struct RcNodeData<U: UiNode> {
    next_id: Cell<u32>,
    owner_id: Cell<u32>,
    waiting_deinit: Cell<bool>,
    inited: Cell<bool>,
    node: RefCell<Option<U>>,
}
impl<U: UiNode> RcNodeData<U> {
    pub fn new(node: Option<U>) -> Self {
        Self {
            next_id: Cell::new(1),
            owner_id: Cell::new(0),
            waiting_deinit: Cell::new(false),
            inited: Cell::new(false),
            node: RefCell::new(node),
        }
    }

    pub fn next_id(&self) -> u32 {
        let id = self.next_id.get();
        self.next_id.set(id.wrapping_add(1));
        id
    }
}

enum SlotNodeState<U: UiNode> {
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
    state: SlotNodeState<U>,
}
impl<S: RcNodeTakeSignal, U: UiNode> UiNode for SlotNode<S, U> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        match &self.state {
            SlotNodeState::Inactive(wk) => {
                if self.take_signal.take(ctx) {
                    if let Some(rc) = wk.upgrade() {
                        if rc.inited.get() {
                            rc.waiting_deinit.set(true);
                            self.state = SlotNodeState::Activating(rc);
                            ctx.updates.update(); // notify the other slot to deactivate.
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

    fn update(&mut self, ctx: &mut WidgetContext) {
        match &self.state {
            SlotNodeState::Inactive(wk) => {
                if self.take_signal.take(ctx) {
                    if let Some(rc) = wk.upgrade() {
                        if rc.inited.get() {
                            rc.waiting_deinit.set(true);
                            self.state = SlotNodeState::Activating(rc);
                            ctx.updates.update(); // notify the other slot to deactivate.
                        } else {
                            // node already free to take.
                            rc.node.borrow_mut().as_mut().unwrap().init(ctx);
                            rc.inited.set(true);
                            rc.owner_id.set(self.slot_id);
                            self.state = SlotNodeState::Active(rc);
                            ctx.updates.layout();
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
                }
            }
            SlotNodeState::Active(rc) => {
                if rc.waiting_deinit.take() {
                    if rc.inited.take() {
                        rc.node.borrow_mut().as_mut().unwrap().deinit(ctx);
                    }
                    ctx.updates.update(); // notify the other slot to activate.
                } else {
                    rc.node.borrow_mut().as_mut().unwrap().update(ctx);
                }
            }
            SlotNodeState::ActiveDeinited(_) => {
                panic!("`SlotNode` in `ActiveDeinited` state on update")
            }
            SlotNodeState::Dropped => {}
        }
    }

    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        if let SlotNodeState::Active(rc) = &self.state {
            rc.node.borrow_mut().as_mut().unwrap().update_hp(ctx);
        }
    }

    fn measure(&mut self, ctx: &mut LayoutContext, available_size: LayoutSize) -> LayoutSize {
        if let SlotNodeState::Active(rc) = &self.state {
            rc.node.borrow_mut().as_mut().unwrap().measure(ctx, available_size)
        } else {
            LayoutSize::zero()
        }
    }

    fn arrange(&mut self, ctx: &mut LayoutContext, final_size: LayoutSize) {
        if let SlotNodeState::Active(rc) = &self.state {
            rc.node.borrow_mut().as_mut().unwrap().arrange(ctx, final_size);
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
