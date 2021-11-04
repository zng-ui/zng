use zero_ui::prelude::new_property::*;

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
    set_widget_state_update(child, key, value, |_, _| {})
}

/// Helper for declaring properties that set the widget state that affects layout.
///
/// When `value` updates a layout update is requested.
///
/// See [`set_widget_state`] for more details.
pub fn set_widget_layout_state<U, K, V>(child: U, key: K, value: V) -> impl UiNode
where
    U: UiNode,
    K: StateKey,
    K::Type: VarValue,
    V: IntoVar<K::Type>,
{
    set_widget_state_update(child, key, value, |ctx, _| ctx.updates.layout())
}

/// Helper for declaring properties that set the widget state that affects render.
///
/// When `value` updates a new frame render is requested.
///
/// See [`set_widget_state`] for more details.
pub fn set_widget_render_state<U, K, V>(child: U, key: K, value: V) -> impl UiNode
where
    U: UiNode,
    K: StateKey,
    K::Type: VarValue,
    V: IntoVar<K::Type>,
{
    set_widget_state_update(child, key, value, |ctx, _| ctx.updates.render())
}

/// Helper for declaring properties that set the widget state with a custom closure executed when the value updates.
///
/// The `on_update` closure is called every time the `value` variable updates.
///
/// See [`set_widget_state`] for more details.
pub fn set_widget_state_update<U, K, V, H>(child: U, key: K, value: V, on_update: H) -> impl UiNode
where
    U: UiNode,
    K: StateKey,
    K::Type: VarValue,
    V: IntoVar<K::Type>,
    H: FnMut(&mut WidgetContext, &K::Type) + 'static,
{
    struct SetWidgetStateNode<U, K, V, H> {
        child: U,
        key: K,
        var: V,
        on_update: H,
    }
    #[impl_ui_node(child)]
    impl<U, K, V, H> UiNode for SetWidgetStateNode<U, K, V, H>
    where
        U: UiNode,
        K: StateKey,
        K::Type: VarValue,
        V: Var<K::Type>,
        H: FnMut(&mut WidgetContext, &K::Type) + 'static,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            ctx.widget_state.set(self.key, self.var.get(ctx).clone());
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(new) = self.var.clone_new(ctx) {
                (self.on_update)(ctx, &new);
                ctx.widget_state.set(self.key, new);
            }
            self.child.update(ctx);
        }
    }
    SetWidgetStateNode {
        child,
        key,
        var: value.into_var(),
        on_update,
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
        let mut ctx = TestWidgetContext::new();

        assert_eq!(None, wgt.state().get(TestKey));

        wgt.test_init(&mut ctx);
        assert_eq!(Some(&2), wgt.state().get(TestKey));
        value.set(&ctx.vars, 4);
        ctx.apply_updates();
        wgt.test_update(&mut ctx);
        assert_eq!(Some(&4), wgt.state().get(TestKey));
    }

    context_var! {
        struct TestVar: u8 = 1;
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
                self.value.set(ctx.vars, *TestVar::get(ctx)).expect("probe var is read-only");
            }

            fn update(&mut self, ctx: &mut WidgetContext) {
                self.child.update(ctx);
                if let Some(&new) = TestVar::get_new(ctx) {
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
        let mut ctx = TestWidgetContext::new();

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

        let mut ctx = TestWidgetContext::new();

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
        let mut ctx = TestWidgetContext::new();

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
        let mut ctx = TestWidgetContext::new();

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

        let mut ctx = TestWidgetContext::new();

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
