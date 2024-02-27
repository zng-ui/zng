use std::mem;

use zero_ui_layout::unit::PxSize;

use crate::{
    render::{FrameBuilder, FrameUpdate},
    update::{EventUpdate, WidgetUpdates},
    widget::{
        info::{WidgetInfoBuilder, WidgetLayout, WidgetMeasure},
        ui_node, WidgetUpdateMode,
    },
};

use super::*;

/// Represents a node operation in a [`match_node`].
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
impl<'a> UiNodeOp<'a> {
    /// Gets the operation without the associated data.
    pub fn mtd(&self) -> UiNodeOpMethod {
        match self {
            UiNodeOp::Init => UiNodeOpMethod::Init,
            UiNodeOp::Deinit => UiNodeOpMethod::Deinit,
            UiNodeOp::Info { .. } => UiNodeOpMethod::Info,
            UiNodeOp::Event { .. } => UiNodeOpMethod::Event,
            UiNodeOp::Update { .. } => UiNodeOpMethod::Update,
            UiNodeOp::Measure { .. } => UiNodeOpMethod::Measure,
            UiNodeOp::Layout { .. } => UiNodeOpMethod::Layout,
            UiNodeOp::Render { .. } => UiNodeOpMethod::Render,
            UiNodeOp::RenderUpdate { .. } => UiNodeOpMethod::RenderUpdate,
        }
    }

    /// Reborrow the op.
    pub fn reborrow(&mut self) -> UiNodeOp {
        match self {
            UiNodeOp::Init => UiNodeOp::Init,
            UiNodeOp::Deinit => UiNodeOp::Deinit,
            UiNodeOp::Info { info } => UiNodeOp::Info { info },
            UiNodeOp::Event { update } => UiNodeOp::Event { update },
            UiNodeOp::Update { updates } => UiNodeOp::Update { updates },
            UiNodeOp::Measure { wm, desired_size } => UiNodeOp::Measure { wm, desired_size },
            UiNodeOp::Layout { wl, final_size } => UiNodeOp::Layout { wl, final_size },
            UiNodeOp::Render { frame } => UiNodeOp::Render { frame },
            UiNodeOp::RenderUpdate { update } => UiNodeOp::RenderUpdate { update },
        }
    }
}
impl<'a> fmt::Debug for UiNodeOp<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Event { update } => f.debug_struct("Event").field("update", update).finish(),
            Self::Update { updates } => f.debug_struct("Update").field("updates", updates).finish(),
            op => write!(f, "{}", op.mtd()),
        }
    }
}

/// Identifies an [`UiNodeOp`] method without the associated data.
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum UiNodeOpMethod {
    /// The [`UiNodeOp::Init`].
    Init,
    /// The [`UiNodeOp::Deinit`].
    Deinit,
    /// The [`UiNodeOp::Info`].
    Info,
    /// The [`UiNodeOp::Event`].
    Event,
    /// The [`UiNodeOp::Update`].
    Update,
    /// The [`UiNodeOp::Measure`].
    Measure,
    /// The [`UiNodeOp::Layout`].
    Layout,
    /// The [`UiNodeOp::Render`].
    Render,
    /// The [`UiNodeOp::RenderUpdate`].
    RenderUpdate,
}
impl UiNodeOpMethod {
    /// Gets an static string representing the enum variant (CamelCase).
    pub fn enum_name(self) -> &'static str {
        match self {
            UiNodeOpMethod::Init => "Init",
            UiNodeOpMethod::Deinit => "Deinit",
            UiNodeOpMethod::Info => "Info",
            UiNodeOpMethod::Event => "Event",
            UiNodeOpMethod::Update => "Update",
            UiNodeOpMethod::Measure => "Measure",
            UiNodeOpMethod::Layout => "Layout",
            UiNodeOpMethod::Render => "Render",
            UiNodeOpMethod::RenderUpdate => "RenderUpdate",
        }
    }

    /// Gets an static string representing the method name (snake_case).
    pub fn mtd_name(self) -> &'static str {
        match self {
            UiNodeOpMethod::Init => "init",
            UiNodeOpMethod::Deinit => "deinit",
            UiNodeOpMethod::Info => "info",
            UiNodeOpMethod::Event => "event",
            UiNodeOpMethod::Update => "update",
            UiNodeOpMethod::Measure => "measure",
            UiNodeOpMethod::Layout => "layout",
            UiNodeOpMethod::Render => "render",
            UiNodeOpMethod::RenderUpdate => "render_update",
        }
    }
}
impl fmt::Debug for UiNodeOpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl fmt::Display for UiNodeOpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "{}", self.enum_name())
        } else {
            write!(f, "{}", self.mtd_name())
        }
    }
}

