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
///         title.set(ctx.vars, "Clicked!");
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
///             title.set(ctx.vars, "Clicked!");
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
///
/// # Async
///
/// See [`async_clone_move!`](macro@crate::async_clone_move) for creating `async` closures.
#[macro_export]
macro_rules! clone_move {
    ($($tt:tt)+) => { $crate::__clone_move! { [][][] $($tt)+ } }
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

/// Cloning async closure.
///
/// This macro syntax is exactly the same as [`clone_move!`](macro@crate::clone_move), but it expands to an *async closure* that
/// captures a clone of zero or more variables and moves another clone of these variables into the returned future for each call.
///
/// # Example
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::{widget, property, async_clone_move, UiNode, NilUiNode, var::{var, IntoVar}, text::{Text, ToText}, context::WidgetContextMut};
/// # use std::future::Future;
/// #
/// # #[property(event)]
/// # fn on_click_async<C: UiNode, F: Future<Output=()>, H: FnMut(WidgetContextMut, ()) -> F>(child: C, handler: H) -> impl UiNode { child }
/// #
/// # #[widget($crate::window)]
/// # pub mod window {
/// #     use super::*;
/// #
/// #     properties! {
/// #         #[allowed_in_when = false]
/// #         title(impl IntoVar<Text>);
/// #     }
/// #
/// #     fn new_child(title: impl IntoVar<Text>) -> NilUiNode {
/// #         NilUiNode
/// #     }
/// # }
/// # async fn delay() {
/// #   std::future::ready(true).await;
/// # }
/// #
/// # fn demo() {
/// let title = var("Click Me!".to_text());
/// window! {
///     on_click_async = async_clone_move!(title, |ctx, _| {
///         title.set(&ctx, "Clicked!");
///         delay().await;
///         title.set(&ctx, "Async Update!");
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
/// # use zero_ui_core::{widget, property, async_clone_move, UiNode, NilUiNode, var::{var, IntoVar}, text::{Text, ToText}, context::WidgetContextMut};
/// # use std::future::Future;
/// #
/// # #[property(event)]
/// # fn on_click_async<C: UiNode, F: Future<Output=()>, H: FnMut(WidgetContextMut, ()) -> F>(child: C, handler: H) -> impl UiNode { child }
/// #
/// # #[widget($crate::window)]
/// # pub mod window {
/// #     use super::*;
/// #
/// #     properties! {
/// #         #[allowed_in_when = false]
/// #         title(impl IntoVar<Text>);
/// #     }
/// #
/// #     fn new_child(title: impl IntoVar<Text>) -> NilUiNode {
/// #         NilUiNode
/// #     }
/// # }
/// # async fn delay() {
/// #   std::future::ready(true).await;
/// # }
/// #
/// # fn demo() {
/// let title = var("Click Me!".to_text());
/// window! {
///     on_click_async = {
///         let title = title.clone();
///         move |ctx, _| {
///             let title = title.clone();
///             async move {
///                 title.set(&ctx, "Clicked!");
///                 delay().await;
///                 title.set(&ctx, "Async Update!");
///             }
///         }
///     };
///     title;
/// }
/// # ;
/// # }
/// ```
#[macro_export]
macro_rules! async_clone_move {
    ($($tt:tt)+) => { $crate::__async_clone_move! { [{}{}][][] $($tt)+ } }
}
#[doc(hidden)]
#[macro_export]
macro_rules! __async_clone_move {
    // match start of mut var
    ([$($done:tt)*][][] mut $($rest:tt)+) => {
        $crate::__async_clone_move! {
            [$($done)*]
            [mut]
            []
            $($rest)+
        }
    };

    // match one var deref (*)
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] * $($rest:tt)+) => {
        $crate::__async_clone_move! {
            [$($done)*]
            [$($mut:tt)?]
            [$($deref)* *]
            $($rest)+
        }
    };

    // match end of a variable
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] $var:ident, $($rest:tt)+) => {
        $crate::__async_clone_move! {
            @var
            [$($done)*]
            [$($mut)?]
            [$($deref)*]
            $var,
            $($rest)+
        }
    };

    // include one var
    (@var [ { $($closure_clones:tt)* }{ $($async_clones:tt)* } ][$($mut:tt)?][$($deref:tt)*] $var:ident, $($rest:tt)+) => {
        $crate::__async_clone_move! {
            [
                {
                    $($closure_clones)*
                    let $var = ( $($deref)* $var ).clone();
                }
                {
                    $($async_clones)*
                    let $($mut)? $var = $var.clone();
                }
            ]
            []
            []
            $($rest)+
        }
    };

    // match start of closure inputs
    ([$($done:tt)*][][] | $($rest:tt)+) => {
        $crate::__async_clone_move! {
            @args
            [$($done)*]
            []
            $($rest)+
        }
    };

    // match start of closure without input, the closure body is in a block
    ([ { $($closure_clones:tt)* }{ $($async_clones:tt)* } ][][] || { $($rest:tt)+ }) => {
        {
            $($closure_clones)*
            move || {
                $($async_clones)*
                async move {
                    $($rest)+
                }
            }
        }
    };
    // match start of closure without input, the closure body is **not** in a block
    ([ { $($closure_clones:tt)* }{ $($async_clones:tt)* } ][][] || $($rest:tt)+ ) => {
        {
            $($closure_clones)*
            move || {
                $($async_clones)*
                async move {
                    $($rest)+
                }
            }
        }
    };

    // match end of closure inputs, the closure body is in a block
    (@args [  { $($closure_clones:tt)* }{ $($async_clones:tt)* } ] [$($args:tt)*] | { $($rest:tt)+ }) => {
        {
            $($closure_clones)*
            move |$($args)*| {
                $($async_clones)*
                async move {
                    $($rest)+
                }
            }
        }
    };
    // match end of closure inputs, the closure body is in a block
    (@args [  { $($closure_clones:tt)* }{ $($async_clones:tt)* } ] [$($args:tt)*] | $($rest:tt)+) => {
        {
            $($closure_clones)*
            move |$($args)*| {
                $($async_clones)*
                async move {
                    $($rest)+
                }
            }
        }
    };

    // match a token in closure inputs
    (@args [$($done:tt)*] [$($args:tt)*] $arg_tt:tt $($rest:tt)+) => {
        $crate::__async_clone_move! {
            @args
            [$($done)*]
            [$($args)* $arg_tt]
            $($rest)+
        }
    };
}

