use std::{fmt, mem, ops};

use zng_layout::unit::PxSize;

use crate::{
    render::{FrameBuilder, FrameUpdate},
    update::{EventUpdate, WidgetUpdates},
    widget::{
        WidgetUpdateMode,
        info::{WidgetInfoBuilder, WidgetLayout, WidgetMeasure},
    },
};

use super::*;

/// Represents a node operation in a [`match_node`].
///
/// [`match_node`]: fn@match_node
#[non_exhaustive]
pub enum UiNodeOp<'a> {
    /// The [`UiNode::init`].
    ///
    /// Initialize the node in a new UI context.
    ///
    /// Common init operations are subscribing to variables and events and initializing data.
    /// You can use [`WIDGET`] to subscribe events and vars, the subscriptions live until the widget is deinited.
    ///
    /// This operation can be called again after a [`Deinit`].
    ///
    /// [`Deinit`]: Self::Deinit
    Init,
    /// The [`UiNode::deinit`].
    ///
    /// Deinitialize the node in the current UI context.
    ///
    /// Common deinit operations include dropping allocations and handlers.
    ///
    /// [`Init`] can be called again after this.
    ///
    /// [`Init`]: Self::Init
    Deinit,
    /// The [`UiNode::info`].
    ///
    /// Build widget info.
    ///
    /// This operation is called every time there are structural changes in the UI tree such as a node added or removed, you
    /// can also request an info rebuild using [`WIDGET.update_info`].
    ///
    /// Only nodes in widgets that requested info rebuild and nodes in their ancestors receive this call. Other
    /// widgets reuse their info in the new info tree. The widget's latest built info is available in [`WIDGET.info`].
    ///
    /// Note that info rebuild has higher priority over event, update, layout and render, this means that if you set a variable
    /// and request info update the next info rebuild will still observe the old variable value, you can work around this issue by
    /// only requesting info rebuild after the variable updates.
    ///
    /// [`WIDGET.update_info`]: crate::widget::WIDGET::update_info
    /// [`WIDGET.info`]: crate::widget::WIDGET::info
    Info {
        /// Info builder.
        info: &'a mut WidgetInfoBuilder,
    },
    /// The [`UiNode::event`].
    ///
    /// Receive an event.
    ///
    /// Every call to this operation is for a single update of a single event type, you can listen to events
    /// by subscribing to then on [`Init`] and using the [`Event::on`] method during this operation to detect the event.
    ///
    /// Note that events sent to descendant nodes also flow through the match node and are automatically delegated if
    /// you don't manually delegate. Automatic delegation happens after the operation is handled, you can call
    /// `child.event` to manually delegate before handling.
    ///
    /// When an ancestor handles the event before the descendants this is a ***preview*** handling, so match nodes handle
    /// event operations in preview by default.
    ///
    /// [`Init`]: Self::Init
    /// [`Event::on`]: crate::event::Event::on
    Event {
        /// Event update args and targets.
        update: &'a EventUpdate,
    },
    /// The [`UiNode::update`].
    ///
    /// Receive variable and other non-event updates.
    ///
    /// Calls to this operation aggregate all updates that happen in the last pass, multiple variables can be new at the same time.
    /// You can listen to variable updates by subscribing to then on [`Init`] and using the [`Var::get_new`] method during this operation
    /// to receive the new values.
    ///
    /// Common update operations include reacting to variable changes to generate an intermediary value
    /// for layout or render. You can use [`WIDGET`] to request layout and render. Note that for simple variables
    /// that are used directly on layout or render you can subscribe to that operation directly, skipping update.
    ///
    /// [`Init`]: Self::Init
    /// [`Var::get_new`]: zng_var::Var::get_new
    Update {
        /// Update targets
        updates: &'a WidgetUpdates,
    },
    /// The [`UiNode::measure`].
    ///
    /// Compute the widget size given the contextual layout metrics without actually updating the widget layout.
    ///
    /// Implementers must set `desired_size` to the same size [`Layout`] sets for the given [`LayoutMetrics`], without
    /// affecting the actual widget render. Panel widgets that implement some complex layouts need to get
    /// the estimated widget size for a given layout context, this value is used to inform the actual [`Layout`] call.
    ///
    /// Nodes that implement [`Layout`] must also implement this operation, the [`LAYOUT`] context can be used to retrieve the metrics,
    /// the [`WidgetMeasure`] field can be used to communicate with the parent layout, such as disabling inline layout, the
    /// [`PxSize`] field must be set to the desired size given the layout context.
    ///
    /// [`Layout`]: Self::Layout
    /// [`LayoutMetrics`]: zng_layout::context::LayoutMetrics
    /// [`LAYOUT`]: zng_layout::context::LAYOUT
    /// [`PxSize`]: zng_layout::unit::PxSize
    Measure {
        /// Measure pass state.
        wm: &'a mut WidgetMeasure,
        /// Return value, the widget's desired size after measure.
        desired_size: &'a mut PxSize,
    },
    /// The [`UiNode::layout`].
    ///
    /// Compute the widget layout given the contextual layout metrics.
    ///
    /// Implementers must also implement [`Measure`]. This operation is called by the parent layout once the final constraints
    /// for the frame are defined, the [`LAYOUT`] context can be used to retrieve the constraints, the [`WidgetLayout`] field
    /// can be used to communicate layout metadata such as inline segments to the parent layout, the [`PxSize`] field must be
    /// set to the final size given the layout context.
    ///
    /// Only widgets and ancestors that requested layout or use metrics that changed since last layout receive this call. Other
    /// widgets reuse the last layout result.
    ///
    /// Nodes that render can also implement this operation just to observe the latest widget size, if changes are detected
    /// the [`WIDGET.render`] method can be used to request render.
    ///
    /// [`Measure`]: Self::Measure
    /// [`LayoutMetrics`]: zng_layout::context::LayoutMetrics
    /// [`constraints`]: zng_layout::context::LayoutMetrics::constraints
    /// [`WIDGET.render`]: crate::widget::WIDGET::render
    /// [`LAYOUT`]: zng_layout::context::LAYOUT
    /// [`PxSize`]: zng_layout::unit::PxSize
    Layout {
        /// Layout pass state.
        wl: &'a mut WidgetLayout,
        /// Return value, the widget's final size after layout.
        final_size: &'a mut PxSize,
    },
    /// The [`UiNode::render`].
    ///
    /// Generate render instructions and update transforms and hit-test areas.
    ///
    /// This operation does not generate pixels immediately, it generates *display items* that are visual building block instructions
    /// for the renderer that will run after the window *display list* is built.
    ///
    /// Only widgets and ancestors that requested render receive this call, other widgets reuse the display items and transforms
    /// from the last frame.
    Render {
        /// Frame builder.
        frame: &'a mut FrameBuilder,
    },
    /// The [`UiNode::render_update`].
    ///
    /// Update values in the last generated frame.
    ///
    /// Some display item values and transforms can be updated directly, without needing to rebuild the display list. All [`FrameBuilder`]
    /// methods that accept a [`FrameValue<T>`] input can be bound to an ID that can be used to update that value.
    ///
    /// Only widgets and ancestors that requested render update receive this call. Note that if any other widget in the same window
    /// requests render all pending render update requests are upgraded to render requests.
    ///
    /// [`FrameValue<T>`]: crate::render::FrameValue
    RenderUpdate {
        /// Fame updater.
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
    pub fn reborrow(&mut self) -> UiNodeOp<'_> {
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
impl fmt::Debug for UiNodeOp<'_> {
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
/// The closure node can delegate to `child`, when the `closure` itself does not delegate, the `child` methods
/// are called after the closure returns. See [`MatchNodeChild`] for more details.
///
/// This is a convenient way of declaring anonymous nodes, such as those that implement a property function. By leveraging
/// closure captures, state can be easily declared and used, without the verbosity of declaring a struct.
///
/// # Examples
///
/// The example declares a property node that implements multiple UI node operations.
///
/// ```
/// # fn main() { }
/// # use zng_app::{*, widget::{*, node::*, builder::*}};
/// # use zng_var::*;
/// # use zng_layout::context::LAYOUT;
/// #[property(LAYOUT)]
/// pub fn count_layout(child: impl IntoUiNode, enabled: impl IntoVar<bool>) -> UiNode {
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
/// # See Also
///
/// See also [`match_node_leaf`] that declares a leaf node (no child) and [`match_widget`] that can extend a widget node.
///
/// [`match_widget`]: fn@match_widget
pub fn match_node(child: impl IntoUiNode, closure: impl FnMut(&mut MatchNodeChild, UiNodeOp) + Send + 'static) -> UiNode {
    #[cfg(feature = "dyn_closure")]
    let closure: Box<dyn FnMut(&mut MatchNodeChild, UiNodeOp) + Send> = Box::new(closure);

    match_node_impl(child.into_node(), closure)
}

fn match_node_impl(child: UiNode, closure: impl FnMut(&mut MatchNodeChild, UiNodeOp) + Send + 'static) -> UiNode {
    struct MatchNode<F> {
        child: MatchNodeChild,
        closure: F,
    }
    impl<F: FnMut(&mut MatchNodeChild, UiNodeOp) + Send + 'static> UiNodeImpl for MatchNode<F> {
        fn children_len(&self) -> usize {
            self.child.node.0.children_len()
        }

        fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
            self.child.node.0.with_child(index, visitor);
        }
        // !!: TODO delegate default methods too

        fn init(&mut self) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Init);

            if !mem::take(&mut self.child.delegated) {
                self.child.node.0.init();
            }
        }

        fn deinit(&mut self) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Deinit);

            if !mem::take(&mut self.child.delegated) {
                self.child.node.0.deinit();
            }
        }

        fn info(&mut self, info: &mut WidgetInfoBuilder) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Info { info });

            if !mem::take(&mut self.child.delegated) {
                self.child.node.0.info(info);
            }
        }

        fn event(&mut self, update: &EventUpdate) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Event { update });

            if !mem::take(&mut self.child.delegated) {
                self.child.node.0.event(update);
            }
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Update { updates });

            if !mem::take(&mut self.child.delegated) {
                self.child.node.0.update(updates);
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

                self.child.node.0.measure(wm)
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

                self.child.node.0.layout(wl)
            } else {
                size
            }
        }

        fn render(&mut self, frame: &mut FrameBuilder) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Render { frame });

            if !mem::take(&mut self.child.delegated) {
                self.child.node.0.render(frame);
            }
        }

        fn render_update(&mut self, update: &mut FrameUpdate) {
            self.child.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::RenderUpdate { update });

            if !mem::take(&mut self.child.delegated) {
                self.child.node.0.render_update(update);
            }
        }
    }
    MatchNode {
        child: MatchNodeChild {
            node: child,
            delegated: false,
        },
        closure,
    }
    .into_node()
}

