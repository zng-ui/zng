///Implements `T: IntoVar<U>`, `T: IntoValue<U>` and optionally `U: From<T>` without boilerplate.
///
/// Unfortunately we cannot provide a trait impl of `IntoVar` and `IntoValue` for all `From` in Rust stable, because
/// that would block all manual implementations of the trait, so you need to implement it manually to
/// enable the easy-to-use parameters.
///
/// You can use this macro to implement `U: From<T>`, `T: IntoVar<U>` and `T: IntoValue<U>` at the same time.
/// The macro syntax is one or more functions with signature `fn from(t: T) -> U`. The [`LocalVar<U>`]
/// type is selected for variables.
///
/// Optionally you can declare generics using the pattern `fn from<const N: usize>(t: &'static [T; N]) -> U`
/// with multiple generic types and constraints, but not `where` constraints. You can also destruct the input
/// if it is a tuple using the pattern `fn from((a, b): (A, B)) -> U`, but no other pattern matching in
/// the input is supported.
///
/// The `U: From<T>` implement is optional, you can use the syntax `fn from(t: T) -> U;` to only generate
/// the `T: IntoVar<U>` and `T: IntoValue<U>` implementations using an already implemented `U: From<T>`.
///
/// [`LocalVar<U>`]: crate::LocalVar
///
/// # Examples
///
/// The example declares an `enum` that represents the values possible in a property `foo` and
/// then implements conversions from literals the user may want to type in a widget:
///
/// ```
/// # use zero_ui_var::impl_from_and_into_var;
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
/// }///
/// # pub struct Foo;
/// # fn assert(_: impl zero_ui_var::IntoVar<FooValue> + Into<FooValue>) { }
/// # assert(true);
/// # assert("on");
/// ```
///
/// The value then can be used in a property:
///
/// ```
/// # macro_rules! _demo { () => {
/// #[property(CONTEXT)]
/// pub fn foo(child: impl UiNode, value: impl IntoVar<FooValue>) -> impl UiNode {
///     // ..
/// }
/// # }}
/// ```
#[macro_export]
macro_rules! impl_from_and_into_var {
    ($($tt:tt)+) => {
        $crate::__impl_from_and_into_var! { $($tt)* }
    };
}

use parking_lot::RwLock;

use crate::AnyVarHookArgs;

use super::{animation::ModifyInfo, VarHandle, VarHook, VarModify, VarUpdateId, VarValue, VARS};

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
    // INPUT ARRAY:
    (
        =input=>
        [$($config:tt)*]
        ([ $($destruct:tt)+ ] : $Input:ty) $($rest:tt)+
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

struct VarMeta {
    last_update: VarUpdateId,
    hooks: Vec<VarHook>,
    animation: ModifyInfo,
}
impl VarMeta {
    fn push_hook(&mut self, pos_modify_action: Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync>) -> VarHandle {
        let (hook, weak) = VarHandle::new(pos_modify_action);
        self.hooks.push(weak);
        hook
    }

    fn skip_modify(&mut self) -> bool {
        let curr_anim = VARS.current_modify();
        if curr_anim.importance() < self.animation.importance() {
            return true;
        }
        self.animation = curr_anim;
        false
    }
}

#[cfg(dyn_closure)]
struct VarDataInner {
    value: Box<dyn crate::AnyVarValue>,
    meta: VarMeta,
}

#[cfg(not(dyn_closure))]
struct VarDataInner<T> {
    value: T,
    meta: VarMeta,
}

#[cfg(dyn_closure)]
pub(super) struct VarData<T: VarValue>(RwLock<VarDataInner>, std::marker::PhantomData<T>);

#[cfg(not(dyn_closure))]
pub(super) struct VarData<T: VarValue>(RwLock<VarDataInner<T>>);

impl<T: VarValue> VarData<T> {
    pub fn new(value: T) -> Self {
        #[cfg(dyn_closure)]
        let value = Box::new(value);
        let inner = RwLock::new(VarDataInner {
            value,
            meta: VarMeta {
                last_update: VarUpdateId::never(),
                hooks: vec![],
                animation: ModifyInfo::never(),
            },
        });

        #[cfg(dyn_closure)]
        {
            Self(inner, std::marker::PhantomData)
        }

        #[cfg(not(dyn_closure))]
        {
            Self(inner)
        }
    }

    pub fn into_value(self) -> T {
        #[cfg(dyn_closure)]
        {
            *self.0.into_inner().value.into_any().downcast::<T>().unwrap()
        }
        #[cfg(not(dyn_closure))]
        {
            self.0.into_inner().value
        }
    }

    /// Read the value.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        #[cfg(dyn_closure)]
        {
            f(self.0.read().value.as_any().downcast_ref::<T>().unwrap())
        }

        #[cfg(not(dyn_closure))]
        {
            f(&self.0.read().value)
        }
    }

    pub fn last_update(&self) -> VarUpdateId {
        self.0.read().meta.last_update
    }

    pub fn is_animating(&self) -> bool {
        self.0.read().meta.animation.is_animating()
    }

    pub fn modify_importance(&self) -> usize {
        self.0.read().meta.animation.importance()
    }

    pub fn push_hook(&self, pos_modify_action: Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync>) -> VarHandle {
        self.0.write().meta.push_hook(pos_modify_action)
    }

    pub fn push_animation_hook(&self, handler: Box<dyn FnOnce() + Send>) -> Result<(), Box<dyn FnOnce() + Send>> {
        self.0.write().meta.animation.hook_animation_stop(handler)
    }

    #[cfg(dyn_closure)]
    pub fn apply_modify(&self, modify: Box<dyn FnOnce(&mut VarModify<T>) + 'static>) {
        apply_modify(
            &self.0,
            Box::new(move |v| {
                let mut value = VarModify::new(v.as_any().downcast_ref::<T>().unwrap());
                modify(&mut value);
                let (notify, new_value, update, tags) = value.finish();
                (
                    notify,
                    match new_value {
                        Some(v) => Some(Box::new(v)),
                        None => None,
                    },
                    update,
                    tags,
                )
            }),
        )
    }

    #[cfg(not(dyn_closure))]
    pub fn apply_modify(&self, modify: impl FnOnce(&mut VarModify<T>) + 'static) {
        apply_modify(&self.0, modify)
    }
}

