use zng_layout::unit::PxSize;
use zng_var::{BoxedVar, Var};

use crate::{
    update::{EventUpdate, WidgetUpdates},
    widget::{WIDGET, WidgetHandlesCtx},
};

use super::{BoxedUiNode, BoxedUiNodeList, UiNode, UiNodeList, UiNodeListObserver};

/// Builds a node that can be one of multiple options, selected by the first condition that is `true`, or a fallback default.
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
            wgt_handles: WidgetHandlesCtx::new(),
        }
    }
}

/// Builds a node list that can be one of multiple options, selected by the first condition that is `true`, or a fallback default.
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
            wgt_handles: WidgetHandlesCtx::new(),
        }
    }
}

struct WhenUiNode {
    default: BoxedUiNode,
    conditions: Vec<(BoxedVar<bool>, BoxedUiNode)>,
    current: usize,
    wgt_handles: WidgetHandlesCtx,
}
impl WhenUiNode {
    fn child_mut_with_handles(&mut self) -> (&mut BoxedUiNode, &mut WidgetHandlesCtx) {
        let child = if self.current == usize::MAX {
            &mut self.default
        } else {
            &mut self.conditions[self.current].1
        };
        (child, &mut self.wgt_handles)
    }

    fn change_child(&mut self, new: usize) {
        {
            let (child, wgt_handles) = self.child_mut_with_handles();
            WIDGET.with_handles(wgt_handles, || child.deinit());
            wgt_handles.clear();
        }

        self.current = new;

        {
            let (child, wgt_handles) = self.child_mut_with_handles();
            WIDGET.with_handles(wgt_handles, || child.init());
        }

        WIDGET.update_info().layout().render();
    }

    fn with<R>(&mut self, f: impl FnOnce(&mut BoxedUiNode) -> R) -> R {
        let (child, wgt_handles) = self.child_mut_with_handles();
        WIDGET.with_handles(wgt_handles, || f(child))
    }
}
impl UiNode for WhenUiNode {
    fn init(&mut self) {
        self.current = usize::MAX;
        for (i, (c, _)) in self.conditions.iter().enumerate() {
            if self.current == usize::MAX && c.get() {
                self.current = i;
            }
            WIDGET.sub_var(c);
        }
        self.with(|c| c.init());
    }

    fn deinit(&mut self) {
        self.with(|c| c.deinit());
        self.wgt_handles.clear();
    }

    fn info(&mut self, info: &mut crate::widget::info::WidgetInfoBuilder) {
        self.with(|c| c.info(info));
    }

    fn event(&mut self, update: &EventUpdate) {
        self.with(|c| c.event(update));
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        let mut any = false;
        for (i, (c, _)) in self.conditions.iter().enumerate() {
            if i < self.current {
                if c.get() {
                    // if activated < current
                    self.change_child(i);
                    return;
                }
            } else if i == self.current {
                if c.get() {
                    // if did not deactivate current
                    any = true;
                    break;
                }
            } else if c.get() {
                // if deactivated current and had another active after
                self.change_child(i);
                return;
            }
        }

        if !any && self.current != usize::MAX {
            // if no longer has not active condition.
            self.change_child(usize::MAX);
            return;
        }

        // only update if did not change
        // to not update before first info build
        self.with(|c| c.update(updates));
    }

    fn measure(&mut self, wm: &mut crate::widget::info::WidgetMeasure) -> PxSize {
        self.with(|c| c.measure(wm))
    }

    fn layout(&mut self, wl: &mut crate::widget::info::WidgetLayout) -> PxSize {
        self.with(|c| c.layout(wl))
    }

    fn render(&mut self, frame: &mut crate::render::FrameBuilder) {
        self.with(|c| c.render(frame))
    }

    fn render_update(&mut self, update: &mut crate::render::FrameUpdate) {
        self.with(|c| c.render_update(update))
    }
}

struct WhenUiNodeList {
    default: BoxedUiNodeList,
    conditions: Vec<(BoxedVar<bool>, BoxedUiNodeList)>,
    current: usize,
    wgt_handles: WidgetHandlesCtx,
}
impl WhenUiNodeList {
    fn children(&self) -> &BoxedUiNodeList {
        if self.current == usize::MAX {
            &self.default
        } else {
            &self.conditions[self.current].1
        }
    }

    fn children_mut_with_handles(&mut self) -> (&mut BoxedUiNodeList, &mut WidgetHandlesCtx) {
        let child = if self.current == usize::MAX {
            &mut self.default
        } else {
            &mut self.conditions[self.current].1
        };
        (child, &mut self.wgt_handles)
    }

    fn change_children(&mut self, observer: &mut dyn UiNodeListObserver, new: usize) {
        {
            let (child, wgt_handles) = self.children_mut_with_handles();
            WIDGET.with_handles(wgt_handles, || child.deinit_all());
            wgt_handles.clear();
        }

        self.current = new;

        {
            let (child, wgt_handles) = self.children_mut_with_handles();
            WIDGET.with_handles(wgt_handles, || child.init_all());
        }

        observer.reset();
        WIDGET.update_info().layout().render();
    }
}

impl UiNodeList for WhenUiNodeList {
    fn with_node<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        self.children_mut_with_handles().0.with_node(index, f)
    }

    fn for_each<F>(&mut self, f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode),
    {
        self.children_mut_with_handles().0.for_each(f)
    }

    fn par_each<F>(&mut self, f: F)
    where
        F: Fn(usize, &mut BoxedUiNode) + Send + Sync,
    {
        self.children_mut_with_handles().0.par_each(f)
    }

    fn par_fold_reduce<T, I, F, R>(&mut self, identity: I, fold: F, reduce: R) -> T
    where
        T: Send + 'static,
        I: Fn() -> T + Send + Sync,
        F: Fn(T, usize, &mut BoxedUiNode) -> T + Send + Sync,
        R: Fn(T, T) -> T + Send + Sync,
    {
        self.children_mut_with_handles().0.par_fold_reduce(identity, fold, reduce)
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

        let (children, wgt_handles) = self.children_mut_with_handles();
        WIDGET.with_handles(wgt_handles, || children.init_all());
    }

    fn deinit_all(&mut self) {
        let (children, wgt_handles) = self.children_mut_with_handles();
        WIDGET.with_handles(wgt_handles, || children.deinit_all());
        wgt_handles.clear();
    }

    fn update_all(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
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

        let (children, wgt_handles) = self.children_mut_with_handles();
        WIDGET.with_handles(wgt_handles, || children.update_all(updates, observer));
    }

    fn info_all(&mut self, info: &mut crate::widget::info::WidgetInfoBuilder) {
        let (children, wgt_handles) = self.children_mut_with_handles();
        WIDGET.with_handles(wgt_handles, || {
            children.info_all(info);
        })
    }

    fn event_all(&mut self, update: &EventUpdate) {
        let (children, wgt_handles) = self.children_mut_with_handles();
        WIDGET.with_handles(wgt_handles, || {
            children.event_all(update);
        })
    }
}
