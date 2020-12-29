use crate::context::*;
use crate::render::{FrameBuilder, FrameUpdate};
use crate::units::*;

/// Generates default implementations of [`UiNode`](zero_ui::core::UiNode) methods.
///
/// # Arguments
///
/// The macro attribute takes arguments that indicate how the missing methods will be generated.
///
/// ## Single Node Delegate
///
/// Set this two arguments to delegate to a single node:
///
/// * `delegate: &impl UiNode` - Expression that borrows the node, you can use `self` in the expression.
/// * `delegate_mut: &mut impl UiNode` - Exclusive borrow the node.
///
/// ## Multiple Nodes Delegate
///
/// Set this two arguments to delegate to a widget list:
///
/// * `delegate_list: & impl WidgetList` - Expression that borrows the list.
/// * `delegate_list_mut: &mut impl WidgetList` - Exclusive borrow the list.
///
/// Or, set this two arguments to delegate to a node iterator sequence:
///
/// * `delegate_iter: impl Iterator<& impl UiNode>` - Expression that creates a borrowing iterator.
/// * `delegate_iter_mut: impl Iterator<&mut impl UiNode>` - Exclusive borrowing iterator.
///
/// ## Shorthand
///
/// You can also use shorthand for common delegation:
///
/// * `child` is the same as `delegate: &self.child, delegate_mut: &mut self.child`.
/// * `children` is the same as `delegate_list: &self.children, delegate_list_mut: &mut self.children`.
/// * `children_iter` is the same as `delegate_iter: self.children.iter(), delegate_iter_mut: self.children.iter_mut()`.
///
/// ## None
///
/// And for nodes without descendants you can use:
/// * `none`
///
/// # Validation
/// If delegation is configured but no delegation occurs in the manually implemented methods
/// you get the error ``"auto impl delegates call to `{}` but this manual impl does not"``.
///
/// To disable this error use `#[allow_missing_delegate]` in the method or in the `impl` block.
///
/// # Usage Examples
///
/// Given an UI node `struct`:
/// ```
/// # use zero_ui::core::units::LayoutSize;
/// struct FillColorNode<C> {
///     color: C,
///     final_size: LayoutSize,
/// }
/// ```
///
/// In an `UiNode` trait impl block, annotate the impl block with `#[impl_ui_node(..)]` and only implement custom methods.
///
/// ```
/// # use zero_ui::prelude::new_property::*;
/// # struct FillColorNode<C> { color: C, final_size: LayoutSize, }
/// #[impl_ui_node(none)]
/// impl<C: VarLocal<Rgba>> UiNode for FillColorNode<C> {
///     fn render(&self, frame: &mut FrameBuilder) {
///         let area = LayoutRect::from_size(self.final_size);
///         frame.push_color(area, (*self.color.get_local()).into());
///     }
/// }
/// ```
///
/// Or, in a inherent impl, annotate the impl block with `#[impl_ui_node(..)]` and custom `UiNode` methods with `#[UiNode]`.
///
/// ```
/// # use zero_ui::prelude::new_property::*;
/// # struct FillColorNode<C> { color: C, final_size: LayoutSize, }
/// #[impl_ui_node(none)]
/// impl<C: VarLocal<Rgba>> FillColorNode<C> {
///     pub fn new(color: C) -> Self {
///         FillColorNode { color, final_size: LayoutSize::zero() }
///     }
///
///     #[UiNode]
///     fn render(&self, frame: &mut FrameBuilder) {
///         let area = LayoutRect::from_size(self.final_size);
///         frame.push_color(area, (*self.color.get_local()).into());
///     }
/// }
/// ```
///
/// In both cases a full `UiNode` implement is generated for the node `struct`, but in the second case the inherent methods
/// are also kept, you can use this to reduce verbosity for nodes with multiple generics.
///
/// ## Delegate to `none`
///
/// Generates defaults for UI components without descendants.
///
/// ### Defaults
///
/// * Init, Updates: Does nothing, blank implementation.
/// * Layout: Fills finite spaces, collapses in infinite spaces.
/// * Render: Does nothing, blank implementation.
///
/// ```
/// # use zero_ui::prelude::new_property::*;
/// # struct FillColorNode<C> { color: C, final_size: LayoutSize }
/// #[impl_ui_node(none)]
/// impl<C: VarLocal<Rgba>> FillColorNode<C> {
///     pub fn new(color: C) -> Self {
///          FillColorNode { color, final_size: LayoutSize::zero() }
///     }
///
///     #[UiNode]
///     fn render(&self, frame: &mut FrameBuilder) {
///         let area = LayoutRect::from_size(self.final_size);
///         frame.push_color(area, (*self.color.get_local()).into())
///     }
/// }
/// ```
/// Expands to:
///
/// ```
/// # use zero_ui::prelude::new_property::*;
/// # struct FillColorNode<C> { color: C, final_size: LayoutSize }
/// impl<C: VarLocal<Rgba>> FillColorNode<C> {
///     pub fn new(color: C) -> Self {
///          FillColorNode { color, final_size: LayoutSize::zero() }
///     }
/// }
///
/// impl<C: VarLocal<Rgba>> zero_ui::core::UiNode for FillColorNode<C> {
///     #[inline]
///     fn init(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) { }
///
///     #[inline]
///     fn update(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) { }
///
///     #[inline]
///     fn update_hp(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) { }
///
///     #[inline]
///     fn measure(&mut self, available_size: zero_ui::core::units::LayoutSize, ctx: &mut zero_ui::core::context::LayoutContext) -> zero_ui::core::units::LayoutSize {
///         let mut size = available_size;
///         if zero_ui::core::is_layout_any_size(size.width) {
///             size.width = 0.0;
///         }
///         if zero_ui::core::is_layout_any_size(size.height) {
///             size.height = 0.0;
///         }
///         size
///     }
///
///     #[inline]
///     fn arrange(&mut self, final_size: zero_ui::core::units::LayoutSize, ctx: &mut zero_ui::core::context::LayoutContext) { }
///
///     #[inline]
///     fn render(&self, frame: &mut zero_ui::core::render::FrameBuilder) {
///         // empty here when you don't implement render.
///
///         let area = LayoutRect::from_size(self.final_size);
///         frame.push_color(area, (*self.color.get_local()).into())
///     }
///
///     #[inline]
///     fn render_update(&self, update: &mut zero_ui::core::render::FrameUpdate) { }
///
///     #[inline]
///     fn deinit(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) { }
/// }
/// ```
///
/// ## Delegate to one (`child` or `delegate, delegate_mut`)
///
/// Generates defaults for UI components with a single child node. This is the most common mode,
/// used by property nodes.
///
/// ### Defaults
///
/// * Init, Updates: Delegates to same method in child.
/// * Layout: Is the same size as child.
/// * Render: Delegates to child render.
///
/// ```
/// # use zero_ui::prelude::new_property::*;
/// struct DelegateChildNode<C: UiNode> { child: C }
///
/// #[impl_ui_node(child)]
/// impl<C: UiNode> UiNode for DelegateChildNode<C> { }
/// ```
///
/// Expands to:
///
/// ```
/// # use zero_ui::prelude::new_property::*;
/// # struct DelegateChildNode<C: UiNode> { child: C }
/// impl<C: UiNode> UiNode for DelegateChildNode<C> {
///     #[inline]
///     fn init(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         let child = { &mut self.child };
///         child.init(ctx)
///     }
///
///     #[inline]
///     fn update(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         let child = { &mut self.child };
///         child.update(ctx)
///     }
///
///     #[inline]
///     fn update_hp(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         let child = { &mut self.child };
///         child.update_hp(ctx)
///     }
///
///     #[inline]
///     fn measure(&mut self, available_size: zero_ui::core::units::LayoutSize, ctx: &mut zero_ui::core::context::LayoutContext) -> zero_ui::core::units::LayoutSize {
///         let child = { &mut self.child };
///         child.measure(available_size, ctx)
///     }
///
///     #[inline]
///     fn arrange(&mut self, final_size: zero_ui::core::units::LayoutSize, ctx: &mut zero_ui::core::context::LayoutContext) {
///         let child = { &mut self.child };
///         child.arrange(final_size, ctx)
///     }
///
///     #[inline]
///     fn render(&self, frame: &mut zero_ui::core::render::FrameBuilder) {
///         let child = { &self.child };
///         child.render(frame)
///     }
///
///     #[inline]
///     fn render_update(&self, update: &mut zero_ui::core::render::FrameUpdate) {
///         let child = { &self.child };
///         child.render_update(update)
///     }
///
///     #[inline]
///     fn deinit(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         let child = { &mut self.child };
///         child.deinit(ctx)
///     }
/// }
/// ```
///
/// ## Delegate to many (`children` or `delegate_list, delegate_list_mut`)
///
/// Generates defaults for UI components with a multiple children widgets. This is used mostly by
/// layout panels.
///
/// ### Defaults
///
/// * Init, Updates: Calls the [`WidgetList`] equivalent method.
/// * Layout: Is the same size as the largest child.
/// * Render: Z-stacks the children. Last child on top.
///
/// ```
/// # use zero_ui::prelude::new_property::*;
/// struct DelegateChildrenNode<C: WidgetList> {
///     children: C,
/// }
/// #[impl_ui_node(children)]
/// impl<C: WidgetList> UiNode for DelegateChildrenNode<C> { }
/// ```
///
/// Expands to:
///
/// ```
/// # use zero_ui::prelude::new_property::*;
/// # struct DelegateChildrenNode { children: WidgetVec }
/// impl UiNode for DelegateChildrenNode {
///     #[inline]
///     fn init(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         let children = { &mut self.children };
///         children.init_all(ctx);
///     }
///
///     #[inline]
///     fn update(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         let children = { &mut self.children };
///         children.update_all(ctx);
///     }
///
///     #[inline]
///     fn update_hp(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         let children = { &mut self.children };
///         children.update_hp_all(ctx);
///     }
///
///     #[inline]
///     fn measure(&mut self, available_size: zero_ui::core::units::LayoutSize, ctx: &mut zero_ui::core::context::LayoutContext) -> zero_ui::core::units::LayoutSize {
///         let children = { &mut self.children };
///         let mut size = zero_ui::core::units::LayoutSize::zero();
///         children.measure_all(|_, _|available_size, |_, desired_size, _| {
///             size = size.max(desired_size);
///         }, ctx);
///         size
///     }
///
///     #[inline]
///     fn arrange(&mut self, final_size: zero_ui::core::units::LayoutSize, ctx: &mut zero_ui::core::context::LayoutContext) {
///         let children = { &mut self.children };
///         children.arrange_all(|_, _|final_size, ctx);
///     }
///
///     #[inline]
///     fn render(&self, frame: &mut zero_ui::core::render::FrameBuilder) {
///         let children = { &self.children };
///         children.render_all(|_|zero_ui::core::units::LayoutPoint::zero(), frame);
///     }
///
///     #[inline]
///     fn render_update(&self, update: &mut zero_ui::core::render::FrameUpdate) {
///         let children = { &self.children };
///         children.render_update_all(update);
///     }
///
///     #[inline]
///     fn deinit(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         let children = { &mut self.children };
///         children.deinit_all(ctx);
///     }
/// }
/// ```
///
///
/// ## Delegate to many nodes (`children_iter` or `delegate_iter, delegate_iter_mut`)
///
/// Generates defaults for UI components with a multiple children nodes. This must be used only
/// when a widget list cannot be used.
///
/// ### Defaults
///
/// * Init, Updates: Delegates to same method in each child.
/// * Layout: Is the same size as the largest child.
/// * Render: Z-stacks the children. Last child on top.
///
/// ```
/// # use zero_ui::prelude::new_property::*;
/// struct DelegateChildrenNode {
///     children: WidgetVec,
/// }
/// #[impl_ui_node(children_iter)]
/// impl UiNode for DelegateChildrenNode { }
/// ```
///
/// Expands to:
///
/// ```
/// # use zero_ui::prelude::new_property::*;
/// # struct DelegateChildrenNode { children: WidgetVec }
/// impl UiNode for DelegateChildrenNode {
///     #[inline]
///     fn init(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         for child in { self.children.iter_mut() } {
///             child.init(ctx)
///         }
///     }
///
///     #[inline]
///     fn update(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         for child in { self.children.iter_mut() } {
///             child.update(ctx)
///         }
///     }
///
///     #[inline]
///     fn update_hp(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         for child in { self.children.iter_mut() } {
///             child.update_hp(ctx)
///         }
///     }
///
///     #[inline]
///     fn measure(&mut self, available_size: zero_ui::core::units::LayoutSize, ctx: &mut zero_ui::core::context::LayoutContext) -> zero_ui::core::units::LayoutSize {
///         let mut size = zero_ui::core::units::LayoutSize::zero();
///         for child in { self.children.iter_mut() } {
///            size = child.measure(available_size, ctx).max(size);
///         }
///         size
///     }
///
///     #[inline]
///     fn arrange(&mut self, final_size: zero_ui::core::units::LayoutSize, ctx: &mut zero_ui::core::context::LayoutContext) {
///         for child in { self.children.iter_mut() } {
///             child.arrange(final_size, ctx)
///         }
///     }
///
///     #[inline]
///     fn render(&self, frame: &mut zero_ui::core::render::FrameBuilder) {
///         for child in { self.children.iter() } {
///             child.render(frame)
///         }
///     }
///
///     #[inline]
///     fn render_update(&self, update: &mut zero_ui::core::render::FrameUpdate) {
///         for child in { self.children.iter() } {
///             child.render_update(update)
///         }
///     }
///
///     #[inline]
///     fn deinit(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         for child in { self.children.iter_mut() } {
///             child.deinit(ctx)
///         }
///     }
/// }
/// ```
pub use zero_ui_proc_macros::impl_ui_node;

