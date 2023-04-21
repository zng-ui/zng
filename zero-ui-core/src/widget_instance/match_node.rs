use std::mem;

use super::*;

/// Represents a node operation in [`match_node`].
///
/// [`match_node`]: fn@match_node
#[non_exhaustive]
pub enum UiNodeOp<'a> {
    /// The [`UiNode::init`].
    Init,
    /// The [`UiNode::deinit`].
    Deinit,
    /// The [`UiNode::info`].
    Info {
        ///
        info: &'a mut WidgetInfoBuilder,
    },
    /// The [`UiNode::event`].
    Event {
        ///
        update: &'a EventUpdate,
    },
    /// The [`UiNode::update`].
    Update {
        ///
        updates: &'a WidgetUpdates,
    },
    /// The [`UiNode::measure`].
    Measure {
        ///
        wm: &'a mut WidgetMeasure,
        /// The measure return value.
        desired_size: &'a mut PxSize,
    },
    /// The [`UiNode::layout`].
    Layout {
        ///
        wl: &'a mut WidgetLayout,
        /// The layout return value.
        final_size: &'a mut PxSize,
    },
    /// The [`UiNode::render`].
    Render {
        ///
        frame: &'a mut FrameBuilder,
    },
    /// The [`UiNode::render_update`].
    RenderUpdate {
        ///
        update: &'a mut FrameUpdate,
    },
}

/// Creates a node that is implemented as a closure that matches over [`UiNodeOp`].
///
/// The closure node delegates to `child`, when the `closure` itself does not delegate, the `child` method
/// is called after the closure returns.
///
/// This is a convenient way of declaring *anonymous* nodes, such as those that implement a property function. By leveraging
/// closure capture state storing can be easily declared and used.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::{*, widget_instance::*};
/// #[property(LAYOUT)]
/// pub fn count_layout(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
///     let enabled = enabled.into_var();
///     let mut layout_count = 0;
///
///     match_node(child, move |child, op| match op {
///         UiNode::Init => {
///             WIDGET.sub_var(&enabled);
///         }
///         UiNode::Update => {
///             if let Some(true) = enabled.get_new() {
///                 println!("layout count reset");
///                 layout_count = 0;
///             }
///         }
///         UiNodeOp::Measure { wm, desired_size } => {
///             let s = child.measure(wm);
///             *desired_size = LAYOUT.constrains().fill_size_or(s);
///         }
///         UiNodeOp::Layout { wl, final_size } => {
///             if enabled.get() {
///                 layout_count += 1;
///                 println!("layout {layout_count}");
///             }
///             let s = child.layout(wl);
///             *final_size = LAYOUT.constrains().fill_size_or(s);
///         }
///         _ => {}
///     })
/// }
/// ```
#[cfg(dyn_node)]
pub fn match_node<C: UiNode>(child: C, closure: impl FnMut(&mut MatchNodeChild<BoxedUiNode>, UiNodeOp) + Send + 'static) -> impl UiNode {
    #[cfg(dyn_closure)]
    let closure: Box<dyn FnMut(&mut MatchNodeChild<BoxedUiNode>, UiNodeOp) + Send> = Box::new(closure);

    match_node_impl(child.boxed(), closure)
}

/// Creates a node that is implemented as a closure that matches over [`UiNodeOp`].
///
/// The closure node delegates to `child`, when the `closure` itself does not delegate, the `child` method
/// is called after the closure returns.
///
/// This is a convenient way of declaring *anonymous* nodes, such as those that implement a property function. By leveraging
/// closure capture state storing can be easily implemented.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::{*, widget_instance::*};
/// #[property(LAYOUT)]
/// pub fn count_layout(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
///     let enabled = enabled.into_var();
///     let mut layout_count = 0;
///
///     match_node(child, move |child, op| match op {
///         UiNode::Init => {
///             WIDGET.sub_var(&enabled);
///         }
///         UiNode::Update => {
///             if let Some(true) = enabled.get_new() {
///                 println!("layout count reset");
///                 layout_count = 0;
///             }
///         }
///         UiNodeOp::Measure { wm, desired_size } => {
///             let s = child.measure(wm);
///             *desired_size = LAYOUT.constrains().fill_size_or(s);
///         }
///         UiNodeOp::Layout { wl, final_size } => {
///             if enabled.get() {
///                 layout_count += 1;
///                 println!("layout {layout_count}");
///             }
///             let s = child.layout(wl);
///             *final_size = LAYOUT.constrains().fill_size_or(s);
///         }
///         _ => {}
///     })
/// }
/// ```
#[cfg(not(dyn_node))]
pub fn match_node<C: UiNode>(child: C, closure: impl FnMut(&mut MatchNodeChild<C>, UiNodeOp) + Send + 'static) -> impl UiNode {
    #[cfg(dyn_closure)]
    let closure: Box<dyn FnMut(&mut MatchNodeChild<C>, UiNodeOp) + Send> = Box::new(closure);

    match_node_impl(child, closure)
}

