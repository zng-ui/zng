//! The implicit mixin, default constructors and properties used in all or most widgets.

use crate::context::{state_key, LayoutContext, LazyStateMap, WidgetContext};
use crate::render::{FrameBuilder, FrameUpdate, WidgetInfo, WidgetTransformKey};
use crate::units::LayoutSize;
use crate::var::{context_var, IntoVar, VarLocal, Vars};
use crate::{impl_ui_node, property, widget_mixin, widget_mixin2, NilUiNode, UiNode, Widget, WidgetId};

#[cfg(debug_assertions)]
use crate::units::PixelGridExt;

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
        /// Widget id. Set to  a [unique id](WidgetId::new_unique()) by default.
        id -> widget_id: WidgetId::new_unique();

        /// If events are enabled in the widget and descendants, `true` by default.
        enabled;
    }
}

// TODO: Change name to implicit_mixin when the new widget_mixin is completed.
/// Mix-in inherited implicitly by all [widgets](widget!).
#[widget_mixin2($crate::widget_base::implicit_mixin2)]
pub mod implicit_mixin2 {
    use super::{enabled, widget_id, WidgetId};

    properties! {
        /// Widget id. Set to  a [unique id](WidgetId::new_unique()) by default.
        widget_id as id = WidgetId::new_unique();
    }

    properties! {
        /// If events are enabled in the widget and descendants, `true` by default.
        enabled;
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
    default_widget_new2(child, id_args.unwrap())
}
// TODO replace to default_widget_new with this when old widget macro is removed.
#[inline]
pub fn default_widget_new2(child: impl UiNode, id: WidgetId) -> impl Widget {
    WidgetNode {
        id,
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

state_key! {
    struct EnabledState: bool;
}

context_var! {
    /// Don't use this directly unless you read all the enabled related
    /// source code here and in core/window.rs
    #[doc(hidden)]
    pub struct IsEnabledVar: bool = return &true;
}

/// Extension method for accessing the [`enabled`] state of widgets.
pub trait WidgetEnabledExt {
    /// Gets the widget enabled state.
    ///
    /// The implementation for [`LazyStateMap`] and [`Widget`] only get the state configured
    /// in the widget, if a parent widget is disabled that does not show here. Use [`IsEnabled`]
    /// to get the inherited state from inside a widget.
    ///
    /// The implementation for [`WidgetInfo`] gets if the widget and all ancestors are enabled.
    fn enabled(&self) -> bool;
}
impl WidgetEnabledExt for LazyStateMap {
    fn enabled(&self) -> bool {
        self.get(EnabledState).copied().unwrap_or(true)
    }
}
impl<W: Widget> WidgetEnabledExt for W {
    fn enabled(&self) -> bool {
        self.state().enabled()
    }
}
impl<'a> WidgetEnabledExt for WidgetInfo<'a> {
    fn enabled(&self) -> bool {
        self.meta().enabled() && self.parent().map(|p| p.enabled()).unwrap_or(true)
    }
}

/// Contextual [`enabled`] accessor.
pub struct IsEnabled;
impl IsEnabled {
    /// Gets the enabled state in the current `vars` context.
    pub fn get(vars: &Vars) -> bool {
        *IsEnabledVar::var().get(vars)
    }
}

struct EnabledNode<C: UiNode, E: VarLocal<bool>> {
    child: C,
    enabled: E,
}
impl<C: UiNode, E: VarLocal<bool>> EnabledNode<C, E> {
    fn with_context(&mut self, vars: &Vars, f: impl FnOnce(&mut C)) {
        if IsEnabled::get(vars) {
            if *self.enabled.get(vars) {
                // context already enabled
                f(&mut self.child);
            } else {
                // we are disabling
                let child = &mut self.child;
                vars.with_context_bind(IsEnabledVar, &self.enabled, || f(child));
            }
        } else {
            // context already disabled
            f(&mut self.child);
        }
    }
}
#[impl_ui_node(child)]
impl<C: UiNode, E: VarLocal<bool>> UiNode for EnabledNode<C, E> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        if !*self.enabled.init_local(ctx.vars) {
            ctx.widget_state.set(EnabledState, false);
        }
        self.with_context(ctx.vars, |c| c.init(ctx));
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.with_context(ctx.vars, |c| c.deinit(ctx));
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(&state) = self.enabled.update_local(ctx.vars) {
            ctx.widget_state.set(EnabledState, state);
            ctx.updates.render(); // TODO meta updates without a new frame?
        }
        self.with_context(ctx.vars, |c| c.update(ctx));
    }

    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        self.with_context(ctx.vars, |c| c.update_hp(ctx));
    }

    fn render(&self, frame: &mut FrameBuilder) {
        if !*self.enabled.get_local() {
            frame.meta().set(EnabledState, false);
        }
        self.child.render(frame);
    }
}

/// If events are enabled in the widget and its descendants.
///
/// This property sets the enabled state of the widget, to probe the enabled state in `when` clauses
/// use [`is_enabled`]. To probe from inside the implementation of widgets use [`IsEnabled::get`].
/// To probe the widget state use [`WidgetEnabledExt`].
///
/// # Events
///
/// Most `on_<event>` properties do not fire when the widget is disabled. The event properties that ignore
/// the enabled status mention this in their documentation.
///
/// Most app events ([`Event`](crate:core::event::Event)) still get generated by the app extensions.
/// [`MouseDownEvent`](crate::core::mouse::MouseDownEvent) for example is emitted for a click in a disabled widget.
/// The enabled parents of the disabled widget can handle this event.
///
/// # Focus
///
/// Disabled widgets are not focusable. The focus manager skips disabled widgets.
#[property(context)]
pub fn enabled(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    EnabledNode {
        child,
        enabled: enabled.into_local(),
    }
}