/// Child node of [`match_node`].
///
/// When the closure does not delegate to this node the delegation happens after the closure returns.
///
/// [`match_node`]: fn@match_node
pub struct MatchNodeChild {
    node: UiNode,
    delegated: bool,
}
impl MatchNodeChild {
    /// Flags the current operation as *delegated*, stopping the default delegation after the closure ends.
    ///
    /// Note that each node operation methods already flags this.
    #[inline(always)]
    pub fn delegated(&mut self) {
        self.delegated = true;
    }

    /// If the current operation was already delegated to the child.
    #[inline(always)]
    pub fn has_delegated(&self) -> bool {
        self.delegated
    }

    /// Borrow the actual child.
    ///
    /// Note that if you delegate using this reference you must call [`delegated`].
    ///
    /// [`delegated`]: Self::delegated
    #[inline(always)]
    pub fn node(&mut self) -> &mut UiNode {
        &mut self.node
    }
    /// Borrow the actual child implementation.
    ///
    /// Note that if you delegate using this reference you must call [`delegated`].
    ///
    /// # Panics
    ///
    /// Panics if the child node implementation does not match.
    ///
    /// [`delegated`]: Self::delegated
    #[inline(always)]
    pub fn node_impl<U: UiNodeImpl>(&mut self) -> &mut U {
        self.node.downcast_mut::<U>().unwrap()
    }

