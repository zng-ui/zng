#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Macros for declaring clone-move closures and async blocks.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

///<span data-del-macro-root></span> Clone move closure.
///
/// A common pattern when creating `'static` closures is to capture clones by `move`, this way the closure is `'static`
/// and the cloned values are still available after creating the closure. This macro facilitates this pattern.
///
/// # Examples
///
/// In the example `bar` is *clone-moved* into the `'static` closure given to `foo`.
///
/// ```
/// # use zng_clone_move::clmv;
/// fn foo(mut f: impl FnMut(bool) + 'static) {
///     f(true);
/// }
///
/// let bar = "Cool!".to_owned();
/// foo(clmv!(bar, |p| {
///     if p { println!("cloned: {bar}") }
/// }));
///
/// println!("original: {bar}");
/// ```
///
/// Expands to:
///
/// ```
/// # use zng_clone_move::clmv;
/// # fn foo(mut f: impl FnMut(bool) + 'static) {
/// #     f(true);
/// # }
/// # let bar = "Cool!".to_owned();
/// foo({
///     let bar = bar.clone();
///     move |p| {
///         if p { println!("cloned: {bar}") }
///     }
/// });
/// # println!("original: {bar}");
/// ```
///
/// # Other Patterns
///
/// Sometimes you want to clone an *inner deref* of the value, or you want the clone to be `mut`, you can annotate the
/// variables cloned to achieve these effects.
///
/// ```
/// # use zng_clone_move::clmv;
/// # use std::sync::Arc;
/// fn foo(mut f: impl FnMut(bool) + 'static) {
///     f(true);
/// }
///
/// let bar = Arc::new("Cool!".to_string());
/// foo(clmv!(mut *bar, |p| {
///     bar.push_str("!");
///     if p { println!("cloned String not Arc: {bar}") }
/// }));
///
/// println!("original: {bar}");
/// ```
///
/// Expands to:
///
/// ```
/// # use zng_clone_move::clmv;
/// # use std::sync::Arc;
/// # fn foo(mut f: impl FnMut(bool) + 'static) {
/// #     f(true);
/// # }
/// # let bar = Arc::new("Cool!".to_string());
/// foo({
///     let mut bar = (*bar).clone();
///     move |p| {
///         bar.push_str("!");
///         if p { println!("cloned String not Arc: {bar}") }
///     }
/// });
/// # println!("original: {bar}");
/// ```
///
/// # Async
///
/// See [`async_clmv_fn!`] for creating `async` closures or [`async_clmv!`] for creating clone move futures.
#[macro_export]
macro_rules! clmv {
    ($($tt:tt)+) => { $crate::__clmv! { [][][] $($tt)+ } }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __clmv {
    // match start of mut var
    ([$($done:tt)*][][] mut $($rest:tt)+) => {
        $crate::__clmv! {
            [$($done)*]
            [mut]
            []
            $($rest)+
        }
    };

    // match one var deref (*)
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] * $($rest:tt)+) => {
        $crate::__clmv! {
            [$($done)*]
            [$($mut)?]
            [$($deref)* *]
            $($rest)+
        }
    };

    // match end of a variable
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] $var:ident, $($rest:tt)+) => {
        $crate::__clmv! {
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

/// <span data-del-macro-root></span> Async clone move block.
///
/// This macro is similar to [`clmv!`], but creates a future instead of a closure.
///
/// A common pattern when creating `'static` futures is to capture clones by `move`, this way the future is `'static`
/// and the cloned values are still available after creating the future. This macro facilitates this pattern.
///
/// # Examples
///
/// In the example `bar` is *clone-moved* into the `'static` future given to `foo`.
///
/// ```
/// # use std::{time::Duration};
/// # use zng_clone_move::*;
/// # trait TimeUnits { fn ms(self) -> Duration; }
/// # impl TimeUnits for u64 { fn ms(self) -> Duration { Duration::from_millis(self) } }
/// # async fn deadline(_d: Duration) { }
/// async fn foo(mut f: impl Future<Output=()> + 'static) {
///     f.await;
/// }
///
/// let bar = "Cool!".to_owned();
/// foo(async_clmv!(bar, {
///     deadline(100.ms()).await;
///     println!("cloned: {bar}")
/// }));
///
/// println!("original: {bar}");
/// ```
///
/// Expands to:
///
/// ```
/// # use std::{time::Duration};
/// # use zng_clone_move::*;
/// # async fn foo(mut f: impl Future<Output=()> + 'static) {
/// #     f.await;
/// # }
/// # let bar = "Cool!".to_owned();
/// # trait TimeUnits { fn ms(self) -> Duration; }
/// # impl TimeUnits for u64 { fn ms(self) -> Duration { Duration::from_millis(self) } }
/// # async fn deadline(_d: Duration) { }
/// foo({
///     let bar = bar.clone();
///     async move {
///         deadline(100.ms()).await;
///         println!("cloned: {bar}")
///     }
/// });
/// # println!("original: {bar}");
/// ```
///
/// # Other Patterns
///
/// Sometimes you want to clone an *inner deref* of the value, or you want the clone to be `mut`, you can annotate the
/// variables cloned to achieve these effects.
///
/// ```
/// # use std::{sync::Arc};
/// # use zng_clone_move::*;
/// async fn foo(mut f: impl Future<Output=()> + 'static) {
///     f.await;
/// }
///
/// let bar = Arc::new("Cool!".to_string());
/// foo(async_clmv!(mut *bar, {
///     bar.push_str("!");
///     println!("cloned String not Arc: {bar}");
/// }));
///
/// println!("original: {bar}");
/// ```
///
/// Expands to:
///
/// ```
/// # use std::{sync::Arc};
/// # use zng_clone_move::*;
/// # async fn foo(mut f: impl Future<Output=()> + 'static) {
/// #     f.await;
/// # }
/// # let bar = Arc::new("Cool!".to_string());
/// foo({
///     let mut bar = (*bar).clone();
///     async move {
///         bar.push_str("!");
///         println!("cloned String not Arc: {bar}")
///     }
/// });
/// # println!("original: {bar}");
/// ```
#[macro_export]
macro_rules! async_clmv {
    ($($tt:tt)+) => {
        $crate::__async_clmv! { [][][] $($tt)+ }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __async_clmv {
    // match start of mut var
    ([$($done:tt)*][][] mut $($rest:tt)+) => {
        $crate::__async_clmv! {
            [$($done)*]
            [mut]
            []
            $($rest)+
        }
    };

    // match one var deref (*)
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] * $($rest:tt)+) => {
        $crate::__async_clmv! {
            [$($done)*]
            [$($mut)?]
            [$($deref)* *]
            $($rest)+
        }
    };

    // match end of a variable
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] $var:ident, $($rest:tt)+) => {
        $crate::__async_clmv! {
            [
                $($done)*
                let $($mut)? $var = ( $($deref)* $var ).clone();
            ]
            []
            []
            $($rest)+
        }
    };

    // match block
    ([$($done:tt)*][][] { $($block:tt)* }) => {
        {
            $($done)*
            async move { $($block)* }
        }
    };
}

