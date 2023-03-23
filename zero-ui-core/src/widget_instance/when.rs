use crate::{
    context::WidgetUpdates,
    context::WIDGET,
    event::{EventHandles, EventUpdate},
    ui_node,
    var::{BoxedVar, Var, VarHandles},
};

use super::{BoxedUiNode, BoxedUiNodeList, UiNode, UiNodeList, UiNodeListObserver};

/// Builds a node that can be one of multiple options selected by the first condition that is `true` or a fallback default.
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

/// Builds a node list that can be one of multiple options selected by the first condition that is `true` or a fallback default.
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

    fn change_child(&mut self, new: usize) {
        {
            let (child, var_handles, event_handles) = self.child_mut_with_handles();
            WIDGET.with_handles(var_handles, event_handles, || child.deinit());
            var_handles.clear();
            event_handles.clear();
        }

        self.current = new;

        {
            let (child, var_handles, event_handles) = self.child_mut_with_handles();
            WIDGET.with_handles(var_handles, event_handles, || child.init());
        }

        WIDGET.update_info().layout().render();
    }
}
#[ui_node(
    delegate = self.child(),
    delegate_mut = self.child_mut_with_handles().0,
)]
impl UiNode for WhenUiNode {
    fn init(&mut self) {
        self.current = usize::MAX;
        for (i, (c, _)) in self.conditions.iter().enumerate() {
            if self.current == usize::MAX && c.get() {
                self.current = i;
            }
            WIDGET.sub_var(c);
        }

        let (child, var_handles, event_handles) = self.child_mut_with_handles();
        WIDGET.with_handles(var_handles, event_handles, || child.init());
    }

    fn deinit(&mut self) {
        let (child, var_handles, event_handles) = self.child_mut_with_handles();
        WIDGET.with_handles(var_handles, event_handles, || child.deinit());
        var_handles.clear();
        event_handles.clear();
    }

    fn event(&mut self, update: &mut EventUpdate) {
        let (child, var_handles, event_handles) = self.child_mut_with_handles();
        WIDGET.with_handles(var_handles, event_handles, || child.event(update));
    }

    fn update(&mut self, updates: &mut WidgetUpdates) {
        let mut any = false;
        for (i, (c, _)) in self.conditions.iter().enumerate() {
            if i < self.current {
                // if activated < current
                if c.get() {
                    any = true;
                    self.change_child(i);

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
                self.change_child(i);

                break;
            }
        }

        if !any && self.current != usize::MAX {
            // if no longer has not active condition.
            self.change_child(usize::MAX);
        }

        let (child, var_handles, event_handles) = self.child_mut_with_handles();
        WIDGET.with_handles(var_handles, event_handles, || child.update(updates));
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

    fn change_children(&mut self, observer: &mut dyn UiNodeListObserver, new: usize) {
        {
            let (child, var_handles, event_handles) = self.children_mut_with_handles();
            WIDGET.with_handles(var_handles, event_handles, || child.deinit_all());
            var_handles.clear();
            event_handles.clear();
        }

        self.current = new;

        {
            let (child, var_handles, event_handles) = self.children_mut_with_handles();
            WIDGET.with_handles(var_handles, event_handles, || child.init_all());
        }

        observer.reseted();
        WIDGET.update_info().layout().render();
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

    fn par_each<F>(&self, f: F)
    where
        F: Fn(usize, &BoxedUiNode) + Send + Sync,
    {
        self.children().par_each(f)
    }

    fn par_each_mut<F>(&mut self, f: F)
    where
        F: Fn(usize, &mut BoxedUiNode) + Send + Sync,
    {
        self.children_mut_with_handles().0.par_each_mut(f)
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

    fn init_all(&mut self) {
        self.current = usize::MAX;
        for (i, (c, _)) in self.conditions.iter().enumerate() {
            if self.current == usize::MAX && c.get() {
                self.current = i;
            }
            WIDGET.sub_var(c);
        }

        let (children, var_handles, event_handles) = self.children_mut_with_handles();
        WIDGET.with_handles(var_handles, event_handles, || children.init_all());
    }

    fn deinit_all(&mut self) {
        let (children, var_handles, event_handles) = self.children_mut_with_handles();
        WIDGET.with_handles(var_handles, event_handles, || children.deinit_all());
        var_handles.clear();
        event_handles.clear();
    }

    fn update_all(&mut self, updates: &mut WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        let mut any = false;
        for (i, (c, _)) in self.conditions.iter().enumerate() {
            if i < self.current {
                // if activated < current
                if c.get() {
                    any = true;
                    self.change_children(observer, i);

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
                self.change_children(observer, i);

                break;
            }
        }

        if !any && self.current != usize::MAX {
            // if no longer has not active condition.
            self.change_children(observer, usize::MAX);
        }

        let (children, var_handles, event_handles) = self.children_mut_with_handles();
        WIDGET.with_handles(var_handles, event_handles, || children.update_all(updates, observer));
    }

    fn event_all(&mut self, update: &mut EventUpdate) {
        let (children, var_handles, event_handles) = self.children_mut_with_handles();
        WIDGET.with_handles(var_handles, event_handles, || {
            children.event_all(update);
        })
    }
}
