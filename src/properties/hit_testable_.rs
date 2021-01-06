use crate::prelude::new_property::*;

struct HitTestableNode<U: UiNode, H: VarLocal<bool>> {
    child: U,
    hit_testable: H,
}
impl<U: UiNode, H: VarLocal<bool>> HitTestableNode<U, H> {
    fn with_context(&mut self, vars: &Vars, f: impl FnOnce(&mut U)) {
        if IsHitTestable::get(vars) {
            if *self.hit_testable.get(vars) {
                // context already hit-testable
                f(&mut self.child);
            } else {
                // we are disabling
                let child = &mut self.child;
                vars.with_context_bind(IsHitTestableVar, &self.hit_testable, || f(child));
            }
        } else {
            // context already not hit-testable
            f(&mut self.child);
        }
    }
}
#[impl_ui_node(child)]
impl<U: UiNode, H: VarLocal<bool>> UiNode for HitTestableNode<U, H> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        if !*self.hit_testable.init_local(ctx.vars) {
            ctx.widget_state.set(HitTestableState, false);
        }
        self.with_context(ctx.vars, |c| c.init(ctx));
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.with_context(ctx.vars, |c| c.deinit(ctx));
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(&state) = self.hit_testable.update_local(ctx.vars) {
            ctx.widget_state.set(HitTestableState, state);
            ctx.updates.render();
        }
        self.with_context(ctx.vars, |c| c.update(ctx));
    }

    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        self.with_context(ctx.vars, |c| c.update_hp(ctx));
    }

    fn render(&self, frame: &mut FrameBuilder) {
        if !self.hit_testable.get_local() {
            frame.push_not_hit_testable(|frame| self.child.render(frame));
        } else {
            self.child.render(frame);
        }
    }
}

/// If the widget and its descendants are visible during hit-testing.
///
/// This property sets the hit-test visibility of the widget, to probe the state in `when` clauses
/// use [`is_hit_testable`](fn@is_hit_testable). To probe from inside the implementation of widgets use [`IsHitTestable::get`].
/// To probe the widget state use [`WidgetHitTestableExt`].
///
/// # Events
///
/// Events that use hit-testing to work are effectively disabled by setting this to `false`. That includes
/// all mouse and touch events. Because of this properties that use mouse events to work,
/// like [`cursor`](fn@super::cursor) get disabled too.
#[property(context)]
pub fn hit_testable(child: impl UiNode, hit_testable: impl IntoVar<bool>) -> impl UiNode {
    HitTestableNode {
        child,
        hit_testable: hit_testable.into_local(),
    }
}

context_var! {
    struct IsHitTestableVar: bool = return &true;
}

/// Contextual [`hit_testable`](fn@hit_testable) accessor.
pub struct IsHitTestable;
impl IsHitTestable {
    /// Gets the hit-testable state in the current `vars` context.
    pub fn get(vars: &Vars) -> bool {
        *IsHitTestableVar::var().get(vars)
    }
}

state_key! {
    struct HitTestableState: bool;
}

/// Extension method for accessing the [`hit_testable`](fn@hit_testable) state of widgets.
pub trait WidgetHitTestableExt {
    /// Gets the widget hit-test visibility.
    ///
    /// The implementation for [`LazyStateMap`] and [`Widget`] only get the state configured
    /// in the widget, if a parent widget is not hit-testable that does not show here. Use [`IsHitTestable`]
    /// to get the inherited state from inside a widget.
    ///
    /// The implementation for [`WidgetInfo`] gets if the widget and all ancestors are hit-test visible.
    fn hit_testable(&self) -> bool;
}
impl WidgetHitTestableExt for LazyStateMap {
    fn hit_testable(&self) -> bool {
        self.get(HitTestableState).copied().unwrap_or(true)
    }
}
impl<'a> WidgetHitTestableExt for WidgetInfo<'a> {
    fn hit_testable(&self) -> bool {
        self.meta().hit_testable() && self.parent().map(|p| p.hit_testable()).unwrap_or(true)
    }
}
