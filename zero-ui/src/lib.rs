#![warn(unused_extern_crates)]
// examples of `widget! { .. }` and `#[property(..)]` need to be declared
// outside the main function, because they generate a `mod` with `use super::*;`
// that does not import `use` clauses declared inside the parent function.
#![allow(clippy::needless_doctest_main)]
#![warn(missing_docs)]
#![cfg_attr(doc_nightly, feature(doc_cfg))]
#![cfg_attr(doc_nightly, feature(doc_notable_trait))]
// suppress nag about very simple boxed closure signatures.
#![allow(clippy::type_complexity)]

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
//! zero-ui = "0.1"
//! zero-ui-view = "0.1"
//! ```
//!
//! Then create your first window:
//!
//! ```no_run
//! # mod zero_ui_view { pub fn init() { } }
//! use zero_ui::prelude::*;
//!
//! fn main() {
//!     zero_ui_view::init();
//!
//!     App::default().run_window(|_| {
//!         let size = var_from((800, 600));
//!         window! {
//!             title = size.map(|s: &Size| formatx!("Button Example - {s}"));
//!             size;
//!             content = button! {
//!                 on_click = hn!(|_,_| {
//!                     println!("Button clicked!");
//!                 });
//!                 margin = 10;
//!                 size = (300, 200);
//!                 align = Align::CENTER;
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
//! the high level blocks compile down to the most basic at zero-cost. This can be surprising when you see put together:
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
//! The example demonstrates the [`button!`] widget, you may think that [`on_click`] and [`font_size`] are implemented in the widget,
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
//! `is_foo` indicates a property that reports a widget state.
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
//! #[property(layout)]
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
//!         offset.modify(ctx, |mut m|m.left += 50.0);
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
//!         flag.modify(ctx.vars, move |mut f| *f = new_value);
//!     });
//! };
//! ```
//!
//! See the **[`Var<T>`]** documentation for indebt information about accessing variable values.
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
//!             n => formatx!("Clicked {n} Times!")
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
//!             n => formatx!("Clicked {n} Times!")
//!         }
//!     });
//!     handle.perm();
//!     window! {
//!         content = button! {
//!             content = text(count_text);
//!             on_click = hn!(|ctx, _| {
//!                 count.modify(ctx, |mut c| *c += 1);
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
//!             status.send(Status::Info("Starting..".to_text())).ok();
//!
//!             heavy_lifting(status.clone()).await;             
//!
//!             status.send(Status::Idle).ok();             
//!         });
//!     });
//! };
//!
//! async fn heavy_lifting(status: VarSender<Status>) {
//!     status.send(Status::Info("Working..".to_text())).ok();
//!     todo!()
//! }
//! ```
//!
//! ## Event Handlers
//!
//! Events are unit structs that implement [`Event`], they represent an action or occurrence such as a key press or an window opening.
//! Events are naturally **broadcast**, every window and widget *receives* every event message. The event messages are
//! structs that implement [`EventArgs`], and these *arguments* are delivered to all widgets in its [`delivery_list`].
//! Early listeners can also use the [`EventArgs`] to signal later listeners that an event has been
//! handled by calling [`propagation().stop()`].
//!
//! You usually don't setup a widget event handler directly, but instead use a property that does the message filtering and only
//! calls your handler if the message is valid in the widget and not already handled. These *event properties* follow a common pattern,
//! each is named with the `on_` prefix and each receive an [`WidgetHandler<T>`] as input. The handler is essentially a closure that
//! takes the [`WidgetContext`] and event arguments as input, the handler trait allows properties to receive both **mut** and **once**
//! closures as well as both sync and **async** closures. These closures can be declared using macros that also enable ***clone-move***
//! capturing.
//!
//! ### Synchronous Handlers
//!
//! Synchronous event handlers block the UI thread when they are running, only after they return the next handler is called.
//! They can spawn async parallel tasks but they cannot `.await`. You can use the [`hn!`] and [`hn_once!`] macros to declare these
//! handlers, they essentially declare a `FnMut` and `FnOnce` that capture by `move`. The macros can be used to *capture-by-clone* too.
//!
//! ```
//! use zero_ui::prelude::*;
//! let mut count = 0;
//!
//! button! {
//!     on_click = hn!(|_, _| {
//!         count += 1;
//!         println!("Clicked {count} time{}", if count > 1 { "s" } else { "" });
//!     });
//!     content = text("Click Me!");
//! }
//! # ;
//! ```
//!
//! The [`hn_once!`] is very similar to write but it will only handle the event one time and you can consume the captured values during
//! the call. In the example below the captured `data` can be dropped inside the handler because it will only run once.
//!
//! ```
//! # use zero_ui::prelude::*;
//! let mut count = 0;
//! let data = vec![0, 1, 0];
//!
//! # button! {
//! on_click = hn_once!(|_, _| {
//!     count += 1;
//!     assert_eq!(1, count);
//!     drop(data);
//! });
//! #   content = text("Click Me!");
//! # }
//! # ;
//! ```
//!
//! The first parameter is an `&mut` exclusive borrow of the [`WidgetContext`], the second parameter is a `&` shared borrow of the
//! event arguments. You can use the first parameter just by naming it, but to use the second parameter must declare the arguments
//! type before you can use it. This is due to a limitation in Rust's type inference.
//!
//! ```
//! # use zero_ui::prelude::*;
//! # button! {
//! on_click = hn!(|ctx, args: &ClickArgs| {
//!     args.propagation().stop();
//!     println!("Click handled by {}", args.target);
//! });
//! #   content = text("Click Me!");
//! # }
//! # ;
//! ```
//!
//! The handler macros can all *capture-by-move*, named variables are cloned before moving into the closure, so the original
//! variable is still free to use after the handler is declared.
//!
//! ```
//! # use zero_ui::prelude::*;
//! let count = var(0u32);
//!
//! button! {
//!     on_click = hn!(count, |ctx, _| {
//!         count.modify(ctx, |mut c| *c += 1);
//!     });
//!     content = text(count.map_to_text());
//! }
//! # ;
//! ```
//!
//! ### Async Handlers
//!
//! Asynchronous event handlers can use `.await` to yield execution to the next handler without finishing handling the event. If
//! your handler code depends on the result of an IO, network or compute operation, you should use an async handler. The [`async_hn!`]
//! and [`async_hn_once!`] macros can be used to declare async handlers.
//!
//! ```
//! # use zero_ui::prelude::*;
//! # let status = var("Waiting Click..".to_text());
//! button! {
//!     on_click = async_hn!(status, |ctx, _| {
//!         status.set(&ctx, "Loading..");
//!         match task::wait(|| std::fs::read("some/data")).await {
//!             Ok(data) => {
//!                 status.set(&ctx, formatx!("Loaded {} bytes. Saving..", data.len()));
//!                 task::wait(move || std::fs::write("data.bin", data)).await;
//!                 status.set(&ctx, "Done.");
//!             },
//!             Err(e) => status.set(&ctx, e.to_text()),
//!         }
//!     });
//!#    content = text("Save");
//! }
//! # ;
//! ```
//!
//! The first parameter is an [`WidgetContextMut`] value, the second parameter is a clone of the event arguments. Like [`hn!`] you
//! can use the first parameter just by naming it, but the second parameter must declare the arguments type.
//!
//! ```
//! # use zero_ui::prelude::*;
//! # let status = var("Waiting Double Click..".to_text());
//! # async fn some_task(status: RcVar<Text>) { }
//! button! {
//!     on_click = async_hn!(status, |ctx, args: ClickArgs| {
//!         if args.is_double() {
//!             some_task(status.clone()).await;
//!             status.set(&ctx, "Done.");
//!         }
//!     });
//!#    content = text("Run");
//! }
//! # ;
//! ```
//!
//! Like [`hn!`] the macro closure captures by `move` and can be used to *capture-by-clone*. This feature is even more important
//! in async closures due to the fact they spawn [futures] that also capture by move, when a variable is captured by clone it is
//! automatically cloned again for each handler call making the variable accessible to potentially more then one handler task at
//! the same time.
//!
//! You can use [`async_hn_once!`] to avoid this second cloning, in this case the event is only handled once so any captured
//! data can be safely moved in the async task, and the task can move the data further.
//!
//! ```
//! # use zero_ui::prelude::*;
//! # let status = var("Waiting Click..".to_text());
//! let data = vec![0, 1];
//! button! {
//!     on_click = async_hn_once!(|ctx, _| {
//!         task::wait(move || std::fs::write("data.bin", data)).await;
//!     });
//!#    content = text("Save");
//! }
//! # ;
//! ```
//!
//! ### Event Routes
//!
//! Event properties are usually declared in pairs, a *on_event* and a *on_pre_event*. The *pre* event is the **preview**, it is
//! called before the main event and can be used to stop the main handler from seeing the event using [`propagation().stop()`] method
//! that is available in all event arguments.
//!
//! ```
//! # use zero_ui::prelude::*;
//! button! {
//!     on_pre_click = hn!(|_, a: &ClickArgs|{
//!         if a.is_double() {
//!             a.propagation().stop();
//!         }
//!     });
//!     on_click = hn!(|_, a: &ClickArgs|{
//!         assert!(!a.is_double());
//!         println!("Clicked!");
//!     });
//! #   content = text("!");
//! }
//! # ;
//! ```
//!
//! The preview handlers are called before the widget content receives the event message and the main handlers are called after.
//! Most event arguments provide a [`delivery_list`] that targets a widget **and** its parent widgets. This
//! makes the event propagation follow a **route** in the UI tree. Starting from the window root, every widget all the way to the
//! target widget gets to *preview* the event, if none stops the propagation the main handlers are called, first in the target
//! widget and then all the way back to the window.
//!
//! ```
//! # use zero_ui::prelude::*;
//! window! {
//!     on_pre_click = hn!(|_, _| println!("window.on_pre_click"));
//!     on_click = hn!(|_, _| println!("window.on_click"));
//!
//!     content = container! {
//!         on_pre_click = hn!(|_, _| println!("container.on_pre_click"));
//!         on_click = hn!(|_, _| println!("container.on_click"));
//!
//!         content = button! {
//!             on_pre_click = hn!(|_, _| println!("button.on_pre_click"));
//!             on_click = hn!(|_, _| println!("button.on_click"));
//!
//!             content = text("Click Me!");
//!         };
//!     };
//! }
//! # ;
//! ```
//! A Click in the button prints:
//! ```text
//! window.on_pre_click
//! container.on_pre_click
//! button.on_pre_click
//! button.on_click
//! container.on_click
//! window.on_click
//! ```
//!
//! The preview route is sometimes called *tunneling* or *capturing* and the main route is sometimes called *bubbling*. Not
//! all event properties exist in these two routes, some events are *direct*, meaning they exist in the scope of a widget only,
//! the preview handler is called and then the main handler, but only in the same widget. And finally some rare events are
//! unfiltered and visible in all widgets, this is a *broadcast* event, each window receives the event, *oldest-first*, and in
//! each window every widget receives the event, *depth-first*, the preview handlers in this case only preview their branch
//! of the UI tree.
//!
//! ## Commands
//!
//! Command are unit structs that implement [`Command`] and [`Event`]. They are special events that represent an app action and do
//! not have a predefined *emitter*. Widgets can implement command handlers allowing then to be controlled from user interactions
//! that are implemented in a different widget. Commands types have associated metadata that can be used for communication
//! between handlers and emitters, or for enabling new behavior. Every command type has [`enabled`][cmd_enabled] and [`has_handlers`] variables
//! but extra metadata can be added, using extension traits, most commands have a [`name`][cmd_name] and [`info`][cmd_info] and the gesture module
//! provides a [`shortcut`][cmd_shortcut] that enables command activation using shortcut presses.
//!
//! ```
//! use zero_ui::prelude::*;
//!
//! button! {
//!     on_click = hn!(|ctx, _| { CopyCommand.notify(ctx, None); });
//!     content = text(CopyCommand.name());
//!     enabled = CopyCommand.enabled();
//!     visibility = CopyCommand.has_handlers().map_into();
//! }
//! # ;
//! ```
//!  
//! The example above declares a "Copy" button, the button causes a copy operation on click, but it does not known what
//! is copied, or how. If there is any [`CopyCommand`] handlers created the button will be visible and if any of these handlers is enabled
//! the button will be enabled. The button content uses the default display name provided by the [`CopyCommand`].
//!
//! Not shown in the example is the fact that the [`CopyCommand`] has default [`shortcut`][cmd_shortcut] values too, so pressing "Ctrl+C"
//! will also notify the command, because the [`GestureManager`] implements this interaction for all enabled commands that have a shortcut.
//!
//! See the **[`command`]** module for more information, including how to declare new commands, modify command metadata and how to handle a command event.
//!
//! ## Contexts
//!
//! A simplified overview of the memory ownership in an app is, every widget is owned by their parent widget, and the root widget is owned
//! by their parent window and every window is owned by the app. When you have a mutable reference in an event handler there is a borrow chain
//! that goes all the way up to the running app, data from the parent widget, window and app may be of interest for the event handler code.
//! These *contextual borrows* are packed in a **context struct**.
//!
//! ```
//! # use zero_ui::prelude::*;
//! # fn test() -> impl WidgetHandler<()> {
//! # let foo_var = var(true);
//! # static FOO_ID: zero_ui::core::context::StaticStateId<bool> = zero_ui::core::context::StaticStateId::new_unique();
//! hn!(|ctx, _| {
//!     let value_ref = foo_var.get(ctx.vars);
//!     let service_ref = Windows::req(ctx.services);
//!     let state_ref = ctx.widget_state.get(&FOO_ID);
//! })
//! # }
//! ```
//!
//! The most used context structs are the [`WidgetContext`] and [`AppContext`], but all contexts follow the same pattern, they are shared with
//! an `&mut` exclusive borrow and contains public members that are also borrows for the shared data. The members are public to allow partial
//! borrows of the context, so that a variable and a service can be borrowed at the same time, more specialized contexts have all
//! the general data and add the local parent's data, the [`WidgetContext`] shares all the data from the window parent's [`AppContext`] but
//! also from the immediate parent widget.
//!
//! See the **[`context`]** module for more information about all the context structs.
//!
//! ## Services
//!
//! A service is a type with an instance registered in [`Services`]. The [`Services`] collection is available in most context structs,
//! and service instances are usually registered during the app initialization by the app extensions, so services are singletons that
//! represent a set of operations and states, it is a very generalized concept, for example, the [`Windows`] service is used to open
//! and close windows, list the open windows and control if the app is exited when all windows are closed.
//!
//! ```
//! use zero_ui::prelude::*;
//!
//! # let _ =
//! button! {
//!     content = text("Open Window");
//!     on_click = hn!(|ctx, _| {
//!         Windows::req(ctx).open(|_| window! {
//!             content = text("Hello!");
//!         });
//!     });
//! }
//! # ;
//! ```
//!
//! The example above, requests the [`Windows`] service, and then creates an [`open`][win_open] request.
//!
//! Acquiring a service reference exclusively borrows the [`Services`], but you can borrow more then one service at the same time:
//!
//! ```
//! # use zero_ui::prelude::*;
//! # use zero_ui::core::keyboard::Keyboard;
//! # use zero_ui::core::mouse::Mouse;
//! # fn test(ctx: &mut WidgetContext) {
//! let (mouse, keyboard) = <(Mouse, Keyboard)>::req(ctx);
//! # }
//! ```
//!
//! There is a quick runtime check that the types are not the same in this case, otherwise like borrowing a single service, it compiles
//! down to a direct reference to the service instances.
//!
//! You can learn more about services in the documentation of the **[`service`]** module.
//!
//! ## States
//!
//!
//!
//! ## Tasks
//!
//!  
//!
//! ## App Extensions
//!
//!  
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
//! [`button!`]: mod@crate::widgets::button
//! [`text()`]: fn@crate::widgets::text
//! [`text!`]: mod@crate::widgets::text
//! [`v_stack!`]: mod@crate::widgets::layouts::v_stack
//! [`font_size`]: fn@crate::widgets::text::properties::font_size
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
//! [`Event`]: crate::core::event::Event
//! [`EventArgs`]: crate::core::event::EventArgs
//! [`delivery_list`]: crate::core::event::EventArgs::delivery_list
//! [`propagation().stop()`]: crate::core::event::EventPropagationHandle::stop
//! [`WidgetHandler<T>`]: crate::core::handler::WidgetHandler
//! [`hn!`]: macro@crate::core::handler::hn
//! [`hn_once!`]: macro@crate::core::handler::hn_once
//! [`async_hn!`]: macro@crate::core::handler::async_hn
//! [`async_hn_once!`]: macro@crate::core::handler::async_hn_once
//! [`WidgetContext`]: crate::core::context::WidgetContext
//! [`WidgetContextMut`]: crate::core::context::WidgetContextMut
//! [`Command`]: crate::core::command::Command
//! [`has_handlers`]: crate::core::command::Command::has_handlers
//! [cmd_enabled]: crate::core::command::Command::enabled
//! [cmd_name]: crate::core::command::CommandNameExt::name
//! [cmd_info]: crate::core::command::CommandInfoExt::info
//! [cmd_shortcut]: crate::core::gesture::CommandShortcutExt::shortcut
//! [`CopyCommand`]: crate::properties::commands::CopyCommand
//! [`GestureManager`]: crate::core::gesture::GestureManager
//! [futures]: std::future::Future
//! [`AppContext`]: crate::core::context::AppContext
//! [`context`]: crate::core::context
//! [`Service`]: crate::core::service::Service
//! [`Services`]: crate::core::service::Services
//! [`Windows`]: crate::core::window::Windows
//! [win_open]: crate::core::window::Windows::open

