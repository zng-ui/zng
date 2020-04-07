extern crate proc_macro;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use proc_macro_hack::proc_macro_hack;

#[macro_use]
mod util;

mod impl_ui_node;
mod property;
pub(crate) mod widget;
pub(crate) mod widget_new;

use widget::CallKind;

/// Generates default implementations of [`UiNode`](zero_ui::core::UiNode) methods.
///
/// # Arguments
///
/// The macro attribute takes arguments that indicate how the missing methods will be generated.
///
/// * `delegate: {&}, delegate_mut: {&mut}`.
///
/// * `delegate_iter: {Iterator<&>}, delegate_iter_mut: {Iterator<&mut>}`.
///
/// You can also use shorthand for common delegation:
///
/// * `child` is the same as `delegate: &self.child, delegate_mut: &mut self.child`.
///
/// * `children` is the same as `delegate_iter: self.children.iter(), delegate_iter_mut: self.children.iter_mut()`.
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
///
/// ### Expands to
///
/// ```
/// impl<C: LocalVar<ColorF>> NoneDelegateSample<C> {
///     pub fn new(color: C) -> Self {
///          FillColor { color: final_size: LayoutSize::zero() }
///     }
/// }
///
/// impl<C: LocalVar<ColorF>> zero_ui::core::UiNode for NoneDelegateSample<C> {
///
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
///         if size.width.is_infinite() {
///             size.width = 0.0;
///         }
///         if size.height.is_infinite() {
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
///
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
/// ### Expands to
///
/// ```
/// impl<C: UiNode> UiNode ChildDelegateSample<C> {
///
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
///
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
/// ### Expands to
///
/// ```
/// impl<C: UiNode> UiNode ChildrenDelegateSample<C> {
///
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
///
/// }
/// ```
#[proc_macro_attribute]
pub fn impl_ui_node(args: TokenStream, input: TokenStream) -> TokenStream {
    impl_ui_node::gen_impl_ui_node(args, input)
}

/// Declares a new widget property.
///
/// # Argument
///
/// The macro attribute takes one argument that indicates what is the priority of applying the property in a widget.
///
/// The priorities by outermost first:
///
/// * `context`: The property setups some widget context metadata. It is applied around all other properties of the widget.
/// * `event`: The property is an event handler. It is applied inside the widget context but around all other properties.
/// * `outer`: The property does something visual around the widget, like a margin or border.
/// It is applied around the core visual properties of the widget.
/// * `size`: The property defines the size boundary of the widget.
/// * `inner`: The property does something visual inside the widget, like fill color.
/// It is applied inside all other properties of the widget.
///
/// # Usage
///
/// Annotate a standalone function to transform it into a property module.
///
/// The function must take at least two arguments and return the property [node](zero_ui::core::UiNode). The first
/// argument must be the property child node, the other arguments the property values.
///
/// It is recommended that the property values have the type [`IntoVar<T>`](zero_ui::core::var::IntoVar) and the
/// property node supports [`Var<T>`](zero_ui::core::var::Var) updates.
///
/// # Expands to
///
/// TODO
#[proc_macro_attribute]
pub fn property(args: TokenStream, input: TokenStream) -> TokenStream {
    property::expand_property(args, input)
}