    /// Delegate [`UiNode::init`].
    #[inline(always)]
    pub fn init(&mut self) {
        self.node.0.init();
        self.delegated = true;
    }

    /// Delegate [`UiNode::deinit`].
    #[inline(always)]
    pub fn deinit(&mut self) {
        self.node.0.deinit();
        self.delegated = true;
    }

    /// Delegate [`UiNode::info`].
    #[inline(always)]
    pub fn info(&mut self, info: &mut WidgetInfoBuilder) {
        self.node.0.info(info);
        self.delegated = true;
    }

    /// Delegate [`UiNode::event`].
    #[inline(always)]
    pub fn event(&mut self, update: &EventUpdate) {
        self.node.0.event(update);
        self.delegated = true;
    }

    /// Delegate [`UiNode::update`].
    #[inline(always)]
    pub fn update(&mut self, updates: &WidgetUpdates) {
        self.node.0.update(updates);
        self.delegated = true;
    }

    /// Delegate [`UiNode::update_list`].
    #[inline(always)]
    pub fn update_list(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        self.node.0.update_list(updates, observer);
        self.delegated = true;
    }

    /// Delegate [`UiNode::measure`].
    #[inline(always)]
    #[must_use]
    pub fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
        self.delegated = true;
        self.node.0.measure(wm)
    }

    /// Delegate [`UiNode::measure_list`].
    #[inline(always)]
    #[must_use]
    pub fn measure_list(
        &mut self,
        wm: &mut WidgetMeasure,
        measure: impl Fn(usize, &mut UiNode, &mut WidgetMeasure) -> PxSize + Sync,
        fold_size: impl Fn(PxSize, PxSize) -> PxSize + Sync,
    ) -> PxSize {
        self.delegated = true;
        self.node.0.measure_list(wm, &measure, &fold_size)
    }

    /// Delegate [`UiNode::layout`].
    #[inline(always)]
    #[must_use]
    pub fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        self.delegated = true;
        self.node.0.layout(wl)
    }

    /// Delegate [`UiNode::layout_list`].
    #[inline(always)]
    #[must_use]
    pub fn layout_list(
        &mut self,
        wl: &mut WidgetLayout,
        layout: impl Fn(usize, &mut UiNode, &mut WidgetLayout) -> PxSize + Sync,
        fold_size: impl Fn(PxSize, PxSize) -> PxSize + Sync,
    ) -> PxSize {
        self.delegated = true;
        self.node.0.layout_list(wl, &layout, &fold_size)
    }

    /// Delegate [`UiNode::render`].
    #[inline(always)]
    pub fn render(&mut self, frame: &mut FrameBuilder) {
        self.node.0.render(frame);
        self.delegated = true;
    }

    /// Delegate [`UiNode::render_update`].
    #[inline(always)]
    pub fn render_update(&mut self, update: &mut FrameUpdate) {
        self.node.0.render_update(update);
        self.delegated = true;
    }

    /// Delegate [`UiNode::op`].
    pub fn op(&mut self, op: UiNodeOp) {
        self.node.op(op);
        self.delegated = true;
    }
}

