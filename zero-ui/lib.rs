#![warn(unused_extern_crates)]
// examples of `widget! { .. }` and `#[property(..)]` need to be declared
// outside the main function, because they generate a `mod` with `use super::*;`
// that does not import `use` clauses declared inside the parent function.
#![allow(clippy::needless_doctest_main)]
#![warn(missing_docs)]

//! Zero-Ui is the pure Rust GUI framework with batteries included.
//!
//! It provides all that you need to create a beautiful, fast and responsive multi-platform GUI apps, it includes many features
//! that allow you to get started quickly, without sacrificing customization or performance. With features like gesture events,
//! common widgets, layouts, data binding, async tasks, accessibility and localization
//! you can focus on what makes your app unique, not the boilerplate required to get modern apps up to standard.
//!
//! When you do need to customize, Zero-Ui is rightly flexible, you can create new widgets or customize existing ones, not just
//! new looks but new behavior, at a lower level you can introduce new event types or new event sources, making custom hardware seamless
//! integrate into the framework.
//!
//! # Usage
//!
//! First add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! zero-ui = "0.2"
//! ```
//!
//! Then create your first window:
//!
//! ```no_run
//! use zero_ui::prelude::*;
//!
//! fn main() {
//!     App::default().run_window(|_| {
//!         let size = var_from((800, 600));
//!         window! {
//!             title = size.map(|s: &Size| formatx!("Button Example - {}", s));
//!             size;
//!             content = button! {
//!                 on_click = hn!(|_,_| {
//!                     println!("Button clicked!");
//!                 });
//!                 margin = 10;
//!                 size = (300, 200);
//!                 align = Alignment::CENTER;
//!                 font_size = 28;
//!                 content = text("Click Me!");
//!             }
//!         }
//!     })
//! }
//! ```
//!
//! # Building Blocks
//!
//! Zero-Ui apps are completely formed from modular building blocks and those blocks are formed from more basic blocks still,
//! most of the high level blocks compile down to the most basic at zero-cost. This can be surprising when you see put together:
//!
//! ```
//! # use zero_ui::prelude::*;
//! button! {
//!     on_click = hn!(|_, _| println!("Clicked!"));
//!     content = text("Click Me!");
//!     font_size = 28;
//! }
//! # ;
//! ```
//!
//! The example demonstrates the [`button!`] widget, you may thing the [`on_click`] and [`font_size`] are implemented in the widget,
//! but they are not. The button widget only knows that it has a `content` that is another widget, it makes this content looks like a button.
//!
//! In this case [`text()`] is another widget that renders text, and [`font_size`] is a property that sets the font size for all texts
//! inside the widget it is set in. Similarly [`on_click`] is a property that makes the widget clickable. Widgets are build from
//! properties and properties are built from a lower level block, the [`UiNode`].
//!
//! You can make a small app knowing only the high-level blocks, but a passing understanding of how they are formed can help you make the
//! most of them. The following is a summary of the high-level blocks with links for further reading on how they work:
//!
//! ## Widgets
//!
//! Widgets, also known as controls, are the building blocks of the final GUI, items such as a button, text-box, scroll-bar and label are widgets.
//! In Zero-Ui they are usually a module/macro combo with the same name, some widgets also add a shorthand function.
//!
//! You can think of a widget as a set of properties that work well together, widgets can preset, rename or require properties, they can
//! also *inherit* from other widgets. They are **instantiated using a macro** for each widget, the macro lets you assign properties using
//! a declarative syntax, all widgets are open-ended, meaning you can use any property with any widget.
//!
//! ```
//! # use zero_ui::prelude::*;
//! let text_a = text! {
//!     text = "Hello!";
//!     color = colors::BLACK;
//!     background_gradient = 45.deg(), [rgb(255, 0, 0), rgb(0, 255, 0)];
//!     margin = 10;
//! };
//!
//! let text_b = text("Hello!");
//! ```
//!
//! The example instantiate two [`text!`] widgets, `text_a` uses the full macro, the `text` and `color` properties are mentioned in
//! widget documentation but `background_gradient` and `margin` are not. The `text_b` demonstrates the shorthand function [`text()`]
//! that for assigning the `text` property directly.
//!
//! This crate provides most of the common widgets in the **[`zero_ui::widgets`]** module. That module documentation also explains widgets
//! in detail.
//!
//! ### Declaring Widgets
//!
//! Widgets are declared as a module marked with the [`#[widget]`][#widget] attribute. Its very easy to declare a widget, you should try it when
//! you find yourself duplicating the same widget/property/value combo in multiple places.
//!
//! ```
//! use zero_ui::prelude::*;
//! use zero_ui::prelude::new_widget::*;
//!
//! #[widget($crate::red_button)]
//! mod red_button {
//!      use super::*;
//!     inherit!(zero_ui::widgets::button);
//!     
//!     properties! {
//!         background_color = colors::RED.darken(50.pct());
//!         text_color = colors::WHITE;
//!       
//!         when self.is_pressed {
//!             background_color = colors::RED.darken(30.pct());
//!         }
//!     }
//! }
//!
//! # fn main() {
//! let btn = red_button! {
//!     content = text("!");
//!     on_click = hn!(|_, _| println!("Alert!"));
//! };
//! # }
//! ```
//!
//! The example demonstrates a simple [`button!`] derived widget, all the widgets in this crate are declared using the **[`#[widget]`]**
//! attribute, the documentation for the attribute explains widget declaration in detail.
//!
//! ## Layouts
//!
//! Widgets can contains, none, one or many other widgets, some widgets are specialized into arranging other widgets on the screen. These
//! are called *layout widgets*.
//!
//! ```
//! # use zero_ui::prelude::*;
//! #
//! let menu = v_stack! {
//!     spacing = 5;
//!     items = widgets![
//!         button! { content = text("New") },
//!         button! { content = text("Load") },
//!         button! { content = text("Save") },
//!     ];
//! };
//! ```
//!
//! The example demonstrates the [`v_stack!`] layout widget, it stacks other widgets vertically with an optional spacing in between then.
//! All the built-in layouts are in the **[`zero_ui::widgets::layouts`]**.
//!
//! ## Properties
//!
//! Properties are the most important building block, most of the code that goes into forming a widget is implemented in properties.
//! Assigning a property in a widget causes it to insert its own code in the *widget*, if a property is not assigned it has zero cost,
//! this means that a widget designer never needs to worry about the cost of adding a rarely used widget behavior, because it will not
//! cost anything, unless it is used.
//!
//! ```
//! # use zero_ui::prelude::*;
//! #
//! let wgt = blank! {
//!     // single value assign:
//!     margin = (10, 5);
//!
//!     // multi value assign:
//!     background_gradient = 45.deg(), [rgb(255, 0, 0), rgb(0, 255, 0)];
//! };
//! let wgt = blank! {
//!     // multi value using the named value syntax:
//!     background_gradient = {
//!         axis: 45.deg(),
//!         stops: [rgb(255, 0, 0), rgb(0, 255, 0)]
//!     };
//! };
//! ```
//!
//! Some property kinds can be identified using the prefix of their names, `on_foo` indicates that the property setups an event handler,
//! `is_foo` indicates a property that reports an widget state.
//!
//! ### Declaring Properties
//!
//! Properties are declared as a function marked with the [`#[property]`][#property] attribute. The first parameter contains the other properties
//! from the widget, the function wraps this into their own code and returns the appended code, that will probably be fed into another
//! property.
//!
//! ```
//! # fn main() { }
//! use zero_ui::prelude::new_property::*;
//!
//! #[property(outer)]
//! pub fn margin(child: impl UiNode, margin: impl IntoVar<SideOffsets>) -> impl UiNode {
//!     // ..
//!#    child
//! }
//! ```
//!
//! When assigned in a widget only the second plus parameters are the property input, the first parameter is set by the widget.
//!
//! ```
//! # use zero_ui::prelude::*;
//! # use blank as foo;
//! let wgt = foo! {
//!     margin = 10;
//! };
//! ```
//!
//! The mechanism properties use to append their own code to widgets is beyond the scope of this introduction, the documentation
//! of the **[`#[property]`][#property]** and **[`#[impl_ui_node]`][#impl_ui_node]** attributes explains it in detail.
//!
//! ## Variables
//!
//! Due to the declarative nature of properties, you cannot reassign a property. When you assign a property in a widget you are actually
//! helping to define the widget. The solution to this is to assign it once to a value that changes, the property can then update when the value
//! updates, this is sometimes called *data-binding*, we just call then variables. By supporting any value that implements [`Var<T>`]
//! properties can work with both updating and unchanging values, and if you use an unchanging value the code that responds to variable
//! changes is optimized away.
//!
//! Usually the trait [`IntoVar<T>`] is used to receive variable inputs, every type that is `Debug + Clone` implements this trait,
//! types used in properties also tend to implement a *shorthand syntax* by converting from simpler types. For example the [`margin`]
//! property input type is [`SideOffsets`], it converts from multiple different *shorthand types*:
//!
//! ```
//! # use zero_ui::prelude::*;
//! # let _ = blank! {
//! // Same margin all around:
//! margin = 10;
//! # };
//!
//! # let _ = blank! {
//! // (top-bottom, left-right):
//! margin = (10, 20);
//! # };
//!
//! # let _ = blank! {
//! // (top, right, bottom, left):
//! margin = (10, 20, 30, 40);
//! # };
//!
//! # let _ = blank! {
//! // direct value:
//! margin = SideOffsets::new_all(10);
//! # };
//! ```
//!
//! As you can see a variety of inputs kinds are supported, all still statically typed so they are validated by the Rust type system.
//! But the real power of variables shows when you use variable that update, you can declare one using [`var()`] or [`var_from()`]:
//!
//! ```
//! # use zero_ui::prelude::*;
//! let offset = var_from(10);
//! let moving_btn = button! {
//!     margin = offset.clone();
//!     on_click = hn!(|ctx, _| {
//!         offset.modify(ctx, |m|m.left += 50.0);
//!     });
//!     content = text("Click to Move!")
//! };
//! ```
//!
//! The button moves to the right when clicked, the `margin` starts at `10` and every click the variable is modified, this causes
//! the `margin` property to request a re-layout and render. Note that the variable is now *shared* between two places, variables
//! that update are *counted* references to a shared value, the one created in the example is called [`RcVar<T>`].
//!
//! ### Variable Get/Set
//!
//! Variable bridge two styles of programming, when you are wiring properties using variables the code is *declarative* but when
//! you actually access their value the code is, usually, *imperative*. The most common place where variables are changed is in event
//! handlers, the [`Var<T>`] trait provides methods for getting and setting the value.
//!
//! ```
//! # use zero_ui::prelude::*;
//! let flag = var(false);
//! let btn = button! {
//!     content = text(flag.map_to_text());
//!     on_click = hn!(|ctx, _| {
//!         flag.set(ctx.vars, !flag.copy(ctx.vars));
//!     });
//! };
//! ```
//!
//! The `copy` method gets a copy of the current value, the `set` method schedules a new value for the variable.
//! Value changes **don't apply immediately**, when you set a variable the new value will be visible only in the next app
//! update, this is done so that variable observers are always synchronized, it is not possible to enter a state where a
//! part of the screen is showing a different value because it is changed in between.
//!
//! This synchronization is done using the Rust borrow checker, every value access is done using a reference to [`Vars`]
//! and only one [`Vars`] instance exists per app. Internally [`Vars`] is exclusive-borrowed when it is time to apply
//! variable changes, asserting that there is no dangling reference, without needing any run-time mechanism like `RefCell`.
//!
//! The [`Var<T>`] trait provides other methods for getting, there is `copy`, `get` for referencing the value and `get_clone` for cloning.
//! The same for settings, there is `set` that replaces the value, `modify` that schedules a closure that modifies the value and `set_ne`
//! that checks for value equality before causing an update. You can also `touch` a variable to cause an update without changing the value.
//!
//! ```
//! # use zero_ui::prelude::*;
//! let flag = var(false);
//! let btn = button! {
//!     content = text(flag.map_to_text());
//!     on_click = hn!(|ctx, _| {
//!         let new_value = !*flag.get(ctx.vars);
//!         // 3 methods doing the same thing.
//!         flag.set(ctx.vars, new_value);
//!         flag.set_ne(ctx.vars, new_value);
//!         flag.modify(ctx.vars, |f| **f = new_value);
//!     });
//! };
//! ```
//!
//! See the [`Var<T>`] documentation for indebt information about accessing variable values.
//!
//! ### Variable Mapping
//!
//! You can generate new variables that **map** from a source variable, every time the source variable changes a *mapping function*
//! is applied to generated a mapped value, both the source and mapped variable updating at the same time.
//!
//! ```
//! # use zero_ui::prelude::*;
//! let count = var(0u32);
//! let clicker = button! {
//!     content = text(count.map(|c| {
//!         match c {
//!             0 => "Click Me!".to_text(),
//!             1 => "Clicked 1 Time!".to_text(),
//!             n => formatx!("Clicked {} Times!", n)
//!         }
//!     }));
//!     on_click = hn!(|ctx, _| {
//!         let next = count.copy(ctx) + 1;
//!         count.set(ctx, next);
//!     });
//! };
//! ```
//!
//! In the example the source variable `count` is mapped into a [`Text`] for the button content. Every time the button is clicked
//! the text changes, but the event handler only needs to know about the source variable. There is a variety of different mappings
//! that can be done, including bidirectional mapping, see the [`Var<T>`] documentation for an inadept explanation of variable mapping.
//!
//! ### Variable Binding
//!
//! Variable mapping always generate a new variable, if you have two variables you can **bind** then instead. Bound variables
//! update at the same time, liked mapped variables, but with the advantage that you can *unbind* then and still use both variables.
//!
//! ```no_run
//! # use zero_ui::prelude::*;
//! App::default().run_window(|ctx| {
//!     let count = var(0u32);
//!     let count_text = var_from("Click Me!");
//!     let handle = count.bind_map(ctx, &count_text, |_, c| {
//!         match c {
//!             1 => "Clicked 1 Time!".to_text(),
//!             n => formatx!("Clicked {} Times!", n)
//!         }
//!     });
//!     handle.permanent();
//!     window! {
//!         content = button! {
//!             content = text(count_text);
//!             on_click = hn!(|ctx, _| {
//!                 count.modify(ctx, |c| **c += 1);
//!             });
//!         }
//!     }
//! })
//! ```
//!
//! Notice the differences between mapping and binding, first we need a context to access the [`Vars`] reference, second the
//! text variable already has a value and it is only overwritten when the count variable updates, and
//! finally the bind method returned a [`VarBindingHandle`] that must be dealt with.
//!
//! ### Variable Send/Receive
//!
//! Variables are not `Send` and you can only get/set then in the app thread. Together with the get/set requirements they
//! synchronize for free, and are very cheap but also limited. To solve this the [`Var<T>`] provides two methods for creating
//! sender/receiver channels to a variable. The general idea is you wire the GUI using variables, mapping and binding, reducing
//! the number of variables that control to whole thing, a *view-model* if you will, then you create channels to these variables
//! to control then from the business side of your app, that can exist as a multi-thread task.
//!
//! ```
//! # use zero_ui::prelude::*;
//! #[derive(Clone, Debug)]
//! enum Status {
//!     Idle,
//!     Info(Text)
//! }
//!
//! // single var that controls the button.
//! let task_status = var(Status::Idle);
//!
//! let start_btn = button! {
//!     // content derived from the status.
//!     content = text(task_status.map(|s| match s {
//!         Status::Idle => "Start".to_text(),
//!         Status::Info(t) => t.clone()
//!     }));
//!
//!     // `on_click` only works when the button is enabled.
//!     enabled = task_status.map(|s| matches!(s, Status::Idle));
//!
//!     on_click = hn!(|ctx, _| {
//!         // the status sender.
//!         let status = task_status.sender(ctx);
//!         task::spawn(async move {
//!             status.send(Status::Info("Starting..".to_text()));
//!
//!             heavy_lifting(status.clone()).await;             
//!
//!             status.send(Status::Idle);             
//!         });
//!     });
//! };
//!
//! async fn heavy_lifting(VarSender<Status>) {
//!     status.send(Status::Info("Working..".to_text()));
//!     todo!()
//! }
//! ```
//!
//! ## Handlers
//!
//! ## Commands
//!
//! ## Contexts
//!
//! ## Services
//!
//! ## States
//!
//! ## Tasks
//!
//! ## App Extensions
//!
//! # Logging
//!
//! This crate integrates with the [`log`] crate, in debug builds it registers a minimal logger that prints all warmings
//! and errors to `stderr`. You can override this by registering your own logger before starting the app. We recommend only including
//! another logger in release builds, or setting-up your own `stderr` logger for debug builds, this way you don't miss any error or warning.
//!
//! ```
//! # mod log4rs { fn init_file(file: &'static str, config: ()) -> Result<(), ()> { Ok(()) } }
//! use zero_ui::prelude::*;
//!
//! fn main() {
//!     #[cfg(not(debug_assertions))]
//!     log4rs::init_file("log4rs.yml", Default::default()).unwrap();
//!
//!     let app = App::default();
//! }
//! ```
//!
//! # Release Build
//!
//! To build the application for release just use `cargo build --release`, the result is a single portable executable file. Most
//! of Zero-UI dependencies are statically linked, the external dependencies are **OpenGL 3.2** in all systems and **FreeType** plus
//! **FontConfig** in Linux systems. As a rule of thumb if the system can run Firefox it can run your app.
//!
//! ## Windows Subsystem
//!
//! In Windows if you open your executable from the Explorer you will see a **Console Window** alongside your app window.
//! To remove it you need to add `#![windows_subsystem = "windows"]` at the top of your crate's `main.rs`. Except this also stops debug
//! error prints from showing, so we recommend using the `cfg_attr` attribute to only apply the `windows_subsystem` attribute in
//! release builds.
//!
//! ```
//! #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//!
//! use zero_ui::prelude::*;
//!
//! fn main () {
//!     // ..
//! }
//! ```
//!
//! In other operating systems the `windows_subsystem` attribute does nothing and does not cause an error, so you can just copy & paste
//! that attribute line in your crate to support Windows releases.
//!
/*
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
//!
//! [`button!`]: mod@crate::widgets::button
//! [`text()`]: fn@crate::widgets::text::text
//! [`text!`]: mod@crate::widgets::text::text
//! [`v_stack!`]: mod@crate::widgets::layouts::v_stack
//! [`font_size`]: fn@crate::properties::text_theme::font_size
//! [`margin`]: fn@crate::properties::margin
//! [`on_click`]: fn@crate::properties::events::gesture::on_click
//! [`UiNode`]: crate::core::UiNode
//! [`log`]: https://docs.rs/log
//! [`Var<T>`]: crate::core::var::Var
//! [`IntoVar<T>`]: crate::core::var::IntoVar
//! [`var()`]: crate::core::var::var
//! [`var_from()`]: crate::core::var::var_from
//! [`Text`]: crate::core::text::Text
//! [`Vars`]: crate::core::var::Vars
//! [`VarBindingHandle`]: crate::core::var::VarBindingHandle
//! [`SideOffsets`]: crate::core::units::SideOffsets
//! [`RcVar<T>`]: crate::core::var::RcVar
//! [#widget]: macro@crate::core::widget
//! [#property]: macro@crate::core::property
//! [#impl_ui_node]: macro@crate::core::impl_ui_node

// to make the proc-macro $crate substitute work in doc-tests.
#[doc(hidden)]
#[allow(unused_extern_crates)]
extern crate self as zero_ui;

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
    /// # use zero_ui::core::{impl_ui_node, UiNode, BoxedUiNode};
    /// struct MyNode {
    ///     children: Vec<BoxedUiNode>
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
    /// # use zero_ui::core::{impl_ui_node, UiNode, BoxedUiNode};
    /// # struct MyNode { children: Vec<BoxedUiNode> }
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
    /// # use zero_ui::core::{impl_ui_node, UiNode, BoxedUiNode, context::WidgetContext};
    /// # struct MyNode { child: BoxedUiNode }
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
    ///         println!("{}", self.value.get(ctx));
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
    /// as a namespace for any item you wish to add.
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
    /// # Properties
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
    /// Note that the property can be removed during instantiation by using [`remove`](#remove).
    ///
    /// ## Required
    ///
    /// You can mark a property as *required*, meaning, the property must have a value during the widget instantiation,
    /// and the property cannot be unset or removed. To mark the property use the pseudo-attribute `#[required]`.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// # #[widget($crate::foo)]
    /// # pub mod foo {
    /// #   use zero_ui::properties::margin as bar;
    /// properties! {
    ///     #[required]
    ///     bar;
    /// }
    /// # }
    /// ```
    ///
    /// In the example above the required property must be set during the widget instantiation or a compile error is generated.
    /// If another widget inherits from this one is also cannot remove the required property.
    ///
    /// You can also give the required property a default value:
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// # #[widget($crate::foo)]
    /// # pub mod foo {
    /// #   use zero_ui::properties::margin as bar;
    /// properties! {
    ///     #[required]
    ///     bar = 42;
    /// }
    /// # }
    /// ```
    ///
    /// In this case the property does not need to be set during instantiation, but it cannot be unset.
    ///
    /// Note that captured properties are also marked required without the need for the pseudo-attribute.
    ///
    /// ## Remove
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
    ///     remove { content_align }
    /// }
    /// # }
    /// ```
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
    ///     foo(impl IntoVar<bool>) = false;
    /// }
    ///
    /// fn new_child(foo: impl IntoVar<bool>) -> impl UiNode {
    ///     let label = foo.into_var().map(|f|formatx!("foo: {:?}", f));
    ///     text(label)
    /// }
    /// # }
    /// ```
    ///
    /// A property declared like this must be captured by the widget that is declaring it, a compile error is generated if it isn't.
    ///
    /// You can set the property [`allowed_in_when`](zero_ui::core::property#when-integration) value using the pseudo-attribute
    /// `#[allowed_in_when = <bool>]`.
    ///
    /// ### Captures Are Required
    ///
    /// Captured properties are marked as [required](#required) in the widgets that declare then, there is no need to explicitly
    /// annotate then with `#[required]`, for widget instance users it behaves exactly like a required property.
    ///
    /// If the property is not explicitly marked however, widget inheritors can *remove* the property by declaring new
    /// initialization functions that no longer capture the property. If it **is** marked explicitly then in must be captured
    /// by inheritors, even if the source property was not `capture_only`.
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
    /// Widgets have two *groups* of properties, one is understood as applying to the widget, the other as applying to the [*child*](#fn-new_child).
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
    /// ## When
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
    ///     when ST_VALUE && *#{FooVar::new()} && bar() {
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
    /// The [default function](zero_ui::core::widget_base::implicit_base::new_child) does not capture any property and simply outputs
    /// the [`NilUiNode`](zero_ui::core::NilUiNode) value.
    ///
    /// ## `fn new`
    ///
    /// The `new` initialization function defines the outer most type of the widget, if must take at least one input that is a generic
    /// that allows any [`UiNode`](zero_ui::core::UiNode), although not required you probably want to capture the
    /// implicit [`id`](mod@zero_ui::core::widget_base::implicit_base#wp-id) property.
    ///
    /// The output can be any type, if you want the widget to be compatible with most layout slots the type must implement
    /// [`Widget`](zero_ui::core::Widget) and it is recommended that you use the [default function](zero_ui::core::widget_base::implicit_base::new)
    /// to generate the widget.
    ///
    /// The [default function](zero_ui::core::widget_base::implicit_base::new) captures the [`id`](mod@zero_ui::core::widget_base::implicit_base#wp-id)
    /// property and returns a [`Widget`](zero_ui::core::Widget) node that establishes a widget context.
    ///
    /// # `inherit!`
    ///
    /// Widgets can inherit from one other widget and one or more other mix-ins using the pseudo-macro `inherit!(widget::path);`.
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
    /// Subsequent inherits override properties with the same name as previously inherited. Properties
    /// and functions declared in the new widget override inherited items.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::{widget, widget_mixin};
    /// #[widget_mixin($crate::foo)]
    /// pub mod foo {
    ///     properties! {
    ///         zero_ui::properties::margin = 10;
    ///     }
    /// }
    ///
    /// #[widget_mixin($crate::bar)]
    /// pub mod bar {
    ///     properties! {
    ///         zero_ui::properties::margin = 20;
    ///     }
    /// }
    ///
    /// #[widget($crate::foo_bar)]
    /// pub mod foo_bar {
    ///     inherit!(super::foo);
    ///     inherit!(super::bar);
    ///
    ///     fn new_child() -> impl zero_ui::core::UiNode {
    /// #       use zero_ui::widgets::text::text;
    ///         text("Bar!")
    ///     }
    /// }
    /// ```
    ///
    /// In the example above `foo_bar` has a property named `margin` with default value `20`, and its child
    /// is a text widget that prints `"Bar!"`.
    ///
    /// ## Implicit
    ///
    /// Every widget that does not inherit from another widget automatically inherits from
    /// [`implicit_base`](mod@zero_ui::core::widget_base::implicit_base) before all other inherits.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// #[widget($crate::not_empty)]
    /// pub mod not_empty { }
    /// ```
    ///
    /// In the example above `not_empty` contains the properties and new functions defined in the
    /// [`implicit_base`](mod@zero_ui::core::widget_base::implicit_base).
    ///
    /// <div style='display:none'>
    pub use zero_ui_core::widget;

    /// Expands a module to a widget mix-in module.
    ///
    /// Widget mix-ins can only be inherited by other widgets and mix-ins, they cannot be instantiated.
    ///
    /// See the [`#[widget(..)]`][#widget] documentation for how to declare, the only difference
    /// from a full widget is that you can only inherit other mix-ins and cannot declare
    /// the `new_child` and `new` functions.
    /// <div style='display:none'>
    ///
    /// [#widget]: macro@widget
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
        app::App,
        async_clone_move,
        border::{BorderSides, BorderStyle, LineOrientation},
        clone_move,
        color::{
            self, blur, brightness, colors, contrast, drop_shadow, grayscale, hex, hsl, hsla, hue_rotate, opacity, rgb, rgba, saturate,
            sepia, Rgba,
        },
        command::{CommandInfoExt, CommandNameExt},
        context::WidgetContext,
        event::Events,
        focus::{DirectionalNav, Focus, FocusChangedArgs, FocusExt, ReturnFocusChangedArgs, TabIndex, TabNav},
        gesture::{shortcut, ClickArgs, CommandShortcutExt, GestureKey, Shortcut, ShortcutArgs, Shortcuts},
        gradient::{stops, ExtendMode, GradientStop, GradientStops},
        handler::*,
        keyboard::{CharInputArgs, Key, KeyInputArgs, KeyState, ModifiersChangedArgs, ModifiersState},
        mouse::{ButtonState, MouseButton, MouseMoveArgs},
        node_vec, nodes,
        render::WidgetPath,
        service::Services,
        take_if, take_on, take_on_init, task,
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
        var::{merge_var, state_var, switch_var, var, var_from, IntoVar, RcVar, Var, Vars},
        widget_base::Visibility,
        widget_vec, widgets,
        window::{
            AppRunWindowExt, AutoSize, CursorIcon, StartPosition, Window, WindowChrome, WindowCloseRequestedArgs, WindowIcon,
            WindowMoveArgs, WindowOpenArgs, WindowResizeArgs, WindowState, Windows, WindowsExt,
        },
        RcNode, UiNode, UiNodeList, Widget, WidgetId, WidgetList, WidgetVec,
    };

    #[doc(no_inline)]
    pub use crate::properties::*;
    #[doc(no_inline)]
    pub use crate::widgets::*;

    #[doc(no_inline)]
    pub use crate::properties::border::*;
    #[doc(no_inline)]
    pub use crate::properties::commands::*;
    #[doc(no_inline)]
    pub use crate::properties::events::{gesture::*, keyboard::*, mouse::on_mouse_move};
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
    ///         if let Some(new_value) = self.value.get_new(ctx) {
    ///             todo!()
    ///         }
    ///     }
    /// }
    /// ```
    pub mod new_property {
        #[doc(no_inline)]
        pub use crate::core::border::*;
        #[doc(no_inline)]
        pub use crate::core::color::{self, *};
        #[doc(no_inline)]
        pub use crate::core::context::*;
        #[doc(no_inline)]
        pub use crate::core::event::*;
        #[doc(no_inline)]
        pub use crate::core::gesture::*;
        #[doc(no_inline)]
        pub use crate::core::handler::*;
        #[doc(no_inline)]
        pub use crate::core::keyboard::KeyState;
        #[doc(no_inline)]
        pub use crate::core::mouse::ButtonState;
        #[doc(no_inline)]
        pub use crate::core::render::*;
        #[doc(no_inline)]
        pub use crate::core::task::{self, AppTask, WidgetTask};
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
        pub use crate::core::border::*;
        #[doc(no_inline)]
        pub use crate::core::color::*;
        #[doc(no_inline)]
        pub use crate::core::context::*;
        #[doc(no_inline)]
        pub use crate::core::event::*;
        #[doc(no_inline)]
        pub use crate::core::handler::*;
        #[doc(no_inline)]
        pub use crate::core::render::*;
        #[doc(no_inline)]
        pub use crate::core::task::{self, AppTask, WidgetTask};
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
