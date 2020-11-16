//! Core infrastructure required for creating components and running an app.

pub mod animation;
pub mod app;
pub mod color;
pub mod context;
pub mod debug;
pub mod event;
pub mod focus;
pub mod gesture;
pub mod keyboard;
pub mod mouse;
pub mod profiler;
pub mod render;
pub mod service;
pub mod sync;
pub mod text;
pub mod types;
pub mod units;
pub mod var;
pub mod window;

use context::{LayoutContext, LazyStateMap, WidgetContext};
use render::{FrameBuilder, FrameUpdate, WidgetTransformKey};
use units::LayoutSize;

use self::units::PixelGridExt;

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
                f.is_finite() || is_layout_any_size(f)
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

impl<T: UiNode> Widget for WidgetNode<T> {
    #[inline]
    fn id(&self) -> WidgetId {
        self.id
    }
    #[inline]
    fn state(&self) -> &LazyStateMap {
        &self.state
    }
    #[inline]
    fn state_mut(&mut self) -> &mut LazyStateMap {
        &mut self.state
    }
    #[inline]
    fn size(&self) -> LayoutSize {
        self.size
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

/// This is called by the default widgets `new_child` function.
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

/// Gets if the value indicates that any size is available during layout (positive infinity)
#[inline]
pub fn is_layout_any_size(f: f32) -> bool {
    f.is_infinite() && f.is_sign_positive()
}

/// Value that indicates that any size is available during layout.
pub const LAYOUT_ANY_SIZE: f32 = f32::INFINITY;

/// A mixed vector of [`Widget`] types.
pub type UiVec = Vec<Box<dyn Widget>>;

/// A map of TypeId -> Box<dyn Any>.
type AnyMap = fnv::FnvHashMap<std::any::TypeId, Box<dyn std::any::Any>>;

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
/// * `delegate: &impl UiNode` - Expression that borrows the node, you can use `self`.
/// * `delegate_mut: &mut impl UiNode` - Exclusive borrow the node.
///
/// ## Multiple Nodes Delegate
///
/// Set this two arguments to delegate to a node sequence:
///
/// * `delegate_iter: impl Iterator<& impl UiNode>` - Expression that creates a borrowing iterator, you can use `self`.
/// * `delegate_iter_mut: impl Iterator<&mut impl UiNode>` - Exclusive borrowing iterator.
///
/// ## Shorthand
///
/// You can also use shorthand for common delegation:
///
/// * `child` is the same as `delegate: &self.child, delegate_mut: &mut self.child`.
/// * `children` is the same as `delegate_iter: self.children.iter(), delegate_iter_mut: self.children.iter_mut()`.
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
/// ## Delegate to many (`children` or `delegate_iter, delegate_iter_mut`)
///
/// Generates defaults for UI components with a multiple children nodes. This is used mostly by
/// layout panels.
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
///     children: UiVec,
/// }
/// #[impl_ui_node(children)]
/// impl UiNode for DelegateChildrenNode { }
/// ```
///
/// Expands to:
///
/// ```
/// # use zero_ui::prelude::new_property::*;
/// # struct DelegateChildrenNode { children: UiVec }
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
///         let mut size = Default::default();
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
pub use zero_ui_macros::impl_ui_node;

/// Expands a function to a widget property module.
///
/// # Arguments
///
/// The macro attribute takes arguments that configure how the property can be used in widgets.
///
/// **Required**
///
/// The first argument is required and indicates when the property is set in relation to the other properties in a widget.
/// The valid values are: [`context`](#context), [`event`](#event), [`outer`](#outer), [`size`](#size), [`inner`](#inner) or
/// [`capture_only`](#capture_only).
///
/// **Optional**
///
/// Optional arguments can be set after the required, they use the `name: value` syntax. Currently there is only one
/// [`allowed_in_when`](#when-conditions).
///
/// # Function
///
/// The macro attribute must be set in a stand-alone function that sets the property by modifying the UI node tree.
///
/// ## Arguments and Output
///
/// The function argument and return type requirements are the same for normal properties (not `capture_only`).
///
/// ### Normal Properties
///
/// Normal properties must take at least two arguments, the first argument is the child [`UiNode`](zero_ui::core::UiNode), the other argument(s)
/// are the property values. The function must return a type that implements `UiNode`. The first argument must support any type that implements
/// `UiNode`. All of these requirements are validated at compile time.
///
/// ```
/// # fn main() { }
/// use zero_ui::core::{property, UiNode, impl_ui_node, var::{Var, IntoVar}, context::WidgetContext};
///
/// struct MyNode<C, V> { child: C, value: V }
/// #[impl_ui_node(child)]
/// impl<C: UiNode, V: Var<&'static str>> UiNode for MyNode<C, V> {
///     fn init(&mut self, ctx: &mut WidgetContext) {
///         self.child.init(ctx);
///         println!("{}", self.value.get(ctx.vars));
///     }
/// }
///
/// /// Property docs.
/// #[property(context)]
/// pub fn my_property(child: impl UiNode, value: impl IntoVar<&'static str>) -> impl UiNode {
///     MyNode { child, value: value.into_var() }
/// }
/// ```
///
/// ### `capture_only`
///
/// Capture-only properties do not modify the UI node tree, they exist only as a named bundle of arguments that widgets capture to use internally.
/// At least one argument is required. The return type must be never (`!`) and the property body must be empty.
///
/// ```
/// # fn main() { }
/// use zero_ui::core::{property, var::IntoVar, text::Text};
///
/// /// Property docs.
/// #[property(capture_only)]
/// pub fn my_property(value: impl IntoVar<Text>) -> ! { }
/// ```
/// ## Limitations
///
/// There are some limitations to what kind of function can be used:
///
/// * Only standalone safe functions are supported, type methods, `extern` functions and `unsafe` are not supported.
/// * Only sized 'static types are supported.
/// * All stable generics are supported, generic bounds, impl trait and where clauses, const generics are not supported.
/// * Const functions are not supported. You need generics to support any type of UI node but generic const functions are unstable.
/// * Async functions are not supported.
/// * Only the simple argument pattern `name: T` are supported. Destructuring arguments or discard (_) are not supported.
///
/// ## Name
///
/// The property name follows some conventions that are enforced at compile time.
///
/// * `on_` prefix: Can only be used for `event` or `capture_only` properties and must take only a single event handler value.
/// * `is_` prefix: Can only take a single [`StateVar`](zero_ui::core::var::StateVar) value.
///
/// # Priority
///
/// Except for `capture_only` the other configurations indicate the priority that the property must be applied to form a widget.
///
/// ## `context`
///
/// The property is applied after all other so that they can setup information associated with the widget that the other properties
/// can use. Context variables and widget state use this priority.
///
/// You can easily implement this properties using [`with_context_var`](zero_ui::properties::with_context_var)
/// and [`set_widget_state`](zero_ui::properties::set_widget_state).
///
/// ## `event`
///
/// Event properties are the next priority, they are set after all others except `context`, this way events can be configured by the
/// widget context properties but also have access to the widget visual they contain.
///
/// It is strongly encouraged that the event handler signature matches the one from [`on_event`](zero_ui::properties::events::on_event).
///
/// ## `outer`
///
/// Properties that shape the visual outside of the widget, the [`margin`](zero_ui::properties::margin) property is an example.
///
/// ## `size`
///
/// Properties that set the widget visual size. Most widgets are sized automatically by their content, if the size is configured by a user value
/// the property has this priority.
///
/// ## `inner`
///
/// Properties that are set first, so they end-up inside of all other widget properties. Most of the properties that render use this priority.
///
/// # When Conditions
///
/// Most properties can be used in widget when condition expressions, by default all properties that don't have the `on_` prefix are allowed.
/// This can be overridden by setting the optional argument `allowed_in_when`.
///
/// ## State Probing
///
/// Properties with the `is_` prefix are special, they output information about the widget instead of shaping it. They are automatically set
/// to a new probing variable when used in an widget when condition expression.
pub use zero_ui_macros::property;

/// Declares a new widget macro and module.
///
/// Widgets are a bundle of [property blocks](#property-blocks), [when blocks](#when-blocks) and [initialization functions](#initialization-functions).
///
/// # Header
///
/// The widget header declares the widget name, [documentation](#attributes), [visibility](#visibility) and what other widgets and mix-ins
/// are [inherited](#inheritance) into the new one.
///
/// ```
/// # fn main() { }
/// # use zero_ui::core::widget;
/// # use zero_ui::widgets::{container, mixins::focusable_mixin};
/// widget! {
///     /// Widget documentation.
///     pub button: container + focusable_mixin;
/// }
/// ```
///
/// ## Attributes
///
/// All attributes are transferred to the generated module. Conditional compilation (`#[cfg]`) attributes are also applied to the generated macro.
/// Extra documentation about the widget properties is auto-generated and added to the module as well.
///
/// ```
/// # use zero_ui::core::widget;
/// widget! {
///     /// Widget documentation.
///     #[cfg(debug_assertions)]
///     widget_name;
/// }
/// ```
///
/// ## Visibility
///
/// The visibility is transferred to the widget module and macro and supports all visibility configurations.
///
/// ```
/// # use zero_ui::core::widget;
/// widget! {
///     pub(crate) widget_name;
/// }
/// ```
///
/// ## Inheritance
///
/// Widgets can optionally 'inherit' from other widgets and widget mix-ins.
///
/// ```
/// # fn main() { }
/// # use zero_ui::core::widget;
/// # use zero_ui::widgets::{container, mixins};
/// widget! {
///     pub foo: container;
/// }
///
/// widget! {
///     pub bar: container + mixins::focusable_mixin;
/// }
/// ```
///
/// Widgets inheritance works by 'importing' all properties, when blocks and init functions into the new widget.
/// All widgets automatically inherit from [`implicit_mixin`](mod@zero_ui::widgets::mixins::implicit_mixin) (after all other inherits).
///
/// ### Conflict Resolution
///
/// Properties and functions of the same name are overwritten by the left-most import or by the new widget declaration.
///
/// When blocks with conditions that are no longer valid are removed.
///
/// # Property Blocks
///
/// Property blocks contains a list of [property declarations](#property-declarations) grouped by the [target](#target) of the properties.
///
/// ```
/// # fn main() { }
/// # use zero_ui::core::widget;
/// # use zero_ui::properties::margin;
/// widget! {
///     pub foo;
///
///     default {
///         margin: 2.0;
///     }
///     default_child {
///         padding -> margin: 5.0;
///     }
/// }
/// ```
///
/// # Target
///
/// The property targets are selected by the keyword used to open a property block, `default` properties are applied
/// to the widget normally, `default_child` properties are applied first so that they affect the widget child node before
/// all other properties.
///
/// ## Property Declarations
///
/// Properties are declared by their [name](#name-resolution) follow by optional [remapping](#remapping), default or
/// special value and terminated by semi-colon (`;`). They can also have documentation attributes.
///
/// ### Name Resolution
///
/// If a property with the same name is inherited that is the property, if not then is is assumed that a
/// [`property`](zero_ui::core::property) module is with the same name is imported.
///
/// You can only use single names, module paths are not allowed. You can only declare a property with the same name once,
///
/// ### Remapping
///
/// New properties can map to other properties, meaning the other property is applied when the new property is used. This is also
/// the only way to apply the same property twice.
///
/// ```
/// # fn main() { }
/// # use zero_ui::core::{widget, property, UiNode, var::IntoVar};
/// # #[property(context)]
/// # fn other_property(child: impl UiNode, value: impl IntoVar<bool>) -> impl UiNode { child }
/// widget! {
/// # widget_name;
///     //..
///     
///     default {
///         new_property -> other_property;
///     }
/// }
/// ```
///
/// ### Default Value
///
/// Properties can have a default value. If they do the property is applied automatically during widget
/// instantiation using the default value if the user does not set the property.
///
/// ```
/// # fn main() { }
/// # use zero_ui::core::{widget, property, UiNode, var::IntoVar, text::Text};
/// # #[property(context)]
/// # pub fn my_property(child: impl UiNode, value: impl IntoVar<Text>) -> impl UiNode { child }
/// widget! {
/// # widget_name;
///     //..
///     
///     default {
///         my_property: "value";
///         foo -> my_property: "value";
///     }
/// }
/// ```
///
/// Properties without a default value are only applied if the user sets then.
///
/// ### `required!`
///
/// Properties declared with the `required!` special value must be set by the user during widget initialization.
///
/// ```
/// # fn main() { }
/// # use zero_ui::core::widget;
/// # use zero_ui::properties::events::on_click;
/// widget! {
/// # widget_name;
///     //..
///     
///     default {
///         on_click: required!;
///     }
/// }
/// ```
///
/// [Captured](#initialization-functions) properties are also required.
///
/// ### `unset!`
///
/// Removes an inherited property by redeclaring then with the `unset!` special value.
///
/// # When Blocks
///
/// When blocks assign properties when a condition is true, the condition references properties and is always updated
/// if the referenced values are [vars](zero_ui::core::var::Var).
///
/// ```
/// # fn main() { }
/// # use zero_ui::core::{widget, color::rgb};
/// # use zero_ui::properties::{background::background_color, states::is_pressed};
/// widget! {
/// # widget_name;
/// # default { background_color: rgb(0, 0, 0); }
///     //..
///     
///     when self.is_pressed {
///         background_color: rgb(0.3, 0.3, 0.3);
///     }
/// }
/// ```
///
/// ## Condition
///
/// The `when` condition is an expression similar to the `if` condition. In it you can reference properties by using the `self.` prefix, at least one
/// property reference is required.
///
/// If the first property argument is referenced by `self.property`, to reference other arguments you can use `self.property.1` or `self.property.arg_name`.
///
/// ```
/// # fn main() { }
/// # use zero_ui::core::{widget, color::rgb};
/// # use zero_ui::properties::{background::background_color, title, states::is_pressed};
/// widget! {
/// # widget_name;
/// # default { title: "value"; background_color: rgb(0, 0, 0); }
///     //..
///     
///     when self.title == "value" && self.is_pressed {
///         background_color: rgb(255, 0, 255);
///     }
/// }
/// ```
///
/// If the property arguments are [vars](zero_ui::core::var::Var) the when condition is reevaluated after any variable changes.
///
/// The referenced properties must have a default value, be [`required`](#required) or be a [state property](zero_ui::core::property#state-probing).
/// If the user [unsets](#unset) a referenced property the whole when block is not instantiated.
///
/// ## Assigns
///
/// Inside the when block you can assign properties using `property_name: "value";`.  
/// The assigned property must have a default value or be [`required`](#required).
/// If the user [unsets](#unset) the property it is removed from the when block.
///
/// # Initialization Functions
///
/// Every widget has two initialization functions, [`new_child`](#new_child) and [`new`](#new). They are like other rust standalone
/// functions except the input arguments have no explicit type.
///
/// ## `new_child`
///
/// Initializes the inner most node of the widget.
///
/// ```
/// # fn main() { }
/// # use zero_ui::core::{widget, UiNode};
/// # use zero_ui::properties::capture_only::widget_child;
/// widget! {
///     pub container;
///     
///     default_child {
///         content -> widget_child: required!;
///     }
///     
///     fn new_child(content) -> impl UiNode {
///         content.unwrap()
///     }
/// }
/// ```
///
/// The function must return a type that implements [`UiNode`](zero_ui::core::UiNode). It has no required arguments but
/// can [capture](#property-capturing) property arguments.
///
/// If omitted the left-most inherited widget `new_child` is used, if the widget only inherits from mix-ins
/// [`default_widget_new_child`](zero_ui::core::default_widget_new_child) is used.
///
/// ## `new`
///
/// Initializes the outer wrapper of the widget.
///
/// ```
/// # fn main() { }
/// # use zero_ui::core::{widget, color::rgb, var::IntoVar, WidgetId, text::Text, color::Rgba};
/// # use zero_ui::properties::title;
/// # use zero_ui::properties::background::background_color;
/// # use zero_ui::widgets::container;
/// # pub struct Window { } impl Window { pub fn new(child: impl zero_ui::core::UiNode, id: impl IntoVar<WidgetId>, title: impl IntoVar<Text>, background_color: impl IntoVar<Rgba>) -> Self { todo!() } }
/// widget! {
///     pub window: container;
///     
///     default {
///         title: "New Window";
///         background_color: rgb(1.0, 1.0, 1.0);
///     }
///     
///     fn new(child, id, title, background_color) -> Window {
///         Window::new(child, id.unwrap(), title.unwrap(), background_color.unwrap())
///     }
/// }
/// ```
///
/// The function can return any type, but if the type does not implement [`Widget`](zero_ui::core::Widget)
/// it cannot be the content of most other container widgets.
///
/// The first argument is required, it can have any name but the type is `impl UiNode`,
/// it contains the UI node tree formed by the widget properties and `new_child`.
/// After the first argument it can [capture](#property-capturing) property arguments.
///
/// If omitted the left-most inherited widget `new` is used, if the widget only inherits from mix-ins
/// [`default_widget_new`](zero_ui::core::default_widget_new) is used.
///
/// ## Property Capturing
///
/// The initialization functions can capture properties by listing then in the function input. The argument type is an `impl property_name::Args`.
///
/// Captured properties are not applied during widget instantiation, the arguments are moved to the function that captured then.
/// Because they are required for calling the initialization functions they are automatically marked 'required'.
///
/// # Internals
///
/// TODO details of internal code generated.
pub use zero_ui_macros::widget;

/// Declares a new widget mix-in module.
///
/// Widget mix-ins can be inherited by other mix-ins and widgets, but cannot be instantiated.
///
/// # Syntax
///
/// The syntax is the same as in [`widget!`](macro.widget.html), except
/// you cannot write the `new` and `new_child` functions.
///
/// ```
/// # fn main() { }
/// # use zero_ui::prelude::new_widget::{widget_mixin, focusable, border, is_focused_hgl, foreground_highlight, SideOffsets};
/// # use zero_ui::widgets::mixins::{FocusHighlightDetailsVar, FocusHighlightWidthsVar, FocusHighlightOffsetsVar};
/// widget_mixin! {
///     /// Focusable widget mix-in. Enables keyboard focusing on the widget and adds a focused
///     /// highlight border.
///     pub focusable_mixin;
///
///     default {
///
///         /// Enables keyboard focusing in the widget.
///         focusable: true;
///
///         /// A border overlay that is visible when the widget is focused.
///         focus_highlight -> foreground_highlight: {
///             widths: SideOffsets::new_all(0.0),
///             offsets: SideOffsets::new_all(0.0),
///             details: FocusHighlightDetailsVar
///         };
///     }
///
///     when self.is_focused_hgl {
///         focus_highlight: {
///             widths: FocusHighlightWidthsVar,
///             offsets: FocusHighlightOffsetsVar,
///             details: FocusHighlightDetailsVar
///         };
///     }
/// }
/// ```
///
/// # Expands to
///
/// The macro expands to a module declaration with the same name and visibility.
///
/// All documentation is incorporated into specially formatted HTML that uses the
/// rust-doc stylesheets to present the widget mix-in as a first class item. See
/// [`focusable_mixin`](mod@zero_ui::widgets::mixins::focusable_mixin) for an example.
///
/// ## Internals
///
/// In the generated module some public but doc-hidden items are generated, this items
/// are used during widget instantiation.
pub use zero_ui_macros::widget_mixin;

/// Creates a [`UiVec`](zero_ui::core::UiVec) containing the arguments.
///
/// # Example
///
/// ```
/// # use zero_ui::core::ui_vec;
/// # use zero_ui::widgets::text::text;
/// let widgets = ui_vec![
///     text("Hello"),
///     text("World!")
/// ];
/// ```
/// `ui_vec!` automatically boxes each widget.
pub use zero_ui_macros::ui_vec;