unique_id! {
    /// Unique id of a widget.
    ///
    /// # Details
    /// Underlying value is a `NonZeroU64` generated using a relaxed global atomic `fetch_add`,
    /// so IDs are unique for the process duration, but order is not guaranteed.
    ///
    /// Panics if you somehow reach `u64::max_value()` calls to `new`.
    pub struct WidgetId;
}

/// An Ui tree node.
pub trait UiNode: 'static {
    /// Called every time the node is plugged in an Ui tree.
    fn init(&mut self, ctx: &mut WidgetContext);

    /// Called every time the node is unplugged from an Ui tree.
    fn deinit(&mut self, ctx: &mut WidgetContext);

    /// Called every time a low pressure event update happens.
    ///
    /// # Event Pressure
    /// See [`update_hp`](UiNode::update_hp) for more information about event pressure rate.
    fn update(&mut self, ctx: &mut WidgetContext);

    /// Called every time a high pressure event update happens.
    ///
    /// # Event Pressure
    /// Some events occur a lot more times then others, for performance reasons this
    /// event source may choose to be propagated in this high-pressure lane.
    ///
    /// Event sources that are high pressure mention this in their documentation.
    fn update_hp(&mut self, ctx: &mut WidgetContext);

    /// Called every time a layout update is needed.
    ///
    /// # Arguments
    /// * `available_size`: The total available size for the node. Can contain positive infinity to
    /// indicate the parent will accommodate [any size](is_layout_any_size). Finite values are pixel aligned.
    /// * `ctx`: Measure context.
    ///
    /// # Return
    /// Return the nodes desired size. Must not contain infinity or NaN. Must be pixel aligned.
    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize;

    /// Called every time a layout update is needed, after [`measure`](UiNode::measure).
    ///
    /// # Arguments
    /// * `final_size`: The size the parent node reserved for the node. Must reposition its contents
    /// to fit this size. The value does not contain infinity or NaNs and is pixel aligned.
    /// TODO args docs.
    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext);

    /// Called every time a new frame must be rendered.
    ///
    /// # Arguments
    /// * `frame`: Contains the next frame draw instructions.
    fn render(&self, frame: &mut FrameBuilder);

    /// Called every time a frame can be updated without fully rebuilding.
    ///
    /// # Arguments
    /// * `update`: Contains the frame value updates.
    fn render_update(&self, update: &mut FrameUpdate);

    /// Box this node, unless it is already `Box<dyn UiNode>`.
    fn boxed(self) -> Box<dyn UiNode>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}
