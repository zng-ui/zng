use zero_ui::prelude::new_property::*;

/// Helper for declaring properties that set a context var.
///
/// # Example
/// ```
/// # fn main() -> () { }
/// use zero_ui::properties::with_context_var;
/// use zero_ui::core::{UiNode, var::{IntoVar, context_var}, property};
///
/// context_var! {
///     pub struct FontSizeVar: u32 = const 14;
/// }
///
/// /// Sets the [`FontSizeVar`] context var.
/// #[property(context)]
/// pub fn font_size(child: impl UiNode, size: impl IntoVar<u32>) -> impl UiNode {
///     with_context_var(child, FontSizeVar, size)
/// }
/// ```
pub fn with_context_var<T: VarValue>(child: impl UiNode, var: impl ContextVar<Type = T>, value: impl IntoVar<T>) -> impl UiNode {
    struct WithContextVarNode<U, C, V> {
        child: U,
        var: C,
        value: V,
    }
    #[impl_ui_node(child)]
    impl<U, T, C, V> UiNode for WithContextVarNode<U, C, V>
    where
        U: UiNode,
        T: VarValue,
        C: ContextVar<Type = T>,
        V: Var<T>,
    {
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
    WithContextVarNode {
        child,
        var,
        value: value.into_var(),
    }
}

/// Helper for declaring properties that set the widget state.
///
/// The state key is set in [`widget_state`](WidgetContext::widget_state) on init and is kept updated.
///
/// # Example
/// ```
/// # fn main() -> () { }
/// use zero_ui::core::{property, context::{state_key, WidgetContext}, var::IntoVar, UiNode, Widget};
/// use zero_ui::properties::set_widget_state;
///
/// state_key! {
///     pub struct FooKey: u32;
/// }
///
/// #[property(context)]
/// pub fn foo(child: impl UiNode, value: impl IntoVar<u32>) -> impl UiNode {
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
pub fn set_widget_state<U, K, V>(child: U, key: K, value: V) -> impl UiNode
where
    U: UiNode,
    K: StateKey,
    K::Type: VarValue,
    V: IntoVar<K::Type>,
{
    struct SetWidgetStateNode<U, K, V> {
        child: U,
        key: K,
        var: V,
    }
    #[impl_ui_node(child)]
    impl<U, K, V> UiNode for SetWidgetStateNode<U, K, V>
    where
        U: UiNode,
        K: StateKey,
        K::Type: VarValue,
        V: Var<K::Type>,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            ctx.widget_state.set(self.key, self.var.get(ctx.vars).clone());
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(new) = self.var.get_new(ctx.vars) {
                ctx.widget_state.set(self.key, new.clone());
            }
            self.child.update(ctx);
        }
    }
    SetWidgetStateNode {
        child,
        key,
        var: value.into_var(),
    }
}