/// Creates a node that is implemented as a closure that matches over [`UiNodeOp`] and delegates to another child node.
///
/// The closure node delegates to `child`, when the `closure` itself does not delegate, the `child` methods
/// are called after the closure returns.
///
/// This is a convenient way of declaring *anonymous* nodes, such as those that implement a property function. By leveraging
/// closure capture state storing can be easily declared and used.
///
/// See [`match_node_list`] to create a match node that delegates to multiple children, or [`match_node_leaf`] to create a node
/// that does not delegate.
///
/// # Warning
///
/// The child type is changed to [`BoxedUiNode`] when build with the `dyn_node` feature, if you want to access the child directly
/// using [`MatchNodeChild::child`] you can use [`match_node_typed`] instead, or have the child type always be [`BoxedUiNode`].
///
/// # Widget
///
/// The match node will not delegate [`UiNode::is_widget`] and other widget methods, you can use [`match_widget`] for that.
///
/// # Examples
///
/// ```
/// # fn main() { }
/// # use zero_ui_app::{*, widget::{*, node::*, builder::*}};
/// # use zero_ui_var::*;
/// # use zero_ui_layout::context::LAYOUT;
/// #[property(LAYOUT)]
/// pub fn count_layout(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
///     let enabled = enabled.into_var();
///     let mut layout_count = 0;
///
///     match_node(child, move |child, op| match op {
///         UiNodeOp::Init => {
///             WIDGET.sub_var(&enabled);
///         }
///         UiNodeOp::Update { .. } => {
///             if let Some(true) = enabled.get_new() {
///                 println!("layout count reset");
///                 layout_count = 0;
///             }
///         }
///         UiNodeOp::Measure { wm, desired_size } => {
///             let s = child.measure(wm);
///             *desired_size = LAYOUT.constraints().fill_size_or(s);
///         }
///         UiNodeOp::Layout { wl, final_size } => {
///             if enabled.get() {
///                 layout_count += 1;
///                 println!("layout {layout_count}");
///             }
///             let s = child.layout(wl);
///             *final_size = LAYOUT.constraints().fill_size_or(s);
///         }
///         _ => {}
///     })
/// }
/// ```
///
/// [`match_node_typed`]: fn@match_node_typed
/// [`match_widget`]: fn@match_widget
#[cfg(dyn_node)]
pub fn match_node<C: UiNode>(child: C, closure: impl FnMut(&mut MatchNodeChild<BoxedUiNode>, UiNodeOp) + Send + 'static) -> impl UiNode {
    #[cfg(dyn_closure)]
    let closure: Box<dyn FnMut(&mut MatchNodeChild<BoxedUiNode>, UiNodeOp) + Send> = Box::new(closure);

    match_node_impl(child.boxed(), closure)
}