// to make the proc-macro $crate substitute work in doc-tests.
#[doc(hidden)]
#[allow(unused_extern_crates)]
extern crate self as zero_ui;

#[doc(no_inline)]
pub use zero_ui_core as core;

pub(crate) mod crate_util;
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
/// There are prelude modules for other contexts, [`new_property`] for
/// creating new properties, [`new_widget`] for creating new widgets.
///
/// The [`rayon`] crate's prelude is inlined in the preludes.
///
/// [`new_property`]: crate::prelude::new_property
/// [`new_widget`]: crate::prelude::new_widget
/// [`rayon`]: https://docs.rs/rayon
pub mod prelude {
    #[cfg(feature = "http")]
    #[doc(no_inline)]
    pub use crate::core::task::http::Uri;

    #[doc(no_inline)]
    pub use crate::core::{
        app::App,
        async_clone_move,
        border::{BorderSides, BorderStyle, LineOrientation, LineStyle},
        clone_move,
        color::{self, colors, filters, hex, hsl, hsla, rgb, rgba, Rgba},
        command::{Command, CommandArgs, CommandInfoExt, CommandNameExt, CommandScope},
        context::{AppContext, WidgetContext, WindowContext},
        event::Events,
        focus::{DirectionalNav, Focus, FocusChangedArgs, ReturnFocusChangedArgs, TabIndex, TabNav},
        gesture::{shortcut, ClickArgs, CommandShortcutExt, GestureKey, Shortcut, ShortcutArgs, Shortcuts},
        gradient::{stops, ExtendMode, GradientStop, GradientStops},
        handler::*,
        image::ImageSource,
        keyboard::{CharInputArgs, Key, KeyInputArgs, KeyState, ModifiersChangedArgs, ModifiersState},
        mouse::{ButtonState, MouseButton, MouseMoveArgs},
        node_vec, nodes,
        render::RenderMode,
        service::{ServiceTuple, Services},
        task::{self, rayon::prelude::*},
        text::{
            font_features::{
                CapsVariant, CharVariant, CnVariant, EastAsianWidth, FontPosition, FontStyleSet, JpVariant, NumFraction, NumSpacing,
                NumVariant,
            },
            formatx, lang, FontFeatures, FontName, FontNames, FontStretch, FontStyle, FontWeight, Fonts, Hyphens, LineBreak, Text,
            TextAlign, TextTransformFn, ToText, UnderlinePosition, UnderlineSkip, WhiteSpace, WordBreak,
        },
        ui_list::{z_index, SortedWidgetVec, SortedWidgetVecRef, WidgetVec, WidgetVecRef, ZIndex},
        units::{
            rotate, scale, scale_x, scale_xy, scale_y, skew, skew_x, skew_y, translate, translate_x, translate_y, Align, AngleUnits,
            ByteUnits, EasingStep, EasingTime, FactorUnits, Length, LengthUnits, Line, LineFromTuplesBuilder, LineHeight, Point, Px,
            PxPoint, PxSize, Rect, RectFromTuplesBuilder, SideOffsets, Size, TimeUnits, Transform, Vector,
        },
        var::{
            animation, easing, expr_var, merge_var, state_var, switch_var, var, var_default, var_from, IntoVar, RcVar, Var, VarReceiver,
            VarSender, VarValue, Vars, VarsRead,
        },
        widget_base::HitTestMode,
        widget_info::{InteractionPath, Visibility},
        widget_vec, widgets,
        window::{
            AppRunWindowExt, AutoSize, CursorIcon, FocusIndicator, HeadlessAppWindowExt, MonitorId, MonitorQuery, StartPosition,
            WidgetInfoChangedEvent, Window, WindowChangedArgs, WindowChrome, WindowCloseRequestedArgs, WindowIcon, WindowId,
            WindowOpenArgs, WindowState, WindowTheme, WindowVars, Windows,
        },
        FillUiNode, NilUiNode, RcNode, UiNode, UiNodeList, Widget, WidgetId, WidgetList, WidgetPath,
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
    pub use crate::properties::transform::{transform, *};
    #[doc(no_inline)]
    pub use crate::widgets::text::properties::{
        font_family, font_size, font_stretch, font_style, font_weight, letter_spacing, line_height, tab_length, text_align, text_color,
        text_transform, word_spacing, TEXT_COLOR_VAR,
    };

    #[doc(no_inline)]
    pub use crate::widgets::image::properties::ImageFit;
    #[doc(no_inline)]
    pub use crate::widgets::layouts::*;
    #[doc(no_inline)]
    pub use crate::widgets::scroll::ScrollMode;
    #[doc(no_inline)]
    pub use crate::widgets::theme::theme_generator;
    #[doc(no_inline)]
    pub use crate::widgets::window::{nodes::WINDOW_THEME_VAR, AnchorMode, LayerIndex, WindowLayers};

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
        pub use crate::core::command::*;
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
        pub use crate::core::task::{self, rayon::prelude::*, ui::AppTask, ui::WidgetTask};
        #[doc(no_inline)]
        pub use crate::core::text::Text;
        #[doc(no_inline)]
        pub use crate::core::units::{self, *};
        #[doc(no_inline)]
        pub use crate::core::var::*;
        #[doc(no_inline)]
        pub use crate::core::widget_base::HitTestMode;
        #[doc(no_inline)]
        pub use crate::core::window::{WidgetInfoChangedEvent, WindowId};
        #[doc(no_inline)]
        pub use crate::core::{
            impl_ui_node, node_vec, nodes, property,
            ui_list::{SortedWidgetVec, SortedWidgetVecRef, UiListObserver, UiNodeList, WidgetList, WidgetVec, WidgetVecRef},
            widget,
            widget_base::interactive_node,
            widget_info::{
                InteractionPath, Interactivity, Visibility, WidgetBorderInfo, WidgetBoundsInfo, WidgetInfoBuilder, WidgetLayout,
                WidgetSubscriptions,
            },
            widget_mixin, widget_vec, widgets, BoxedUiNode, BoxedWidget, FillUiNode, UiNode, Widget, WidgetId,
        };
        #[doc(no_inline)]
        pub use crate::widgets::{layouts::stack_nodes, view_generator, DataUpdate, ViewGenerator};
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
        pub use crate::core::command::*;
        #[doc(no_inline)]
        pub use crate::core::context::*;
        #[doc(no_inline)]
        pub use crate::core::event::*;
        #[doc(no_inline)]
        pub use crate::core::handler::*;
        #[doc(no_inline)]
        pub use crate::core::image::Image;
        #[doc(no_inline)]
        pub use crate::core::render::*;
        #[doc(no_inline)]
        pub use crate::core::task::{self, rayon::prelude::*, ui::AppTask, ui::WidgetTask};
        #[doc(no_inline)]
        pub use crate::core::text::*;
        #[doc(no_inline)]
        pub use crate::core::units::*;
        #[doc(no_inline)]
        pub use crate::core::var::*;
        #[doc(no_inline)]
        pub use crate::core::window::{CursorIcon, WidgetInfoChangedEvent, WindowId};
        #[doc(no_inline)]
        pub use crate::core::{
            impl_ui_node, node_vec, nodes, property,
            ui_list::{
                z_index, SortedWidgetVec, SortedWidgetVecRef, UiListObserver, UiNodeList, WidgetList, WidgetVec, WidgetVecRef, ZIndex,
                ZSortedWidgetList,
            },
            widget,
            widget_base::{implicit_base, HitTestMode},
            widget_info::{
                InteractionPath, Interactivity, Visibility, WidgetBorderInfo, WidgetBoundsInfo, WidgetInfo, WidgetInfoBuilder,
                WidgetLayout, WidgetSubscriptions,
            },
            widget_mixin, widget_vec, widgets,
            window::WindowTheme,
            BoxedUiNode, BoxedWidget, DynWidget, FillUiNode, UiNode, Widget, WidgetId,
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
        pub use crate::properties::transform::{transform, *};
        #[doc(no_inline)]
        pub use crate::properties::*;
        #[doc(no_inline)]
        pub use crate::widgets::mixins::*;
        #[doc(no_inline)]
        pub use crate::widgets::text::properties::{
            font_family, font_size, font_stretch, font_style, font_weight, letter_spacing, line_height, tab_length, text_align, text_color,
            text_transform, word_spacing,
        };
        #[doc(no_inline)]
        pub use crate::widgets::{
            container, element,
            layouts::{stack_nodes, stack_nodes_layout_by},
            themable, theme,
            theme::ThemeGenerator,
            view_generator,
            window::nodes::WINDOW_THEME_VAR,
            DataUpdate, ViewGenerator,
        };
    }
}

/// Standalone documentation.
///
/// This module contains empty modules that hold *integration docs*, that is
/// documentation that cannot really be associated with API items because they encompass
/// multiple items.
pub mod docs {
    /// `README.md`
    ///
    #[doc = include_str!("../../README.md")]
    pub mod readme {}

    /// `CHANGELOG.md`
    ///
    #[doc = include_str!("../../CHANGELOG.md")]
    pub mod changelog {}
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
