#![warn(unused_extern_crates)]
// examples of `widget! { .. }` and `#[property(..)]` need to be declared
// outside the main function, because they generate a `mod` with `use super::*;`
// that does not import `use` clauses declared inside the parent function.
#![allow(clippy::needless_doctest_main)]

//! Core infrastructure required for creating components and running an app.

#[macro_use]
extern crate bitflags;

// to make the proc-macro $crate substitute work in doc-tests.
#[doc(hidden)]
#[allow(unused_extern_crates)]
extern crate self as zero_ui_core;

#[macro_use]
mod crate_macros;

#[doc(hidden)]
pub use paste::paste;

pub mod animation;
pub mod app;
pub mod color;
pub mod context;
pub mod debug;
pub mod event;
pub mod focus;
pub mod gesture;
pub mod gradient;
pub mod keyboard;
pub mod mouse;
pub mod profiler;
pub mod render;
pub mod service;
pub mod sync;
pub mod text;
pub mod units;
pub mod var;
pub mod widget_base;
pub mod window;

mod ui_node;
pub use ui_node::*;

mod ui_list;
pub use ui_list::*;

// proc-macros used internally during widget creation.
#[doc(hidden)]
pub use zero_ui_proc_macros::{widget_declare, widget_inherit, widget_new, widget_stage2, widget_stage3};

/// Gets if the value indicates that any size is available during layout (positive infinity)
#[inline]
pub fn is_layout_any_size(f: f32) -> bool {
    f.is_infinite() && f.is_sign_positive()
}

/// Value that indicates that any size is available during layout.
pub const LAYOUT_ANY_SIZE: f32 = f32::INFINITY;

