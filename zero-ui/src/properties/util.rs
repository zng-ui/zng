use zero_ui::prelude::new_property::*;
pub use zero_ui_core::render::webrender_api::PrimitiveFlags;

/// Sets the [`PrimitiveFlags`] of the widget stacking context.
///
/// This is a low level property that helps tagging special widgets for the renderer.
///
/// In particular scrollbars and scroll-thumbs need to set this to their respective flags.
#[property(context, default(PrimitiveFlags::empty()))]
pub fn primitive_flags(child: impl UiNode, flags: impl IntoVar<PrimitiveFlags>) -> impl UiNode {
    struct PrimitiveFlagsNode<C, F> {
        child: C,
        flags: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: Var<PrimitiveFlags>> UiNode for PrimitiveFlagsNode<C, F> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.flags);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);
            if self.flags.is_new(ctx) {
                ctx.updates.render();
            }
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_inner_flags(self.flags.copy(ctx), |frame| self.child.render(ctx, frame));
        }
    }
    PrimitiveFlagsNode {
        child,
        flags: flags.into_var(),
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
        ctx.subscriptions(|ctx, subs| wgt.subscriptions(ctx, subs));

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

    #[property(fill)]
    fn test_var_probe(child: impl UiNode, value: impl Var<u8>) -> impl UiNode {
        struct TestVarProbeNode<C, V> {
            child: C,
            value: V,
        }
        #[impl_ui_node(child)]
        impl<C: UiNode, V: Var<u8>> UiNode for TestVarProbeNode<C, V> {
            fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                subscriptions.var(ctx, &TestVar::new());
                self.child.subscriptions(ctx, subscriptions);
            }

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
        ctx.subscriptions(|ctx, subs| wgt.subscriptions(ctx, subs));
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
        ctx.subscriptions(|ctx, subs| wgt.subscriptions(ctx, subs));
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
        ctx.subscriptions(|ctx, subs| wgt.subscriptions(ctx, subs));
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
