#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Batch updated variables in an app context.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
// suppress nag about very simple boxed closure signatures.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]
#![deny(clippy::future_not_send)]

macro_rules! trace_debug_error {
    ($result:expr) => {
        if let Err(e) = $result {
            tracing::debug!("{e}")
        }
    };
}

mod var_value;
pub use var_value::*;

mod var_any;
pub use var_any::*;

mod var;
pub use var::*;

pub(crate) mod var_impl;
pub use var_impl::*;

pub mod animation;

mod vars;
pub use vars::*;

mod vec;
pub use vec::*;

mod impls;

pub(crate) mod future;

///Implements `T: IntoVar<U>`, `T: IntoValue<U>` and optionally `U: From<T>` without boilerplate.
///
/// The macro syntax is one or more functions with signature `fn from(t: T) -> U`. The [`const_var`]
/// kind is used for variables. The syntax also supports generic types and constraints, but not `where` constraints.
/// You can also destructure the input if it is a tuple using the pattern `fn from((a, b): (A, B)) -> U`, but no other pattern
/// matching in the input is supported.
///
/// The `U: From<T>` implement is optional, you can use the syntax `fn from(t: T) -> U;` to only generate
/// the `T: IntoVar<U>` and `T: IntoValue<U>` implementations using an already implemented `U: From<T>`.
///
/// # Examples
///
/// The example declares an `enum` that represents the values possible in a property `foo` and
/// then implements conversions from literals the user may want to type in a widget:
///
/// ```
/// # use zng_var::*;
/// #[derive(Debug, Clone, PartialEq)]
/// pub enum FooValue {
///     On,
///     Off,
///     NotSet
/// }
///
/// impl_from_and_into_var! {
///     fn from(b: bool) -> FooValue {
///         if b {
///             FooValue::On
///         } else {
///             FooValue::Off
///         }
///     }
///
///     fn from(s: &str) -> FooValue {
///         match s {
///             "on" => FooValue::On,
///             "off" => FooValue::Off,
///             _ => FooValue::NotSet
///         }
///     }
///
///     fn from(f: Foo) -> FooValue;
/// }
///
/// impl From<Foo> for FooValue {
///     fn from(foo: Foo) -> Self {
///         Self::On
///     }
/// }
/// # pub struct Foo;
/// # fn assert(_: impl IntoVar<FooValue> + IntoValue<FooValue>) { }
/// # assert(true);
/// # assert("on");
/// ```
#[macro_export]
macro_rules! impl_from_and_into_var {
    ($($tt:tt)+) => {
        $crate::__impl_from_and_into_var! { $($tt)* }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __impl_from_and_into_var {
    // START:
    (
        $(#[$docs:meta])*
        fn from ( $($input:tt)+ )
        $($rest:tt)+
    ) => {
        $crate::__impl_from_and_into_var! {
            =input=>
            [
                input { $($input)+ }
                generics { }
                docs { $(#[$docs])* }
            ]
            ( $($input)+ ) $($rest)+
        }
    };
    // GENERICS START:
    (
        $(#[$docs:meta])*
        fn from <
        $($rest:tt)+
    ) => {
        $crate::__impl_from_and_into_var! {
            =generics=>
            [
                generics { < }
                docs { $(#[$docs])* }
            ]
            $($rest)+
        }
    };
    // GENERICS END `>`:
    (
        =generics=>
        [
            generics { $($generics:tt)+ }
            $($config:tt)*
        ]

        >( $($input:tt)+ ) $($rest:tt)+
    ) => {
        $crate::__impl_from_and_into_var! {
            =input=>
            [
                input { $($input)+ }
                generics { $($generics)+ > }
                $($config)*
            ]
            ( $($input)+ ) $($rest)+
        }
    };
    // GENERICS END `>>`:
    (
        =generics=>
        [
            generics { $($generics:tt)+ }
            $($config:tt)*
        ]

        >>( $($input:tt)+ ) $($rest:tt)+
    ) => {
        $crate::__impl_from_and_into_var! {
            =input=>
            [
                input { $($input)+ }
                generics { $($generics)+ >> }
                $($config)*
            ]
            ( $($input)+ ) $($rest)+
        }
    };
    // collect generics:
    (
        =generics=>
        [
            generics { $($generics:tt)+ }
            $($config:tt)*
        ]

        $tt:tt $($rest:tt)+
    ) => {
        //zng_proc_macros::trace! {
        $crate::__impl_from_and_into_var! {
            =generics=>
            [
                generics { $($generics)+ $tt }
                $($config)*
            ]
            $($rest)*
        }
        //}
    };
    // INPUT SIMPLE:
    (
        =input=>
        [$($config:tt)*]
        ($ident:ident : $Input:ty $(,)?) $($rest:tt)+
    ) => {
        $crate::__impl_from_and_into_var! {
            =output=>
            [
                input_type { $Input }
                $($config)*
            ]
            $($rest)+
        }
    };
    // INPUT TUPLE:
    (
        =input=>
        [$($config:tt)*]
        (( $($destructure:tt)+ ) : $Input:ty $(,)?) $($rest:tt)+
    ) => {
        $crate::__impl_from_and_into_var! {
            =output=>
            [
                input_type { $Input }
                $($config)*
            ]
            $($rest)+
        }
    };
    // INPUT ARRAY:
    (
        =input=>
        [$($config:tt)*]
        ([ $($destructure:tt)+ ] : $Input:ty $(,)?) $($rest:tt)+
    ) => {
        $crate::__impl_from_and_into_var! {
            =output=>
            [
                input_type { $Input }
                $($config)*
            ]
            $($rest)+
        }
    };

    // OUTPUT (without From):
    (
        =output=>
        [
            input_type { $Input:ty }
            input { $($input:tt)+ }
            generics { $($generics:tt)* }
            docs { $($docs:tt)* }
        ]
        -> $Output:ty
        ;

        $($rest:tt)*
    ) => {
        impl $($generics)* $crate::IntoVar<$Output> for $Input {
            $($docs)*

            fn into_var(self) -> $crate::Var<$Output> {
                $crate::IntoVar::into_var(<$Output as From<$Input>>::from(self))
            }
        }

        impl $($generics)* $crate::IntoValue<$Output> for $Input { }

        // NEXT CONVERSION:
        $crate::__impl_from_and_into_var! {
            $($rest)*
        }
    };

    // OUTPUT (with From):
    (
        =output=>
        [
            input_type { $Input:ty }
            input { $($input:tt)+ }
            generics { $($generics:tt)* }
            docs { $($docs:tt)* }
        ]
        -> $Output:ty
        $convert:block

        $($rest:tt)*
    ) => {
        impl $($generics)* From<$Input> for $Output {
            $($docs)*

            fn from($($input)+) -> Self
            $convert
        }

        impl $($generics)* $crate::IntoVar<$Output> for $Input {
            $($docs)*

            fn into_var(self) -> $crate::Var<$Output> {
                $crate::IntoVar::into_var(<$Output as From<$Input>>::from(self))
            }
        }

        impl $($generics)* $crate::IntoValue<$Output> for $Input { }

        // NEXT CONVERSION:
        $crate::__impl_from_and_into_var! {
            $($rest)*
        }
    };

    () => {
        // END
    };
}
