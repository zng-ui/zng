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
    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize;

    /// Called every time a layout update is needed, after [`measure`](UiNode::measure).
    ///
    /// # Arguments
    /// * `final_size`: The size the parent node reserved for the node. Must reposition its contents
    /// to fit this size. The value does not contain infinity or NaNs and is pixel aligned.
    /// TODO args docs.
    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext);

    /// Called every time a new frame must be rendered.
    ///
    /// # Arguments
    /// * `frame`: Contains the next frame draw instructions.
    fn render(&self, frame: &mut FrameBuilder);

    /// Called every time a frame can be updated without fully rebuilding.
    ///
    /// # Arguments
    /// * `update`: Contains the frame value updates.
    fn render_update(&self, update: &mut FrameUpdate);

    /// Box this node, unless it is already `Box<dyn UiNode>`.
    fn boxed(self) -> Box<dyn UiNode>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}
#[impl_ui_node(delegate: self.as_ref(), delegate_mut: self.as_mut())]
impl UiNode for Box<dyn UiNode> {
    fn boxed(self) -> Box<dyn UiNode> {
        self
    }
}

/// Represents an widget [`UiNode`].
pub trait Widget: UiNode {
    fn id(&self) -> WidgetId;

    fn state(&self) -> &LazyStateMap;
    fn state_mut(&mut self) -> &mut LazyStateMap;

    /// Last arranged size.
    fn size(&self) -> LayoutSize;

    /// Box this widget node, unless it is already `Box<dyn Widget>`.
    fn boxed_widget(self) -> Box<dyn Widget>
    where
        Self: Sized,
    {
        Box::new(self)
    }

    /// Run [`UiNode::init`] using the [`TestWidgetContext`].
    #[cfg(test)]
    fn test_init(&mut self, ctx: &mut TestWidgetContext) {
        ctx.widget_context(&mut LazyStateMap::default(), |ctx| {
            self.init(ctx);
        });
    }

    /// Run [`UiNode::update`] using the [`TestWidgetContext`].
    #[cfg(test)]
    fn test_update(&mut self, ctx: &mut TestWidgetContext) {
        ctx.widget_context(&mut LazyStateMap::default(), |ctx| {
            self.update(ctx);
        });
    }
}
#[impl_ui_node(delegate: self.as_ref(), delegate_mut: self.as_mut())]
impl UiNode for Box<dyn Widget> {}
impl Widget for Box<dyn Widget> {
    #[inline]
    fn id(&self) -> WidgetId {
        self.as_ref().id()
    }
    #[inline]
    fn state(&self) -> &LazyStateMap {
        self.as_ref().state()
    }
    #[inline]
    fn state_mut(&mut self) -> &mut LazyStateMap {
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
    fn measure(&mut self, _: LayoutSize, _: &mut LayoutContext) -> LayoutSize {
        LayoutSize::zero()
    }
}

/// A UI node that does not contain any other node, fills the available space, but renders nothing.
pub struct FillUiNode;
#[impl_ui_node(none)]
impl UiNode for FillUiNode {}

// Used by #[impl_ui_node] to validate custom delegation.
pub mod ui_node_asserts {
    use crate::{UiNode, UiNodeList};

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
}
