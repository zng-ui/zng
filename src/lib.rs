#![warn(unused_extern_crates)]
// examples of `widget! { .. }` and `#[property(..)]` need to be declared
// outside the main function, because they generate a `mod` with `use super::*;`
// that does not import `use` clauses declared inside the parent function.
#![allow(clippy::needless_doctest_main)]

//! Zero-Ui is a pure Rust UI framework.
//!
//! # Example
//! ```no_run
//! use zero_ui::prelude::*;
//!
//! fn main() {
//!     App::default().run_window(|_| {
//!         let size = var_from((800., 600.));
//!         let title = size.map(|s: &Size| formatx!("Button Example - {}", s));
//!         window! {
//!             size;
//!             title;
//!             content = example();
//!         }
//!     })
//! }
//!
//! fn example() -> impl Widget {
//!     button! {
//!         on_click = |_,_| {
//!             println!("Button clicked!");
//!         };
//!         margin = 10.0;
//!         size = (300.0, 200.0);
//!         align = Alignment::CENTER;
//!         font_size = 28;
//!         content = text("Click Me!");
//!     }
//! }
//! ```
//!
//! # Architecture
//!
//! Zero-Ui apps are organized in a hierarchy of contexts that represents the lifetime of components.
//!
//! ```text
//! +----------------------------------------------------------------------+
//! | # Level 0 - App                                                      |
//! |                                                                      |
//! |  App, AppExtension, AppContext, AppServices, Events                  |
//! |                                                                      |
//! | +------------------------------------------------------------------+ |
//! | | # Level 1 - Window                                               | |
//! | |                                                                  | |
//! | |   Window, WindowContext, WindowServices, WindowId                | |
//! | |                                                                  | |
//! | | +--------------------------------------------------------------+ | |
//! | | | # Level 2 - Widget                                           | | |
//! | | |                                                              | | |
//! | | | UiNode, WidgetContext, widget!, #[property], WidgetId        | | |
//! | | |                                                              | | |
//! | | +--------------------------------------------------------------+ | |
//! | +------------------------------------------------------------------+ |
//! +----------------------------------------------------------------------+
//! ```
//!
//! ## Level 0 - App
//!
//! Components at this level live for the duration of the application, the root type is [`App`](crate::core::app::App).
//! An app is built from multiple extensions ([`AppExtension`](crate::core::app::AppExtension)) and then [`run`](crate::core::app::AppExtended::run).
//!
//! When the app is run, before the main loop starts, the extensions are [init](crate::core::app::AppExtension) with access to an especial context
//! [`AppInitContext`](crate::core::context::AppInitContext). [Services](#services) and [events](#services) can only be registered with
//! this context, they live for the duration of the application.
//!
//! After the app init, the main loop starts and the other extension methods are called with the [`AppContext`](crate::core::context::AppContext).
//!
//! ### Services
//!
//! Services are utilities that can be accessed by every component in every level, this includes [opening windows](crate::core::window::Windows)
//! and [shutting down](crate::core::app::AppProcess) the app it-self. All services implement [`AppService`](crate::core::service::AppService)
//! and can be requested from a [`AppServices`](crate::core::service::AppServices) that is provided by every context.
//!
//! ### Events
//!
//! Events are a list of [`EventArgs`](crate::core::event::EventArgs) that can be observed every update. New events can be generated from the
//! app extension methods or from other events. All events implement [`Event`](crate::core::event::Event) and a listener can be requested from
//! an [`Events`](crate::core::event::Events) that is provided by every context.
//!
//! Note that in [Level 2](#level-2-widget) events are abstracted further into a property that setups a listener and call a handler for every
//! event update.
//!
//! ## Level 1 - Window
//!
//! Components at this level live for the duration of a window instance. A window owns instances [window services](window-services)
//! and the root widget, it manages layout and rendering the widget tree.
//!
//! By default the [`WindowManager`](crate::core::window::WindowManager) extension sets-up the window contexts,
//! but that is not a requirement, you can implement your own *windows*.
//!
//! ### Window Services
//!
//! Services that require a [`WindowContext`](crate::core::context::WindowContext) to be instantiated. They live for the
//! duration of the window instance and every window has the same services. Window service builders must be registered only at
//! [Level 0](level-0-app) during the app initialization, the builders then are called during the window initialization to instantiate
//! the window services.
//!
//! These services can be requested from a [`WindowServices`](crate::core::service::WindowServices) that is provided by the window
//! and widget contexts.
//!
//! ## Level 2 - Widget
//!
//! The UI tree is composed of types that implement [`UiNode`](crate::core::UiNode), they can own one or more child nodes
//! that are also UI nodes, some special nodes introduce a new [`WidgetContext`](crate::core::context::WidgetContext) that
//! defines that sub-tree branch as a widget.
//!
//! The behavior and appearance of a widget is defined in these nodes, a widget
//! is usually composed of multiple nodes, one that defines the context, another that defines its unique behavior
//! plus more nodes introduced by [properties](#properties) that modify the widget.
//!
//! ### Properties
//!
//! A property is in essence a function that takes an UI node and some other arguments and returns a new UI node.
//! This is in fact the signature used for declaring one, see [`#[property]`](crate::core::property) for more details.
//!
//! Multiple properties are grouped together to form a widget.
//!
//! ### Widgets
//!
//! A widget is a bundle of preset properties plus two optional functions, see [`widget!`](crate::core::widget) for more details.
//!
//! During widget instantiation an UI node tree is build by feeding each property node to a subsequent property, in a debug build
//! inspector nodes are inserted also, to monitor the properties. You can press `CTRL+SHIT+I` to inspect a window.
//!
//! The widget root node introduces a new [`WidgetContext`](crate::core::context::WidgetContext) that can be accessed by all
//! widget properties.
//!
//! # State
//!
//! TODO how to keep state, and contextual states.
//!
//! ## Variables
//!
//! TODO
//!
//! # Updates
//!
//! TODO how the UI is updated.
//!
//! # Async
//!
//! TODO how to run async tasks that interact back with the UI.
//!
//! # Lifecycle Overview
//!
//!
//! ```text
//! +------------------------------------+
//! | # Setup                            |
//! |                  ↓↑                |
//! | App::default().extend(CustomExt)   |
//! |      ::empty()                     |
//! +------------------------------------+
//!    |
//!    | .run(|ctx: &mut AppContext| { .. })
//!    | .run_window(|ctx: &mut AppContext| { window! { .. } })
//!    ↓
//! +---------------------------------------+
//! | # Run                                 |
//! |                                       |
//! | services.register(AppProcess)         |
//! |    |                                  |
//! |    ↓            ↓↑                    |
//! | AppExtension::init(AppInitContext)    |
//! |    |                                  |
//! |    ↓     ↓OS  ↓timer  ↓UpdateNotifier |
//! | +---------------------------------------------+
//! | | # EventLoop                                 |
//! | |                                             |
//! | |  AppExtension ↓↑                            |
//! | |      ::on_window_event(..)                  |
//! | |      ::on_device_event(..)                  |
//! | |      ::on_new_frame_ready(..)               |
//! | |   |                                         |
//! | |   ↓      ↓update                            |
//! | | +-----------------------------------------------+
//! | | | # Updates                                     |
//! | | |                                               |
//! | | |   ↓↑ sync - pending assign, notify requests   |
//! | | |   ↓↑ vars - setup new values                  |
//! | | |   ↓↑ events - setup update arguments          |
//! | | |   ↓                                           |
//! | | |   if any -> AppExtension::update(..) ↑        |
//! | | |   |            UiNode::update(..)             |
//! | | |   |            UiNode::update_hp(..)          |
//! | | |   |               event handlers              |
//! | | |   ↓                                           |
//! | | +-----------------------------------------------+
//! | |     ↓                                        |
//! | | +-----------------------------------------------+
//! | | | # Layout/Render                               |
//! | | |                                               |
//! | | | AppExtension::update_display(..)              |
//! | | |           UiNode::measure(..)                 |
//! | | |           UiNode::arrange(..)                 |
//! | | |           UiNode::render(..)                  |
//! | | |           UiNode::render_update(..)           |
//! | | +-----------------------------------------------+
//! | |     ↓                                       |
//! | |   EventLoop                                 |
//! | +---------------------------------------------+
//! |   | AppProcess::shutdown()            |
//! |   ↓                                   |
//! |   0                                   |
//! +---------------------------------------+
//! ```

