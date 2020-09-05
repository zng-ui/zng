extern crate proc_macro;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

#[macro_use]
mod util;

mod hex_color;
mod impl_ui_node;
pub(crate) mod property;
pub(crate) mod widget_new;
mod widget_stage1;
mod widget_stage2;
pub(crate) mod widget_stage3;

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
/// struct FillColor<C> {
///     color: C,
///     final_size: LayoutSize,
/// }
/// ```
///
/// In an `UiNode` trait impl block, annotate the impl block with `#[impl_ui_node(..)]` and only implement custom methods.
///
/// ```
/// #[impl_ui_node(none)]
/// impl<C: Var<ColorF>> UiNode for FillColor<C> {
///     fn render(&self, f: &mut FrameBuilder) {
///         frame.push_color(LayoutRect::from_size(self.final_size), *self.color.get_local());
///     }
/// }
/// ```
///
/// Or, in a inherent impl, annotate the impl block with `#[impl_ui_node(..)]` and custom `UiNode` methods with `#[UiNode]`.
///
/// ```
/// #[impl_ui_node(none)]
/// impl<C: LocalVar<ColorF>> FillColor<C> {
///     pub fn new(color: C) -> Self {
///         FillColor { color: final_size: LayoutSize::zero() }
///     }
///
///     #[UiNode]
///     fn render(&self, frame: &mut FrameBuilder) {
///         frame.push_color(LayoutRect::from_size(self.final_size), *self.color.get_local());
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
/// #[impl_ui_node(none)]
/// impl<C: LocalVar<ColorF>> NoneDelegateSample<C> {
///     pub fn new(color: C) -> Self {
///          FillColor { color: final_size: LayoutSize::zero() }
///     }
/// }
/// ```
/// Expands to:
///
/// ```
/// impl<C: LocalVar<ColorF>> NoneDelegateSample<C> {
///     pub fn new(color: C) -> Self {
///          FillColor { color: final_size: LayoutSize::zero() }
///     }
/// }
///
/// impl<C: LocalVar<ColorF>> zero_ui::core::UiNode for NoneDelegateSample<C> {
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
///     fn measure(&mut self, available_size: zero_ui::core::types::LayoutSize) -> zero_ui::core::types::LayoutSize {
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
///     fn arrange(&mut self, final_size: zero_ui::core::types::LayoutSize) { }
///
///     #[inline]
///     fn render(&self, frame: &mut zero_ui::core::render::FrameBuilder) { }
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
/// #[impl_ui_node(child)]
/// impl<C: UiNode> UiNode for ChildDelegateSample<C> { }
/// ```
///
/// Expands to:
///
/// ```
/// impl<C: UiNode> UiNode ChildDelegateSample<C> {
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
///     fn measure(&mut self, available_size: zero_ui::core::types::LayoutSize) -> zero_ui::core::types::LayoutSize {
///         let child = { &mut self.child };
///         child.measure(available_size)
///     }
///
///     #[inline]
///     fn arrange(&mut self, final_size: zero_ui::core::types::LayoutSize) {
///         let child = { &mut self.child };
///         child.arrange(final_size)
///     }
///
///     #[inline]
///     fn render(&self, frame: &mut zero_ui::core::render::FrameBuilder) {
///         let child = { &self.child };
///         child.render(frame)
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
/// #[impl_ui_node(children)]
/// impl<C: UiNode> UiNode for ChildrenDelegateSample<C> { }
/// ```
///
/// Expands to:
///
/// ```
/// impl<C: UiNode> UiNode ChildrenDelegateSample<C> {
///     #[inline]
///     fn init(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         for child in { self.child.iter_mut() } {
///             child.init(ctx)
///         }
///     }
///
///     #[inline]
///     fn update(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         for child in { self.child.iter_mut() } {
///             child.update(ctx)
///         }
///     }
///
///     #[inline]
///     fn update_hp(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         for child in { self.child.iter_mut() } {
///             child.update_hp(ctx)
///         }
///     }
///
///     #[inline]
///     fn measure(&mut self, available_size: zero_ui::core::types::LayoutSize) -> zero_ui::core::types::LayoutSize {
///         let mut size = Default::default();
///         for child in { self.child.iter_mut() } {
///            size = child.measure(available_size).max(size);
///         }
///         size
///     }
///
///     #[inline]
///     fn arrange(&mut self, final_size: zero_ui::core::types::LayoutSize) {
///         for child in { self.child.iter_mut() } {
///             child.arrange(final_size)
///         }
///     }
///
///     #[inline]
///     fn render(&self, frame: &mut zero_ui::core::render::FrameBuilder) {
///         for child in { self.child.iter() } {
///             child.render(frame)
///         }
///     }
///
///     #[inline]
///     fn deinit(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         for child in { self.child.iter_mut() } {
///             child.deinit(ctx)
///         }
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn impl_ui_node(args: TokenStream, input: TokenStream) -> TokenStream {
    impl_ui_node::gen_impl_ui_node(args, input)
}

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
/// use zero_ui::core::{property, UiNode, impl_ui_node, var::Var, context::WidgetContext};
///
/// struct MyNode<C, V> { child: C, value: V }
/// #[impl_ui_node(child)]
/// impl<C: UiNode, V: Var<&'static str>> UiNode for MyNode<C, V> {
///     fn init(&self, ctx: &mut WidgetContext) {
///         self.child.init(ctx);
///         println!("{}", self.value.get(ctx.vars));
///     }
/// }
///
/// /// Property docs.
/// #[property(context)]
/// pub fn my_property(child: impl UiNode, value: impl Var<&'static str>) -> impl UiNode {
///     MyNode { child, value }
/// }
/// ```
///
/// ### `capture_only`
///
/// Capture-only properties do not modify the UI node tree, they exist only as a named bundle of arguments that widgets capture to use internally.
/// At least one argument is required. The return type must be never (`!`) and the property body must be empty.
///
/// ```
/// use zero_ui::core::{property, var::Var};
///
/// /// Property docs.
/// #[property(capture_only)]
/// pub fn my_property(value: impl Var<&'static str>) -> ! { }
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
/// * `is_` prefix: Can only take a single [`IsStateVar`](zero_ui::core::var::IsStateVar) value.
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
/// and [`set_widget_state`].
///
/// ## `event`
///
/// Event properties are the next priority, they are set after all others except `context`, this way events can be configured by the
/// widget context properties but also have access to the widget visual they contain.
///
/// It is strongly encouraged that the event handler be an [`FnMut`] with [`OnEventArgs`](zero_ui::properties::OnEventArgs) input.
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
#[proc_macro_attribute]
pub fn property(args: TokenStream, input: TokenStream) -> TokenStream {
    property::expand(args, input)
}

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
/// widget! {
///     /// Widget documentation.
///     #[foo(bar)]
///     widget_name;
/// }
/// ```
///
/// ## Visibility
///
/// The visibility is transferred to the widget module and macro and supports all visibility configurations.
///
/// ```
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
/// widget! {
///     pub foo: container;
/// }
///
/// widget! {
///     pub bar: foo + widgets::focusable_mixin;
/// }
/// ```
///
/// Widgets inheritance works by 'importing' all properties, when blocks and init functions into the new widget.
/// All widgets automatically inherit from [`implicit_mixin`](zero_ui::widgets::mixins::implicit_mixin) (after all other inherits).
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
/// widget! {
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
/// widget! {
///     default {
///         new_property: "value";
///         foo -> bar: "value";
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
/// widget! {
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
/// widget! {
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
/// widget! {
///     when self.foo.argument == "value" && self.is_state {
///         bar: "foo is value";
///     }
/// }
/// ```
///
/// If the property arguments are [vars](zero_ui::core::var::Var) the when condition is reevaluated after any variable changes.
///
/// The referenced properties must have a default value, be [`required`](#required) or be a [state property](#zero_ui::core::property#state-probing).
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
/// widget! {
///     pub window;
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
#[proc_macro]
pub fn widget(input: TokenStream) -> TokenStream {
    widget_stage1::expand(false, input)
}

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
/// widget_mixin! {
///     /// Mix-in documentation.
///     pub focusable_mixin;
///
///     default {
///         /// Documentation for this property in this widget mix-in.
///         focusable: true;
///
///         focused_border -> border: {
///             widths: LayoutSideOffsets::new_all_same(0.0),
///             details: FocusedBorderDetails
///         };
///     }
///
///     when self.is_focused {
///         focused_border: {
///             widths: FocusedBorderWidths,
///             details: FocusedBorderDetails
///         };
///     }
///
///     // Unlike `widget!`, the custom `new` and `new_child` functions are not permitted here.
/// }
/// ```
///
/// # Expands to
///
/// The macro expands to a module declaration with the same name and visibility.
///
/// All documentation is incorporated into specially formatted HTML that uses the
/// rust-doc stylesheets to present the widget mix-in as a first class item. See
/// [`focusable_mixin`](zero_ui::widgets::focusable_mixin) for an example.
///
/// ## Internals
///
/// In the generated module some public but doc-hidden items are generated, this items
/// are used during widget instantiation.
#[proc_macro]
pub fn widget_mixin(input: TokenStream) -> TokenStream {
    widget_stage1::expand(true, input)
}

