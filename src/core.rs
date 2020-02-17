//! Core infrastruture required for runing a zero-ui app.

pub mod animation;
pub mod app;
pub mod context;
pub mod event;
pub mod focus;
pub mod font;
pub mod gesture;
pub mod keyboard;
pub mod mouse;
pub mod profiler;
pub mod render;
pub mod types;
pub mod var;
pub mod window;

use crate::impl_ui_node;
use context::{LazyStateMap, WidgetContext};
use render::FrameBuilder;
use types::{LayoutSize, WidgetId};

/// An Ui tree node.
pub trait UiNode: 'static {
    /// Called every time the node is plugged in an Ui tree.
    fn init(&mut self, ctx: &mut WidgetContext);

    /// Called every time the node is unplugged from an Ui tree.
    fn deinit(&mut self, ctx: &mut WidgetContext);

    /// Called every time a low pressure event update happens.
    ///
    /// # Event Pressure
    /// See [update_hp](UiNode::update_hp) for more information about event pressure rate.
    fn update(&mut self, ctx: &mut WidgetContext);

    /// Called every time a high pressure event update happens.
    ///
    /// # Event Pressure
    /// Some events occur alot more times then others, for performance reasons this
    /// event source may choose to be propagated in the this hight pressure lane.
    ///
    /// Event sources that are high pressure mention this in their documentation.
    fn update_hp(&mut self, ctx: &mut WidgetContext);

    /// Called every time a layout update is needed.
    ///
    /// # Arguments
    /// * `available_size`: The total available size for the node. Can contain positive infinity to
    /// indicate the parent will accommodate any size.
    ///
    /// # Return
    /// Must return the nodes desired size. Must not contain infinity or NaN.
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize;

    /// Called every time a layout update is needed, after [measure](UiNode::measure).
    ///
    /// # Arguments
    /// * `final_size`: The size the parent node reserved for the node. Must reposition its contents
    /// to fit this size. The value does not contain infinity or NaNs.
    fn arrange(&mut self, final_size: LayoutSize);

    /// Called every time a new frame must be rendered.
    ///
    /// # Arguments
    /// * `frame`: Contains the next frame draw instructions.
    fn render(&self, frame: &mut FrameBuilder);

    /// Box this node, unless it is already `Box<dyn UiNode>`.
    fn boxed(self) -> Box<dyn UiNode>
    where
        Self: Sized + 'static,
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

struct Widget<T: UiNode> {
    id: WidgetId,
    state: LazyStateMap,
    child: T,
    area: LayoutSize,
}

#[impl_ui_node(child)]
impl<T: UiNode> UiNode for Widget<T> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        let child = &mut self.child;
        ctx.widget_context(self.id, &mut self.state, |ctx| child.init(ctx));
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        let child = &mut self.child;
        ctx.widget_context(self.id, &mut self.state, |ctx| child.deinit(ctx));
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        let child = &mut self.child;
        ctx.widget_context(self.id, &mut self.state, |ctx| child.update(ctx));
    }

    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        let child = &mut self.child;
        ctx.widget_context(self.id, &mut self.state, |ctx| child.update_hp(ctx));
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.area = final_size;
        self.child.arrange(final_size);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_widget(self.id, self.area, &self.child);
    }
}

/// This is called from widget macros to validate `id: ?;`.
#[inline]
#[doc(hidden)]
pub fn validate_widget_id_args(id: WidgetId) -> WidgetId {
    id
}

/// This is used in widget macros to validate `id: {id: ?};`.
#[doc(hidden)]
pub struct ValidateWidgetIdArgs {
    pub id: WidgetId,
}

/// This is called by the default widget! new_child function.
#[inline]
#[doc(hidden)]
pub fn default_new_widget_child<C: UiNode>(child: C) -> C {
    child
}

/// This is called by the default widget! new function.
#[inline]
#[doc(hidden)]
pub fn default_new_widget(child: impl UiNode, id: WidgetId) -> impl UiNode {
    Widget {
        id,
        state: LazyStateMap::default(),
        child,
        area: LayoutSize::zero(),
    }
}