/// Creates a node that is implemented as a closure that matches over [`UiNodeOp`] and delegates to another child node.
///
/// The closure node delegates to `child`, when the `closure` itself does not delegate, the `child` methods
/// are called after the closure returns.
///
/// This is a convenient way of declaring *anonymous* nodes, such as those that implement a property function. By leveraging
/// closure capture state storing can be easily implemented.
///
/// See [`match_node_list`] to create a match node that delegates to multiple children, or [`match_node_leaf`] to create a node
/// that does not delegate.
///
/// # Warning
///
/// The child type is changed to [`BoxedUiNode`] when build with the `dyn_node` feature, if you want to access the child directly
/// using [`MatchNodeChild::child`] you can use [`match_node_typed`] instead, or have the child type always be [`BoxedUiNode`].
///
/// # Widget
///
/// The match node will not delegate [`UiNode::is_widget`] and other widget methods, you can use [`match_widget`] for that.
///
/// # Examples
///
/// ```
/// # fn main() { }
/// # use zero_ui_app::{*, widget_node::*, widget_builder::*};
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
///             *desired_size = LAYOUT.constraints().fill_size_or(s);
///         }
///         UiNodeOp::Layout { wl, final_size } => {
///             if enabled.get() {
///                 layout_count += 1;
///                 println!("layout {layout_count}");
///             }
///             let s = child.layout(wl);
///             *final_size = LAYOUT.constraints().fill_size_or(s);
///         }
///         _ => {}
///     })
/// }
/// ```
///
/// [`match_node_typed`]: fn@match_node_typed
/// [`match_widget`]: fn@match_widget
#[cfg(not(dyn_node))]
pub fn match_node<C: UiNode>(child: C, closure: impl FnMut(&mut MatchNodeChild<C>, UiNodeOp) + Send + 'static) -> impl UiNode {
    match_node_typed(child, closure)
}

/// Like [`match_node`], but does not change the child type when build with `dyn_node`.
///
/// [`match_node`]: fn@match_node
pub fn match_node_typed<C: UiNode>(child: C, closure: impl FnMut(&mut MatchNodeChild<C>, UiNodeOp) + Send + 'static) -> impl UiNode {
    #[cfg(dyn_closure)]
    let closure: Box<dyn FnMut(&mut MatchNodeChild<C>, UiNodeOp) + Send> = Box::new(closure);

    match_node_impl(child, closure)
}

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
                self.child.child.init();
            }
        }

        fn deinit(&mut self) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Deinit);

            if !mem::take(&mut self.child.delegated) {
                self.child.child.deinit();
            }
        }

        fn info(&mut self, info: &mut WidgetInfoBuilder) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Info { info });

            if !mem::take(&mut self.child.delegated) {
                self.child.child.info(info);
            }
        }

        fn event(&mut self, update: &EventUpdate) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Event { update });

            if !mem::take(&mut self.child.delegated) {
                self.child.child.event(update);
            }
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Update { updates });

            if !mem::take(&mut self.child.delegated) {
                self.child.child.update(updates);
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
                if size != PxSize::zero() {
                    // this is an error because the child will be measured if the return size is zero,
                    // flagging delegated ensure consistent behavior.
                    tracing::error!("measure changed size without flagging delegated");
                    return size;
                }

                self.child.child.measure(wm)
            } else {
                size
            }
        }

        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            self.child.delegated = false;

            let mut size = PxSize::zero();
            (self.closure)(&mut self.child, UiNodeOp::Layout { wl, final_size: &mut size });

            if !mem::take(&mut self.child.delegated) {
                if size != PxSize::zero() {
                    // this is an error because the child will be layout if the return size is zero,
                    // flagging delegated ensure consistent behavior.
                    tracing::error!("layout changed size without flagging delegated");
                    return size;
                }

                self.child.child.layout(wl)
            } else {
                size
            }
        }

        fn render(&mut self, frame: &mut FrameBuilder) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Render { frame });

            if !mem::take(&mut self.child.delegated) {
                self.child.child.render(frame);
            }
        }

        fn render_update(&mut self, update: &mut FrameUpdate) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::RenderUpdate { update });

            if !mem::take(&mut self.child.delegated) {
                self.child.child.render_update(update);
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

    /// If the current operation was already delegated to the child.
    pub fn has_delegated(&self) -> bool {
        self.delegated
    }

    /// Borrow the actual child.
    ///
    /// Note that if you delegate using this reference you must call [`delegated`].
    ///
    /// **Warning:** that [`match_node`] changes the child type to [`BoxedUiNode`] when build with the `dyn_node` feature.
    /// To get a consistent child type use the [`BoxedUiNode`] directly or use [`match_node_typed`].
    ///
    /// [`delegated`]: Self::delegated
    /// [`match_node`]: fn@match_node
    pub fn child(&mut self) -> &mut C {
        &mut self.child
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

    fn is_widget(&self) -> bool {
        self.child.is_widget()
    }

    fn with_context<R, F>(&mut self, update_mode: WidgetUpdateMode, f: F) -> Option<R>
    where
        F: FnOnce() -> R,
    {
        self.child.with_context(update_mode, f)
    }
}

/// Creates a node that is implemented as a closure that matches over [`UiNodeOp`] and does not delegate to any child node.
pub fn match_node_leaf(closure: impl FnMut(UiNodeOp) + Send + 'static) -> impl UiNode {
    #[ui_node(struct MatchNodeLeaf {
        closure: impl FnMut(UiNodeOp) + Send + 'static,
    })]
    impl UiNode for MatchNodeLeaf {
        fn init(&mut self) {
            (self.closure)(UiNodeOp::Init);
        }

        fn deinit(&mut self) {
            (self.closure)(UiNodeOp::Deinit);
        }

        fn info(&mut self, info: &mut WidgetInfoBuilder) {
            (self.closure)(UiNodeOp::Info { info });
        }

        fn event(&mut self, update: &EventUpdate) {
            (self.closure)(UiNodeOp::Event { update });
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            (self.closure)(UiNodeOp::Update { updates });
        }

        fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
            let mut size = PxSize::zero();
            (self.closure)(UiNodeOp::Measure {
                wm,
                desired_size: &mut size,
            });
            size
        }

        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            let mut size = PxSize::zero();
            (self.closure)(UiNodeOp::Layout { wl, final_size: &mut size });
            size
        }

        fn render(&mut self, frame: &mut FrameBuilder) {
            (self.closure)(UiNodeOp::Render { frame });
        }

        fn render_update(&mut self, update: &mut FrameUpdate) {
            (self.closure)(UiNodeOp::RenderUpdate { update });
        }
    }
    MatchNodeLeaf { closure }
}

