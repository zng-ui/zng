//! Variables API.
//!
//! The [`Var<T>`] trait represents an observable value. The [`IntoVar<T>`] trait is the primary property input
//! kind and the reason setting properties is so versatile. Variables can be a simple value, a shared reference to a value or
//! a contextual value, some variables are also derived from others and update when the source variable update.
//!
//! Properties and widgets can subscribe to a variable to update when the variable value changes, this features enables most
//! of the dynamic UI behavior, from binding one widget to another to animation.
//!
//! # Value
//!
//! The simplest variable is [`LocalVar<T>`], it represents an unchanging value that is shared by cloning. All values of types
//! that implement [`VarValue`] automatically convert `IntoVar<T>` to this variable type. For this reason you don't usually need
//! to write it.
//!
//! ```
//! use zero_ui::prelude::*;
//!
//! fn local(size: impl IntoVar<layout::Size>) {
//!     let size = size.into_var();
//!     assert!(size.capabilities().is_always_static());
//!     assert!(size.capabilities().is_always_read_only());
//! }
//!
//! local(layout::Size::new(10, 10));
//! local((10, 10));
//! local(10);
//! ```
//!
//! The example above declares a `LocalVar<Size>` 3 times with equal value. The `(10, 10)` and `10` values are type conversions
//! implemented by the `Size` type. Type conversions are very easy to implement with the help of the [`impl_from_and_into_var!`] macro,
//! most of the types used by properties implement conversions that enable a form of shorthand syntax.
//!
//! # Share & Modify
//!
//! The [`ArcVar<T>`] variable represents a shared value that can be modified, the [`var`] function instantiates it.
//! 
//! The example below declares a button that grows taller every click. The variable is shared between the height property
//! and the click handler. On click the height is increased, this schedules an update that applies the new value and notifies
//! all subscribers.
//! 
//! ```
//! use zero_ui::prelude::*;
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
//! use zero_ui::prelude::*;
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
//! The example above demonstrates the delayed update of a variable. If multiple widgets set the same variable on the same update only
//! the last value set will be used, widgets update in parallel by default so it is difficult to predict who is the last. The [`modify`](Var::modify)
//! method can be used register a closure that can modify the value, this closure will observe the partially updated value that may already be
//! modified by other widgets.
//! 
//! ```
//! !!: TODO
//! ```
//! 
//! # Mapping
//! 
//! !!:
//! 
//! # Binding
//! 
//! !!:
//! 
//! # Animating
//! 
//! !!:
//! 
//! # Response
//! 
//! !!:
//! 
//! # Contextual
//! 
//! !!:
//! 
//! # Merge
//!
//! !!:
//! 
//! # Full API
//!
//! See [`zero_ui_var`] for the full var API.

pub use zero_ui_var::types::{
    AnyWhenVarBuilder, ArcCowVar, ArcWhenVar, ContextualizedVar, ReadOnlyVar, Response, VecChange, WeakArcVar, WeakContextInitHandle,
    WeakContextualizedVar, WeakReadOnlyVar, WeakWhenVar,
};
pub use zero_ui_var::{
    context_var, expr_var, getter_var, impl_from_and_into_var, merge_var, response_done_var, response_var, state_var, var, var_default,
    var_from, when_var, AnyVar, AnyVarValue, AnyWeakVar, ArcEq, ArcVar, BoxedAnyVar, BoxedAnyWeakVar, BoxedVar, BoxedWeakVar,
    ContextInitHandle, ContextVar, IntoValue, IntoVar, LocalVar, MergeVarBuilder, ObservableVec, ReadOnlyArcVar, ReadOnlyContextVar,
    ResponderVar, ResponseVar, TraceValueArgs, Var, VarCapabilities, VarHandle, VarHandles, VarHookArgs, VarModify, VarPtr, VarUpdateId,
    VarValue, WeakVar, VARS,
};

pub use zero_ui_app::widget::{AnyVarSubscribe, VarLayout, VarSubscribe};

/// Var animation types and functions.
pub mod animation {
    pub use zero_ui_var::animation::{
        Animation, AnimationController, AnimationHandle, AnimationTimer, ChaseAnimation, ModifyInfo, NilAnimationObserver, Transition,
        TransitionKeyed, Transitionable, WeakAnimationHandle,
    };

    /// Common easing functions.
    pub mod easing {
        pub use zero_ui_var::animation::easing::{
            back, bounce, circ, cubic, cubic_bezier, ease_in, ease_in_out, ease_out, ease_out_in, elastic, expo, linear, none, quad, quart,
            quint, reverse, reverse_out, sine, step_ceil, step_floor, Bezier, EasingFn, EasingModifierFn, EasingStep, EasingTime,
        };
    }
}