#[inline(always)]
fn match_node_impl<C: UiNode>(child: C, closure: impl FnMut(&mut MatchNodeChild<C>, UiNodeOp) + Send + 'static) -> impl UiNode {
    #[ui_node(struct MatchNode<C: UiNode> {
        child: MatchNodeChild<C>,
        closure: impl FnMut(&mut MatchNodeChild<C>, UiNodeOp) + Send + 'static,
    })]
    impl UiNode for MatchNode {
        fn init(&mut self) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Init);

            if !mem::take(&mut self.child.delegated) {
                self.child.init();
            }
        }

        fn deinit(&mut self) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Deinit);

            if !mem::take(&mut self.child.delegated) {
                self.child.deinit();
            }
        }

        fn info(&mut self, info: &mut WidgetInfoBuilder) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Info { info });

            if !mem::take(&mut self.child.delegated) {
                self.child.info(info);
            }
        }

        fn event(&mut self, update: &EventUpdate) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Event { update });

            if !mem::take(&mut self.child.delegated) {
                self.child.event(update);
            }
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Update { updates });

            if !mem::take(&mut self.child.delegated) {
                self.child.update(updates);
            }
        }

        fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
            self.child.delegated = false;

            let mut size = PxSize::zero();
            (self.closure)(
                &mut self.child,
                UiNodeOp::Measure {
                    wm,
                    desired_size: &mut size,
                },
            );

            if !mem::take(&mut self.child.delegated) {
                self.child.measure(wm)
            } else {
                size
            }
        }

        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            self.child.delegated = false;

            let mut size = PxSize::zero();
            (self.closure)(&mut self.child, UiNodeOp::Layout { wl, final_size: &mut size });

            if !mem::take(&mut self.child.delegated) {
                self.child.layout(wl)
            } else {
                size
            }
        }

        fn render(&mut self, frame: &mut FrameBuilder) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Render { frame });

            if !mem::take(&mut self.child.delegated) {
                self.child.render(frame);
            }
        }

        fn render_update(&mut self, update: &mut FrameUpdate) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::RenderUpdate { update });

            if !mem::take(&mut self.child.delegated) {
                self.child.render_update(update);
            }
        }
    }
    MatchNode {
        child: MatchNodeChild { child, delegated: false },
        closure,
    }
}

/// Child node of [`match_node`].
///
/// When the closure does not delegate to this node the delegation happens after the closure returns.
///
/// [`match_node`]: fn@match_node
pub struct MatchNodeChild<C> {
    child: C,
    delegated: bool,
}
impl<C: UiNode> MatchNodeChild<C> {
    /// Flags the current operation as *delegated*, stopping the default delegation after the closure ends.
    ///
    /// Note that each node operation methods already flags this.
    pub fn delegated(&mut self) {
        self.delegated = true;
    }

    /// Calls the [`UiNodeOp`].
    pub fn op(&mut self, op: UiNodeOp) {
        match op {
            UiNodeOp::Init => self.init(),
            UiNodeOp::Deinit => self.deinit(),
            UiNodeOp::Info { info } => self.info(info),
            UiNodeOp::Event { update } => self.event(update),
            UiNodeOp::Update { updates } => self.update(updates),
            UiNodeOp::Measure { wm, desired_size } => *desired_size = self.measure(wm),
            UiNodeOp::Layout { wl, final_size } => *final_size = self.layout(wl),
            UiNodeOp::Render { frame } => self.render(frame),
            UiNodeOp::RenderUpdate { update } => self.render_update(update),
        }
    }
}
impl<C: UiNode> UiNode for MatchNodeChild<C> {
    fn init(&mut self) {
        self.child.init();
        self.delegated = true;
    }

    fn deinit(&mut self) {
        self.child.deinit();
        self.delegated = true;
    }

    fn info(&mut self, info: &mut WidgetInfoBuilder) {
        self.child.info(info);
        self.delegated = true;
    }

    fn event(&mut self, update: &EventUpdate) {
        self.child.event(update);
        self.delegated = true;
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        self.child.update(updates);
        self.delegated = true;
    }

    fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
        self.delegated = true;
        self.child.measure(wm)
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        self.delegated = true;
        self.child.layout(wl)
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        self.child.render(frame);
        self.delegated = true;
    }

    fn render_update(&mut self, update: &mut FrameUpdate) {
        self.child.render_update(update);
        self.delegated = true;
    }
}
