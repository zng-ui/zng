use crate::{
    context::{WidgetContext, WidgetUpdates},
    event::{EventHandles, EventUpdate},
    ui_node,
    var::{BoxedVar, Var, VarHandles},
};

use super::{BoxedUiNode, BoxedUiNodeList, UiNode, UiNodeList, UiNodeListObserver};

///Builds a node that can be one of multiple options selected by the first condition that is `true` or a fallback default.
///
/// When the selected node changes the previous one is deinited and the new one is inited.
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

///Builds a node list that can be one of multiple options selected by the first condition that is `true` or a fallback default.
///
/// When the selected list changes the previous one is deinited and the new one is inited.
pub struct WhenUiNodeListBuilder {
    default: BoxedUiNodeList,
    conditions: Vec<(BoxedVar<bool>, BoxedUiNodeList)>,
}
impl WhenUiNodeListBuilder {
    /// New with list that is used when no condition is active.
    pub fn new(default: impl UiNodeList) -> Self {
        Self {
            default: default.boxed(),
            conditions: vec![],
        }
    }

    /// Push a conditional list.
    ///
    /// When `condition` is `true` and no previous inserted condition is `true` the `list` is used.
    pub fn push(&mut self, condition: impl Var<bool>, list: impl UiNodeList) {
        self.conditions.push((condition.boxed(), list.boxed()));
    }

    /// Build a list that is always the first `true` condition or the default.
    pub fn build(self) -> impl UiNodeList {
        WhenUiNodeList {
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
                    any = true;
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

struct WhenUiNodeList {
    default: BoxedUiNodeList,
    conditions: Vec<(BoxedVar<bool>, BoxedUiNodeList)>,
    current: usize,
    var_handles: VarHandles,
    event_handles: EventHandles,
}
impl WhenUiNodeList {
    fn children(&self) -> &BoxedUiNodeList {
        if self.current == usize::MAX {
            &self.default
        } else {
            &self.conditions[self.current].1
        }
    }

    fn children_mut_with_handles(&mut self) -> (&mut BoxedUiNodeList, &mut VarHandles, &mut EventHandles) {
        let child = if self.current == usize::MAX {
            &mut self.default
        } else {
            &mut self.conditions[self.current].1
        };
        (child, &mut self.var_handles, &mut self.event_handles)
    }

    fn change_children(&mut self, ctx: &mut WidgetContext, observer: &mut dyn UiNodeListObserver, new: usize) {
        {
            let (child, var_handles, event_handles) = self.children_mut_with_handles();
            ctx.with_handles(var_handles, event_handles, |ctx| child.deinit_all(ctx));
            var_handles.clear();
            event_handles.clear();
        }

        self.current = new;

        {
            let (child, var_handles, event_handles) = self.children_mut_with_handles();
            ctx.with_handles(var_handles, event_handles, |ctx| child.init_all(ctx));
        }

        observer.reseted();
    }
}

impl UiNodeList for WhenUiNodeList {
    fn with_node<R, F>(&self, index: usize, f: F) -> R
    where
        F: FnOnce(&BoxedUiNode) -> R,
    {
        self.children().with_node(index, f)
    }

    fn with_node_mut<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        self.children_mut_with_handles().0.with_node_mut(index, f)
    }

    fn for_each<F>(&self, f: F)
    where
        F: FnMut(usize, &BoxedUiNode) -> bool,
    {
        self.children().for_each(f)
    }

    fn for_each_mut<F>(&mut self, f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode) -> bool,
    {
        self.children_mut_with_handles().0.for_each_mut(f)
    }

    fn len(&self) -> usize {
        self.children().len()
    }

    fn boxed(self) -> BoxedUiNodeList {
        Box::new(self)
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        self.children_mut_with_handles().0.drain_into(vec)
    }

    fn init_all(&mut self, ctx: &mut WidgetContext) {
        self.current = usize::MAX;
        for (i, (c, _)) in self.conditions.iter().enumerate() {
            if self.current == usize::MAX && c.get() {
                self.current = i;
            }
            ctx.sub_var(c);
        }

        let (children, var_handles, event_handles) = self.children_mut_with_handles();
        ctx.with_handles(var_handles, event_handles, |ctx| children.init_all(ctx));
    }

    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        let (children, var_handles, event_handles) = self.children_mut_with_handles();
        ctx.with_handles(var_handles, event_handles, |ctx| children.deinit_all(ctx));
        var_handles.clear();
        event_handles.clear();
    }

    fn update_all(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        let mut any = false;
        for (i, (c, _)) in self.conditions.iter().enumerate() {
            if i < self.current {
                // if activated < current
                if c.get() {
                    any = true;
                    self.change_children(ctx, observer, i);

                    break;
                }
            } else if i == self.current {
                // if deactivated current
                if c.get() {
                    any = true;
                    break;
                }
            } else if c.get() {
                // if deactivated current and had another active after
                any = true;
                self.change_children(ctx, observer, i);

                break;
            }
        }

        if !any && self.current != usize::MAX {
            // if no longer has not active condition.
            self.change_children(ctx, observer, usize::MAX);
        }

        let (children, var_handles, event_handles) = self.children_mut_with_handles();
        ctx.with_handles(var_handles, event_handles, |ctx| children.update_all(ctx, updates, observer));
    }

    fn event_all(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
        let (children, var_handles, event_handles) = self.children_mut_with_handles();
        ctx.with_handles(var_handles, event_handles, |ctx| {
            children.event_all(ctx, update);
        })
    }
}
