use crate::{
    context::{WidgetContext, WidgetUpdates},
    event::{EventHandles, EventUpdate},
    ui_node,
    var::{BoxedVar, Var, VarHandles},
};

use super::{BoxedUiNode, UiNode};

///Builds a node that can be one of multiple options selected by the first condition that is `true` or a fallback default.
///
/// When the selected node changed the previous one is deinited and the new one is inited.
pub struct WhenUiNodeBuilder {
    default: BoxedUiNode,
    conditions: Vec<(BoxedVar<bool>, BoxedUiNode)>,
}
impl WhenUiNodeBuilder {
    /// New with node that is used when no condition is active.
    pub fn new(default: impl UiNode) -> Self {
        Self {
            default: default.boxed(),
            conditions: vec![],
        }
    }

    /// Push a conditional node.
    ///
    /// When `condition` is `true` and no previous inserted condition is `true` the `node` is used.
    pub fn push(&mut self, condition: impl Var<bool>, node: impl UiNode) {
        self.conditions.push((condition.boxed(), node.boxed()));
    }

    /// Build a node that is always the first `true` condition or the default.
    pub fn build(self) -> impl UiNode {
        WhenUiNode {
            default: self.default,
            conditions: self.conditions,
            current: usize::MAX,
            var_handles: VarHandles::default(),
            event_handles: EventHandles::default(),
        }
    }
}

struct WhenUiNode {
    default: BoxedUiNode,
    conditions: Vec<(BoxedVar<bool>, BoxedUiNode)>,
    current: usize,
    var_handles: VarHandles,
    event_handles: EventHandles,
}
impl WhenUiNode {
    fn child(&self) -> &BoxedUiNode {
        if self.current == usize::MAX {
            &self.default
        } else {
            &self.conditions[self.current].1
        }
    }

    fn child_mut_with_handles(&mut self) -> (&mut BoxedUiNode, &mut VarHandles, &mut EventHandles) {
        let child = if self.current == usize::MAX {
            &mut self.default
        } else {
            &mut self.conditions[self.current].1
        };
        (child, &mut self.var_handles, &mut self.event_handles)
    }

    fn change_child(&mut self, ctx: &mut WidgetContext, new: usize) {
        {
            let (child, var_handles, event_handles) = self.child_mut_with_handles();
            ctx.with_handles(var_handles, event_handles, |ctx| child.deinit(ctx));
            var_handles.clear();
            event_handles.clear();
        }

        self.current = new;

        {
            let (child, var_handles, event_handles) = self.child_mut_with_handles();
            ctx.with_handles(var_handles, event_handles, |ctx| child.init(ctx));
        }
    }
}
#[ui_node(
    delegate = self.child(),
    delegate_mut = self.child_mut_with_handles().0,
)]
impl UiNode for WhenUiNode {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.current = usize::MAX;
        for (i, (c, _)) in self.conditions.iter().enumerate() {
            if self.current == usize::MAX && c.get() {
                self.current = i;
            }
            ctx.sub_var(c);
        }

        let (child, var_handles, event_handles) = self.child_mut_with_handles();
        ctx.with_handles(var_handles, event_handles, |ctx| child.init(ctx));
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        let (child, var_handles, event_handles) = self.child_mut_with_handles();
        ctx.with_handles(var_handles, event_handles, |ctx| child.deinit(ctx));
        var_handles.clear();
        event_handles.clear();
    }

    fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
        let (child, var_handles, event_handles) = self.child_mut_with_handles();
        ctx.with_handles(var_handles, event_handles, |ctx| child.event(ctx, update));
    }

    fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
        let mut any = false;
        for (i, (c, _)) in self.conditions.iter().enumerate() {
            if i < self.current {
                // if activated < current
                if c.get() {
                    any = true;
                    self.change_child(ctx, i);

                    break;
                }
            } else if i == self.current {
                // if deactivated current
                if c.get() {
                    break;
                }
            } else if c.get() {
                // if deactivated current and had another active after
                any = true;
                self.change_child(ctx, i);

                break;
            }
        }

        if !any && self.current != usize::MAX {
            // if no longer has not active condition.
            self.change_child(ctx, usize::MAX);
        }

        let (child, var_handles, event_handles) = self.child_mut_with_handles();
        ctx.with_handles(var_handles, event_handles, |ctx| child.update(ctx, updates));
    }
}