#[impl_ui_node(delegate: self.as_ref(), delegate_mut: self.as_mut())]
impl UiNode for Box<dyn UiNode> {
    fn boxed(self) -> Box<dyn UiNode> {
        self
    }
}

/// Represents an widget [`UiNode`].
pub trait Widget: UiNode {
    fn id(&self) -> WidgetId;

    fn state(&self) -> &LazyStateMap;
    fn state_mut(&mut self) -> &mut LazyStateMap;

    /// Last arranged size.
    fn size(&self) -> LayoutSize;

    /// Box this widget node, unless it is already `Box<dyn Widget>`.
    fn boxed_widget(self) -> Box<dyn Widget>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}
#[impl_ui_node(delegate: self.as_ref(), delegate_mut: self.as_mut())]
impl UiNode for Box<dyn Widget> {}
impl Widget for Box<dyn Widget> {
    #[inline]
    fn id(&self) -> WidgetId {
        self.as_ref().id()
    }
    #[inline]
    fn state(&self) -> &LazyStateMap {
        self.as_ref().state()
    }
    #[inline]
    fn state_mut(&mut self) -> &mut LazyStateMap {
        self.as_mut().state_mut()
    }
    #[inline]
    fn size(&self) -> LayoutSize {
        self.as_ref().size()
    }
    #[inline]
    fn boxed_widget(self) -> Box<dyn Widget> {
        self
    }
}

