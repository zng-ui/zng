use zng_layout::unit::PxSize;
use zng_var::Var;

use crate::{
    update::{EventUpdate, WidgetUpdates},
    widget::{
        WIDGET, WidgetHandlesCtx,
        node::{IntoUiNode, UiNodeImpl},
    },
};

use super::{UiNode, UiNodeListObserver};

/// Builds a node that can be one of multiple options, selected by the first condition that is `true`, or a fallback default.
///
/// When the selected node changes the previous one is deinited and the new one is inited.
///
/// The when node delegates everything, children, list and widget to the selected node, it *becomes* the selected node.
pub struct WhenUiNodeBuilder {
    default: UiNode,
    conditions: Vec<(Var<bool>, UiNode)>,
}
impl WhenUiNodeBuilder {
    /// New with node that is used when no condition is active.
    pub fn new(default: impl IntoUiNode) -> Self {
        Self {
            default: default.into_node(),
            conditions: vec![],
        }
    }

    /// Push a conditional node.
    ///
    /// When `condition` is `true` and no previous inserted condition is `true` the `node` is used.
    pub fn push(&mut self, condition: Var<bool>, node: impl IntoUiNode) {
        self.conditions.push((condition, node.into_node()));
    }

    /// Build a node that is always the first `true` condition or the default.
    pub fn build(self) -> UiNode {
        UiNode::new(WhenUiNode {
            default: self.default,
            conditions: self.conditions,
            current: usize::MAX,
            wgt_handles: WidgetHandlesCtx::new(),
        })
    }
}

struct WhenUiNode {
    default: UiNode,
    conditions: Vec<(Var<bool>, UiNode)>,
    current: usize,
    wgt_handles: WidgetHandlesCtx,
}
impl WhenUiNode {
    fn child_mut_with_handles(&mut self) -> (&mut UiNode, &mut WidgetHandlesCtx) {
        let child = if self.current == usize::MAX {
            &mut self.default
        } else {
            &mut self.conditions[self.current].1
        };
        (child, &mut self.wgt_handles)
    }

    fn child_ref(&self) -> &UiNode {
        if self.current == usize::MAX {
            &self.default
        } else {
            &self.conditions[self.current].1
        }
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

    fn with<R>(&mut self, f: impl FnOnce(&mut UiNode) -> R) -> R {
        let (child, wgt_handles) = self.child_mut_with_handles();
        WIDGET.with_handles(wgt_handles, || f(child))
    }

    fn update_when(&mut self) -> bool {
        let mut any = false;
        for (i, (c, _)) in self.conditions.iter().enumerate() {
            if i < self.current {
                if c.get() {
                    // if activated < current
                    self.change_child(i);
                    return true;
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
                return true;
            }
        }

        if !any && self.current != usize::MAX {
            // if no longer has not active condition.
            self.change_child(usize::MAX);
            return true;
        }
        false
    }
}
impl UiNodeImpl for WhenUiNode {
    fn children_len(&self) -> usize {
        self.child_ref().0.children_len()
    }

    fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
        self.child_mut_with_handles().0.0.with_child(index, visitor);
    }

    fn init(&mut self) {
        self.current = usize::MAX;
        for (i, (c, _)) in self.conditions.iter().enumerate() {
            if self.current == usize::MAX && c.get() {
                self.current = i;
            }
            WIDGET.sub_var(c);
        }
        self.with(|c| c.0.init());
    }

    fn deinit(&mut self) {
        self.with(|c| c.0.deinit());
        self.wgt_handles.clear();
    }

    fn info(&mut self, info: &mut crate::widget::info::WidgetInfoBuilder) {
        self.with(|c| c.0.info(info));
    }

    fn event(&mut self, update: &EventUpdate) {
        self.with(|c| c.0.event(update));
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        if !self.update_when() {
            // only update if did not change
            // to not update before first info build
            self.with(|c| c.0.update(updates));
        }
    }
    fn update_list(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        if self.update_when() {
            observer.reset();
        } else {
            self.with(|c| c.0.update_list(updates, observer));
        }
    }

    fn measure(&mut self, wm: &mut crate::widget::info::WidgetMeasure) -> PxSize {
        self.with(|c| c.0.measure(wm))
    }

    fn layout(&mut self, wl: &mut crate::widget::info::WidgetLayout) -> PxSize {
        self.with(|c| c.0.layout(wl))
    }

    fn render(&mut self, frame: &mut crate::render::FrameBuilder) {
        self.with(|c| c.0.render(frame))
    }

    fn render_update(&mut self, update: &mut crate::render::FrameUpdate) {
        self.with(|c| c.0.render_update(update))
    }

    fn is_list(&self) -> bool {
        self.child_ref().0.is_list()
    }

    fn for_each_child(&mut self, visitor: &mut dyn FnMut(usize, &mut UiNode)) {
        self.child_mut_with_handles().0.0.for_each_child(visitor);
    }

    fn par_each_child(&mut self, visitor: &(dyn Fn(usize, &mut UiNode) + Sync)) {
        self.child_mut_with_handles().0.0.par_each_child(visitor);
    }

    fn par_fold_reduce(
        &mut self,
        identity: zng_var::BoxAnyVarValue,
        fold: &(dyn Fn(zng_var::BoxAnyVarValue, usize, &mut UiNode) -> zng_var::BoxAnyVarValue + Sync),
        reduce: &(dyn Fn(zng_var::BoxAnyVarValue, zng_var::BoxAnyVarValue) -> zng_var::BoxAnyVarValue + Sync),
    ) -> zng_var::BoxAnyVarValue {
        self.child_mut_with_handles().0.0.par_fold_reduce(identity, fold, reduce)
    }

    fn measure_list(
        &mut self,
        wm: &mut crate::widget::info::WidgetMeasure,
        measure: &(dyn Fn(usize, &mut UiNode, &mut crate::widget::info::WidgetMeasure) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        self.with(|c| c.0.measure_list(wm, measure, fold_size))
    }

    fn layout_list(
        &mut self,
        wl: &mut crate::widget::info::WidgetLayout,
        layout: &(dyn Fn(usize, &mut UiNode, &mut crate::widget::info::WidgetLayout) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        self.with(|c| c.0.layout_list(wl, layout, fold_size))
    }

    fn render_list(
        &mut self,
        frame: &mut crate::render::FrameBuilder,
        render: &(dyn Fn(usize, &mut UiNode, &mut crate::render::FrameBuilder) + Sync),
    ) {
        self.with(|c| c.0.render_list(frame, render))
    }

    fn render_update_list(
        &mut self,
        update: &mut crate::render::FrameUpdate,
        render_update: &(dyn Fn(usize, &mut UiNode, &mut crate::render::FrameUpdate) + Sync),
    ) {
        self.with(|c| c.0.render_update_list(update, render_update))
    }

    fn as_widget(&mut self) -> Option<&mut dyn super::WidgetUiNodeImpl> {
        self.child_mut_with_handles().0.0.as_widget()
    }
}
