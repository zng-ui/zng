use crate::prelude::new_property::*;
use std::ops;

struct VisibilityNode<C: UiNode, V: VarLocal<Visibility>> {
    child: C,
    visibility: V,
}
impl<C: UiNode, V: VarLocal<Visibility>> VisibilityNode<C, V> {
    fn with_context(&mut self, vars: &Vars, f: impl FnOnce(&mut C)) {
        match *VisibilityVar::var().get(vars) {
            // parent collapsed => all descendants collapsed
            Visibility::Collapsed => f(&mut self.child),
            // parent hidden =>
            Visibility::Hidden => {
                // if we are collapsed
                if let Visibility::Collapsed = self.visibility.get(vars) {
                    // our branch is collapsed
                    let child = &mut self.child;
                    vars.with_context_bind(VisibilityVar, &self.visibility, || f(child));
                } else {
                    // otherwise same as parent
                    f(&mut self.child)
                }
            }
            // parent visible =>
            Visibility::Visible => {
                if let Visibility::Visible = self.visibility.get(vars) {
                    // and we are also visible, same as parent
                    f(&mut self.child)
                } else {
                    // or, our visibility is different
                    let child = &mut self.child;
                    vars.with_context_bind(VisibilityVar, &self.visibility, || f(child));
                }
            }
        }
    }
}
impl<C: UiNode, V: VarLocal<Visibility>> UiNode for VisibilityNode<C, V> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        let vis = *self.visibility.init_local(ctx.vars);
        ctx.widget_state.set(VisibilityState, vis);

        self.with_context(ctx.vars, |c| c.init(ctx));
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.with_context(ctx.vars, |c| c.deinit(ctx));
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(&vis) = self.visibility.update_local(ctx.vars) {
            ctx.widget_state.set(VisibilityState, vis);
            ctx.updates.layout();
        }
        self.with_context(ctx.vars, |c| c.update(ctx));
    }

    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        self.with_context(ctx.vars, |c| c.update_hp(ctx));
    }

    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        match *self.visibility.get_local() {
            Visibility::Visible | Visibility::Hidden => self.child.measure(available_size, ctx),
            Visibility::Collapsed => LayoutSize::zero(),
        }
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        if let Visibility::Visible = self.visibility.get_local() {
            self.child.arrange(final_size, ctx)
        }
    }

    fn render(&self, frame: &mut FrameBuilder) {
        if let Visibility::Visible = self.visibility.get_local() {
            self.child.render(frame);
        } else {
            frame
                .cancel_widget()
                .expect("visibility not set before `FrameBuilder::open_widget_display`");
        }
    }

    fn render_update(&self, update: &mut FrameUpdate) {
        if let Visibility::Visible = self.visibility.get_local() {
            self.child.render_update(update);
        } else {
            update.cancel_widget();
        }
    }
}

/// Sets the widget visibility.
#[property(context)]
pub fn visibility(child: impl UiNode, visibility: impl IntoVar<Visibility>) -> impl UiNode {
    VisibilityNode {
        child,
        visibility: visibility.into_local(),
    }
}