/// A map of TypeId -> Box<dyn Any>.
type AnyMap = fnv::FnvHashMap<std::any::TypeId, Box<dyn std::any::Any>>;

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
/// Normal properties must take at least two arguments, the first argument is the child [`UiNode`](crate::UiNode), the other argument(s)
/// are the property values. The function must return a type that implements `UiNode`. The first argument must support any type that implements
/// `UiNode`. All of these requirements are validated at compile time.
///
/// ```
/// # fn main() { }
/// use zero_ui_core::{property, UiNode, impl_ui_node, var::{Var, IntoVar}, context::WidgetContext};
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
/// # use zero_ui_core::{property, var::IntoVar, text::Text};
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
/// * `is_` prefix: Can only take a single [`StateVar`](crate::var::StateVar) value.
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
/// You can easily implement this properties using [`with_context_var`](crate::properties::with_context_var)
/// and [`set_widget_state`](crate::properties::set_widget_state).
///
/// ## `event`
///
/// Event properties are the next priority, they are set after all others except `context`, this way events can be configured by the
/// widget context properties but also have access to the widget visual they contain.
///
/// It is strongly encouraged that the event handler signature matches the one from [`on_event`](crate::properties::events::on_event).
///
/// ## `outer`
///
/// Properties that shape the visual outside of the widget, the [`margin`](crate::properties::margin) property is an example.
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
pub use zero_ui_proc_macros::property;

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
/// # use zero_ui_core::widget;
/// # // use zero_ui_core::widgets::{container, mixins::focusable_mixin};
/// widget! {
///     /// Widget documentation.
///     pub button;//: container + focusable_mixin;
/// }
/// ```
///
/// ## Attributes
///
/// All attributes are transferred to the generated module. Conditional compilation (`#[cfg]`) attributes are also applied to the generated macro.
/// Extra documentation about the widget properties is auto-generated and added to the module as well.
///
/// ```
/// # use zero_ui_core::widget;
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
/// # use zero_ui_core::widget;
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
/// # use zero_ui_core::widget;
/// widget! {
///     pub foo;
/// }
/// ```
///
/// Widgets inheritance works by 'importing' all properties, when blocks and init functions into the new widget.
/// All widgets automatically inherit from [`implicit_mixin`](mod@crate::widgets::mixins::implicit_mixin) (after all other inherits).
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
/// # use zero_ui_core::widget;
/// widget! {
///     pub foo;
///
///     default {
///         enabled: false;
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
/// [`property`](crate::property) module is with the same name is imported.
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
/// # use zero_ui_core::{widget, property, UiNode, var::IntoVar};
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
/// # use zero_ui_core::{widget, property, UiNode, var::IntoVar, text::Text};
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
/// # use zero_ui_core::widget;
/// # use zero_ui_core::widget_base::enabled as on_click;
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
/// if the referenced values are [vars](crate::var::Var).
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::{widget, UiNode, property, color::{rgb, Rgba}, var::{IntoVar, StateVar}};
/// # #[property(inner)] pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode { child }
/// # #[property(context)] pub fn is_pressed(child: impl UiNode, state: StateVar) -> impl UiNode { child }
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
/// # use zero_ui_core::{widget, UiNode, property, color::{rgb, Rgba}, text::Text, var::{IntoVar, StateVar}};
/// # #[property(inner)] pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode { child }
/// # #[property(inner)] pub fn title(child: impl UiNode, title: impl IntoVar<Text>) -> impl UiNode { child }
/// # #[property(context)] pub fn is_pressed(child: impl UiNode, state: StateVar) -> impl UiNode { child }
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
/// If the property arguments are [vars](crate::var::Var) the when condition is reevaluated after any variable changes.
///
/// The referenced properties must have a default value, be [`required`](#required) or be a [state property](crate::property#state-probing).
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
/// # use zero_ui_core::{widget, UiNode};
/// // widget! {
/// //     pub container;
/// //     
/// //     default_child {
/// //         content -> widget_child: required!;
/// //     }
/// //     
/// //     fn new_child(content) -> impl UiNode {
/// //         content.unwrap()
/// //     }
/// // }
/// ```
///
/// The function must return a type that implements [`UiNode`](crate::UiNode). It has no required arguments but
/// can [capture](#property-capturing) property arguments.
///
/// If omitted the left-most inherited widget `new_child` is used, if the widget only inherits from mix-ins
/// [`default_widget_new_child`](crate::default_widget_new_child) is used.
///
/// ## `new`
///
/// Initializes the outer wrapper of the widget.
///
/// ```
/// # fn main() { }
/// # // use zero_ui_core::{widget, color::rgb, var::IntoVar, WidgetId, text::Text, color::Rgba};
/// # // use zero_ui_core::properties::title;
/// # // use zero_ui_core::properties::background::background_color;
/// # // use zero_ui_core::widgets::container;
/// # // pub struct Window { } impl Window { pub fn new(child: impl crate::UiNode, id: impl IntoVar<WidgetId>, title: impl IntoVar<Text>, background_color: impl IntoVar<Rgba>) -> Self { todo!() } }
/// // widget! {
/// //     pub window: container;
/// //     
/// //     default {
/// //         title: "New Window";
/// //         background_color: rgb(1.0, 1.0, 1.0);
/// //     }
/// //     
/// //     fn new(child, id, title, background_color) -> Window {
/// //         Window::new(child, id.unwrap(), title.unwrap(), background_color.unwrap())
/// //     }
/// // }
/// ```
///
/// The function can return any type, but if the type does not implement [`Widget`](crate::Widget)
/// it cannot be the content of most other container widgets.
///
/// The first argument is required, it can have any name but the type is `impl UiNode`,
/// it contains the UI node tree formed by the widget properties and `new_child`.
/// After the first argument it can [capture](#property-capturing) property arguments.
///
/// If omitted the left-most inherited widget `new` is used, if the widget only inherits from mix-ins
/// [`default_widget_new`](crate::default_widget_new) is used.
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
pub use zero_ui_proc_macros::widget;

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
/// # // fn main() { }
/// # // use zero_ui_core::prelude::new_widget::{widget_mixin, focusable, border, is_focused_hgl, foreground_highlight, SideOffsets};
/// # // use zero_ui_core::widgets::mixins::{FocusHighlightDetailsVar, FocusHighlightWidthsVar, FocusHighlightOffsetsVar};
/// // widget_mixin! {
/// //     /// Focusable widget mix-in. Enables keyboard focusing on the widget and adds a focused
/// //     /// highlight border.
/// //     pub focusable_mixin;
/// //
/// //     default {
/// //
/// //         /// Enables keyboard focusing in the widget.
/// //         focusable: true;
/// //
/// //         /// A border overlay that is visible when the widget is focused.
/// //         focus_highlight -> foreground_highlight: {
/// //             widths: SideOffsets::new_all(0.0),
/// //             offsets: SideOffsets::new_all(0.0),
/// //             details: FocusHighlightDetailsVar
/// //         };
/// //     }
/// //
/// //     when self.is_focused_hgl {
/// //         focus_highlight: {
/// //             widths: FocusHighlightWidthsVar,
/// //             offsets: FocusHighlightOffsetsVar,
/// //             details: FocusHighlightDetailsVar
/// //         };
/// //     }
/// // }
/// ```
///
/// # Expands to
///
/// The macro expands to a module declaration with the same name and visibility.
///
/// All documentation is incorporated into specially formatted HTML that uses the
/// rust-doc stylesheets to present the widget mix-in as a first class item. See
/// [`focusable_mixin`](mod@crate::widgets::mixins::focusable_mixin) for an example.
///
/// ## Internals
///
/// In the generated module some public but doc-hidden items are generated, this items
/// are used during widget instantiation.
pub use zero_ui_proc_macros::widget_mixin;

pub use zero_ui_proc_macros::widget2;
pub use zero_ui_proc_macros::widget_mixin2;
