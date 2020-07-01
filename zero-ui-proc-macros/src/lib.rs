extern crate proc_macro;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use proc_macro_hack::proc_macro_hack;

#[macro_use]
mod util;

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
/// The macro replaces the function with a module with the same name, documentation and visibility. The module contains
/// two public function `set` that is the original function and `args` that packs the property values into a single unit.
///
/// It also contains various traits for representing the arguments packed in a single unit:
///
/// * `ArgsNamed`: Contains a method named after each argument, the methods return references.
/// * `ArgsNumbered`: Contains one method for each argument named after their position (`arg0 .. argN`). The methods return references.
/// * `ArgsUnwrap`: Unwraps the arguments into a tuple with each argument in their original position.
/// * `Args`: Implements all other arg traits, can be constructed by calling `property::args(..)`.
///
/// ## Internals
///
/// The generated module also includes some public but doc-hidden items, they are used during widget initialization to
/// support named property arguments and implement the property priorities sorting.
///
/// # Naming Convention
///
/// Most properties are named after what they add to the widget. A widget has a `margin` or is `enabled`. Some properties
/// have a special prefix, `on_` for events and `is_` for widget state probing.
///
/// ## Event Listeners
///
/// Properties that setup an event listener are named `on_<event_name>` and take a direct closure (not a var). It is
/// recommended that you use [`on_event`](zero_ui::properties::on_event) to implement event properties. If not possible
/// try to use [`OnEventArgs`](zero_ui::properties::OnEventArgs) at least.
///
/// ```
/// #[property(event)]
/// pub fn on_key_input(
///     child: impl UiNode,
///     handler: impl FnMut(&mut OnEventArgs<KeyInputArgs>) + 'static
/// ) -> impl UiNode {
///     on_event(child, KeyInput, handler)
/// }
/// ```
///
/// ## State Probing
///
/// Properties that provide a `bool` value when the widget is in a state are named `is_<state_name>` and take a `bool` var.
///
/// ```
/// #[property(context)]
/// pub fn is_pressed(child: impl UiNode, state: impl Var<bool>) -> impl UiNode {
///     IsPressed {
///         ..
///     }
/// }
/// ```
/// Normal boolean properties are not named with the `is_` prefix. For example `enabled` and `clip_to_bounds` are boolean
/// but they react to the user value not the other way around.
///
/// ## Context Variables
///
/// Every public [`ContextVar`](zero_ui::core::var::ContextVar) must have a property that sets the var, they use the same
/// name of the variable in snake_case. You can use [`with_context_var`](zero_ui::properties::with_context_var) to implement
/// then.
///
/// ```
/// #[property(context)]
/// pub fn font_size(child: impl UiNode, size: impl IntoVar<u32>) -> impl UiNode {
///     with_context_var(child, FontSize, size)
/// }
/// ```
#[proc_macro_attribute]
pub fn property(args: TokenStream, input: TokenStream) -> TokenStream {
    //property_old::expand_property(args, input)
    property::expand(args, input)
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
/// * _**Custom Initialization:**_ Each widget can have two functions `new_child` and `new`. This functions receive and return
/// `UiNodes` and can also capture property values and use then in customized ways.
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
/// Properties can be set within three different blocks, `default {}`, `default_child {}` and `when $expr {}`.
///
/// Properties in the `default_child` blocks are applied directly to the widget child before all others, those in the
/// `default` blocks are applied to the `new_child` function result and those in the `when` blocks change the other
/// properties when the condition is true.
///
/// ## Setting Properties
///
/// Properties can be set just like in a widget macro:
///
/// ```
/// widget! {
///     pub button;
///
///     default {
///         /// Documentation for this property in the widget.
///         background_color: rgb(0, 200, 0);
///         border: {
///             widths: 1.0,
///             details: rgb(0, 100, 0)
///         };
///     }
/// }
/// ```
/// The example code presets `background_color` and `border` for the widget. If users of `button!` don't set
/// this two properties, the preset values are used for every button.
///
/// Setting the same property name again overrides the previous value.
///
/// ## Special Property Values
///
/// Properties can also be set to `unset!` or `required!`.
///
/// ### `unset!`
///
/// Unset can be used to remove an inherited property preset.
///
/// ```
/// widget! {
///     pub button: container;
///
///     default_child {
///         content_align: unset!;
///     }
/// }
/// ```
/// The example code inherits `content_align` from `container`, it is preset to `Alignment::CENTER`. You could set it
/// to a different alignment but the property would still be used for every `button!` user, setting it to `unset!` makes
/// the property have no default value.
///
/// The `button!` users can still set it, but if they don't no property `UiNode` is inserted.
///
/// ### `required!`
///
/// Required properties don't have a preset value but widget users must set then.
///
/// ```
/// widget! {
///     pub button;
///
///     default {
///         on_click: required!;
///     }
/// }
/// ```
/// The example code requires `button!` users to set `on_click`, if they don't set it they get the
/// compile error ``"missing required property `on_click`"``.
///
/// ##  New Properties
///
/// New property names can be defined in the context of the widget, they use the implementation of another property
/// but have a special name in the widget.
///
/// ```
/// widget! {
///     pub container;
///
///     default_child {
///         padding -> margin;
///         content_align -> align: Alignment::CENTER;
///     }
/// }
/// ```
/// The example code defines two new properties, `padding` and `content_align`. The two properties
/// can be set by users of the `container!` widget.
///
/// The `padding` property has no default value, but if the users of `container!` set it a `margin` is applied
/// (to the container child in this case).
///
/// The `content_align` property has a default value, so `align` is applied automatically but can be
///  overridden by `container!` users.
///
/// New properties are not aliases, users of `container!` can set both `padding` and `margin` and both are applied.
///
/// ## Conditional Properties
///
/// Properties can be conditionally set using `when` blocks.
///
/// ```
/// widget! {
///     pub button;
///
///     default {
///         background_color: rgb(50, 50, 50);
///     }
///
///     when self.is_mouse_over {
///         background_color: rgb(70, 70, 70);
///     }
/// }
/// ```
/// The example code changes the `background_color` property value every time `is_mouse_over` is `true`. Properties
/// can be set normally inside the `when` blocks and the condition expression can reference properties using
/// `self.property_name`.
///
/// The condition expression is like the `if` expression, supporting any expression that results in a `bool` value. If
/// you reference a property inside the expression the condition refreshes when the property changes.
///
/// All of the following are valid:
///
/// ```
/// when true { }
///
/// when self.is_state { }
///
/// when self.is_state && self.is_another_state { }
///
/// when self.property == "Value" { }
///
/// when some_fn(self.property) { }
/// ```
/// The only requirement is that the property has a value that implements [`Default`](Default) if you did not set it
/// in a `default` or `default_child` block.
///
/// ## Custom Initialization
///
/// All widgets have two functions, `new_child` and `new`, they have a default implementation but can be overridden by the widget.
///
/// ```
/// widget! {
///     pub my_widget;
///
///     default {
///         on_event: required!;
///     }
///
///     #[inline]
///     fn new_child(child) -> impl UiNode {
///         special::set(child, true)
///     }
///
///     /// Custom docs for `my_widget::new`.
///     #[inline]
///     fn new(child, id, on_event) -> MyWidget {
///         MyWidget {
///             child,
///             id: id.unwrap().0,
///             handler: on_event.unwrap().0
///         }
///     }
/// }
/// ```
/// The example code provides a custom definition for both functions. The functions need to have at least one
/// parameter and return a value, `new_child` return type must implement [`UiNode`](zero_ui::core::UiNode).
///
/// Both functions take at least one argument that is the child `UiNode`, followed by property captures. You don't write the argument types, the
/// first argument is `impl UiNode` the others are `impl <property>::Args`.
///
/// The initialization functions can capture a property by mentioning then in their args, in the example code `new` captures `id` and `on_event`. When
/// a property is captured they behave like a normal property from the widget user perspective, but the property is not actually set, the property arguments
/// are passed to the capturing function.
///
/// The first argument of `new_child` is the widget child wrapped in the widget child properties, for `new` it is the result of `new_child`
/// wrapped in all the widget properties. The return type of `new` does not need to implement `UiNode`.
///
/// ### Default
///
/// By default `new_child` calls [`default_widget_new_child`](zero_ui::core::default_widget_new_child) and `new` calls
/// [`default_widget_new`](zero_ui::core::default_widget_new).
///
/// ### Inheritance
///
/// Widgets do not inherit initialization, if you want to use the initialization of another widget you must call the `other::new` or `other::new_child`
/// functions manually inside the custom initialization for your widget.
///
/// ```
/// widget! {
///     pub my_window: window;
///
///     default {
///         my_property: 10;
///     }
///
///     fn new(child, root_id, title, size, background_color) -> Window {
///         println!("Initializing {:?}", root_id);
///         window::new(child, root_id, title, size, background_color)
///     }
/// }
/// ```
///
/// # Widget Expands To
///
/// The macro expands to a module declaration with the same name and visibility, and a doc-hidden
/// `macro_rules!` macro of the same name. If the widget is `pub` the new macro is `#[macro_export]`.
///
/// In the generated module you can find the two functions `new` and `new_child`, they are used automatically
/// when the widget is instantiated but you can also call then manually. Manual calls can be used to include
/// inherited widgets custom initialization.
///
/// All documentation is incorporated into specially formatted HTML that uses the
/// rust-doc stylesheets to present the widget as a first class item. See
/// [`window`](zero_ui::widgets::window) for an example.
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
    widget_stage1::expand(false, input)
    //widget::expand_widget(CallKind::Widget, input)
}

/// Declares a new widget mix-in module.
///
/// Widget mix-ins can be inherited by other mix-ins and widgets, but cannot be instantiated.
///
/// # Syntax
///
/// The syntax is the same as in [`widget!`](../zero_ui/macro.widget.html), except
/// you cannot write the `new` and `new_child` functions.
///
/// ```
/// widget_mixin! {
///     /// Mix-in documentation.
///     pub focusable_mixin;
///
///     default {
///
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

/// Recursive inherited tokens inclusion. Called by the expansion of widget_state1 and widget_stage2.
#[proc_macro]
pub fn widget_stage2(input: TokenStream) -> TokenStream {
    widget_stage2::expand(input)
}

/// Final widget or mixin expansion. Called by the final expansion of widget_stage2.
#[proc_macro]
pub fn widget_stage3(input: TokenStream) -> TokenStream {
    widget_stage3::expand(input)
}

/// Instantiate widgets. Is called by widget macros generated by [`widget!`](widget).
#[proc_macro_hack]
pub fn widget_new(input: TokenStream) -> TokenStream {
    widget_new::expand(input)
}

#[proc_macro_attribute]
pub fn delete(args: TokenStream, input: TokenStream) -> TokenStream {
    TokenStream::new()
}