/// Creates a widget that is implemented as a closure that matches over [`UiNodeOp`] and delegates to another child widget.
///
/// The returned node will delegate to `child` like [`match_node`] does, and will also delegate [`UiNode::is_widget`] and
/// [`UiNode::with_context`]. Note that the `closure` itself will not run inside [`UiNode::with_context`].
///
/// Note that unlike the [`match_node`] the `W` type is always preserved, the feature `dyn_node` is ignored here.
///
/// [`match_node`]: fn@match_node
pub fn match_widget<W: UiNode>(child: W, closure: impl FnMut(&mut MatchWidgetChild<W>, UiNodeOp) + Send + 'static) -> impl UiNode {
    #[ui_node(struct MatchWidget<C: UiNode> {
        child: MatchWidgetChild<C>,
        closure: impl FnMut(&mut MatchWidgetChild<C>, UiNodeOp) + Send + 'static,
    })]
    impl UiNode for MatchWidget {
        fn is_widget(&self) -> bool {
            self.child.0.child.is_widget()
        }

        fn with_context<R, F: FnOnce() -> R>(&mut self, update_mode: WidgetUpdateMode, f: F) -> Option<R> {
            self.child.0.child.with_context(update_mode, f)
        }

        fn init(&mut self) {
            self.child.0.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Init);

            if !mem::take(&mut self.child.0.delegated) {
                self.child.0.child.init();
            }
        }

        fn deinit(&mut self) {
            self.child.0.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Deinit);

            if !mem::take(&mut self.child.0.delegated) {
                self.child.0.child.deinit();
            }
        }

        fn info(&mut self, info: &mut WidgetInfoBuilder) {
            self.child.0.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Info { info });

            if !mem::take(&mut self.child.0.delegated) {
                self.child.0.child.info(info);
            }
        }

        fn event(&mut self, update: &EventUpdate) {
            self.child.0.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Event { update });

            if !mem::take(&mut self.child.0.delegated) {
                self.child.0.child.event(update);
            }
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            self.child.0.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Update { updates });

            if !mem::take(&mut self.child.0.delegated) {
                self.child.0.child.update(updates);
            }
        }

        fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
            self.child.0.delegated = false;

            let mut size = PxSize::zero();
            (self.closure)(
                &mut self.child,
                UiNodeOp::Measure {
                    wm,
                    desired_size: &mut size,
                },
            );

            if !mem::take(&mut self.child.0.delegated) {
                if size != PxSize::zero() {
                    // this is an error because the child will be measured if the return size is zero,
                    // flagging delegated ensure consistent behavior.
                    tracing::error!("measure changed size without flagging delegated");
                    return size;
                }

                self.child.0.child.measure(wm)
            } else {
                size
            }
        }

        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            self.child.0.delegated = false;

            let mut size = PxSize::zero();
            (self.closure)(&mut self.child, UiNodeOp::Layout { wl, final_size: &mut size });

            if !mem::take(&mut self.child.0.delegated) {
                if size != PxSize::zero() {
                    // this is an error because the child will be layout if the return size is zero,
                    // flagging delegated ensure consistent behavior.
                    tracing::error!("layout changed size without flagging delegated");
                    return size;
                }

                self.child.0.child.layout(wl)
            } else {
                size
            }
        }

        fn render(&mut self, frame: &mut FrameBuilder) {
            self.child.0.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Render { frame });

            if !mem::take(&mut self.child.0.delegated) {
                self.child.0.child.render(frame);
            }
        }

        fn render_update(&mut self, update: &mut FrameUpdate) {
            self.child.0.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::RenderUpdate { update });

            if !mem::take(&mut self.child.0.delegated) {
                self.child.0.child.render_update(update);
            }
        }
    }
    MatchWidget {
        child: MatchWidgetChild(MatchNodeChild { child, delegated: false }),
        closure,
    }
}