/*!
<script>
// hide macros from doc root
document.addEventListener('DOMContentLoaded', function() {
    var macros = document.getElementById('macros');
    macros.nextElementSibling.remove();
    macros.remove();

    var side_bar_anchor = document.querySelector("li a[href='#macros']").remove();
 })
</script>
*/

// to make the proc-macro $crate substitute work in doc-tests.
#[doc(hidden)]
#[allow(unused_extern_crates)]
extern crate self as zero_ui;

/// Calls `eprintln!("error: {}", format_args!($))` with `error` colored bright red and bold.
#[allow(unused)]
macro_rules! error_println {
    ($($tt:tt)*) => {{
        use colored::*;
        eprintln!("{}: {}", "error".bright_red().bold(), format_args!($($tt)*))
    }}
}

/// Calls `eprintln!("warning: {}", format_args!($))` with `warning` colored bright yellow and bold.
#[allow(unused)]
macro_rules! warn_println {
    ($($tt:tt)*) => {{
        use colored::*;
        eprintln!("{}: {}", "warning".bright_yellow().bold(), format_args!($($tt)*))
    }}
}

#[allow(unused)]
#[cfg(debug_assertions)]
macro_rules! print_backtrace {
    () => {
        eprintln!("\n\n\n=========BACKTRACE=========\n{:?}", backtrace::Backtrace::new())
    };
}