/// Creates a node that is implemented as a closure that matches over [`UiNodeOp`] and does not delegate to any child node.
pub fn match_node_leaf(closure: impl FnMut(UiNodeOp) + Send + 'static) -> UiNode {
    struct MatchNodeLeaf<F> {
        closure: F,
    }
    impl<F: FnMut(UiNodeOp) + Send + 'static> UiNodeImpl for MatchNodeLeaf<F> {
        fn children_len(&self) -> usize {
            0
        }
        fn with_child(&mut self, _: usize, _: &mut dyn FnMut(&mut UiNode)) {}
        // !!: TODO no-op for defaults too

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
    UiNode::new(MatchNodeLeaf { closure })
}

/// Creates a widget that is implemented as a closure that matches over [`UiNodeOp`] and delegates to another child widget.
///
/// The returned node will delegate to `child` like [`match_node`] does, and will also delegate [`UiNode::as_widget`].
///
/// Note that the `closure` itself will not run inside [`WidgetUiNode::with_context`].
pub fn match_widget(child: impl IntoUiNode, closure: impl FnMut(&mut MatchWidgetChild, UiNodeOp) + Send + 'static) -> UiNode {
    struct MatchWidget<F> {
        child: MatchWidgetChild,
        closure: F,
    }
    impl<F: FnMut(&mut MatchWidgetChild, UiNodeOp) + Send + 'static> UiNodeImpl for MatchWidget<F> {
        fn children_len(&self) -> usize {
            self.child.0.node.children_len()
        }

        fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
            self.child.0.node.with_child(index, visitor);
        }
        // !!: TODO others too

        fn init(&mut self) {
            self.child.0.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Init);

            if !mem::take(&mut self.child.0.delegated) {
                self.child.0.node.0.init();
            }
        }

        fn deinit(&mut self) {
            self.child.0.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Deinit);

            if !mem::take(&mut self.child.0.delegated) {
                self.child.0.node.0.deinit();
            }
        }

        fn info(&mut self, info: &mut WidgetInfoBuilder) {
            self.child.0.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Info { info });

            if !mem::take(&mut self.child.0.delegated) {
                self.child.0.node.0.info(info);
            } else {
                #[cfg(debug_assertions)]
                if self
                    .child
                    .0
                    .node
                    .as_widget()
                    .map(|mut w| {
                        w.with_context(WidgetUpdateMode::Ignore, || {
                            WIDGET.pending_update().contains(crate::update::UpdateFlags::INFO)
                        })
                    })
                    .unwrap_or(false)
                {
                    // this is likely an error, but a child widget could have requested info again
                    tracing::warn!(target: "match_widget-pending", "pending info build after info delegated in {:?}", WIDGET.id());
                }
            }
        }

        fn event(&mut self, update: &EventUpdate) {
            self.child.0.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Event { update });

            if !mem::take(&mut self.child.0.delegated) {
                self.child.0.node.0.event(update);
            }
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            self.child.0.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Update { updates });

            if !mem::take(&mut self.child.0.delegated) {
                self.child.0.node.0.update(updates);
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
                    tracing::error!("measure changed size without flagging delegated in {:?}", WIDGET.id());
                    return size;
                }

                self.child.0.node.0.measure(wm)
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
                    tracing::error!("layout changed size without flagging delegated in {:?}", WIDGET.id());
                    return size;
                }

                self.child.0.node.0.layout(wl)
            } else {
                #[cfg(debug_assertions)]
                if self
                    .child
                    .0
                    .node
                    .as_widget()
                    .map(|mut w| {
                        w.with_context(WidgetUpdateMode::Ignore, || {
                            WIDGET.pending_update().contains(crate::update::UpdateFlags::LAYOUT)
                        })
                    })
                    .unwrap_or(false)
                {
                    // this is likely an error, but a child widget could have requested layout again,
                    tracing::warn!(target: "match_widget-pending", "pending layout after layout delegated in {:?}", WIDGET.id());
                }
                size
            }
        }

        fn render(&mut self, frame: &mut FrameBuilder) {
            self.child.0.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::Render { frame });

            if !mem::take(&mut self.child.0.delegated) {
                self.child.0.node.0.render(frame);
            } else {
                #[cfg(debug_assertions)]
                if self
                    .child
                    .0
                    .node
                    .as_widget()
                    .map(|mut w| {
                        w.with_context(WidgetUpdateMode::Ignore, || {
                            WIDGET.pending_update().contains(crate::update::UpdateFlags::RENDER)
                        })
                    })
                    .unwrap_or(false)
                {
                    // this is likely an error, but a child widget could have requested render again,
                    tracing::warn!(target: "match_widget-pending", "pending render after render delegated in {:?}", WIDGET.id());
                }
            }
        }

        fn render_update(&mut self, update: &mut FrameUpdate) {
            self.child.0.delegated = false;

            (self.closure)(&mut self.child, UiNodeOp::RenderUpdate { update });

            if !mem::take(&mut self.child.0.delegated) {
                self.child.0.node.0.render_update(update);
            } else {
                #[cfg(debug_assertions)]
                if self
                    .child
                    .0
                    .node
                    .as_widget()
                    .map(|mut w| {
                        w.with_context(WidgetUpdateMode::Ignore, || {
                            WIDGET.pending_update().contains(crate::update::UpdateFlags::RENDER_UPDATE)
                        })
                    })
                    .unwrap_or(false)
                {
                    // this is likely an error, but a child widget could have requested render_update again,
                    tracing::warn!(target: "match_widget-pending", "pending render_update after render_update delegated in {:?}", WIDGET.id());
                }
            }
        }
    }
    MatchWidget {
        child: MatchWidgetChild(MatchNodeChild {
            node: child.into_node(),
            delegated: false,
        }),
        closure,
    }
    .into_node()
}

/// Child node of [`match_widget`].
///
/// This node delegates like [`MatchNodeChild`] plus delegates [`UiNode::as_widget`].
///
/// [`match_widget`]: fn@match_widget
pub struct MatchWidgetChild(MatchNodeChild);
impl ops::Deref for MatchWidgetChild {
    type Target = MatchNodeChild;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ops::DerefMut for MatchWidgetChild {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