/// A UI node that does not contain any other node, does not take any space and renders nothing.
pub struct NilUiNode;
#[impl_ui_node(none)]
impl UiNode for NilUiNode {
    fn measure(&mut self, _: LayoutSize, _: &mut LayoutContext) -> LayoutSize {
        LayoutSize::zero()
    }
}

/// A UI node that does not contain any other node, fills the available space, but renders nothing.
pub struct FillUiNode;
#[impl_ui_node(none)]
impl UiNode for FillUiNode {}

// This is called by the default widgets `new_child` function.
///
/// See [widgets](crate::widgets) for more details.
///
/// Returns a [`NilUiNode`].
#[inline]
pub fn default_widget_new_child() -> impl UiNode {
    NilUiNode
}

/// This is called by the default widgets `new` function.
///
/// See [widgets](crate::widgets) for more details.
///
/// A new widget context is introduced by this function. `child` is wrapped in a node that calls
/// [`WidgetContext::widget_context`](WidgetContext::widget_context) and [`FrameBuilder::push_widget`] to define the widget.
#[inline]
pub fn default_widget_new(child: impl UiNode, id_args: impl zero_ui::properties::capture_only::widget_id::Args) -> impl Widget {
    WidgetNode {
        id: id_args.unwrap(),
        transform_key: WidgetTransformKey::new_unique(),
        state: LazyStateMap::default(),
        child,
        size: LayoutSize::zero(),
    }
}