/// Declares a new widget macro and module.
///
/// Widgets are a preset of properties with optional custom initialization.
///
/// Things that can defined in widgets:
///
/// * _**Default Properties:**_ If the user of your widget does not set the property the default
/// value is used.
///
/// * _**New Properties:**_ New properties that internally map to another property. New property names
/// do not override their internal property, allowing the user to set both.
///
/// * _**Required Properties:**_ Setting a property with `required!` forces the user to set
/// the property during use.
///
/// * _**Conditional Properties:**_ You can use `when` blocks to conditionally set properties.
///
/// * _**Retargeted Properties:**_ Usually properties apply according to their priority, widgets can
/// define that some properties are applied early.
///
/// # Syntax
///
/// Widgets start with a declaration of the visibility and main documentation.
///
/// ```
/// widget! {
///     /// Widget documentation.
///     pub button;
/// }
/// ```
///
/// The example code declares a public widget named `button`.
///
/// ## Inheritance
///
/// Widgets can include properties from other widgets and widget mix-ins.
///
/// ```
/// widget! {
///     pub button: container + focusable_mixin;
/// }
/// ```
///
/// The example core declares a widget that inherits the properties from the
/// `container` widget and `focusable_mixin` mix-in.
///
/// Properties are inherited left-to-right so `container` first then `focusable_mixin` on-top. Properties
/// with the same name get overridden.
///
/// All widgets also inherit from [`implicit_mixin`](zero_ui::widgets::implicit_mixin) before all other inherits.
///
/// ## Properties
///
/// TODO
///
/// ## Early Properties
///
/// TODO
///
/// ## Conditional Properties
///
/// TODO
///
/// ## Special Property Values
///
/// TODO
///
/// ## Custom Initialization
///
/// TODO
///
/// # Expands to
///
/// The macro expands to a module declaration with the same name and visibility, and a doc-hidden
/// `macro_rules!` macro of the same name. If the widget is `pub` the new macro is `#[macro_export]`.
///
/// In the generated module you can find the two functions `new` and `new_child`, they are used automatically
/// when the widget is instantiated but you can also call then manually. Manual calls can be used to include
/// inherited widgets custom initialization.
///
/// ## Internals
///
/// In the generated module some public but doc-hidden items are generated, this items
/// are used during widget instantiation.
///
/// ## Why a macro/mod pair
///
/// When [Macros 2.0](https://github.com/rust-lang/rust/issues/39412) is stable this will change, but
/// for now the macro and module pair simulate macro namespaces, you import all macros from the widgets
/// crate at the start:
/// ```
/// #[macro_use]
/// extern crate widget_crate;
/// ```
/// but the widget macros use the short path to the module so you still need to write:
/// ```
/// use widget_crate::widgets::button;
/// ```
#[proc_macro]
pub fn widget(input: TokenStream) -> TokenStream {
    widget::expand_widget(CallKind::Widget, input)
}

/// Declares a new widget mix-in module.
///
/// Widget mix-ins can be inherited by other mix-ins and widgets but cannot be instantiated.
///
/// # Syntax
///
/// The syntax is the same as in [`widget!`](../zero_ui/macro.widget.html), except
/// you cannot write the `new` and `new_child` functions.
///
/// ```
/// widget_mixin! {
///     /// Mix-in documentation.
///     ///
///     /// * By convention mix-in names have a suffix `_mixin`.
///     /// * `pub` is optional and all visibility modifiers are supported.
///     /// * The mix-in can inherit from others by using `.._mixin: other + another;`.
///     pub focusable_mixin;
///
///     // Properties that are applied to the widget that inherits the mix-in.
///     //
///     // Multiple `default` and `default_child` blocks can be declared,
///     // see the `widget!` macro for more details about property targets.
///     default {
///
///         /// Documentation for this property in this widget mix-in.
///         //
///         // See the `widget!` macro documentation for more
///         // details about how this properties can be declared and set.
///         focusable: true;
///
///         // New property names can be declared for the widget only, They use another property
///         // internally but don't override the other property.
///         //
///         // In this case `focused_border` is a new property for widgets that
///         // inherit `focusable_mixin`, but `border` can also be set for the same widgets.
///         focused_border -> border: {
///             widths: LayoutSideOffsets::new_all_same(0.0),
///             details: FocusedBorderDetails
///         };
///     }
///
///     // Conditional property values that are applied to the widget that inherits the mix-in.
///     //
///     // Multiple `when` blocks can be declared. See the `widget!` macro documentation for more
///     // details about conditional property values.
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
    widget::expand_widget(CallKind::Mixin, input)
}

/// Macro used by [`widget!`](widget).
#[proc_macro]
pub fn widget_inherit(input: TokenStream) -> TokenStream {
    widget::expand_widget(CallKind::Inherit, input)
}

/// Macro used by [`widget_mixin!`](widget_mixin).
#[proc_macro]
pub fn widget_mixin_inherit(input: TokenStream) -> TokenStream {
    widget::expand_widget(CallKind::MixinInherit, input)
}

/// Macro used by macros generated by [`widget!`](widget).
#[doc(hidden)]
#[proc_macro_hack]
pub fn widget_new(input: TokenStream) -> TokenStream {
    widget_new::expand_widget_new(input)
}
