///Implements `T: IntoVar<U>`, `T: IntoValue<U>` and optionally `U: From<T>` without boilerplate.
///
/// The macro syntax is one or more functions with signature `fn from(t: T) -> U`. The [`LocalVar<U>`]
/// type is selected for variables. The syntax also supports generic types and constraints, but not `where` constraints.
/// You can also destructure the input if it is a tuple using the pattern `fn from((a, b): (A, B)) -> U`, but no other pattern
/// matching in the input is supported.
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
/// # use zng_var::impl_from_and_into_var;
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
/// # fn assert(_: impl zng_var::IntoVar<FooValue> + Into<FooValue>) { }
/// # assert(true);
/// # assert("on");
/// ```
#[macro_export]
macro_rules! impl_from_and_into_var {
    ($($tt:tt)+) => {
        $crate::__impl_from_and_into_var! { $($tt)* }
    };
}

use parking_lot::RwLock;

use crate::{AnyVarHookArgs, AnyVarValue};

use super::{VARS, VarHandle, VarHook, VarModify, VarUpdateId, VarValue, animation::ModifyInfo};

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
        let cur_anim = VARS.current_modify();
        if cur_anim.importance() < self.animation.importance() {
            return true;
        }
        self.animation = cur_anim;
        false
    }
}

struct VarDataInner {
    value: Box<dyn AnyVarValue>,
    meta: VarMeta,
}

pub(super) struct VarData(RwLock<VarDataInner>);

impl VarData {
    pub fn new(value: impl VarValue) -> Self {
        Self::new_impl(Box::new(value))
    }
    fn new_impl(value: Box<dyn AnyVarValue>) -> Self {
        Self(RwLock::new(VarDataInner {
            value,
            meta: VarMeta {
                last_update: VarUpdateId::never(),
                hooks: vec![],
                animation: ModifyInfo::never(),
            },
        }))
    }

    pub fn into_value<T: VarValue>(self) -> T {
        *self.0.into_inner().value.into_any().downcast::<T>().unwrap()
    }

    fn read<T: VarValue>(&self) -> parking_lot::MappedRwLockReadGuard<T> {
        let read = self.0.read();
        parking_lot::RwLockReadGuard::map(read, |r| r.value.as_any().downcast_ref::<T>().unwrap())
    }

    /// Read the value.
    pub fn with<T: VarValue, R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&*self.read())
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

    #[cfg(feature = "dyn_closure")]
    pub fn apply_modify<T: VarValue>(&self, modify: Box<dyn FnOnce(&mut VarModify<T>) + 'static>) {
        apply_modify(
            &self.0,
            Box::new(move |v| {
                let mut value = VarModify::new(v.as_any().downcast_ref::<T>().unwrap());
                modify(&mut value);
                let (notify, new_value, update, tags, importance) = value.finish();
                (
                    notify,
                    match new_value {
                        Some(v) => Some(Box::new(v)),
                        None => None,
                    },
                    update,
                    tags,
                    importance,
                )
            }),
        )
    }

    #[cfg(not(feature = "dyn_closure"))]
    pub fn apply_modify<T: VarValue>(&self, modify: impl FnOnce(&mut VarModify<T>) + 'static) {
        apply_modify(
            &self.0,
            Box::new(move |v| {
                let mut value = VarModify::new(v.as_any().downcast_ref::<T>().unwrap());
                modify(&mut value);
                let (notify, new_value, update, tags, importance) = value.finish();
                (
                    notify,
                    match new_value {
                        Some(v) => Some(Box::new(v)),
                        None => None,
                    },
                    update,
                    tags,
                    importance,
                )
            }),
        )
    }
}

fn apply_modify(
    inner: &RwLock<VarDataInner>,
    modify: Box<dyn FnOnce(&dyn AnyVarValue) -> (bool, Option<Box<dyn AnyVarValue>>, bool, Vec<Box<dyn AnyVarValue>>, Option<usize>)>,
) {
    let mut data = inner.write();
    if data.meta.skip_modify() {
        return;
    }

    let data = parking_lot::RwLockWriteGuard::downgrade(data);

    let (notify, new_value, update, tags, custom_importance) = modify(&*data.value);

    if notify {
        drop(data);
        let mut data = inner.write();
        if let Some(nv) = new_value {
            data.value = nv;
        }
        data.meta.last_update = VARS.update_id();

        if let Some(i) = custom_importance {
            data.meta.animation.importance = i;
        }

        if !data.meta.hooks.is_empty() {
            let mut hooks = std::mem::take(&mut data.meta.hooks);

            let meta = parking_lot::RwLockWriteGuard::downgrade(data);

            let args = AnyVarHookArgs::new(&*meta.value, update, &tags);
            hooks.retain(|h| h.call(&args));
            drop(meta);

            let mut data = inner.write();
            hooks.append(&mut data.meta.hooks);
            data.meta.hooks = hooks;
        }

        VARS.wake_app();
    } else if let Some(i) = custom_importance {
        drop(data);
        let mut data = inner.write();
        data.meta.animation.importance = i;
    }
}