/// Recursive include inherited tokens. Called by the expansion of widget_state1 and widget_stage2.
#[proc_macro]
pub fn widget_stage2(input: TokenStream) -> TokenStream {
    widget_stage2::expand(input)
}

/// Final widget or mix-in expansion. Called by the final expansion of widget_stage2.
#[proc_macro]
pub fn widget_stage3(input: TokenStream) -> TokenStream {
    widget_stage3::expand(input)
}

/// Instantiate widgets. Called by widget macros generated by [`widget!`](widget).
#[proc_macro]
pub fn widget_new(input: TokenStream) -> TokenStream {
    widget_new::expand(input)
}

/// Hexadecimal color initialization.
///
/// # Syntax
///
/// `[#|0x]RRGGBB[AA]` or `[#|0x]RGB[A]`.
///
/// An optional prefix `#` or `0x` is supported, after the prefix a hexadecimal integer literal is expected. The literal can be
/// separated using `_`. No integer type suffix is allowed.
///
/// The literal is a sequence of 3 or 4 bytes (red, green, blue and alpha). If the sequence is in pairs each pair is a byte `[00..=FF]`.
/// If the sequence is in single characters this is a shorthand that repeats the character for each byte, e.g. `#012F` equals `#001122FF`.
///
/// # Examples
///
/// ```
/// # use zero_ui::core::color::hex_color;
/// let red = hex_color!(#FF0000);
/// let green = hex_color!(#00FF00);
/// let blue = hex_color!(#0000FF);
/// let red_half_transparent = hex_color!(#FF00007F);
///
/// assert_eq!(red, hex_color!(#F00));
/// assert_eq!(red, hex_color!(0xFF_00_00));
/// assert_eq!(red, hex_color!(FF_00_00));
/// ```
///
#[proc_macro]
pub fn hex_color(input: TokenStream) -> TokenStream {
    hex_color::expand(input)
}
