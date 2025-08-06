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
}
impl UiNodeImpl for WhenUiNode {
    fn children_len(&self) -> usize {
        todo!("!!: TODO")
    }

    fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
        todo!("!!: TODO")
    }

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
