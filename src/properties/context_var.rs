use crate::core2::*;
use crate::{impl_ui_node, property};

struct SetContextVar<U: UiNode, T: VarValue, C: ContextVar<Type = T>, V: Var<T>> {
    child: U,
    var: C,
    value: V,
}
#[impl_ui_node(child)]
impl<U: UiNode, T: VarValue, C: ContextVar<Type = T>, V: Var<T>> UiNode for SetContextVar<U, T, C, V> {
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
        ctx.vars
            .with_context_bind(self.var, &self.value, || child.update_hp(ctx));
    }
}

#[property(context_var)]
pub fn set_context_var<T: VarValue>(
    child: impl UiNode,
    var: impl ContextVar<Type = T>,
    value: impl IntoVar<T>,
) -> impl UiNode {
    SetContextVar {
        child,
        var,
        value: value.into_var(),
    }
}
