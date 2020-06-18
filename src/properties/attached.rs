//! Properties that attach a value to a widget and/or its branch.

use crate::core::context::*;
use crate::core::impl_ui_node;
use crate::core::var::*;
use crate::core::UiNode;

struct WithContextVar<U: UiNode, T: VarValue, C: ContextVar<Type = T>, V: Var<T>> {
    child: U,
    var: C,
    value: V,
}
#[impl_ui_node(child)]
impl<U: UiNode, T: VarValue, C: ContextVar<Type = T>, V: Var<T>> UiNode for WithContextVar<U, T, C, V> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        let child = &mut self.child;
        ctx.vars.with_context_bind(self.var, &self.value, || child.init(ctx));
    }
    fn deinit(&mut self, ctx: &mut WidgetContext) {
        let child = &mut self.child;
        ctx.vars.with_context_bind(self.var, &self.value, || child.deinit(ctx));
    }
    fn update(&mut self, ctx: &mut WidgetContext) {
        let child = &mut self.child;
        ctx.vars.with_context_bind(self.var, &self.value, || child.update(ctx));
    }
    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        let child = &mut self.child;
        ctx.vars.with_context_bind(self.var, &self.value, || child.update_hp(ctx));
    }
}

/// Helper for declaring properties that set a context var.
///
/// # Example
/// ```
/// # fn main() -> () { }
/// use zero_ui::properties::with_context_var;
/// use zero_ui::core::{UiNode, var::{IntoVar, context_var}, property};
///
/// context_var! {
///     pub struct FontSize: u32 = const 14;
/// }
///
/// /// Sets the [`FontSize`](FontSize) context var.
/// #[property(context)]
/// pub fn font_size(child: impl UiNode, size: impl IntoVar<u32>) -> impl UiNode {
///     with_context_var(child, FontSize, size)
/// }
/// ```
pub fn with_context_var<T: VarValue>(child: impl UiNode, var: impl ContextVar<Type = T>, value: impl IntoVar<T>) -> impl UiNode {
    WithContextVar {
        child,
        var,
        value: value.into_var(),
    }
}

struct SetWidgetState<U: UiNode, K: StateKey> {
    child: U,
    pre_init: Option<(K, K::Type)>,
}

#[impl_ui_node(child)]
impl<U: UiNode, K: StateKey> UiNode for SetWidgetState<U, K> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        if let Some((key, value)) = self.pre_init.take() {
            ctx.widget_state.set(key, value);
        }
        self.child.init(ctx);
    }
}

/// Helper for declaring properties that set the widget state.
///
/// On the first [`init`](UiNode::init) `key` and `value` are moved to the [`widget_state`](WidgetContext::widget_state).
///
/// # Example
/// ```
/// # fn main() -> () { }
/// use zero_ui::core::{property, state_key, UiNode, Widget};
/// use zero_ui::properties::set_widget_state;
///
/// state_key! {
///     pub struct FooKey: u32;
/// }
///
/// #[property(context)]
/// pub fn foo(child: impl UiNode, value: u32) -> impl UiNode {
///     set_widget_state(child, FooKey, value)
/// }
///
/// // after the property is used and the widget initializes:
///
/// /// Get the value from outside the widget.
/// fn get_foo_outer(widget: &impl Widget) -> u32 {
///     widget.state().get(FooKey).copied().unwrap_or_default()    
/// }
///
/// /// Get the value from inside the widget.
/// fn get_foo_inner(ctx: &WidgetContext) -> u32 {
///     ctx.widget_state.get(FooKey).copied().unwrap_or_default()    
/// }
/// ```
pub fn set_widget_state<K: StateKey>(child: impl UiNode, key: K, value: K::Type) -> impl UiNode {
    SetWidgetState {
        child,
        pre_init: Some((key, value)),
    }
}
