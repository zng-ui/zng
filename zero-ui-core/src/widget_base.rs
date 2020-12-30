//! The implicit mixin, default constructors and properties used in all or most widgets.

use crate::context::{LayoutContext, LazyStateMap, WidgetContext};
use crate::render::{FrameBuilder, FrameUpdate, WidgetTransformKey};
use crate::units::{LayoutSize, PixelGridExt};
use crate::{impl_ui_node, property, widget_mixin, NilUiNode, UiNode, Widget, WidgetId};

/// Widget id.
///
/// # Implicit
///
/// All widgets automatically inherit from [`implicit_mixin`](module@implicit_mixin) that defines an `id`
/// property that maps to this property and sets a default value of `WidgetId::new_unique()`.
///
/// The default widget `new` function captures this `id` property and uses in the default
/// [`Widget`](crate::core::Widget) implementation.
#[property(capture_only)]
pub fn widget_id(id: WidgetId) -> ! {}

widget_mixin! {
    /// Mix-in inherited implicitly by all [widgets](widget!).
    pub implicit_mixin;

    default {
        /// Unique identifier of the widget.
        /// Set to [`WidgetId::new_unique()`](WidgetId::new_unique()) by default.
        id -> widget_id: WidgetId::new_unique();
    }
}

// This is called by the default widgets `new_child` function.
///
/// See [`widget!`](module@crate::widget) for more details.
///
/// Returns a [`NilUiNode`].
#[inline]
pub fn default_widget_new_child() -> impl UiNode {
    NilUiNode
}

/// This is called by the default widgets `new` function.
///
/// See [`widget!`](module@crate::widget) for more details.
///
/// A new widget context is introduced by this function. `child` is wrapped in a node that calls
/// [`WidgetContext::widget_context`](crate::contextWidgetContext::widget_context) and
/// [`FrameBuilder::push_widget`](crate::render::FrameBuilder::push_widget) to define the widget.
#[inline]
pub fn default_widget_new(child: impl UiNode, id_args: impl widget_id::Args) -> impl Widget {
    WidgetNode {
        id: id_args.unwrap(),
        transform_key: WidgetTransformKey::new_unique(),
        state: LazyStateMap::default(),
        child,
        size: LayoutSize::zero(),
    }
}

struct WidgetNode<T: UiNode> {
    id: WidgetId,
    transform_key: WidgetTransformKey,
    state: LazyStateMap,
    child: T,
    size: LayoutSize,
}

#[impl_ui_node(child)]
impl<T: UiNode> UiNode for WidgetNode<T> {
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

    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        #[cfg(debug_assertions)]
        {
            fn valid_measure(f: f32) -> bool {
                f.is_finite() || crate::is_layout_any_size(f)
            }

            if !valid_measure(available_size.width) || !valid_measure(available_size.height) {
                error_println!(
                    "{:?} `UiNode::measure` called with invalid `available_size: {:?}`, must be finite or `LAYOUT_ANY_SIZE`",
                    self.id,
                    available_size
                );
            }
        }

        let child_size = self.child.measure(available_size, ctx);

        #[cfg(debug_assertions)]
        {
            if !child_size.width.is_finite() || !child_size.height.is_finite() {
                error_println!("{:?} `UiNode::measure` result is not finite: `{:?}`", self.id, child_size);
            } else if !child_size.is_aligned_to(ctx.pixel_grid()) {
                let snapped = child_size.snap_to(ctx.pixel_grid());
                error_println!(
                    "{:?} `UiNode::measure` result not aligned, was: `{:?}`, expected: `{:?}`",
                    self.id,
                    child_size,
                    snapped
                );
                return snapped;
            }
        }
        child_size
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.size = final_size;

        #[cfg(debug_assertions)]
        {
            if !final_size.width.is_finite() || !final_size.height.is_finite() {
                error_println!(
                    "{:?} `UiNode::arrange` called with invalid `final_size: {:?}`, must be finite",
                    self.id,
                    final_size
                );
            } else if !final_size.is_aligned_to(ctx.pixel_grid()) {
                self.size = final_size.snap_to(ctx.pixel_grid());
                error_println!(
                    "{:?} `UiNode::arrange` called with not aligned value, was: `{:?}`, expected: `{:?}`",
                    self.id,
                    final_size,
                    self.size
                );
            }
        }

        self.child.arrange(self.size, ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_widget(self.id, self.transform_key, self.size, &self.child);
    }

    fn render_update(&self, update: &mut FrameUpdate) {
        update.update_widget(self.id, self.transform_key, &self.child);
    }
}
impl<T: UiNode> Widget for WidgetNode<T> {
    #[inline]
    fn id(&self) -> WidgetId {
        self.id
    }
    #[inline]
    fn state(&self) -> &LazyStateMap {
        &self.state
    }
    #[inline]
    fn state_mut(&mut self) -> &mut LazyStateMap {
        &mut self.state
    }
    #[inline]
    fn size(&self) -> LayoutSize {
        self.size
    }
}
