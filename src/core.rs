//! Core infrastructure required for running a zero-ui app.

#[macro_use]
pub mod var;

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
    /// indicate the parent will accommodate any size. You can use [`is_layout_any_size`](is_layout_any_size) for that end.
    ///
    /// # Return
    /// Must return the nodes desired size. Must not contain infinity or NaN.
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize;

    /// Called every time a layout update is needed, after [`measure`](UiNode::measure).
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

/// This is called by the default widgets `new_child` function.
///
/// Nothing is done in this function, `child` is returned directly.
#[inline]
pub fn default_widget_new_child<C: UiNode>(child: C) -> C {
    child
}

/// This is called by the default widgets `new` function.
///
/// A new widget context is introduced by this function. `child` is wrapped in a node that calls
/// [`WidgetContext::widget_context`](WidgetContext::widget_context) and [`FrameBuilder::push_widget`] to define the widget.
#[inline]
pub fn default_widget_new(child: impl UiNode, id_args: impl zero_ui::properties::id::Args) -> impl UiNode {
    Widget {
        id: id_args.unwrap().0,
        state: LazyStateMap::default(),
        child,
        area: LayoutSize::zero(),
    }
}

/// Gets if the value indicates that any size is available during layout (positive infinity)
#[inline]
pub fn is_layout_any_size(f: f32) -> bool {
    f.is_infinite() && f.is_sign_positive()
}