/// Child node of [`match_widget`].
///
/// This node delegates like [`MatchNodeChild<C>`] plus delegates [`UiNode::is_widget`] and [`UiNode::with_context`].
///
/// [`match_widget`]: fn@match_widget
pub struct MatchWidgetChild<C>(MatchNodeChild<C>);
impl<C> MatchWidgetChild<C> {
    /// Flags the current operation as *delegated*, stopping the default delegation after the closure ends.
    ///
    /// Note that each node operation methods already flags this.
    pub fn delegated(&mut self) {
        self.0.delegated = true;
    }

    /// If the current operation was already delegated to the child.
    pub fn has_delegated(&self) -> bool {
        self.0.delegated
    }

    /// Borrow the actual child.
    ///
    /// Note that if you delegate using this reference you must call [`delegated`].
    ///
    /// [`delegated`]: Self::delegated
    /// [`match_node`]: fn@match_node
    pub fn child(&mut self) -> &mut C {
        &mut self.0.child
    }

    /// Adapter to `match_node` child type.
    ///
    /// Note that the returned node does not delegate widget methods.
    pub fn as_match_node(&mut self) -> &mut MatchNodeChild<C> {
        &mut self.0
    }
}
#[ui_node(delegate = &mut self.0)]
impl<C: UiNode> UiNode for MatchWidgetChild<C> {
    fn is_widget(&self) -> bool {
        self.0.child.is_widget()
    }

    fn with_context<R, F: FnOnce() -> R>(&mut self, update_mode: WidgetUpdateMode, f: F) -> Option<R> {
        self.0.child.with_context(update_mode, f)
    }
}