struct WidgetNode<T: UiNode> {
    id: WidgetId,
    transform_key: WidgetTransformKey,
    state: LazyStateMap,
    child: T,
    size: LayoutSize,
}

#[impl_ui_node(child)]
impl<T: UiNode> UiNode for WidgetNode<T> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        let child = &mut self.child;
        ctx.widget_context(self.id, &mut self.state, |ctx| child.init(ctx));
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        let child = &mut self.child;
        ctx.widget_context(self.id, &mut self.state, |ctx| child.deinit(ctx));
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        let child = &mut self.child;
        ctx.widget_context(self.id, &mut self.state, |ctx| child.update(ctx));
    }

    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        let child = &mut self.child;
        ctx.widget_context(self.id, &mut self.state, |ctx| child.update_hp(ctx));
    }

    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        #[cfg(debug_assertions)]
        {
            fn valid_measure(f: f32) -> bool {
                f.is_finite() || crate::is_layout_any_size(f)
            }

            if !valid_measure(available_size.width) || !valid_measure(available_size.height) {
                error_println!(
                    "{:?} `UiNode::measure` called with invalid `available_size: {:?}`, must be finite or `LAYOUT_ANY_SIZE`",
                    self.id,
                    available_size
                );
            }
        }

        let child_size = self.child.measure(available_size, ctx);

        #[cfg(debug_assertions)]
        {
            if !child_size.width.is_finite() || !child_size.height.is_finite() {
                error_println!("{:?} `UiNode::measure` result is not finite: `{:?}`", self.id, child_size);
            } else if !child_size.is_aligned_to(ctx.pixel_grid()) {
                let snapped = child_size.snap_to(ctx.pixel_grid());
                error_println!(
                    "{:?} `UiNode::measure` result not aligned, was: `{:?}`, expected: `{:?}`",
                    self.id,
                    child_size,
                    snapped
                );
                return snapped;
            }
        }
        child_size
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.size = final_size;

        #[cfg(debug_assertions)]
        {
            if !final_size.width.is_finite() || !final_size.height.is_finite() {
                error_println!(
                    "{:?} `UiNode::arrange` called with invalid `final_size: {:?}`, must be finite",
                    self.id,
                    final_size
                );
            } else if !final_size.is_aligned_to(ctx.pixel_grid()) {
                self.size = final_size.snap_to(ctx.pixel_grid());
                error_println!(
                    "{:?} `UiNode::arrange` called with not aligned value, was: `{:?}`, expected: `{:?}`",
                    self.id,
                    final_size,
                    self.size
                );
            }
        }

        self.child.arrange(self.size, ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_widget(self.id, self.transform_key, self.size, &self.child);
    }

    fn render_update(&self, update: &mut FrameUpdate) {
        update.update_widget(self.id, self.transform_key, &self.child);
    }
}
