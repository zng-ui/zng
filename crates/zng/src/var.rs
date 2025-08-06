//! Variables API.
//!
//! The [`Var<T>`] struct represents an observable value. The [`IntoVar<T>`] trait is the primary property input
//! kind and the reason properties inputs are so versatile. Variables can be a simple value, a shared reference to a value or
//! a contextual value, some variables are also derived from others and update when the source variable update.
//!
//! Properties and widgets can subscribe to a variable to update when the variable value changes, this features enables most
//! of the dynamic UI behavior, from binding one widget to another to animation.
//!
//! # Value
//!
//! The simplest variable kind is [`const_var`], it represents an unchanging value that is shared by cloning. All values of types
//! that implement [`VarValue`] automatically convert `IntoVar<T>` to const var, For this reason you don't usually need
//! to write `const_var(_)` when setting properties.
//!
//! ```
//! use zng::prelude::*;
//!
//! fn foo(size: impl IntoVar<layout::Size>) {
//!     let size = size.into_var();
//!     assert!(size.capabilities().is_const());
//!     assert!(size.capabilities().is_always_read_only());
//! }
//!
//! foo(layout::Size::new(10, 10));
//! foo((10, 10));
//! foo(10);
//! ```
//!
//! The example above declares a const `Var<Size>` 3 times with equal value. The `(10, 10)` and `10` values are type conversions
//! implemented by the `Size` type. Type conversions are very easy to implement with the help of the [`impl_from_and_into_var!`] macro,
//! most of the types used by properties implement conversions that enable a form of shorthand syntax.
//!
//! # Share & Modify
//!
//! The [`var`] variable represents a shared value that can be modified.
//!
//! The example below declares a button that grows taller every click. The variable is shared between the height property
//! and the click handler. On click the height is increased, this schedules an update that applies the new value and notifies
//! all subscribers.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! let height = var(2.em());
//! # let _ =
//! Button! {
//!     child = Text!("Taller!");
//!     on_click = hn!(height, |_| { // clone `height` reference for the handler.
//!         height.set(height.get() + 10); // request an update to a new value.
//!     });
//!     layout::align = layout::Align::CENTER;
//!     layout::height; // set the height (shorthand, variable is same name as property)
//! }
//! # ;
//! ```
//!
//! Note that variable updates don't happen immediately, in the handler above the variable is still the previous value after the [`set`](Var::set) call,
//! this is done so that all widgets in a single update react to the same value. The variable values is updated at the end of the current update.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! let number = var(0u8);
//! # let _ =
//! Button! {
//!     child = Text!("Test");
//!     on_click = async_hn!(number, |_| {
//!         assert_eq!(number.get(), 0);
//!         number.set(1);
//!         assert_eq!(number.get(), 0);
//!
//!         task::yield_now().await;
//!         assert_eq!(number.get(), 1);
//!     });
//! }
//! # ;
//! ```
//!
//! The example above demonstrates the delayed update of a variable.
//!
//! If multiple widgets set the same variable on the same update only
//! the last value set will be used, widgets update in parallel by default so it is difficult to predict who is the last. The [`modify`](Var::modify)
//! method can be used register a closure that can modify the value, this closure will observe the partially updated value that may already be
//! modified by other widgets.
//!
//! The example below demonstrates how the `modify` closure observes a value that was just set in the same update cycle.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! let foo = var(0u8);
//! # let _ =
//! Wgt! {
//!     widget::on_init = async_hn!(foo, |_| {
//!         foo.set(1);
//!         assert_eq!(0, foo.get());
//!         foo.modify(|m| {
//!             assert_eq!(1, **m);
//!             **m = 2;
//!         });
//!         assert_eq!(0, foo.get());
//!
//!         foo.wait_update().await;
//!         assert_eq!(2, foo.get());
//!
//!         println!("test ok");
//!     });
//! }
//! # ;
//! ```
//!
//! # Mapping
//!
//! Variables can be mapped to other value types, when the source variable updates the mapping closure re-evaluates and the mapped variable
//! updates, all in the same update cycle, that is both variable will be flagged new at the same time. Mapping can also be bidirectional.
//!
//! The example below demonstrates a button that updates an integer variable that is mapped to a text.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! let count = var(0u32);
//! # let _ =
//! Button! {
//!     child = Text!(count.map(|i| match i {
//!         0 => Txt::from("Click Me!"),
//!         1 => Txt::from("Clicked 1 time!"),
//!         n => formatx!("Clicked {n} times!"),
//!     }));
//!     on_click = hn!(|_| {
//!         count.set(count.get() + 1);
//!     });
//! }
//! # ;
//! ```
//!
//! # Binding
//!
//! Two existing variables can be bound, such that one variable update sets the other. The example below rewrites the mapping
//! demo to use a [`bind_map`](Var::bind_map) instead.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! let count = var(0u32);
//! let label = var(Txt::from("Click Me!"));
//! count
//!     .bind_map(&label, |i| match i {
//!         1 => Txt::from("Clicked 1 time!"),
//!         n => formatx!("Clicked {n} times!"),
//!     })
//!     .perm();
//! # let _ =
//! Button! {
//!     child = Text!(label);
//!     on_click = hn!(|_| {
//!         count.set(count.get() + 1);
//!     });
//! }
//! # ;
//! ```
//!
//! Note that unlike a map the initial value of the output variable is not updated, only subsequent ones. You can use
//! [`set_from`](Var::set_from) to update the initial value too.
//!
//! # Animating
//!
//! Animation is implemented using variables, at the lowest level [`VARS.animate`](VARS::animate) is used to register a closure to be
//! called every frame, the closure can set any number of variables, at a higher level the [`Var::ease`] and [`Var::chase`] methods
//! can be used to animate the value of a variable.
//!
//! The example below uses [`Var::easing`] to animate the window background:
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! let color = var(colors::AZURE.darken(30.pct()));
//! # let _ =
//! Window! {
//!     widget::background_color = color.easing(500.ms(), easing::linear);
//!     child = Button! {
//!         layout::align = layout::Align::TOP;
//!         on_click = hn!(|_|{
//!             let mut c = color::Hsla::from(color.get());
//!             c.hue += 60.0;
//!             color.set(c);
//!         });
//!         child = Text!("Change background color");
//!     }
//! }
//! # ;
//! ```
//!
//! Variables can only be operated by a single animation, when a newer animation or modify affects a variable older animations can no longer
//! affect it, see [`VARS.animate`](VARS::animate) for more details.
//!
//! # Response
//!
//! The [`ResponseVar<T>`] is a specialized variable that represents the result of an async task. You can use `.await` directly
//! in any async handler, but a response var lets you plug a query directly into a property. You can use [`task::respond`] to convert
//! any future into a response var, and you can use [`wait_rsp`] to convert a response var to a future.
//!
//! ```no_run
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! let rsp = task::respond(async {
//!     let url = "https://raw.githubusercontent.com/git/git-scm.com/main/MIT-LICENSE.txt";
//!     match task::http::get_txt(url).await {
//!         Ok(t) => t,
//!         Err(e) => formatx!("{e}"),
//!     }
//! });
//! # let _ =
//! SelectableText!(rsp.map(|r| {
//!     use zng::var::Response::*;
//!     match r {
//!         Waiting => Txt::from("loading.."),
//!         Done(t) => t.clone(),
//!     }
//! }))
//! # ;
//! ```
//!
//! The example above creates a response var from a download future and maps the response to a widget.
//!
//! A response var is paired with a [`ResponderVar<T>`], you can create a *response channel* using the [`response_var`] function.
//!
//! [`task::respond`]: crate::task::respond
//! [`wait_rsp`]: ResponseVar::wait_rsp
//!
//! # Merge
//!
//! The [`merge_var!`] and [`expr_var!`] macros can be used to declare a variable that merges multiple other variable values.
//!
//! The example below demonstrates the two macros.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! let a = var(10u32);
//! let b = var(1u32);
//!
//! // let merge = expr_var!({
//! //     let a = *#{a};
//! //     let b = *#{b.clone()};
//! //     formatx!("{a} + {b} = {}", a + b)
//! // });
//! let merge = merge_var!(a, b.clone(), |&a, &b| {
//!     formatx!("{a} + {b} = {}", a + b)
//! });
//! # let _ =
//! Button! {
//!     child = Text!(merge);
//!     on_click = hn!(|_| b.set(b.get() + 1));
//! }
//! # ;
//! ```
//!
//! # Contextual
//!
//! The [`ContextVar<T>`] type represents a variable that has context depend value, meaning they can produce a different value depending
//! on where they are used. Context vars are declared using the [`context_var!`] macro.
//!
//! The example below declares a context var and a property that sets it. The context var is then used in two texts with
//! two different contexts, the first text will show "Text!", the second will show "Stack!".
//!
//! ```
//! # fn main() { }
//! use zng::prelude::*;
//!
//! context_var! {
//!     static FOO_VAR: Txt = "";
//! }
//!
//! #[zng::widget::property(CONTEXT, default(FOO_VAR))]
//! pub fn foo(child: impl IntoUiNode, foo: impl IntoVar<Txt>) -> UiNode {
//!     zng::widget::node::with_context_var(child, FOO_VAR, foo)
//! }
//!
//! fn demo() -> UiNode {
//!     Stack! {
//!         direction = StackDirection::top_to_bottom();
//!         spacing = 5;
//!         foo = "Stack!";
//!         children = ui_vec![
//!             Text! {
//!                 txt = FOO_VAR;
//!                 foo = "Text!";
//!             },
//!             Text!(FOO_VAR),
//!         ]
//!     }
//! }
//! ```
//!
//! Context variables have all the same capabilities of other variables if the example if `foo` is set to a [`var`]
//! the context var will be editable, and if `FOO_VAR` is mapped the mapping variable is also contextual.
//!
//! # Full API
//!
//! See [`zng_var`] for the full var API.

pub use zng_var::{
    AnyVar, AnyVarValue, AnyWhenVarBuilder, ArcEq, BoxAnyVarValue, ContextInitHandle, ContextVar, IntoValue, IntoVar, MergeVarBuilder,
    ObservableVec, ResponderVar, Response, ResponseVar, VARS, Var, VarCapability, VarEq, VarHandle, VarHandles, VarHookArgs,
    VarInstanceTag, VarModify, VarUpdateId, VarValue, VecChange, WeakAnyVar, WeakVar, any_var_derived, const_var, context_var,
    contextual_var, expr_var, impl_from_and_into_var, merge_var, response_done_var, response_var, var, var_default, var_from, var_getter,
    var_state, when_var,
};

pub use zng_app::widget::{AnyVarSubscribe, OnVarArgs, VarLayout, VarSubscribe};

/// Var animation types and functions.
pub mod animation {
    pub use zng_var::animation::{
        Animation, AnimationController, AnimationHandle, ChaseAnimation, ForceAnimationController, ModifyInfo, Transition, TransitionKeyed,
        Transitionable, WeakAnimationHandle,
    };

    /// Common easing functions.
    pub mod easing {
        pub use zng_var::animation::easing::{
            Bezier, EasingFn, EasingStep, EasingTime, back, bounce, circ, cubic, cubic_bezier, ease_in, ease_in_out, ease_out, ease_out_in,
            elastic, expo, linear, none, quad, quart, quint, reverse, reverse_out, sine, step_ceil, step_floor,
        };
    }
}