/// Creates a node that is implemented as a closure that matches over [`UiNodeOp`] and delegates to multiple children nodes in a list.
///
/// The closure node delegates to `children`, when the `closure` itself does not delegate, the `children` methods
/// are called after the closure returns.
pub fn match_node_list<L: UiNodeList>(
    children: L,
    closure: impl FnMut(&mut MatchNodeChildren<L>, UiNodeOp) + Send + 'static,
) -> impl UiNode {
    #[ui_node(struct MatchNodeList<C: UiNodeList> {
        children: MatchNodeChildren<C>,
        closure: impl FnMut(&mut MatchNodeChildren<C>, UiNodeOp) + Send + 'static,
    })]
    #[allow_(zero_ui::missing_delegate)] // false positive
    impl UiNode for MatchNodeList {
        fn init(&mut self) {
            self.children.delegated = false;

            (self.closure)(&mut self.children, UiNodeOp::Init);

            if !mem::take(&mut self.children.delegated) {
                ui_node_list_default::init_all(&mut self.children.children);
            }
        }

        fn deinit(&mut self) {
            self.children.delegated = false;

            (self.closure)(&mut self.children, UiNodeOp::Deinit);

            if !mem::take(&mut self.children.delegated) {
                ui_node_list_default::deinit_all(&mut self.children.children);
            }
        }

        fn info(&mut self, info: &mut WidgetInfoBuilder) {
            self.children.delegated = false;

            (self.closure)(&mut self.children, UiNodeOp::Info { info });

            if !mem::take(&mut self.children.delegated) {
                ui_node_list_default::info_all(&mut self.children.children, info)
            }
        }

        fn event(&mut self, update: &EventUpdate) {
            self.children.delegated = false;

            (self.closure)(&mut self.children, UiNodeOp::Event { update });

            if !mem::take(&mut self.children.delegated) {
                ui_node_list_default::event_all(&mut self.children.children, update);
            }
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            self.children.delegated = false;

            (self.closure)(&mut self.children, UiNodeOp::Update { updates });

            if !mem::take(&mut self.children.delegated) {
                ui_node_list_default::update_all(&mut self.children.children, updates);
            }
        }

        fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
            self.children.delegated = false;

            let mut size = PxSize::zero();
            (self.closure)(
                &mut self.children,
                UiNodeOp::Measure {
                    wm,
                    desired_size: &mut size,
                },
            );

            if !mem::take(&mut self.children.delegated) {
                if size != PxSize::zero() {
                    // this is an error because the children will be measured if the return size is zero,
                    // flagging delegated ensure consistent behavior.
                    tracing::error!("measure(list) changed size without flagging delegated");
                    return size;
                }

                ui_node_list_default::measure_all(&mut self.children.children, wm)
            } else {
                size
            }
        }

        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            self.children.delegated = false;

            let mut size = PxSize::zero();
            (self.closure)(&mut self.children, UiNodeOp::Layout { wl, final_size: &mut size });

            if !mem::take(&mut self.children.delegated) {
                if size != PxSize::zero() {
                    // this is an error because the children will be layout if the return size is zero,
                    // flagging delegated ensure consistent behavior.
                    tracing::error!("layout(list) changed size without flagging delegated");
                    return size;
                }
                ui_node_list_default::layout_all(&mut self.children.children, wl)
            } else {
                size
            }
        }

        fn render(&mut self, frame: &mut FrameBuilder) {
            self.children.delegated = false;

            (self.closure)(&mut self.children, UiNodeOp::Render { frame });

            if !mem::take(&mut self.children.delegated) {
                ui_node_list_default::render_all(&mut self.children.children, frame);
            }
        }

        fn render_update(&mut self, update: &mut FrameUpdate) {
            self.children.delegated = false;

            (self.closure)(&mut self.children, UiNodeOp::RenderUpdate { update });

            if !mem::take(&mut self.children.delegated) {
                ui_node_list_default::render_update_all(&mut self.children.children, update);
            }
        }
    }
    MatchNodeList {
        children: MatchNodeChildren {
            children,
            delegated: false,
        },
        closure,
    }
}