#[cfg(dyn_closure)]
fn apply_modify(
    inner: &RwLock<VarDataInner>,
    modify: Box<dyn FnOnce(&dyn crate::AnyVarValue) -> (bool, Option<Box<dyn crate::AnyVarValue>>, bool, Vec<Box<dyn crate::AnyVarValue>>)>,
) {
    let mut data = inner.write();
    if data.meta.skip_modify() {
        return;
    }

    let data = parking_lot::RwLockWriteGuard::downgrade(data);

    let (notify, new_value, update, tags) = modify(&*data.value);

    if notify {
        drop(data);
        let mut data = inner.write();
        if let Some(nv) = new_value {
            data.value = nv;
        }
        data.meta.last_update = VARS.update_id();

        if !data.meta.hooks.is_empty() {
            let mut hooks = std::mem::take(&mut data.meta.hooks);

            let meta = parking_lot::RwLockWriteGuard::downgrade(data);

            let args = AnyVarHookArgs::new(&*meta.value, update, &tags);
            call_hooks(&mut hooks, args);
            drop(meta);

            let mut data = inner.write();
            hooks.append(&mut data.meta.hooks);
            data.meta.hooks = hooks;
        }

        VARS.wake_app();
    }
}

#[cfg(not(dyn_closure))]
fn apply_modify<T: VarValue>(inner: &RwLock<VarDataInner<T>>, modify: impl FnOnce(&mut VarModify<T>)) {
    let mut data = inner.write();
    if data.meta.skip_modify() {
        return;
    }

    let meta = parking_lot::RwLockWriteGuard::downgrade(data);
    let mut value = VarModify::new(&meta.value);
    modify(&mut value);
    let (notify, new_value, update, tags) = value.finish();

    if notify {
        drop(meta);
        let mut data = inner.write();
        if let Some(nv) = new_value {
            data.value = nv;
        }
        data.meta.last_update = VARS.update_id();

        if !data.meta.hooks.is_empty() {
            let mut hooks = std::mem::take(&mut data.meta.hooks);

            let meta = parking_lot::RwLockWriteGuard::downgrade(data);

            let args = AnyVarHookArgs::new(&meta.value, update, &tags);
            call_hooks(&mut hooks, args);
            drop(meta);

            let mut data = inner.write();
            hooks.append(&mut data.meta.hooks);
            data.meta.hooks = hooks;
        }

        VARS.wake_app();
    }
}

fn call_hooks(hooks: &mut Vec<VarHook>, args: AnyVarHookArgs) {
    hooks.retain(|h| h.call(&args));
}