/// Cloning async closure that can only be called once.
///
/// This macro syntax is exactly the same as [`async_clone_move!`](macro@crate::async_clone_move), but it does not clone variables
/// again inside the call to move to the returned future. Because if moves the captured variables to the closure returned `Future`
/// it can only be `FnOnce`.
///
/// # Example
///
/// In the example `data` is clone moved to the closure and then moved in the returned `Future`, this only works because the closure
/// is a `FnOnce`.
///
/// ```
/// # use zero_ui_core::{async_clone_move_once, task};
/// # use std::future::Future;
/// fn foo<F: Future<Output=Vec<u32>>>(f: impl FnOnce(String) -> F) { }
///
/// let data = vec![1, 2, 3];
/// foo(async_clone_move_once!(data, |s| {
///     task::wait(move || println!("do async thing: {}", s)).await;
///     data
/// }))
/// ```
#[macro_export]
macro_rules! async_clone_move_once {
    ($($tt:tt)+) => { $crate::__async_clone_move_once! { [][][] $($tt)+ } }
}
#[doc(hidden)]
#[macro_export]
macro_rules! __async_clone_move_once {
    // match start of mut var
    ([$($done:tt)*][][] mut $($rest:tt)+) => {
        $crate::__async_clone_move_once! {
            [$($done)*]
            [mut]
            []
            $($rest)+
        }
    };

    // match one var deref (*)
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] * $($rest:tt)+) => {
        $crate::__async_clone_move_once! {
            [$($done)*]
            [$($mut:tt)?]
            [$($deref)* *]
            $($rest)+
        }
    };

    // match end of a variable
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] $var:ident, $($rest:tt)+) => {
        $crate::__async_clone_move_once! {
            [
                $($done)*
                let $($mut)? $var = ( $($deref)* $var ).clone();
            ]
            []
            []
            $($rest)+
        }
    };

    // match start of closure inputs
    ([$($done:tt)*][][] | $($rest:tt)+) => {
        $crate::__async_clone_move_once! {
            @args
            [$($done)*]
            []
            $($rest)+
        }
    };

    // match start of closure without input, the closure body is in a block
    ([$($done:tt)*][][] || { $($rest:tt)+ }) => {
        {
            $($done)*
            move || {
                async move {
                    $($rest)+
                }
            }
        }
    };
    // match start of closure without input, the closure body is **not** in a block
    ([$($done:tt)*][][] || $($rest:tt)+ ) => {
        {
            $($done)*
            move || {
                async move {
                    $($rest)+
                }
            }
        }
    };

    // match end of closure inputs, the closure body is in a block
    (@args [$($done:tt)*] [$($args:tt)*] | { $($rest:tt)+ }) => {
        {
            $($done)*
            move |$($args)*| {
                async move {
                    $($rest)+
                }
            }
        }
    };
    // match end of closure inputs, the closure body is in a block
    (@args [$($done:tt)*] [$($args:tt)*] | $($rest:tt)+) => {
        {
            $($done)*
            move |$($args)*| {
                async move {
                    $($rest)+
                }
            }
        }
    };

    // match a token in closure inputs
    (@args [$($done:tt)*] [$($args:tt)*] $arg_tt:tt $($rest:tt)+) => {
        $crate::__async_clone_move_once! {
            @args
            [$($done)*]
            [$($args)* $arg_tt]
            $($rest)+
        }
    };
}