/// Implements From and IntoVar without boilerplate.
macro_rules! impl_from_and_into_var {
    ($(
        $(#[$docs:meta])*
        fn from $(< $($T:ident  $(: $TConstrain:path)?),+ $(,)?>)? (
            $($name:ident)? // single ident OR
            $( ( // tuple deconstruct of
                $(
                    $($tuple_names:ident)? // single idents OR
                    $( ( // another tuple deconstruct of
                        $($tuple_inner_names:ident ),+ // inner idents
                    ) )?
                ),+
            ) )?
            : $From:ty) -> $To:ty
            $convert_block:block
    )+) => {
        $(
            impl $(< $($T $(: $TConstrain)?),+ >)? From<$From> for $To {
                $(#[$docs])*
                #[inline]
                fn from(
                    $($name)?
                    $( (
                        $(
                            $($tuple_names)?
                            $( (
                                $($tuple_inner_names),+
                            ) )?
                        ),+
                    ) )?
                    : $From) -> Self
                    $convert_block

            }

            impl $(< $($T $(: $TConstrain + Clone)?),+ >)? $crate::core::var::IntoVar<$To> for $From {
                type Var = $crate::core::var::OwnedVar<$To>;

                $(#[$docs])*
                fn into_var(self) -> Self::Var {
                    $crate::core::var::OwnedVar(self.into())
                }
            }
        )+
    };
}

/// Core infrastructure required for creating components and running an app.
pub mod core {
    /// Expands an `impl` block into a [`UiNode`](crate::core::UiNode) trait implementation.
    ///
    /// Missing [`UiNode`](crate::core::UiNode) methods are generated by this macro. The generation
    /// is configured in the macro arguments. The arguments can be a single keyword or a pair assigns.
    ///
    /// The general idea is you implement only the methods required by your node
    /// and configure this macro to generate the methods that are just boilerplate Ui tree propagation.
    ///
    /// # Delegate to single `impl UiNode`
    ///
    /// If your node contains a single child node, like most property nodes, you can configure the code
    /// generator to delegate the method calls for the child node.
    ///
    /// ```
    /// # use zero_ui::core::{impl_ui_node, UiNode};
    /// struct MyNode<C> {
    ///     child: C
    /// }
    /// #[impl_ui_node(
    ///     // Expression that borrows the delegation target node.
    ///     delegate = &self.child,
    ///     // Expression that exclusive borrows the delegation target node.
    ///     delegate_mut = &mut self.child,
    /// )]
    /// impl<C: UiNode> UiNode for MyNode<C> { }
    /// ```
    ///
    /// If the child node is in a member named `child` you can use this shorthand to the same effect:
    ///
    /// ```
    /// # use zero_ui::core::{impl_ui_node, UiNode};
    /// # struct MyNode<C> { child: C }
    /// #[impl_ui_node(child)]
    /// impl<C: UiNode> UiNode for MyNode<C> { }
    /// ```
    ///
    /// The generated code simply calls the same [`UiNode`](crate::core::UiNode) method in the child.
    ///
    /// # Delegate to a `impl UiNodeList`
    ///
    /// If your node contains multiple children nodes in a type that implements [`UiNodeList`](crate::core::UiNodeList),
    /// you can configure the code generator to delegate to the equivalent list methods.
    ///
    /// ```
    /// # use zero_ui::core::{impl_ui_node, UiNode, UiNodeList};
    /// struct MyNode<L> {
    ///     children: L
    /// }
    /// #[impl_ui_node(
    ///     // Expression that borrows the delegation target list.
    ///     delegate_list = &self.children,
    ///     // Expression that exclusive borrows the delegation target list.
    ///     delegate_list_mut = &mut self.children,
    /// )]
    /// impl<L: UiNodeList> UiNode for MyNode<L> { }
    /// ```
    ///
    /// If the children list is a member named `children` you can use this shorthand to the same effect:
    ///
    /// ```
    /// # use zero_ui::core::{impl_ui_node, UiNode, UiNodeList};
    /// # struct MyNode<L> { children: L }
    /// #[impl_ui_node(children)]
    /// impl<L: UiNodeList> UiNode for MyNode<L> { }
    /// ```
    ///
    /// The generated code simply calls the equivalent [`UiNodeList`](crate::core::UiNodeList) method in the list.
    /// That is the same method name with the `_all` prefix. So `UiNode::init` maps to `UiNodeList::init_all` and so on.
    ///
    /// # Delegate to an `impl IntoIterator<impl UiNode>`
    ///
    /// If your node can produce an iterator of its children nodes you can configure the code generator to delegate
    /// to the same [`UiNode`](crate::core::UiNode) method on each node.
    ///
    /// ```
    /// # use zero_ui::core::{impl_ui_node, UiNode};
    /// struct MyNode {
    ///     children: Vec<Box<dyn UiNode>>
    /// }
    /// #[impl_ui_node(
    ///     delegate_iter = self.children.iter(),
    ///     delegate_iter_mut = self.children.iter_mut(),
    /// )]
    /// impl UiNode for MyNode { }
    /// ```
    ///
    /// If the children nodes are in a member named `children` of a type that has the `.iter()` and `.iter_mut()` methods
    /// you can use this shorthand to the same effect:
    ///
    /// ```
    /// # use zero_ui::core::{impl_ui_node, UiNode};
    /// # struct MyNode { children: Vec<Box<dyn UiNode>> }
    /// #[impl_ui_node(children_iter)]
    /// impl UiNode for MyNode { }
    /// ```
    ///
    /// The generated code calls [`into_iter`](std::iter::IntoIterator::into_iter) and uses the iterator to apply the
    /// same [`UiNode`](crate::core::UiNode) method on each child.
    ///
    /// The generated [`measure`](crate::core::UiNode::measure) code returns the desired size of the largest child.
    ///
    /// The generated [`render`](crate::core::UiNode::render) code simply draws each child on top of the previous one.
    ///
    /// ## Don't Delegate
    ///
    /// If your node does not have any child nodes you can configure the code generator to generate empty missing methods.
    ///
    /// ```
    /// # use zero_ui::core::{impl_ui_node, UiNode};
    /// # struct MyNode { }
    /// #[impl_ui_node(none)]
    /// impl UiNode for MyNode { }
    /// ```
    ///
    /// The generated [`measure`](crate::core::UiNode::measure) code fills the available space or collapses if
    /// any space is available (positive infinity).
    ///
    /// The other generated methods are empty.
    ///
    /// # Validation
    ///
    /// If delegation is configured but no delegation occurs in the manually implemented methods
    /// you get the error ``"auto impl delegates call to `{}` but this manual impl does not"``.
    ///
    /// To disable this error use `#[allow_(zero_ui::missing_delegate)]` in the method or in the `impl` block.
    ///
    /// # Mixing Methods
    ///
    /// You can use the same `impl` block to define [`UiNode`](crate::core::UiNode) methods and
    /// associated methods by using this attribute in a `impl` block without trait. The [`UiNode`](crate::core::UiNode)
    /// methods must be tagged with the `#[UiNode]` pseudo-attribute.
    ///
    /// ```
    /// # use zero_ui::core::{impl_ui_node, UiNode, context::WidgetContext};
    /// # struct MyNode { child: Box<dyn UiNode> }
    /// #[impl_ui_node(child)]
    /// impl MyNode {
    ///     fn do_the_thing(&mut self, ctx: &mut WidgetContext) {
    ///         // ..
    ///     }
    ///
    ///     #[UiNode]
    ///     fn init(&mut self, ctx: &mut WidgetContext) {
    ///         self.child.init(ctx);
    ///         self.do_the_thing(ctx);
    ///     }
    ///
    ///     #[UiNode]
    ///     fn update(&mut self, ctx: &mut WidgetContext) {
    ///         self.child.update(ctx);
    ///         self.do_the_thing(ctx);
    ///     }
    /// }
    /// ```
    ///
    /// The above code expands to two `impl` blocks, one with the associated method and the other with
    /// the [`UiNode`](crate::core::UiNode) implementation.
    ///
    /// This is particularly useful for nodes that have a large amount of generic constrains, you just type then once.
    /// <div style='display:none'>
    #[doc(inline)]
    pub use zero_ui_core::impl_ui_node;

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
    /// Optional arguments can be set after the required, they use the `name = value` syntax. Currently there is only one
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
    /// Normal properties must take at least two arguments, the first argument is the child [`UiNode`](crate::core::UiNode), the other argument(s)
    /// are the property values. The function must return a type that implements `UiNode`. The first argument must support any type that implements
    /// `UiNode`, the other arguments also have type requirements depending on the [priority](#priority) or [allowed-in-when](#when-integration).
    /// All of these requirements are validated at compile time.
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
    /// # use zero_ui::core::{property, var::IntoVar, text::Text};
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
    /// * `is_` prefix: Can only take a single [`StateVar`](crate::core::var::StateVar) value.
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
    /// It is strongly encouraged that the event handler signature matches the one from [`on_event`](crate::core::event::on_event).
    ///
    /// ## `outer`
    ///
    /// Properties that shape the visual outside of the widget, the [`margin`](fn@crate::properties::margin) property is an example.
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
    /// # `when` Integration
    ///
    /// Most properties are expected to work in widget `when` blocks, this is controlled by the optional argument `allowed_in_when`. By default all
    /// properties that don't have the `on_` prefix are allowed. This can be overridden by setting `allowed_in_when = <bool>`.
    ///
    /// If a property is `allowed_in_when` all arguments must be [`impl IntoVar<T>`](crate::core::var::IntoVar). This is validated during
    /// compile time, if you see `allowed_in_when_property_requires_IntoVar_members` in a error message you need to change the type or disable `allowed_in_when`.
    ///
    /// ## State Probing
    ///
    /// Properties with the `is_` prefix are special, they output information about the widget instead of shaping it. They are automatically set
    /// to a new probing variable when used in an widget when condition expression.
    /// <div style='display:none'>
    pub use zero_ui_core::property;

    /// Expands a module to a widget module and macro.
    ///
    /// You can add any valid module item to a widget module, the widget attribute adds two pseudo-macros
    /// [`inherit!`](#inherit) and [`properties!`](#properties), it also constrains functions named [`new_child`](#fn-new_child)
    /// and [`new`](#fn-new).
    ///
    /// After expansion the only visible change to the module is in the documentation appended, the module is still usable
    /// as a namespace for any item you witch to add.
    ///
    /// ```
    /// # fn main() { }
    /// use zero_ui::prelude::new_widget::*;
    ///
    /// #[widget($crate::foo)]
    /// pub mod foo {
    ///     use super::*;
    ///     
    ///     // ..
    /// }
    /// ```
    ///
    /// The widget macro takes one argument, a path to the widget module from [`$crate`](https://doc.rust-lang.org/reference/macros-by-example.html#metavariables).
    /// This is a temporary requirement that will be removed when macros-by-example can reference the `self` module.
    ///
    /// # `properties!`
    ///
    /// Widgets are a *tree-rope* of [Ui nodes](zero_ui::core::UiNode), most of the nodes are defined and configured using
    /// properties. Properties are defined using the `properties! { .. }` pseudo-macro. Multiple `properties!` items can be
    /// used, they are merged during the widget compilation.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// #[widget($crate::foo)]
    /// pub mod foo {
    ///     use zero_ui::properties::*;
    ///
    ///     properties! {
    ///         /// Margin applied by default.
    ///         margin = 10;
    ///     }
    /// }
    /// ```
    ///
    /// ## Property Name
    ///
    /// Only a property of each name can exist in a widget, during the widget instantiation the user can
    /// set these properties by their name without needing to import the property.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// # #[widget($crate::foo)]
    /// # pub mod foo {
    /// #   use zero_ui::properties::margin as foo;
    /// properties! {
    ///     /// Foo docs in this widget.
    ///     foo;
    /// }
    /// # }
    /// ```
    ///
    /// You can also use the full path to a property in place, in this case the property name is the last ident in the path.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// # #[widget($crate::foo)]
    /// # pub mod foo {
    /// properties! {
    ///     /// Margin docs in this widget.
    ///     zero_ui::properties::margin;
    /// }
    /// # }
    /// ```
    ///
    /// And finally you can give a property a new name in place, you can use this to allow the same underlying property multiple times.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// # #[widget($crate::foo)]
    /// # pub mod foo {
    /// properties! {
    ///     /// Foo docs.
    ///     zero_ui::properties::margin as foo;
    ///     /// Bar docs.
    ///     zero_ui::properties::margin as bar;
    /// }
    /// # }
    /// ```
    ///
    /// ## Default Values
    ///
    /// Properties without value are not applied unless the user sets then during instantiation. You can give a property
    /// a default value so that it is always applied.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// # #[widget($crate::foo)]
    /// # pub mod foo {
    /// #   use zero_ui::properties::margin as foo;
    /// properties! {
    ///     /// Foo, default value `10`.
    ///     foo = 10;
    /// }
    /// # }
    /// ```
    ///
    /// Note that the property can be removed during instantiation by setting it to `unset!`.
    ///
    /// ## Special Values
    ///
    /// Properties can be *set* to a special value that changes how they are compiled instead of defining a default value.
    ///
    /// ### `required!`
    ///
    /// Marks a property as required, meaning, during the widget instantiation the user must set the property otherwise an
    /// error is generated.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// # #[widget($crate::foo)]
    /// # pub mod foo {
    /// #   use zero_ui::properties::margin as bar;
    /// properties! {
    ///     bar = required!;
    /// }
    /// # }
    /// ```
    ///
    /// Note that captured properties are also marked required without the need for the special value.
    ///
    /// ### `unset!`
    ///
    /// Removes an [inherited](#inherit) property from the widget.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// # #[widget($crate::foo)]
    /// # pub mod foo {
    /// #    inherit!(zero_ui::widgets::container);
    /// #
    /// properties! {
    ///     content_align = unset!;
    /// }
    /// # }
    /// ```
    ///
    /// Note that inherited captured properties no longer captured are automatically unset.
    ///
    /// ## Property Capture
    ///
    /// The two [initialization functions](#initialization-functions) can *capture* a property.
    /// When a property is captured it is not set by the property implementation, the property value is redirected to
    /// the function and can be used in any way inside, some properties are [capture-only](zero_ui::core::property#capture_only),
    /// meaning they don't have an implementation and must be captured.
    ///
    /// ### Declare For Capture
    ///
    /// You can declare a capture-only property in place:
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// # #[widget($crate::foo)]
    /// # pub mod foo {
    /// #    use zero_ui::core::var::*;
    /// #    use zero_ui::core::UiNode;
    /// #    use zero_ui::core::text::formatx;
    /// #    use zero_ui::widgets::text::text;
    /// #
    /// properties! {
    ///     /// Capture-only property `foo` with default value `false`.
    ///     foo: impl IntoVar<bool> = false;
    /// }
    ///
    /// fn new_child(foo: impl IntoVar<bool>) -> impl UiNode {
    ///     let label = foo.into_var().map(|f|formatx!("foo: {:?}", f));
    ///     text(label)
    /// }
    /// # }
    /// ```
    ///
    /// A property declared like this **must** be captured, if does not to be given a value or explicitly marked [required](#required).
    ///
    /// You can set the property [`allowed_in_when`](zero_ui::core::property#when-integration) value using the pseudo-attribute
    /// `#[allowed_in_when = <bool>]`.
    ///
    /// ## Property Order
    ///
    /// When a widget is initialized properties are set according with their [priority](zero_ui::core::property#priority) followed
    /// by their declaration position. You can place a property in a [`child`](#child) block to have if be set before other properties.
    ///
    /// The property value is initialized by the order the properties are declared, all [`child`](#child) property values are initialized first.
    ///
    /// ### `child`
    ///
    /// Widgets have two *groups* of properties, one is presented as applying to the widget, the other as applying to the [*child*](#fn-new_child).
    /// To define a property in the second group, you can use a `child { .. }` block inside `properties! { }`.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// # #[widget($crate::foo)]
    /// # pub mod foo {
    /// # use zero_ui::properties::margin;
    /// properties! {
    ///     child {
    ///         /// Spacing around the content.
    ///         margin as padding = 10;
    ///     }
    /// }
    /// # }
    /// ```
    ///
    /// ## `when`
    ///
    /// Some widget properties need different values depending on widget state. You can manually implement this
    /// using variable [mapping](zero_ui::core::var::Var::map) and [merging](zero_ui::core::var::merge_var) but a
    /// better way is to use the `when` block.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// # #[widget($crate::foo)]
    /// # pub mod foo {
    /// #    use zero_ui::prelude::new_widget::*;
    /// #
    /// properties! {
    ///     background_color = colors::RED;
    ///
    ///     when self.is_hovered {
    ///         background_color = colors::BLUE;
    ///     }
    ///     when self.is_pressed {
    ///         background_color = colors::GREEN;
    ///     }
    /// }
    /// # }
    /// ```
    ///
    /// When blocks can be declared inside the `properties!` pseudo-macro, they take an expression followed by a block of
    /// property assigns. You can reference widget properties in the expression by using the `self.` prefix.
    ///
    /// In the example above the value of `background_color` will change depending on the interaction with the pointer, if it
    /// is over the widget the background changes to blue, if it is pressed the background changes to green. Subsequent *whens* that
    /// affect the same property have higher priority the previous whens, so when the pointer is over the widget and pressed the last
    /// *when* (pressed color) is applied.
    ///
    /// ### When Expression
    ///
    /// The when expression is a boolean similar to the `if` expression, the difference in that it can reference [variables](zero_ui::core::var::Var)
    /// from properties or other sources, and when these variables updates the expression result updates.
    ///
    /// #### Reference Property
    ///
    /// Use `self.<property>` to reference to an widget property, the value resolves to the variable value of the first member of the property,
    /// if the property has a default value it does not need to be defined in the widget before usage.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::{property, widget, UiNode, var::IntoVar};
    /// #[property(context)]
    /// pub fn foo(
    ///     child: impl UiNode,
    ///     member_a: impl IntoVar<bool>,
    ///     member_b: impl IntoVar<u32>
    /// ) -> impl UiNode {
    ///     // ..
    /// #   let _ = member_a;
    /// #   let _ = member_b;
    /// #   child
    /// }
    ///
    /// #[widget($crate::bar)]
    /// pub mod bar {
    ///     use zero_ui::prelude::new_widget::*;
    ///
    ///     properties! {
    ///         background_color = colors::BLACK;
    ///         super::foo = true, 32;
    ///
    ///         when self.foo {
    ///             background_color = colors::RED;
    ///         }
    ///
    ///         when self.is_pressed {
    ///             background_color = colors::BLUE;
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// In the example above `self.foo` is referencing the `member_a` variable value, note that `foo` was
    /// defined in the widget first. [State](zero_ui::core::property#state-probing) variables have a default value so
    /// `is_pressed` can be used without defining it first in the widget.
    ///
    /// #### Reference Property Member
    ///
    /// A property reference automatically uses the first member, you can reference other members by name or by index.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::{property, widget, UiNode, var::IntoVar};
    /// #[property(context)]
    /// # pub fn foo(child: impl UiNode, member_a: impl IntoVar<bool>, member_b: impl IntoVar<u32>) -> impl UiNode {  
    /// #   let _ = member_a;
    /// #   let _ = member_b;
    /// #   child
    /// # }
    ///
    /// # #[widget($crate::bar)]
    /// # pub mod bar {
    /// #    use zero_ui::prelude::new_widget::*;
    /// properties! {
    ///     background_color = colors::BLACK;
    ///     super::foo = true, 32;
    ///
    ///     when self.foo.member_b == 32 {
    ///         background_color = colors::RED;
    ///     }
    /// }
    /// # }
    /// ```
    ///
    /// In the example above `self.foo.member_b` is referencing the `member_b` variable value. Alternatively you can also use
    /// tuple indexing, `self.foo.1` also references the `member_b` variable value.
    ///
    /// #### Reference Other Items
    ///
    /// Widget when expressions can reference any other `'static` item, not just properties. If the item is a variable and you want
    /// the expression to update when a variable update quote it with `#{<var>}`.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::{property, widget, UiNode, var::IntoVar};
    /// #
    /// # #[widget($crate::bar)]
    /// # pub mod bar {
    /// #    use zero_ui::prelude::new_widget::*;
    /// static ST_VALUE: bool = true;
    ///
    /// context_var! { pub struct FooVar: bool = const true; }
    ///
    /// fn bar() -> bool { true }
    ///
    /// properties! {
    ///     background_color = colors::BLACK;
    ///
    ///     when ST_VALUE && *#{FooVar::var().clone()} && bar() {
    ///         background_color = colors::RED;
    ///     }
    /// }
    /// # }
    /// ```
    ///
    /// In the example above a static value `ST_VALUE`, a context var `FooVar` and a function `bar` are used in the expression. The expression
    /// is (re)evaluated when the context var updates, `FooVar::var()` is evaluated only once during initialization.
    ///
    /// ### Default States
    ///
    /// Properties need to be assigned in a widget to participate in `when` blocks, this is because the generated code needs
    /// to observe changes caused by the property, in the condition expression, or set the property to a default value when no
    /// condition is active, assigned in when.
    ///
    /// If the property has a default value and is not manually set in the widget it is set to the default value automatically.
    ///
    /// Properties added automatically show in the widget documentation like manual properties, the widget user can see and set
    /// then manually.
    ///
    /// Currently only state properties have a default value, this will probably change in the future.
    ///
    /// ### Auto-Disabling
    ///
    /// It is not an error to use a property without default value (manual or auto) in a widget `when` block. If such a property is used
    /// in the condition expression the `when` block is only used during initialization if the user sets the property.
    ///
    /// If such a property is assigned in a `when` block it is also only used if it is set during initialization. In this case other
    /// properties set in the same `when` block still use it.
    ///
    /// You can use this to setup custom widget effects that are only activated if the widget instance actually uses a property.
    ///
    /// # Initialization Functions
    ///
    /// Widgets are a *tree-rope* of [Ui nodes](zero_ui::core::UiNode), the two initialization functions define the
    /// inner ([`new_child`](#fn-new_child)) and outer ([`new`](#fn-new)) boundary of the widget.
    ///
    /// The functions can *capture* properties by having an input of the same name as a widget property.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// #[widget($crate::foo)]
    /// pub mod foo {
    ///     use zero_ui::core::{NilUiNode, units::SideOffsets, var::IntoVar};
    ///     use zero_ui::properties::margin;
    ///
    ///     properties! {
    ///         margin = 10;
    ///     }
    ///
    ///     fn new_child(margin: impl IntoVar<SideOffsets>) -> NilUiNode {
    ///         // .. do something with margin.
    ///         NilUiNode
    ///     }
    /// }
    /// ```
    ///
    /// In the example above the `margin` property is not applied during initialization,
    /// its value is redirected the the `new_child` function. The input type must match the captured property type,
    /// if the property has more then one member the input type is a tuple of the property types.
    ///
    /// Initialization functions are not required, a the new widget inherits from another the functions from the other
    /// widget are used, if not a default implementation is provided. The functions don't need to be public either, only
    /// make then public is there is an utility in calling then manually.
    ///
    /// The functions are identified by name and have extra constrains that are validated during compile time. In general
    /// they cannot be `unsafe`, `async` nor `extern`, they also cannot declare lifetimes nor `const` generics.
    ///
    /// ## `fn new_child`
    ///
    /// The `new_child` initialization function defines the inner most node of the widget, it must output a type that implements
    /// [`UiNode`](zero_ui::core::UiNode).
    ///
    /// The [default function](zero_ui::core::widget_base::default_widget_new_child) does not capture any property and simply outputs
    /// the [`NilUiNode`](zero_ui::core::NilUiNode) value.
    ///
    /// ## `fn new`
    ///
    /// The `new` initialization function defines the outer most type of the widget, if must take at least one input that is a generic
    /// that allows any type that implements [`UiNode`](zero_ui::core::UiNode), although not required you probably want to capture the
    /// implicit [`id`](mod@zero_ui::core::widget_base::implicit_mixin#wp-id) property.
    ///
    /// The output can be any type, if you want the widget to be compatible with most layout slots the type must implement
    /// [`Widget`](zero_ui::core::Widget) and it is recommended that you use the [default function](zero_ui::core::widget_base::default_widget_new)
    /// to generate the widget.
    ///
    /// The [default function](zero_ui::core::widget_base::default_widget_new) captures the [`id`](mod@zero_ui::core::widget_base::implicit_mixin#wp-id)
    /// property and returns a [`Widget`](zero_ui::core::Widget) that properly establishes a widget context during the
    /// [`UiNode`](zero_ui::core::UiNode) method calls.
    ///
    /// # `inherit!`
    ///
    /// Widgets can inherit from one or more other widgets and mix-ins using the pseudo-macro `inherit!(widget::path);`.
    /// An inherit is like an import/reexport of properties and initialization functions.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// #[widget($crate::foo)]
    /// pub mod foo {
    ///     inherit!(zero_ui::widgets::container);
    ///
    ///     // ..
    /// }
    /// ```
    ///
    /// In the example above, the new widget `foo` inherits all the properties and
    /// initialization functions of [`container`](mod@zero_ui::widgets::container).
    ///
    /// ## Override
    ///
    /// Subsequent inherits override properties and functions with the same name as previously inherited, properties
    /// and functions declared in the new widget override inherited items independent of the order the declaration appears
    /// in the source-code.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// #[widget($crate::foo)]
    /// pub mod foo {
    ///     properties! {
    ///         zero_ui::properties::margin = 10;
    ///     }
    ///
    ///     fn new_child() -> zero_ui::core::NilUiNode {
    ///         zero_ui::core::NilUiNode
    ///     }
    /// }
    ///
    /// #[widget($crate::bar)]
    /// pub mod bar {
    ///     properties! {
    ///         zero_ui::properties::margin = 20;
    ///     }
    ///
    ///     fn new_child() -> impl zero_ui::core::UiNode {
    /// #       use zero_ui::widgets::text::text;
    ///         text("Bar!")
    ///     }
    /// }
    ///
    /// #[widget($crate::foo_bar)]
    /// pub mod foo_bar {
    ///     inherit!(super::foo);
    ///     inherit!(super::bar);
    /// }
    /// ```
    ///
    /// In the example above `foo_bar` has a property named `margin` with default value `20`, and its child
    /// is a text widget that prints `"Bar!"`.
    ///
    /// ## Implicit
    ///
    /// Every widget inherits [`implicit_mixin`](mod@zero_ui::core::widget_base::implicit_mixin) before all other inherits.
    ///
    /// If an widget does not inherit from another widget and does not declare a initialization function the defaults,
    /// [`default_widget_new_child`](zero_ui::core::widget_base::default_widget_new_child) and
    /// [`default_widget_new`](zero_ui::core::widget_base::default_widget_new) are used.
    ///
    /// <div style='display:none'>
    pub use zero_ui_core::widget;

    /// Expands a module to a widget mix-in module.
    ///
    /// Widget mix-ins can only be inherited by other widgets and mix-ins, they cannot be instantiated.
    ///
    /// See the [`#[widget(..)]`](macro@widget) documentation for how to declare, the only difference
    /// from a full widget is that you can only inherit other mix-ins and cannot declare
    /// the `new_child` and `new` functions.
    /// <div style='display:none'>
    pub use zero_ui_core::widget_mixin;

    pub use zero_ui_core::*;
}

pub mod properties;
pub mod widgets;

/// All the types you need to start building an app.
///
/// Use glob import (`*`) and start implementing your app.
///
/// ```no_run
/// use zero_ui::prelude::*;
///
/// App::default().run_window(|_| {
///     todo!()
/// })
/// ```
///
/// # Other Preludes
///
/// There are prelude modules for other contexts, [`new_property`](crate::prelude::new_property) for
/// creating new properties, [`new_widget`](crate::prelude::new_widget) for creating new widgets.
pub mod prelude {
    #[doc(no_inline)]
    pub use crate::core::{
        app::{App, ElementState},
        color::{
            self, blur, brightness, colors, contrast, drop_shadow, grayscale, hex, hsl, hsla, hue_rotate, opacity, rgb, rgba, saturate,
            sepia, Rgba,
        },
        context::WidgetContext,
        focus::{DirectionalNav, Focus, TabIndex, TabNav},
        gesture::{shortcut, GestureKey, Shortcut, Shortcuts},
        gradient::{stops, ExtendMode, GradientStop, GradientStops},
        keyboard::{Key, ModifiersState},
        mouse::MouseButton,
        node_vec, nodes,
        render::WidgetPath,
        service::{AppServices, WindowServices},
        sync::Sync,
        text::{
            font_features::{
                CapsVariant, CharVariant, CnVariant, EastAsianWidth, FontPosition, FontStyleSet, JpVariant, NumFraction, NumSpacing,
                NumVariant,
            },
            formatx, FontFeatures, FontName, FontNames, FontStretch, FontStyle, FontWeight, Fonts, Hyphens, LineBreak, Text, TextAlign,
            TextTransformFn, ToText, WhiteSpace, WordBreak,
        },
        units::{
            rotate, skew, translate, Alignment, AngleUnits, FactorUnits, Length, LengthUnits, Line, LineFromTuplesBuilder, LineHeight,
            Point, Rect, RectFromTuplesBuilder, SideOffsets, Size, TimeUnits,
        },
        var::{merge_var, state_var, switch_var, var, var_from, RcVar, Var, VarObj, Vars},
        widget_base::Visibility,
        widget_vec, widgets,
        window::{AppRunWindow, CursorIcon, StartPosition, Window, Windows},
        UiNode, Widget, WidgetId, WidgetList, WidgetVec,
    };

    #[doc(no_inline)]
    pub use crate::properties::*;
    #[doc(no_inline)]
    pub use crate::widgets::*;

    #[doc(no_inline)]
    pub use crate::properties::border::*;
    #[doc(no_inline)]
    pub use crate::properties::events::{gesture::*, keyboard::*};
    #[doc(no_inline)]
    pub use crate::properties::filters::*;
    #[doc(no_inline)]
    pub use crate::properties::focus::*;
    #[doc(no_inline)]
    pub use crate::properties::states::*;
    #[doc(no_inline)]
    pub use crate::properties::text_theme::{
        font_family, font_size, font_stretch, font_style, font_weight, letter_spacing, line_height, tab_length, text_align, text_color,
        text_transform, word_spacing,
    };
    #[doc(no_inline)]
    pub use crate::properties::transform::{transform, *};

    #[doc(no_inline)]
    pub use crate::widgets::layouts::*;
    #[doc(no_inline)]
    pub use crate::widgets::text::{text, *};

    /// All the types you need to declare a new property.
    ///
    /// Use glob import (`*`) and start implement your custom properties.
    ///
    /// ```
    /// # fn main() {}
    /// use zero_ui::prelude::new_property::*;
    ///
    /// #[property(context)]
    /// pub fn my_property(child: impl UiNode, value: impl IntoVar<bool>) -> impl UiNode {
    ///     MyPropertyNode { child, value: value.into_var() }
    /// }
    ///
    /// struct MyPropertyNode<C: UiNode, V: Var<bool>> {
    ///     child: C,
    ///     value: V
    /// }
    /// #[impl_ui_node(child)]
    /// impl<C: UiNode, V: Var<bool>> UiNode for MyPropertyNode<C, V> {
    ///     fn update(&mut self, ctx: &mut WidgetContext) {
    ///         self.child.update(ctx);
    ///         if let Some(new_value) = self.value.get_new(ctx.vars) {
    ///             todo!()
    ///         }
    ///     }
    /// }
    /// ```
    pub mod new_property {
        #[doc(no_inline)]
        pub use crate::core::app::ElementState;
        #[doc(no_inline)]
        pub use crate::core::color::{self, *};
        #[doc(no_inline)]
        pub use crate::core::context::*;
        #[doc(no_inline)]
        pub use crate::core::event::*;
        #[doc(no_inline)]
        pub use crate::core::gesture::*;
        #[doc(no_inline)]
        pub use crate::core::render::*;
        #[doc(no_inline)]
        pub use crate::core::text::Text;
        #[doc(no_inline)]
        pub use crate::core::units::{self, *};
        #[doc(no_inline)]
        pub use crate::core::var::*;
        #[doc(no_inline)]
        pub use crate::core::widget_base::{IsEnabled, WidgetEnabledExt};
        #[doc(no_inline)]
        pub use crate::core::{
            impl_ui_node, is_layout_any_size, node_vec, nodes, property,
            widget_base::{Visibility, VisibilityContext, WidgetListVisibilityExt, WidgetVisibilityExt},
            widget_vec, widgets, FillUiNode, UiNode, UiNodeList, Widget, WidgetId, WidgetList, WidgetVec, LAYOUT_ANY_SIZE,
        };
        #[doc(no_inline)]
        pub use crate::properties::{set_widget_state, with_context_var};
    }

    /// All the types you need to declare a new widget or widget mix-in.
    ///
    /// Use glob import (`*`) and start implement your custom widgets.
    ///
    /// ```
    /// # fn main() { }
    /// use zero_ui::prelude::new_widget::*;
    ///
    /// #[widget($crate::my_widget)]
    /// pub mod my_widget {
    ///     use super::*;
    ///
    ///     properties! {
    ///         background_color = colors::BLUE;
    ///     }
    /// }
    /// ```
    pub mod new_widget {
        #[doc(no_inline)]
        pub use crate::core::color::*;
        #[doc(no_inline)]
        pub use crate::core::context::*;
        #[doc(no_inline)]
        pub use crate::core::render::*;
        #[doc(no_inline)]
        pub use crate::core::text::*;
        #[doc(no_inline)]
        pub use crate::core::units::*;
        #[doc(no_inline)]
        pub use crate::core::var::*;
        #[doc(no_inline)]
        pub use crate::core::{
            impl_ui_node, is_layout_any_size, node_vec, nodes, widget,
            widget_base::{Visibility, VisibilityContext, WidgetListVisibilityExt, WidgetVisibilityExt},
            widget_mixin, widget_vec, widgets, FillUiNode, UiNode, UiNodeList, Widget, WidgetId, WidgetList, WidgetVec, LAYOUT_ANY_SIZE,
        };
        #[doc(no_inline)]
        pub use crate::properties::border::{border, *};
        #[doc(no_inline)]
        pub use crate::properties::events::{self, gesture::*, keyboard::*};
        #[doc(no_inline)]
        pub use crate::properties::filters::*;
        #[doc(no_inline)]
        pub use crate::properties::focus::focusable;
        #[doc(no_inline)]
        pub use crate::properties::focus::*;
        #[doc(no_inline)]
        pub use crate::properties::states::*;
        #[doc(no_inline)]
        pub use crate::properties::text_theme::{
            font_family, font_size, font_stretch, font_style, font_weight, letter_spacing, line_height, tab_length, text_align, text_color,
            text_transform, word_spacing,
        };
        #[doc(no_inline)]
        pub use crate::properties::transform::{transform, *};
        #[doc(no_inline)]
        pub use crate::properties::*;
        #[doc(no_inline)]
        pub use crate::widgets::container;
        #[doc(no_inline)]
        pub use crate::widgets::mixins::*;
    }
}

// see test-crates/no-direct-deps
#[doc(hidden)]
pub fn crate_reference_called() -> bool {
    true
}
#[doc(hidden)]
#[macro_export]
macro_rules! crate_reference_call {
    () => {
        $crate::crate_reference_called()
    };
}