///<span data-del-macro-root></span> Async clone move closure.
///
/// The macro syntax is exactly the same as [`clmv!`](macro@crate::clmv), but it expands to an *async closure* that
/// captures a clone of zero or more variables and moves another clone of these variables into the returned future for each call.
///
/// # Examples
///
/// In the example `bar` is cloned into the closure and then it is cloned again for each future generated by the closure.
///
/// ```
/// # use zng_clone_move::async_clmv_fn;
/// async fn foo<F: Future<Output=()>, H: FnMut(bool) -> F + 'static>(mut f: H) {
///     f(true).await;
/// }
///
/// let bar = "Cool!".to_owned();
/// foo(async_clmv_fn!(bar, |p| {
///     std::future::ready(()).await;
///     if p { println!("cloned: {bar}") }
/// }));
///
/// println!("original: {bar}");
/// ```
///
/// Expands to:
///
/// ```
/// # use zng_clone_move::async_clmv_fn;
/// # async fn foo<F: Future<Output=()>, H: FnMut(bool) -> F + 'static>(mut f: H) {
/// #     f(true).await;
/// # }
/// # let bar = "Cool!".to_owned();
/// foo({
///     let bar = bar.clone();
///     move |p| {
///         let bar = bar.clone();
///         async move {
///             std::future::ready(()).await;
///             if p { println!("cloned: {bar}") }
///         }
///     }
/// });
/// # println!("original: {bar}");
/// ```
/// 
/// Note that this is different from an async closure, it returns `'static` futures that do not borrow the closure.
///
/// # Once
///
/// See [`async_clmv_fn_once!`] for creating `FnOnce` closures.
#[macro_export]
macro_rules! async_clmv_fn {
    ($($tt:tt)+) => { $crate::__async_clmv_fn! { [{}{}][][] $($tt)+ } }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __async_clmv_fn {
    // match start of mut var
    ([$($done:tt)*][][] mut $($rest:tt)+) => {
        $crate::__async_clmv_fn! {
            [$($done)*]
            [mut]
            []
            $($rest)+
        }
    };

    // match one var deref (*)
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] * $($rest:tt)+) => {
        $crate::__async_clmv_fn! {
            [$($done)*]
            [$($mut)?]
            [$($deref)* *]
            $($rest)+
        }
    };

    // match end of a variable
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] $var:ident, $($rest:tt)+) => {
        $crate::__async_clmv_fn! {
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
        $crate::__async_clmv_fn! {
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
        $crate::__async_clmv_fn! {
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
    (@args [ { $($closure_clones:tt)* }{ $($async_clones:tt)* } ] [$($args:tt)*] | { $($rest:tt)+ }) => {
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
    (@args [ { $($closure_clones:tt)* }{ $($async_clones:tt)* } ] [$($args:tt)*] | $($rest:tt)+) => {
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
        $crate::__async_clmv_fn! {
            @args
            [$($done)*]
            [$($args)* $arg_tt]
            $($rest)+
        }
    };
}

///<span data-del-macro-root></span> Async clone move closure that can only be called once.
///
/// The macro syntax is exactly the same as [`async_clmv_fn!`], but it does not clone variables
/// again inside the call to move to the returned future. Because it moves the captured variables to the returned `Future`,
/// the closure can only be `FnOnce`.
///
/// # Examples
///
/// In the example `bar` is cloned into the closure and then moved to the future generated by the closure.
///
/// ```
/// # use zng_clone_move::async_clmv_fn;
/// async fn foo<F: Future<Output=()>, H: FnOnce(bool) -> F + 'static>(mut f: H) {
///     f(true).await;
/// }
///
/// let bar = "Cool!".to_owned();
/// foo(async_clmv_fn!(bar, |p| {
///     std::future::ready(()).await;
///     if p { println!("cloned: {bar}") }
/// }));
///
/// println!("original: {bar}");
/// ```
///
/// Expands to:
///
/// ```
/// # use zng_clone_move::async_clmv_fn;
/// # async fn foo<F: Future<Output=()>, H: FnOnce(bool) -> F + 'static>(mut f: H) {
/// #     f(true).await;
/// # }
/// # let bar = "Cool!".to_owned();
/// foo({
///     let bar = bar.clone();
///     move |p| async move {
///         std::future::ready(()).await;
///         if p { println!("cloned: {bar}") }
///     }
/// });
/// # println!("original: {bar}");
/// ```
/// 
/// Note that this is different from an async once closure, it is an once closure that returns a future, this is so it is more similar with
/// [`async_clmv_fn!`], that macro cannot be implemented an async closure.
/// 
/// [`async_clmv_fn!`]: macro@crate::async_clmv_fn
#[macro_export]
macro_rules! async_clmv_fn_once {
    ($($tt:tt)+) => { $crate::__async_clmv_fn_once! { [][][] $($tt)+ } }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __async_clmv_fn_once {
    // match start of mut var
    ([$($done:tt)*][][] mut $($rest:tt)+) => {
        $crate::__async_clmv_fn_once! {
            [$($done)*]
            [mut]
            []
            $($rest)+
        }
    };

    // match one var deref (*)
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] * $($rest:tt)+) => {
        $crate::__async_clmv_fn_once! {
            [$($done)*]
            [$($mut)?]
            [$($deref)* *]
            $($rest)+
        }
    };

    // match end of a variable
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] $var:ident, $($rest:tt)+) => {
        $crate::__async_clmv_fn_once! {
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
        $crate::__async_clmv_fn_once! {
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
        $crate::__async_clmv_fn_once! {
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
mod async_clmv_fn_tests {
    // if it build it passes

    use std::{future::ready, sync::Arc};

    fn no_clones_no_input() {
        let _ = async_clmv_fn!(|| ready(true).await);
    }

    fn one_clone_no_input(a: &String) {
        let _ = async_clmv_fn!(a, || {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn one_clone_with_derefs_no_input(a: &Arc<String>) {
        let _ = async_clmv_fn!(**a, || {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn two_derefs_no_input(a: &String, b: Arc<String>) {
        let _ = async_clmv_fn!(a, b, || {
            let _: String = a;
            let _: Arc<String> = b;
            ready(true).await
        });
        let _ = (a, b);
    }

    fn one_input(a: &String) {
        let _ = async_clmv_fn!(a, |_args: u32| {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn two_inputs(a: &String) {
        let _ = async_clmv_fn!(a, |_b: u32, _c: Box<dyn std::fmt::Debug>| {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }
}

#[cfg(test)]
#[allow(dead_code)]
#[allow(clippy::ptr_arg)]
mod async_clmv_fn_once_tests {
    // if it build it passes

    use std::{future::ready, sync::Arc};

    fn no_clones_no_input() {
        let _ = async_clmv_fn_once!(|| ready(true).await);
    }

    fn one_clone_no_input(a: &String) {
        let _ = async_clmv_fn_once!(a, || {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn one_clone_with_derefs_no_input(a: &Arc<String>) {
        let _ = async_clmv_fn_once!(**a, || {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn two_derefs_no_input(a: &String, b: Arc<String>) {
        let _ = async_clmv_fn_once!(a, b, || {
            let _: String = a;
            let _: Arc<String> = b;
            ready(true).await
        });
        let _ = (a, b);
    }

    fn one_input(a: &String) {
        let _ = async_clmv_fn_once!(a, |_args: u32| {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn two_inputs(a: &String) {
        let _ = async_clmv_fn_once!(a, |_b: u32, _c: Box<dyn std::fmt::Debug>| {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }
}