#[cfg(test)]
#[allow(dead_code)]
#[allow(clippy::ptr_arg)]
mod async_clone_move_tests {
    // if it build it passes

    use std::{future::ready, rc::Rc};

    fn no_clones_no_input() {
        let _ = async_clone_move!(|| ready(true).await);
    }

    fn one_clone_no_input(a: &String) {
        let _ = async_clone_move!(a, || {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn one_clone_with_derefs_no_input(a: &Rc<String>) {
        let _ = async_clone_move!(**a, || {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn two_derefs_no_input(a: &String, b: Rc<String>) {
        let _ = async_clone_move!(a, b, || {
            let _: String = a;
            let _: Rc<String> = b;
            ready(true).await
        });
        let _ = (a, b);
    }

    fn one_input(a: &String) {
        let _ = async_clone_move!(a, |_ctx: u32| {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn two_inputs(a: &String) {
        let _ = async_clone_move!(a, |_b: u32, _c: Box<dyn std::fmt::Debug>| {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }
}

#[cfg(test)]
#[allow(dead_code)]
#[allow(clippy::ptr_arg)]
mod async_clone_move_once_tests {
    // if it build it passes

    use std::{future::ready, rc::Rc};

    fn no_clones_no_input() {
        let _ = async_clone_move_once!(|| ready(true).await);
    }

    fn one_clone_no_input(a: &String) {
        let _ = async_clone_move_once!(a, || {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn one_clone_with_derefs_no_input(a: &Rc<String>) {
        let _ = async_clone_move_once!(**a, || {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn two_derefs_no_input(a: &String, b: Rc<String>) {
        let _ = async_clone_move_once!(a, b, || {
            let _: String = a;
            let _: Rc<String> = b;
            ready(true).await
        });
        let _ = (a, b);
    }

    fn one_input(a: &String) {
        let _ = async_clone_move_once!(a, |_ctx: u32| {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn two_inputs(a: &String) {
        let _ = async_clone_move_once!(a, |_b: u32, _c: Box<dyn std::fmt::Debug>| {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }
}
