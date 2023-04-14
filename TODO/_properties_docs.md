
/// 
///
/// Inside the widget module the `properties!` pseudo-macro can used to declare properties of the widget. The properties can
/// be assigned, renamed and exported as widget properties.
///
/// ```
/// # fn main() { }
/// use zero_ui_core::{*, widget_builder::*, widget_instance::*, var::*};
///
/// #[property(CONTEXT)]
/// pub fn bar(child: impl UiNode, val: impl IntoVar<bool>) -> impl UiNode {
///   let _ = val;
///   child
/// }
///
/// #[widget($crate::foo)]
/// pub mod foo {
///     inherit!(super::widget_base::base);
///
///     properties! {
///         /// Baz property docs.
///         pub super::bar as baz = true;
///         // inherited property
///         enabled = false;
///     }
/// }
/// ```
///
/// The example above declares an widget that exports the property `baz`, it is also automatically set to `true` and it also
/// sets the inherited [`WidgetBase`] property `enabled` to `false`.
///
/// The property visibility controls if it is assignable in derived widgets or during widget instantiation, in the example above
/// if `baz` was not `pub` it would be set on the widget but it does not get a `baz` property accessible from outside. Inherited
/// visibility cannot be overridden, the `enabled` property is defined as `pub` in [`WidgetBase`] so it is still `pub` in the
/// widget, even though the value was changed.
///
/// You can also export properties without defining a value, the default assign is not required, the property is only instantiated
/// if it is assigned in the final widget instance, but by exporting the property it is available in the widget macro by name without
/// needing a `use` import.
///
/// ## Unset
///
/// If an inherited property is assigned a value you can *unset* this value by assigning the property with the special value `unset!`.
///
/// ```
/// # fn main() { }
/// use zero_ui_core::{*, widget_builder::*, widget_instance::*, var::*};
/// #
/// # #[property(CONTEXT)]
/// # pub fn baz(child: impl UiNode, val: impl IntoVar<bool>) -> impl UiNode {
/// #   let _ = val;
/// #   child
/// # }
/// #
/// # #[widget($crate::foo)]
/// # pub mod foo {
/// #     inherit!(super::widget_base::base);
/// #
/// #     properties! {
/// #         pub super::baz = true;
/// #     }
/// # }
/// #[widget($crate::bar)]
/// pub mod bar {
///     inherit!(crate::foo);
///     
///     properties! {
///         baz = unset!;
///     }
/// }
/// ```
///
/// In the example above the widget `bar` inherits the `foo` widget that defines and sets the `baz` property. Instances of the
/// `bar` property will not include an instance of `baz` because it was `unset!`. Note that this does not remove the property
/// the `bar` widget still exports the `baz` property, it just does not have a default value anymore.
///
/// An `unset!` assign also removes all `when` assigns to the same property, this is unlike normal assigns that just override the
/// *default* value of the property, merged with the `when` assigns.
///
/// ## Multiple Inputs
///
/// Some properties have multiple inputs, you can use a different syntax to assign each input by name or as a comma separated list.
/// In the example below the property `anb` has two inputs `a` and `b`, they are assigned by name in the `named` property and by
/// position in the `unnamed` property. Note that the order of inputs can be swapped in the named init.
///
/// ```
/// # fn main() { }
/// use zero_ui_core::{*, widget_builder::*, widget_instance::*, var::*};
///
/// #[property(CONTEXT)]
/// pub fn anb(child: impl UiNode, a: impl IntoVar<bool>, b: impl IntoVar<bool>) -> impl UiNode {
/// #   child
/// }
///
/// #[widget($crate::foo)]
/// pub mod foo {
///     inherit!(super::widget_base::base);
///
///     properties! {
///         pub super::anb as named = {
///             b: false,
///             a: true,
///         };
///         pub super::anb as unnamed = true, false;
///     }
/// }
/// ```
///
/// ## Generics
///
/// Some properties have named generics, the unnamed `impl` generics are inferred, but the named types must be defined using the *turbo-fish*
/// syntax if you want to set a value.
///
/// ```
/// # fn main() { }
/// use zero_ui_core::{*, widget_builder::*, widget_instance::*, var::*};
///
/// #[property(CONTEXT)]
/// pub fn value<T: VarValue>(child: impl UiNode, value: impl IntoVar<T>) -> impl UiNode {
/// #   child
/// }
///
/// #[widget($crate::foo)]
/// pub mod foo {
///     inherit!(super::widget_base::base);
///
///     properties! {
///         pub super::value::<bool> = true;
///     }
/// }
/// ```
///
/// Note that the property is not exported with the generics, the generic type must be specified in all other assigns, properties
/// are also only identified by their source and name, so an assign with different type still replaces the value.
///
/// ## Capture Only
///
/// Properties can be *captured* during build using the capture methods of [`WidgetBuilding`], captured properties are not
/// instantiated, the args are redirected to the intrinsic implementation of the widget. Every property can be captured, but
/// some properties are always intrinsic to the widget and cannot have a standalone implementation. You can declare *capture-only*
/// properties using the syntax `pub name(T)`, this expands to a [capture-only](property#capture-only) property that is
/// exported by the widget.
///
/// ```
/// # fn main() { }
/// use zero_ui_core::{*, widget_builder::*, widget_instance::*, var::*};
///
/// #[widget($crate::foo)]
/// pub mod foo {
///     use super::*;
///
///     inherit!(widget_base::base);
///
///     properties! {
///         /// Docs.
///         pub bar(impl IntoVar<bool>) = false;
///     }
///
///     fn include(wgt: &mut WidgetBuilder) {
///         wgt.push_build_action(|wgt| {
///             let bar = wgt.capture_var_or_else::<bool, _>(property_id!(Self::bar), || false);
///             println!("bar: {}", bar.get());
///         });
///     }
/// }
/// ```
///
/// In the example above `bar` expands to:
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::{*, widget_builder::*, widget_instance::*, var::*};
/// #[doc(hidden)]
/// #[property(CONTEXT, capture, default(false))]
/// pub fn __bar__(__child__: impl UiNode, bar: impl IntoVar<bool>) -> impl UiNode {
///     __child__
/// }
/// # macro_rules! demo { () => {
/// properties! {
///     /// Docs.
///     pub __bar__ as bar;
/// }
/// # }}
/// ```
///
/// The capture property is re-exported in the widget, and a build action captures it and prints the value. Usually in
/// a captured variable is used in intrinsic nodes that implement a core feature of the widget.
///
/// This shorthand capture property can only have one unnamed input, the input type can be any of the types allowed in property inputs. If
/// the property is assigned the value is used as the property default and a normal property assign is also inserted.
///
/// ## When
///
/// Conditional property assigns can be setup using `when` blocks. A `when` block has a `bool` expression and multiple property assigns,
/// when the expression is `true` each property has the assigned value, unless it is overridden by a later `when` block.
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::{*, widget_builder::*, widget_instance::*, color::*, var::*};
/// #
/// # #[property(FILL)]
/// # pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
/// #   let _ = color;
/// #   child
/// # }
/// #
/// # #[property(LAYOUT)]
/// # pub fn is_pressed(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
/// #   let _ = state;
/// #   child
/// # }
/// #
/// # #[widget($crate::foo)]
/// # pub mod foo {
/// #     use super::*;
/// #
/// #     inherit!(widget_base::base);
/// #
/// properties! {
///     background_color = colors::RED;
///
///     when *#is_pressed {
///         background_color = colors::GREEN;
///     }
/// }
/// # }
/// ```
///
/// ### When Condition
///
/// The `when` block defines a condition expression, in the example above this is `*#is_pressed`. The expression can be any Rust expression
/// that results in a [`bool`] value, you can reference properties in it using the `#` token followed by the property name or path and you
/// can reference variables in it using the `#{var}` syntax. If a property or var is reference the `when` block is dynamic, updating all
/// assigned properties when the expression result changes.
///
/// ### Property Reference
///
/// The most common `when` expression reference is a property, in the example above the `is_pressed` property is instantiated for the widget
/// and it's input read-write var controls when the background is set to green. Note that a reference to the value is inserted in the expression
/// so an extra deref `*` is required. A property can also be referenced with a path, `#properties::is_pressed` also works.
///
/// The syntax seen so far is actually a shorthand way to reference the first input of a property, the full syntax is `#is_pressed.0` or
/// `#is_pressed.state`. You can use the extended syntax to reference inputs of properties with out than one input, the input can be
/// reference by tuple-style index or by name. Note that if the value it self is a tuple or `struct` you need to use the extended syntax
/// to reference a member of the value, `#foo.0.0` or `#foo.0.name`. Methods have no ambiguity, `#foo.name()` is the same as `#foo.0.name()`.
///
/// Not all properties can be referenced in `when` conditions, only inputs of type `impl IntoVar<T>` and `impl IntoValue<T>` are
/// allowed, attempting to reference a different kind of input generates a compile error.
///
/// ### Variable Reference
///
/// Other variable can also be referenced, in a widget declaration only context variables due to placement, but in widget instances any locally
/// declared variable can be referenced. Like with properties the variable value is inserted in the expression as a reference  so you may need
/// to deref in case the var is a simple [`Copy`] value.
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::{*, widget_builder::*, widget_instance::*, color::*, var::*};
/// #
/// # #[property(FILL)]
/// # pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
/// #   let _ = color;
/// #   child
/// # }
/// #
/// context_var! {
///     pub static FOO_VAR: Vec<&'static str> = vec![];
///     pub static BAR_VAR: bool = false;
/// }
/// #
/// # #[widget($crate::foo)]
/// # pub mod foo {
/// #     use super::*;
/// #
/// #     inherit!(widget_base::base);
///
/// properties! {
///     background_color = colors::RED;
///
///     when !*#{BAR_VAR} && #{FOO_VAR}.contains(&"green") {
///         background_color = colors::GREEN;
///     }
/// }
/// # }
/// ```
///
/// ### When Assigns
///
/// Inside the `when` block a list of property assigns is expected, only properties with all inputs of type `impl IntoVar<T>` can ne assigned
/// in `when` blocks, you also cannot `unset!` in when assigns. On instantiation a single instance of the property will be generated, the input
/// vars will track the when expression state and update to the value assigned in the block when it is `true`. When no block is `true` the value
/// assigned to the property outside `when` blocks is used, or the property default value. When more then one block is `true` the *last* one
/// sets the value.
///
/// ### Default Values
///
/// A when assign can be defined by a property without setting a default value, during instantiation if the property declaration has
/// a default value it is used, or if the property was later assigned a value it is used as *default*, if it is not possible to generate
/// a default value the property is not instantiated and the when assign is not used.
///
/// The same apply for properties referenced in the condition expression, note that all `is_state` properties have a default value so
/// it is more rare that a default value is not available. If a condition property cannot be generated the entire when block is ignored.
///
/// # Instantiation
///
/// After the widget macro attribute expands you can still use the module like any other mod, but you can also use it like a macro that
/// accepts property inputs like the `properties!` pseudo-macro, except for the visibility control.
///
/// ```
/// # use zero_ui_core::{*, widget_builder::*, widget_instance::*, color::*, var::*};
/// #
/// # #[property(CONTEXT)]
/// # pub fn bar(child: impl UiNode, val: impl var::IntoVar<bool>) -> impl UiNode {
/// #   let _ = val;
/// #   child
/// # }
/// #
/// # #[property(LAYOUT)]
/// # pub fn margin(child: impl UiNode, val: impl var::IntoVar<u32>) -> impl UiNode {
/// #   let _ = val;
/// #   child
/// # }
/// #
/// # #[property(FILL)]
/// # pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
/// #   let _ = color;
/// #   child
/// # }
/// #
/// # #[property(LAYOUT)]
/// # pub fn is_pressed(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
/// #   let _ = state;
/// #   child
/// # }
/// #
/// # #[widget($crate::foo)]
/// # pub mod foo {
/// #     inherit!(super::widget_base::base);
/// #
/// #     properties! {
/// #         /// Baz property docs.
/// #         pub super::bar as baz = true;
/// #         // inherited property
/// #         enabled = false;
/// #     }
/// # }
/// # fn main() {
/// # let _scope = app::App::minimal();
/// let wgt = foo! {
///     baz = false;
///     margin = 10;
///     
///     when *#is_pressed {
///         background_color = colors::GREEN;
///     }
/// };
/// # }
/// ```
///
/// In the example above  the `baz` property is imported from the `foo!` widget, all widget properties are imported inside the
/// widget macro call, and `foo` exported `pub bar as baz`. The value of `baz` is changed for this instance, the instance also
/// gets a new property `margin`, that was not defined in the widget.
///
/// Most of the features of `properties!` can be used in the widget macro, you can `unset!` properties or rename then using the `original as name`
/// syntax. You can also setup `when` conditions, as demonstrated above, the `background_color` is `GREEN` when `is_pressed`, these properties
/// also don't need to be defined in the widget before use, but if they are they are used instead of the contextual imports.
///
/// ## Init Shorthand
///
/// The generated instantiation widget macro also support the *init shorthand* syntax, where the name of a `let` variable defines the property
/// name and value. In the example below the `margin` property is set on the widget with the value of `margin`.
///
/// ```
/// # macro_rules! demo {
/// # () => {
/// let margin = 10;
/// let wgt = foo! {
///     margin;
/// };
/// # };
/// # }
/// ```