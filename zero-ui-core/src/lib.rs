#![warn(unused_extern_crates)]
// examples of `widget! { .. }` and `#[property(..)]` need to be declared
// outside the main function, because they generate a `mod` with `use super::*;`
// that does not import `use` clauses declared inside the parent function.
#![allow(clippy::needless_doctest_main)]
#![warn(missing_docs)]

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
pub mod border;
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
pub mod task;
pub mod text;
pub mod timer;
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
pub use zero_ui_proc_macros::{property_new, widget_declare, widget_inherit, widget_new};

/// Gets if the value indicates that any size is available during layout (positive infinity)
// TODO move to units
#[inline]
pub fn is_layout_any_size(f: f32) -> bool {
    f.is_infinite() && f.is_sign_positive()
}

/// Value that indicates that any size is available during layout.
pub const LAYOUT_ANY_SIZE: f32 = f32::INFINITY;

/// A map of TypeId -> Box<dyn UnsafeAny>.
type AnyMap = fnv::FnvHashMap<std::any::TypeId, Box<dyn unsafe_any::UnsafeAny>>;

pub use zero_ui_proc_macros::{impl_ui_node, property, widget, widget_mixin};

mod tests;

/// Cloning closure.
///
/// A common pattern when creating widgets is a [variable](crate::var::var) that is shared between a property and an event handler.
/// The event handler is a closure but you cannot just move the variable, it needs to take a clone of the variable.
///
/// This macro facilitates this pattern.
///
/// # Example
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::{widget, clone_move, NilUiNode, var::{var, IntoVar}, text::{Text, ToText}, context::WidgetContext};
/// #
/// # #[widget($crate::window)]
/// # pub mod window {
/// #     use super::*;
/// #
/// #     properties! {
/// #         #[allowed_in_when = false]
/// #         title(impl IntoVar<Text>);
/// #
/// #         #[allowed_in_when = false]
/// #         on_click(impl FnMut(&mut WidgetContext, ()));
/// #     }
/// #
/// #     fn new_child(title: impl IntoVar<Text>, on_click: impl FnMut(&mut WidgetContext, ())) -> NilUiNode {
/// #         NilUiNode
/// #     }
/// # }
/// #
/// # fn demo() {
/// let title = var("Click Me!".to_text());
/// window! {
///     on_click = clone_move!(title, |ctx, _| {
///         title.set(ctx.vars, "Clicked!".into());
///     });
///     title;
/// }
/// # ;
/// # }
/// ```
///
/// Expands to:
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::{widget, clone_move, NilUiNode, var::{var, IntoVar}, text::{Text, ToText}, context::WidgetContext};
/// #
/// # #[widget($crate::window)]
/// # pub mod window {
/// #     use super::*;
/// #
/// #     properties! {
/// #         #[allowed_in_when = false]
/// #         title(impl IntoVar<Text>);
/// #
/// #         #[allowed_in_when = false]
/// #         on_click(impl FnMut(&mut WidgetContext, ()));
/// #     }
/// #
/// #     fn new_child(title: impl IntoVar<Text>, on_click: impl FnMut(&mut WidgetContext, ())) -> NilUiNode {
/// #         NilUiNode
/// #     }
/// # }
/// #
/// # fn demo() {
/// let title = var("Click Me!".to_text());
/// window! {
///     on_click = {
///         let title = title.clone();
///         move |ctx, _| {
///             title.set(ctx.vars, "Clicked!".into());
///         }
///     };
///     title;
/// }
/// # ;
/// # }
/// ```
///
/// # Other Patterns
///
/// Although this macro exists primarily for creating event handlers, you can use it with any Rust variable. The
/// cloned variable can be marked `mut` and you can deref `*` as many times as you need to get to the actual value you
/// want to clone.
///
/// ```
/// # use zero_ui_core::clone_move;
/// # use std::rc::Rc;
/// let foo = vec![1, 2, 3];
/// let bar = Rc::new(vec!['a', 'b', 'c']);
/// let closure = clone_move!(mut foo, *bar, || {
///     foo.push(4);
///     let cloned_vec: Vec<_> = bar;
/// });
/// assert_eq!(foo.len(), 3);
/// ```
///
/// Expands to:
///
/// ```
/// # use zero_ui_core::clone_move;
/// # use std::rc::Rc;
/// let foo = vec![1, 2, 3];
/// let bar = Rc::new(vec!['a', 'b', 'c']);
/// let closure = {
///     let mut foo = foo.clone();
///     let bar = (*bar).clone();
///     move || {
///         foo.push(4);
///         let cloned_vec: Vec<_> = bar;
///     }
/// };
/// assert_eq!(foo.len(), 3);
/// ```
#[macro_export]
macro_rules! clone_move {
    ($($tt:tt)+) => { $crate::__clone_move!{[][][] $($tt)+} }
}
#[doc(hidden)]
#[macro_export]
macro_rules! __clone_move {
    // match start of mut var
    ([$($done:tt)*][][] mut $($rest:tt)+) => {
        $crate::__clone_move! {
            [$($done)*]
            [mut]
            []
            $($rest)+
        }
    };

    // match one var deref (*)
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] * $($rest:tt)+) => {
        $crate::__clone_move! {
            [$($done)*]
            [$($mut:tt)?]
            [$($deref)* *]
            $($rest)+
        }
    };

    // match end of a variable
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] $var:ident, $($rest:tt)+) => {
        $crate::__clone_move! {
            [
                $($done)*
                let $($mut)? $var = ( $($deref)* $var ).clone();
            ]
            []
            []
            $($rest)+
        }
    };

    // match start of closure
    ([$($done:tt)*][][] | $($rest:tt)+) => {
        {
            $($done)*
            move | $($rest)+
        }
    };

    // match start of closure without input
    ([$($done:tt)*][][] || $($rest:tt)+) => {
        {
            $($done)*
            move || $($rest)+
        }
    };
}
