use zero_ui::prelude::new_property::*;

/// Helper for declaring properties that sets a context var.
///
/// # Example
///
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
/// #[property(context, default(FontSizeVar))]
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

/// Helper for declaring properties that sets a context var for the widget only.
///
/// This is similar to [`with_context_var`] except the context var value is visible only inside
/// the `child` nodes that are part of the same widget that is the parent of the return node.
///
/// # Example
///
/// ```
/// # fn main() -> () { }
/// use zero_ui::properties::with_context_var_wgt_only;
/// use zero_ui::core::{UiNode, var::{IntoVar, context_var}, property, border::BorderRadius};
///
/// context_var! {
///     pub struct CornersClipVar: BorderRadius = once BorderRadius::zero();
/// }
///
/// /// Sets widget content clip corner radius.
/// #[property(context)]
/// pub fn corners_clip(child: impl UiNode, radius: impl IntoVar<BorderRadius>) -> impl UiNode {
///     with_context_var_wgt_only(child, CornersClipVar, radius)
/// }
/// ```
pub fn with_context_var_wgt_only<T: VarValue>(child: impl UiNode, var: impl ContextVar<Type = T>, value: impl IntoVar<T>) -> impl UiNode {
    struct WithContextVarWidgetOnlyNode<U, C, V> {
        child: U,
        var: C,
        value: V,
    }
    #[impl_ui_node(child)]
    impl<U, T, C, V> UiNode for WithContextVarWidgetOnlyNode<U, C, V>
    where
        U: UiNode,
        T: VarValue,
        C: ContextVar<Type = T>,
        V: Var<T>,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            let child = &mut self.child;
            ctx.vars.with_context_bind_wgt_only(self.var, &self.value, || child.init(ctx));
        }
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            let child = &mut self.child;
            ctx.vars.with_context_bind_wgt_only(self.var, &self.value, || child.deinit(ctx));
        }
        fn update(&mut self, ctx: &mut WidgetContext) {
            let child = &mut self.child;
            ctx.vars.with_context_bind_wgt_only(self.var, &self.value, || child.update(ctx));
        }
        fn update_hp(&mut self, ctx: &mut WidgetContext) {
            let child = &mut self.child;
            ctx.vars.with_context_bind_wgt_only(self.var, &self.value, || child.update_hp(ctx));
        }
    }
    WithContextVarWidgetOnlyNode {
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
///     widget.state().get::<FooKey>().copied().unwrap_or_default()
/// }
///
/// /// Get the value from inside the widget.
/// fn get_foo_inner(ctx: &WidgetContext) -> u32 {
///     ctx.widget_state.get::<FooKey>().copied().unwrap_or_default()
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
        _key: K,
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
            ctx.widget_state.set::<K>(self.var.get(ctx.vars).clone());
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(new) = self.var.get_new(ctx.vars) {
                ctx.widget_state.set::<K>(new.clone());
            }
            self.child.update(ctx);
        }
    }
    SetWidgetStateNode {
        child,
        _key: key,
        var: value.into_var(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        context::TestWidgetContext,
        var::{var, IntoVar},
        UiNode,
    };
    use crate::widgets::{blank, container, layouts::v_stack};

    state_key! {
        struct TestKey: u8;
    }

    #[property(context)]
    fn set_state_test(child: impl UiNode, value: impl IntoVar<u8>) -> impl UiNode {
        set_widget_state(child, TestKey, value)
    }

    #[test]
    fn set_widget_state_init_and_update() {
        let value = var(2);
        let mut wgt = blank! {
            set_state_test = value.clone();
        };
        let mut ctx = TestWidgetContext::wait_new();

        assert_eq!(None, wgt.state().get::<TestKey>());

        wgt.test_init(&mut ctx);
        assert_eq!(Some(&2), wgt.state().get::<TestKey>());

        value.set(&ctx.vars, 4);
        ctx.apply_updates();
        wgt.test_update(&mut ctx);
        assert_eq!(Some(&4), wgt.state().get::<TestKey>());
    }

    context_var! {
        struct TestVar: u8 = const 1;
    }

    #[property(context)]
    fn with_var_test(child: impl UiNode, value: impl IntoVar<u8>) -> impl UiNode {
        with_context_var(child, TestVar, value)
    }

    #[property(context)]
    fn with_var_wgt_only_test(child: impl UiNode, value: impl IntoVar<u8>) -> impl UiNode {
        with_context_var_wgt_only(child, TestVar, value)
    }

    #[property(inner)]
    fn test_var_probe(child: impl UiNode, value: impl Var<u8>) -> impl UiNode {
        struct TestVarProbeNode<C, V> {
            child: C,
            value: V,
        }
        #[impl_ui_node(child)]
        impl<C: UiNode, V: Var<u8>> UiNode for TestVarProbeNode<C, V> {
            fn init(&mut self, ctx: &mut WidgetContext) {
                self.child.init(ctx);
                self.value.set(ctx.vars, *TestVar::get(ctx.vars)).expect("probe var is read-only");
            }

            fn update(&mut self, ctx: &mut WidgetContext) {
                self.child.update(ctx);
                if let Some(&new) = TestVar::get_new(ctx.vars) {
                    self.value.set(ctx.vars, new).expect("probe var is read-only");
                }
            }
        }
        TestVarProbeNode { child, value }
    }

    #[test]
    fn with_context_var_same_widget() {
        let value = var(2);
        let probe = var(0);
        let mut wgt = blank! {
            with_var_test = value.clone();
            test_var_probe = probe.clone();
        };
        let mut ctx = TestWidgetContext::wait_new();

        wgt.test_init(&mut ctx);
        ctx.apply_updates();

        assert_eq!(&2, probe.get(&ctx.vars));

        value.set(&ctx.vars, 3);
        ctx.apply_updates();
        wgt.test_update(&mut ctx);
        ctx.apply_updates();

        assert_eq!(&3, probe.get(&ctx.vars));
    }

    #[test]
    fn with_context_var_inner_widget() {
        let value = var(2);
        let probe = var(0);

        let mut wgt = container! {
            content = blank! {
                test_var_probe = probe.clone();
            };
            with_var_test = value.clone();
        };

        let mut ctx = TestWidgetContext::wait_new();

        wgt.test_init(&mut ctx);
        ctx.apply_updates();

        assert_eq!(&2, probe.get(&ctx.vars));

        value.set(&ctx.vars, 3);
        ctx.apply_updates();
        wgt.test_update(&mut ctx);
        ctx.apply_updates();

        assert_eq!(&3, probe.get(&ctx.vars));
    }

    #[test]
    fn with_context_var_sibling_not_affected() {
        let value = var(2);
        let probe = var(0);

        let mut wgt = v_stack! {
            items = widgets![
                blank! {
                    with_var_test = value.clone();
                },
                blank! {
                    test_var_probe = probe.clone();
                }
            ];
        };
        let mut ctx = TestWidgetContext::wait_new();

        wgt.test_init(&mut ctx);
        ctx.apply_updates();

        // `1` is the default value.
        assert_eq!(&1, probe.get(&ctx.vars));

        value.set(&ctx.vars, 3);
        ctx.apply_updates();
        wgt.test_update(&mut ctx);
        ctx.apply_updates();

        assert_eq!(&1, probe.get(&ctx.vars));
    }

    #[test]
    fn with_context_var_wgt_only_same_widget() {
        let value = var(2);
        let probe = var(0);
        let mut wgt = blank! {
            with_var_wgt_only_test = value.clone();
            test_var_probe = probe.clone();
        };
        let mut ctx = TestWidgetContext::wait_new();

        wgt.test_init(&mut ctx);
        ctx.apply_updates();

        assert_eq!(&2, probe.get(&ctx.vars));

        value.set(&ctx.vars, 3);
        ctx.apply_updates();
        wgt.test_update(&mut ctx);
        ctx.apply_updates();

        assert_eq!(&3, probe.get(&ctx.vars));
    }

    #[test]
    fn with_context_var_wgt_only_inner_widget_not_affected() {
        let value = var(2);
        let probe = var(0);

        let mut wgt = container! {
            content = blank! {
                test_var_probe = probe.clone();
            };
            with_var_wgt_only_test = value.clone();
        };

        let mut ctx = TestWidgetContext::wait_new();

        wgt.test_init(&mut ctx);
        ctx.apply_updates();

        // `1` is the default value.
        assert_eq!(&1, probe.get(&ctx.vars));

        value.set(&ctx.vars, 3);
        ctx.apply_updates();
        wgt.test_update(&mut ctx);
        ctx.apply_updates();

        assert_eq!(&1, probe.get(&ctx.vars));
    }
}
