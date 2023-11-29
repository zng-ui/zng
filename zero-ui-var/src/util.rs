///<span data-del-macro-root></span> Implements `U: From<T>`, `T: IntoVar<U>` and `T: IntoValue<U>` without boilerplate.
///
/// Unfortunately we cannot provide a widget impl of `IntoVar` and `IntoValue` for all `From` in Rust stable, because
/// that would block all manual implementations of the trait, so you need to implement then manually to
/// enable the easy-to-use properties that are expected.
///
/// You can use this macro to implement both `U: From<T>`, `T: IntoVar<U>` and `T: IntoValue<U>` at the same time.
/// The macro syntax is one or more functions with signature `fn from(t: T) -> U`. The [`LocalVar<U>`]
/// type is selected for variables.
///
/// Optionally you can declare generics using the pattern `fn from<const N: usize>(t: &'static [T; N]) -> U`
/// with multiple generic types and constraints, but not `where` constraints. You can also destruct the input
/// if it is a tuple using the pattern `fn from((a, b): (A, B)) -> U`, but no other pattern matching in
/// the input is supported.
///
/// [`LocalVar<U>`]: crate::var::LocalVar
///
/// # Examples
///
/// The example declares an `enum` that represents the values possible in a property `foo` and
/// then implements conversions from literals the user may want to type in a widget:
///
/// ```
/// # use zero_ui_core::var::impl_from_and_into_var;
/// #[derive(Debug, Clone, PartialEq)]
/// pub enum FooValue {
///     On,
///     Off,
///     NotSet
/// }
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
/// }
/// # fn assert(_: impl zero_ui_core::var::IntoVar<FooValue> + Into<FooValue>) { }
/// # assert(true);
/// # assert("on");
/// ```
///
/// The value then can be used in a property:
///
/// ```
/// # use zero_ui_core::{*, widget_instance::*, var::*};
/// # #[derive(Debug, Clone, PartialEq)]
/// # pub struct FooValue;
/// # impl_from_and_into_var! { fn from(b: bool) -> FooValue { FooValue } }
/// # #[widget($crate::Bar)] pub struct Bar(widget_base::WidgetBase);
/// #[property(CONTEXT)]
/// pub fn foo(child: impl UiNode, value: impl IntoVar<FooValue>) -> impl UiNode {
///     // ..
/// #   child
/// }
///
/// # fn main() {
/// # let _ =
/// Bar! {
///     foo = true;
/// }
/// # ;
/// # }
/// ```
#[macro_export]
macro_rules! impl_from_and_into_var {
    ($($tt:tt)+) => {
        $crate::__impl_from_and_into_var! { $($tt)* }
    };
}

use parking_lot::RwLock;

#[doc(inline)]
pub use crate::impl_from_and_into_var;

use super::{animation::ModifyInfo, VarHandle, VarHook, VarHookArgs, VarModify, VarUpdateId, VarValue, VARS};

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
        //zero_ui_proc_macros::trace! {
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
        ($ident:ident : $Input:ty) $($rest:tt)+
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
        (( $($destruct:tt)+ ) : $Input:ty) $($rest:tt)+
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
    // OUTPUT:
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
            type Var = $crate::LocalVar<$Output>;

            $($docs)*

            fn into_var(self) -> Self::Var {
                $crate::LocalVar(self.into())
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

struct VarDataInner<T> {
    value: T,
    last_update: VarUpdateId,
    hooks: Vec<VarHook>,
    animation: ModifyInfo,
}

pub(super) struct VarData<T: VarValue>(RwLock<VarDataInner<T>>);
impl<T: VarValue> VarData<T> {
    pub fn new(value: T) -> Self {
        Self(RwLock::new(VarDataInner {
            value,
            last_update: VarUpdateId::never(),
            hooks: vec![],
            animation: ModifyInfo::never(),
        }))
    }

    pub fn into_value(self) -> T {
        self.0.into_inner().value
    }

    /// Read the value.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.0.read().value)
    }

    pub fn last_update(&self) -> VarUpdateId {
        self.0.read().last_update
    }

    pub fn is_animating(&self) -> bool {
        self.0.read().animation.is_animating()
    }

    pub fn modify_importance(&self) -> usize {
        self.0.read().animation.importance()
    }

    pub fn push_hook(&self, pos_modify_action: Box<dyn Fn(&VarHookArgs) -> bool + Send + Sync>) -> VarHandle {
        let (hook, weak) = VarHandle::new(pos_modify_action);
        self.0.write().hooks.push(weak);
        hook
    }

    pub fn push_animation_hook(&self, handler: Box<dyn FnOnce() + Send>) -> Result<(), Box<dyn FnOnce() + Send>> {
        self.0.write().animation.hook_animation_stop(handler)
    }

    /// Calls `modify` on the value.
    pub fn apply_modify(&self, modify: impl FnOnce(&mut VarModify<T>)) {
        let mut meta = self.0.write();
        let curr_anim = VARS.current_modify();
        if curr_anim.importance() < meta.animation.importance() {
            return;
        }
        meta.animation = curr_anim;

        let meta = parking_lot::RwLockWriteGuard::downgrade(meta);
        let mut value = VarModify::new(&meta.value);
        modify(&mut value);
        let (notify, new_value, update, tags) = value.finish();

        if notify {
            drop(meta);
            let mut meta = self.0.write();
            if let Some(nv) = new_value {
                meta.value = nv;
            }
            meta.last_update = VARS.update_id();

            let mut hooks = std::mem::take(&mut meta.hooks);

            if !hooks.is_empty() {
                let meta = parking_lot::RwLockWriteGuard::downgrade(meta);

                let args = VarHookArgs::new(&meta.value, update, &tags);
                hooks.retain(|h| h.call(&args));

                drop(meta);

                let mut meta = self.0.write();
                hooks.append(&mut meta.hooks);
                meta.hooks = hooks;
            }

            VARS.wake_app();
        }
    }
}