/// Children node of [`match_node_list`].
///
/// When the closure does not delegate to this list the delegation happens after the closure returns. The
/// [`UiNodeList`] methods that flag as [`delegated`] are all the `*_all` methods and the methods that access mutable
/// references to each child and the [`UiNodeList::with_node`]. You can use the [`children`] accessor to visit
/// children without flagging as delegated.
///
/// [`match_node`]: fn@match_node
/// [`delegated`]: Self::delegated
/// [`children`]: Self::children
pub struct MatchNodeChildren<L> {
    children: L,
    delegated: bool,
}
impl<L: UiNodeList> MatchNodeChildren<L> {
    /// Flags the current operation as *delegated*, stopping the default delegation after the closure ends.
    ///
    /// Note that each `*_all` method and the methods that give mutable access to children already flags this.
    pub fn delegated(&mut self) {
        self.delegated = true;
    }

    /// If the current operation was already delegated to the children.
    pub fn has_delegated(&self) -> bool {
        self.delegated
    }

    /// Reference the children.
    ///
    /// Note that if you delegate using this reference you must call [`delegated`].
    ///
    /// [`delegated`]: Self::delegated
    pub fn children(&mut self) -> &mut L {
        &mut self.children
    }
}
impl<L: UiNodeList> UiNodeList for MatchNodeChildren<L> {
    fn with_node<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        self.delegated = true;
        self.children.with_node(index, f)
    }

    fn for_each<F>(&mut self, f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode),
    {
        self.delegated = true;
        self.children.for_each(f)
    }

    fn par_each<F>(&mut self, f: F)
    where
        F: Fn(usize, &mut BoxedUiNode) + Send + Sync,
    {
        self.delegated = true;
        self.children.par_each(f)
    }

    fn par_fold_reduce<T, I, F, R>(&mut self, identity: I, fold: F, reduce: R) -> T
    where
        T: Send + 'static,
        I: Fn() -> T + Send + Sync,
        F: Fn(T, usize, &mut BoxedUiNode) -> T + Send + Sync,
        R: Fn(T, T) -> T + Send + Sync,
    {
        self.delegated = true;
        self.children.par_fold_reduce(identity, fold, reduce)
    }

    fn len(&self) -> usize {
        self.children.len()
    }

    fn boxed(self) -> BoxedUiNodeList {
        Box::new(self)
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        self.children.drain_into(vec)
    }

    fn init_all(&mut self) {
        self.children.init_all();
        self.delegated = true;
    }

    fn deinit_all(&mut self) {
        self.children.deinit_all();
        self.delegated = true;
    }

    fn update_all(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        self.children.update_all(updates, observer);
        self.delegated = true;
    }

    fn info_all(&mut self, info: &mut WidgetInfoBuilder) {
        self.children.info_all(info);
        self.delegated = true;
    }

    fn event_all(&mut self, update: &EventUpdate) {
        self.children.event_all(update);
        self.delegated = true;
    }

    fn measure_each<F, S>(&mut self, wm: &mut WidgetMeasure, measure: F, fold_size: S) -> PxSize
    where
        F: Fn(usize, &mut BoxedUiNode, &mut WidgetMeasure) -> PxSize + Send + Sync,
        S: Fn(PxSize, PxSize) -> PxSize + Send + Sync,
    {
        self.delegated = true;
        self.children.measure_each(wm, measure, fold_size)
    }

    fn layout_each<F, S>(&mut self, wl: &mut WidgetLayout, layout: F, fold_size: S) -> PxSize
    where
        F: Fn(usize, &mut BoxedUiNode, &mut WidgetLayout) -> PxSize + Send + Sync,
        S: Fn(PxSize, PxSize) -> PxSize + Send + Sync,
    {
        self.delegated = true;
        self.children.layout_each(wl, layout, fold_size)
    }

    fn render_all(&mut self, frame: &mut FrameBuilder) {
        self.children.render_all(frame);
        self.delegated = true;
    }

    fn render_update_all(&mut self, update: &mut FrameUpdate) {
        self.children.render_update_all(update);
        self.delegated = true;
    }
}
