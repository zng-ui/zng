use core::fmt;
use std::{any::TypeId, marker::PhantomData, ops, sync::Arc, time::Duration};

use crate::{
    AnyVar, AnyVarHookArgs, BoxAnyVarValue, VarHandle, VarHandles, VarImpl, VarIsReadOnlyError, VarModify, VarValue, WeakAnyVar,
    animation::{
        Animation, AnimationHandle, ChaseAnimation, Transition, TransitionKeyed, Transitionable,
        easing::{EasingStep, EasingTime},
    },
    contextual_var,
};

use zng_clone_move::clmv;
use zng_txt::{ToTxt, Txt};
use zng_unit::{Factor, FactorUnits as _};

/// Variable of type `T`.
pub struct Var<T: VarValue> {
    pub(crate) any: AnyVar,
    _t: PhantomData<fn() -> T>,
}
impl<T: VarValue> Clone for Var<T> {
    fn clone(&self) -> Self {
        Self {
            any: self.any.clone(),
            _t: PhantomData,
        }
    }
}
impl<T: VarValue> From<Var<T>> for AnyVar {
    fn from(var: Var<T>) -> Self {
        var.any
    }
}
impl<T: VarValue> TryFrom<AnyVar> for Var<T> {
    type Error = AnyVar;

    fn try_from(var: AnyVar) -> Result<Self, Self::Error> {
        var.downcast()
    }
}
impl<T: VarValue> ops::Deref for Var<T> {
    type Target = AnyVar;

    fn deref(&self) -> &Self::Target {
        self.as_any()
    }
}
impl<T: VarValue> Var<T> {
    pub(crate) fn new_any(any: AnyVar) -> Self {
        Var { any, _t: PhantomData }
    }
}
impl<T: VarValue> fmt::Debug for Var<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Var").field(&self.any.0).finish()
    }
}

/// Value.
impl<T: VarValue> Var<T> {
    /// Visit a reference to the current value.
    pub fn with<O>(&self, visitor: impl FnOnce(&T) -> O) -> O {
        let mut once = Some(visitor);
        let mut output = None;
        self.0.with(&mut |v| {
            output = Some(once.take().unwrap()(v.downcast_ref().unwrap()));
        });
        output.unwrap()
    }

    /// Get a clone of the current value.
    pub fn get(&self) -> T {
        self.with(|v| v.clone())
    }

    /// Get a clone of the current value into `value`.
    ///
    /// This uses [`Clone::clone_from`] to reuse the `value` memory if supported.
    pub fn get_into(&self, value: &mut T) {
        self.with(|v| value.clone_from(v));
    }

    /// Visit a reference to the current value if it [`is_new`].
    ///
    /// [`is_new`]: AnyVar::is_new
    pub fn with_new<O>(&self, visitor: impl FnOnce(&T) -> O) -> Option<O> {
        if self.is_new() { Some(self.with(visitor)) } else { None }
    }

    /// Gets a clone of the current value if it [`is_new`].
    ///
    /// [`is_new`]: AnyVar::is_new
    pub fn get_new(&self) -> Option<T> {
        if self.is_new() { Some(self.get()) } else { None }
    }

    /// Gets a clone of the current value into `value` if it [`is_new`].
    ///
    /// This uses [`Clone::clone_from`] to reuse the `value` memory if supported.
    ///
    /// [`is_new`]: AnyVar::is_new
    pub fn get_new_into(&self, value: &mut T) -> bool {
        self.with_new(|v| value.clone_from(v)).is_some()
    }

    /// Schedule `new_value` to be assigned next update.
    pub fn try_set(&self, new_value: impl Into<T>) -> Result<(), VarIsReadOnlyError> {
        self.any.try_set(BoxAnyVarValue::new(new_value.into()))
    }

    /// Schedule `new_value` to be assigned next update.
    ///
    /// If the variable is read-only this is ignored and a DEBUG level log is recorded.
    /// Use [`try_set`] to get an error for read-only vars.
    ///
    /// [`try_set`]: Self::try_set
    pub fn set(&self, new_value: impl Into<T>) {
        trace_debug_error!(self.try_set(new_value))
    }

    /// Schedule `modify` to be called on the value for the next update.
    ///
    /// If the [`VarModify`] value is deref mut the variable will notify an update.
    pub fn try_modify(&self, modify: impl FnOnce(&mut VarModify<T>) + Send + 'static) -> Result<(), VarIsReadOnlyError> {
        self.any.try_modify(move |value| {
            modify(&mut value.downcast::<T>().unwrap());
        })
    }

    /// Schedule `modify` to be called on the value for the next update.
    ///
    /// If the [`VarModify`] value is deref mut the variable will notify an update.
    ///
    /// If the variable is read-only this is ignored and a DEBUG level log is recorded.
    /// Use [`try_modify`] to get an error for read-only vars.
    ///
    /// [`try_modify`]: Self::try_modify
    pub fn modify(&self, modify: impl FnOnce(&mut VarModify<T>) + Send + 'static) {
        trace_debug_error!(self.try_modify(modify))
    }

    /// Schedule a new `value` for the variable, it will be set in the end of the current app update to the updated
    /// value of `other`, so if the other var has already scheduled an update, the updated value will be used.
    ///  
    /// This can be used just before creating a binding to start with synchronized values.
    pub fn try_set_from(&self, other: &Var<T>) -> Result<(), VarIsReadOnlyError> {
        self.any.try_set_from(other)
    }

    /// Schedule a new `value` for the variable, it will be set in the end of the current app update to the updated
    /// value of `other`, so if the other var has already scheduled an update, the updated value will be used.
    ///  
    /// This can be used just before creating a binding to start with synchronized values.
    ///
    /// If the variable is read-only this is ignored and a DEBUG level log is recorded.
    /// Use [`try_set_from`] to get an error for read-only vars.
    ///
    /// [`try_set_from`]: Self::try_set_from
    pub fn set_from(&self, other: &Var<T>) {
        trace_debug_error!(self.try_set_from(other))
    }

    /// Like [`try_set_from`], but uses `map` to produce the new value from the updated value of `other`.
    ///
    /// [`try_set_from`]: Self::try_set_from
    pub fn try_set_from_map<O: VarValue>(
        &self,
        other: &Var<O>,
        map: impl FnOnce(&O) -> T + Send + 'static,
    ) -> Result<(), VarIsReadOnlyError> {
        self.any
            .try_set_from_map(other, move |v| BoxAnyVarValue::new(map(v.downcast_ref::<O>().unwrap())))
    }

    /// Like [`set_from`], but uses `map` to produce the new value from the updated value of `other`.
    ///
    /// If the variable is read-only this is ignored and a DEBUG level log is recorded.
    /// Use [`try_set_from_map`] to get an error for read-only vars.
    ///
    /// [`try_set_from_map`]: Self::try_set_from_map
    /// [`set_from`]: Self::set_from
    pub fn set_from_map<O: VarValue>(&self, other: &Var<O>, map: impl FnOnce(&O) -> T + Send + 'static) {
        trace_debug_error!(self.try_set_from_map(other, map))
    }

    /// Setups a callback for just after the variable value update is applied, the closure runs in the root app context, just like
    /// the `modify` closure. The closure must return `true` to be retained and `false` to be dropped.
    ///
    /// If you modify another variable in the closure modification applies in the same update, variable mapping and
    /// binding is implemented using hooks.
    ///
    /// The variable store a weak reference to the callback if it has the `MODIFY` or `CAPS_CHANGE` capabilities, otherwise
    /// the callback is discarded and [`VarHandle::dummy`] returned.
    pub fn hook(&self, mut on_update: impl FnMut(&VarHookArgs<T>) -> bool + Send + 'static) -> VarHandle {
        self.any
            .hook(move |args: &AnyVarHookArgs| -> bool { on_update(&args.downcast().unwrap()) })
    }

    ///Awaits for a value that passes the `predicate`, including the current value.
    #[allow(clippy::manual_async_fn)] // false positive, async fn futures are not Send + Sync
    pub fn wait_match(&self, predicate: impl Fn(&T) -> bool + Send + Sync) -> impl Future<Output = ()> + Send + Sync {
        self.any.wait_match(move |v| predicate(v.downcast_ref::<T>().unwrap()))
    }

    /// Awaits for an update them [`get`] the value.
    ///
    /// [`get`]: Self::get
    #[allow(clippy::manual_async_fn)] // false positive, async fn futures are not Send + Sync
    pub fn wait_next(&self) -> impl Future<Output = T> + Send + Sync {
        async {
            self.wait_update().await;
            self.get()
        }
    }

    /// Debug helper for tracing the lifetime of a value in this variable.
    ///
    /// The `enter_value` closure is called every time the variable updates, it can return
    /// an implementation agnostic *scope* or *span* `S` that is only dropped when the variable updates again.
    ///
    /// The `enter_value` is also called immediately when this method is called to start tracking the first value.
    ///
    /// Returns a [`VarHandle`] that can be used to stop tracing.
    ///
    /// If this variable can never update the span is immediately dropped and a dummy handle is returned.
    pub fn trace_value<S: Send + 'static>(&self, mut enter_value: impl FnMut(&VarHookArgs<T>) -> S + Send + 'static) -> VarHandle {
        self.any.trace_value(move |args| enter_value(&args.downcast::<T>().unwrap()))
    }
}
/// Value mapping.
impl<T: VarValue> Var<T> {
    /// Create a read-only mapping variable.
    ///
    /// The `map` closure must produce a mapped value from this variable's value.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use zng_var::*;
    /// # use zng_txt::*;
    /// let n_var = var(0u32);
    /// let n_10_var = n_var.map(|n| *n * 10);
    /// let txt_var = n_10_var.map(|n| if *n < 100 { formatx!("{n}!") } else { formatx!("Done!") });
    /// ```
    ///
    /// In the example above the `txt_var` will update every time the `n_var` updates.
    ///
    /// # Capabilities
    ///
    /// If this variable is static the `map` closure is called immediately and dropped, the mapping variable is also static.
    ///
    /// If this variable is a shared reference the `map` closure is called immediately to init the mapping variable and
    /// is called again for every update of this variable. The mapping variable is another shared reference and it holds
    /// a strong reference to this variable.
    ///
    /// If this variable is contextual the initial `map` call is deferred until first usage of the mapping variable. The
    /// mapping variable is also contextual and will init for every context it is used in.
    ///
    /// The mapping variable is read-only, see [`map_bidi`] for read-write mapping.
    ///
    /// If the `map` closure produce an equal value the mapping variable will not update, see also [`filter_map`]
    /// to skip updating for some input values.
    ///
    /// [`map_bidi`]: Self::map_bidi
    /// [`filter_map`]: Self::filter_map
    pub fn map<O: VarValue>(&self, mut map: impl FnMut(&T) -> O + Send + 'static) -> Var<O> {
        self.any.map(move |v| map(v.downcast_ref::<T>().unwrap()))
    }

    /// Create a [`map`] that converts from `T` to `O` using [`Into<O>`].
    ///
    /// [`map`]: Var::map
    pub fn map_into<O>(&self) -> Var<O>
    where
        O: VarValue,
        T: Into<O>,
    {
        self.map(|v| v.clone().into())
    }

    /// Create a [`map`] that converts from `T` to [`Txt`] using [`ToTxt`].
    ///
    /// [`map`]: Var::map
    /// [`Txt`]: Txt
    /// [`ToTxt`]: ToTxt
    pub fn map_to_txt(&self) -> Var<Txt>
    where
        T: ToTxt,
    {
        self.map(ToTxt::to_txt)
    }

    /// Create a [`map`] that references and clones `O` from `T` using `std::ops::Deref<Target = O>`.
    ///
    /// The mapping variable is read-only, see [`map_deref_mut`] for mutable referencing.
    ///
    /// [`map`]: Self::map
    /// [`map_deref_mut`]: Self::map_deref_mut
    pub fn map_deref<O>(&self) -> Var<O>
    where
        O: VarValue,
        T: ops::Deref<Target = O>,
    {
        self.map(|v| ops::Deref::deref(v).clone())
    }

    /// Create a mapping variable that can skip updates.
    ///
    /// The `map` closure is called for every update this variable and if it returns a new value the mapping variable updates.
    ///
    /// If the `map` closure does not produce a value on init the `fallback_init` closure is called.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use zng_var::*;
    /// # use zng_txt::*;
    /// let n_var = var(100u32);
    /// let txt_var = n_var.filter_map(|n| if *n < 100 { Some(formatx!("{n}!")) } else { None }, || "starting...".into());
    /// ```
    ///
    /// In the example above the `txt_var` will update every time the `n_var` updates with value `n < 100`. Because
    /// the `n_var` initial value does not match the filter the fallback value `"starting..."` is used.
    ///
    /// # Capabilities
    ///
    /// If this variable is static the closures are called immediately and dropped, the mapping variable is also static.
    ///
    /// If this variable is a shared reference the closures are called immediately to init the mapping variable and
    /// are called again for every update of this variable. The mapping variable is another shared reference and it holds
    /// a strong reference to this variable.
    ///
    /// If this variable is contextual the initial closures call is deferred until first usage of the mapping variable. The
    /// mapping variable is also contextual and will init for every context it is used in.
    ///
    /// The mapping variable is read-only, see [`filter_map_bidi`] for read-write mapping.
    ///
    /// [`filter_map_bidi`]: Self::filter_map_bidi
    pub fn filter_map<O: VarValue>(
        &self,
        mut map: impl FnMut(&T) -> Option<O> + Send + 'static,
        fallback_init: impl Fn() -> O + Send + 'static,
    ) -> Var<O> {
        self.any.filter_map(move |v| map(v.downcast_ref::<T>().unwrap()), fallback_init)
    }

    /// Create a [`filter_map`] that tries to convert from `T` to `O` using [`TryInto<O>`].
    ///
    /// [`filter_map`]: Var::filter_map
    pub fn filter_try_into<O, I>(&self, fallback_init: I) -> Var<O>
    where
        O: VarValue,
        T: TryInto<O>,
        I: Fn() -> O + Send + Sync + 'static,
    {
        self.filter_map(|v| v.clone().try_into().ok(), fallback_init)
    }

    /// Create a [`filter_map`] that tries to convert from `T` to `O` using [`FromStr`].
    ///
    /// [`filter_map`]: Var::filter_map
    /// [`FromStr`]: std::str::FromStr
    pub fn filter_parse<O, I>(&self, fallback_init: I) -> Var<O>
    where
        O: VarValue + std::str::FromStr,
        T: AsRef<str>,
        I: Fn() -> O + Send + Sync + 'static,
    {
        self.filter_map(|v| v.as_ref().parse().ok(), fallback_init)
    }

    /// Create a bidirectional mapping variable.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use zng_var::*;
    /// # use zng_txt::*;
    /// let n_var = var(0u32);
    /// let n_100_var = n_var.map_bidi(|n| n * 100, |n_100| n_100 / 100);
    /// ```
    ///
    /// In the example above the `n_100_var` will update every time the `n_var` updates and the `n_var` will
    /// update every time the `n_100_var` updates.
    ///
    /// # Capabilities
    ///
    /// If this variable is static the `map` closure is called immediately and dropped, the mapping variable is also static,
    /// the `map_back` closure is ignored.
    ///
    /// If this variable is a shared reference the `map` closure is called immediately to init the mapping variable.
    /// The mapping variable is another shared reference and it holds a strong reference to this variable.
    /// The `map` closure is called again for every update of this variable that is not caused by the mapping variable.
    /// The `map_back` closure is called for every update of the mapping variable that was not caused by this variable.
    ///
    /// If this variable is contextual the initial `map` call is deferred until first usage of the mapping variable. The
    /// mapping variable is also contextual and will init for every context it is used in.
    pub fn map_bidi<O: VarValue>(
        &self,
        mut map: impl FnMut(&T) -> O + Send + 'static,
        mut map_back: impl FnMut(&O) -> T + Send + 'static,
    ) -> Var<O> {
        let mapping = self.map_bidi_any(
            move |input| BoxAnyVarValue::new(map(input.downcast_ref::<T>().unwrap())),
            move |output| BoxAnyVarValue::new(map_back(output.downcast_ref::<O>().unwrap())),
            TypeId::of::<O>(),
        );
        Var::new_any(mapping)
    }

    /// Create a bidirectional mapping variable that modifies back instead of mapping back.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use zng_var::*;
    /// # use zng_txt::*;
    /// let list_var = var(vec!['a', 'b', 'c']);
    /// let first_var = list_var.map_bidi_modify(
    ///     // map:
    ///     |l| l.first().copied().unwrap_or('_'),
    ///     // modify_back:
    ///     |c, l| if l.is_empty() { l.push(*c) } else { l[0] = *c },
    /// );
    /// ```
    ///
    /// In the example above the `first_var` represents the first item on the vector in `list_var`. Note that the `map` closure
    /// works the same as in [`map_bidi`], but the `modify_back` closure modifies the list. This is not a mapping that can be declared
    /// with [`map_bidi`] as the mapping variable does not have the full list to map back.
    ///
    /// # Capabilities
    ///
    /// If this variable is static the `map` closure is called immediately and dropped, the mapping variable is also static,
    /// the `modify_back` closure is ignored.
    ///
    /// If this variable is a shared reference the `map` closure is called immediately to init the mapping variable.
    /// The mapping variable is another shared reference and it holds a strong reference to this variable.
    /// The `map` closure is called again for every update of this variable that is not caused by the mapping variable.
    /// The `modify_back` closure is called for every update of the mapping variable that was not caused by this variable.
    ///
    /// If this variable is contextual the initial `map` call is deferred until first usage of the mapping variable. The
    /// mapping variable is also contextual and will init for every context it is used in.
    ///
    /// Like other mappings and bindings cyclic updates are avoided automatically, if the `modify_back` closure touches/updates the value
    /// a var instance tag is inserted after the closure returns, you do not need to mark it manually.
    ///
    /// [`map_bidi`]: Self::map_bidi
    pub fn map_bidi_modify<O: VarValue>(
        &self,
        mut map: impl FnMut(&T) -> O + Send + 'static,
        mut modify_back: impl FnMut(&O, &mut VarModify<T>) + Send + 'static,
    ) -> Var<O> {
        let mapping = self.map_bidi_modify_any(
            move |input| BoxAnyVarValue::new(map(input.downcast_ref::<T>().unwrap())),
            move |v, m| modify_back(v.downcast_ref::<O>().unwrap(), &mut m.downcast::<T>().unwrap()),
            TypeId::of::<O>(),
        );
        Var::new_any(mapping)
    }

    /// Create a [`map_bidi`] that converts between `T` and `O` using [`Into`].
    ///
    /// [`map_bidi`]: Var::map_bidi
    pub fn map_into_bidi<O>(&self) -> Var<O>
    where
        O: VarValue + Into<T>,
        T: Into<O>,
    {
        self.map_bidi(|t| t.clone().into(), |o| o.clone().into())
    }

    /// Create a [`map_bidi_modify`] that references and clones `O` from `T` using `std::ops::Deref<Target = O>` and
    /// modifies back using `std::ops::DerefMut<Target = O>`.
    ///
    /// [`map_bidi_modify`]: Self::map_bidi_modify
    pub fn map_deref_mut<O>(&self) -> Var<O>
    where
        O: VarValue,
        T: ops::Deref<Target = O>,
        T: ops::DerefMut<Target = O>,
    {
        self.map_bidi_modify(
            |input| T::deref(input).clone(),
            |output, modify| *T::deref_mut(modify) = output.clone(),
        )
    }

    /// Create a bidirectional mapping variable that can skip updates.
    ///
    /// If the `map` closure does not produce a value on init the `fallback_init` closure is called.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use zng_var::*;
    /// # use zng_txt::*;
    /// let n_var = var(0u32);
    /// let n_100_var = n_var.filter_map_bidi(
    ///     |n| Some(n * 100),
    ///     |n_100| {
    ///         let r = n_100 / 100;
    ///         if r < 100 { Some(r) } else { None }
    ///     },
    ///     || 0,
    /// );
    /// ```
    ///
    /// In the example above the `n_100_var` will update every time the `n_var` updates with any value and the `n_var` will
    /// update every time the `n_100_var` updates with a value that `(n_100 / 100) < 100`.
    ///
    /// # Capabilities
    ///
    /// If this variable is static the `map` closure is called immediately and dropped, the mapping variable is also static,
    /// the `map_back` closure is ignored.
    ///
    /// If this variable is a shared reference the `map` closure is called immediately to init the mapping variable.
    /// The mapping variable is another shared reference and it holds a strong reference to this variable.
    /// The `map` closure is called again for every update of this variable that is not caused by the mapping variable.
    /// The `map_back` closure is called for every update of the mapping variable that was not caused by this variable.
    ///
    /// If this variable is contextual the initial `map` call is deferred until first usage of the mapping variable. The
    /// mapping variable is also contextual and will init for every context it is used in.
    pub fn filter_map_bidi<O: VarValue>(
        &self,
        mut map: impl FnMut(&T) -> Option<O> + Send + 'static,
        mut map_back: impl FnMut(&O) -> Option<T> + Send + 'static,
        fallback_init: impl Fn() -> O + Send + 'static,
    ) -> Var<O> {
        let mapping = self.filter_map_bidi_any(
            move |t| map(t.downcast_ref::<T>().unwrap()).map(BoxAnyVarValue::new),
            move |o| map_back(o.downcast_ref::<O>().unwrap()).map(BoxAnyVarValue::new),
            move || BoxAnyVarValue::new(fallback_init()),
            TypeId::of::<O>(),
        );
        Var::new_any(mapping)
    }

    /// Create a [`filter_map_bidi`] that tries to convert between `T` to `O` using [`TryInto`].
    ///
    /// [`filter_map_bidi`]: Var::filter_map_bidi
    pub fn filter_try_into_bidi<O, I>(&self, fallback_init: I) -> Var<O>
    where
        O: VarValue,
        T: TryInto<O>,
        O: TryInto<T>,
        I: Fn() -> O + Send + Sync + 'static,
    {
        self.filter_map_bidi(|v| v.clone().try_into().ok(), |o| o.clone().try_into().ok(), fallback_init)
    }

    /// Create a flat mapping variable that *unwraps* an inner variable stored in the the value of this variable.
    ///
    /// # Capabilities
    ///
    /// If this variable is static the `map` closure is called immediately and dropped and the inner variable is returned.
    ///
    /// If this variable is a shared reference the `map` closure is called immediately to init the mapping variable and
    /// is called again for every update of this variable. The mapping variable is another shared reference and it holds
    /// a strong reference to this variable and to the inner variable.
    ///
    /// If this variable is contextual the initial `map` call is deferred until first usage of the mapping variable. The
    /// mapping variable is also contextual and will init for every context it is used in.
    ///
    /// The mapping variable has the same capabilities of the inner variable, plus [`MODIFY_CHANGES`]. When the inner variable
    /// is writeable the return variable is too.
    ///
    /// [`map`]: Var::map
    /// [`MODIFY_CHANGES`]: crate::VarCapability::MODIFY_CHANGES
    pub fn flat_map<O: VarValue>(&self, mut map: impl FnMut(&T) -> Var<O> + Send + 'static) -> Var<O> {
        self.any.flat_map(move |v| map(v.downcast_ref::<T>().unwrap()))
    }
}
impl<T: VarValue> Var<crate::VarEq<T>> {
    /// Create a [`flat_map`] to the inner variable.
    ///
    /// [`flat_map`]: Self::flat_map
    pub fn flatten(&self) -> Var<T> {
        self.flat_map(|v| v.0.clone())
    }
}
impl<T: VarValue> Var<Vec<T>> {
    /// Create a flat mapping variable that maps each variable entry in `self` to a value entry in the output.
    ///
    /// This is similar to [`flat_map`], but operates over many inner variables in a `Vec` value. Note that the `map`
    /// closure inputs are each item index and the item.
    ///
    /// # Capabilities
    ///
    /// If this variable is static the `map` closure is called immediately for each item and dropped. The selected inner variables
    /// are merged into the mapping variable and will continue updating it.
    ///
    /// If this variable is a shared reference the `map` closure is called immediately for each item to init the result and will
    /// be called again for each item for every update of this variable. The mapping variable is another shared reference and it holds
    /// a strong reference to this variable and the inner variables.
    ///
    /// If this variable is contextual the initial `map` calls is deferred until first usage of the mapping variable. The
    /// mapping variable is also contextual and will init for every context it is used in.
    ///
    /// The mapping variable is read-only.
    ///
    /// [`flat_map`]: Self::flat_map
    pub fn flat_map_vec<O: VarValue>(&self, mut map: impl FnMut(usize, &T) -> Var<O> + Send + 'static) -> Var<Vec<O>> {
        self.flat_map(move |vec| {
            let mut item_vars: Vec<(Var<O>, VarHandle)> = vec.iter().enumerate().map(|(i, it)| (map(i, it), VarHandle::dummy())).collect();
            let out_value: Vec<O> = item_vars.iter().map(|v| v.0.get()).collect();
            let out_var = crate::var(out_value);

            for (i, (item_var, handle)) in item_vars.iter_mut().enumerate() {
                *handle = item_var.bind_modify(&out_var, move |item_value, out_value| {
                    if &out_value.value()[i] != item_value {
                        out_value.value_mut()[i] = item_value.clone();
                    }
                });
            }
            out_var.hold(item_vars).perm();

            out_var.read_only()
        })
    }
}
/// Binding
impl<T: VarValue> Var<T> {
    /// Bind `other` to receive the new values from this variable.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use zng_var::*;
    /// #
    /// let a = var(10);
    /// let b = var(0);
    ///
    /// a.bind(&b).perm();
    /// ```
    ///
    /// In the example above the variable `b` will be set every time the variable `a` updates. Note that the current
    /// value is not propagated, only updates. You can use [`set_bind`] to assign the current value and bind.
    ///
    /// # Capabilities
    ///
    /// If this variable is const or the other variable is always read-only does nothing and returns a dummy handle.
    ///
    /// If any variable is contextual the binding is set on the current context inner variable.
    ///
    /// Neither variable holds the other, only a weak reference is used, if either variable or the handle is dropped the binding
    /// is dropped.
    ///
    /// [`set_bind`]: Self::set_bind
    pub fn bind(&self, other: &Var<T>) -> VarHandle {
        self.any.bind(other)
    }

    /// Like [`bind`] but also sets `other` to the current value.
    ///
    /// Basic usage:
    ///
    /// ```
    /// # fn demo() {
    /// # use zng_var::*;
    /// #
    /// let a = var(10);
    /// let b = var(0);
    ///
    /// a.set_bind(&b).perm();
    /// # }
    /// ```
    ///
    /// In the example above the variable `b` will be set to the current value of `a` and every time the variable `a` updates.
    ///
    /// # Capabilities
    ///
    /// If this variable is const or the other variable is always read-only does nothing and returns a dummy handle.
    ///
    /// If any variable is contextual the binding is set on the current context inner variable.
    ///
    /// Neither variable holds the other, only a weak reference is used, if either variable or the handle is dropped the binding
    /// is dropped.
    ///
    /// [`bind`]: Self::bind
    pub fn set_bind(&self, other: &Var<T>) -> VarHandle {
        self.any.set_bind(other)
    }

    /// Bind `other` to receive the new values mapped from this variable.
    ///
    /// This has the same capabilities as [`bind`], but the `map` closure is called to produce the new value for `other`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use zng_var::*;
    /// # use zng_txt::*;
    /// let a = var(10);
    /// let b = var(Txt::from(""));
    ///
    /// a.bind_map(&b, |&a| formatx!("{:?}", a * 2)).perm();
    /// ```
    ///
    /// In the example above every time the variable `a` updates the variable `b` will be set to the text representation of the value times two.
    ///
    /// [`bind`]: Self::bind
    pub fn bind_map<O: VarValue>(&self, other: &Var<O>, mut map: impl FnMut(&T) -> O + Send + 'static) -> VarHandle {
        self.any.bind_map(other, move |v| map(v.downcast_ref::<T>().unwrap()))
    }

    /// Like [`bind_map`] but also sets `other` to the current value.
    ///
    /// This has the same capabilities as [`set_bind`], but the `map` closure is called to produce the new value for `other`.
    ///
    /// [`bind_map`]: Self::bind_map
    /// [`set_bind`]: Self::set_bind
    pub fn set_bind_map<O: VarValue>(&self, other: &Var<O>, mut map: impl FnMut(&T) -> O + Send + 'static) -> VarHandle {
        self.any.set_bind_map(other, move |v| map(v.downcast_ref::<T>().unwrap()))
    }

    /// Bind `other` to be modified when this variable updates.
    ///
    /// This has the same capabilities as [`bind`], but the `modify` closure is called to modify `other` using a reference to the new value.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use zng_var::*;
    /// #
    /// let a = var(10);
    /// let b = var(vec![1, 2, 3]);
    /// a.bind_modify(&b, |&a, b| {
    ///     if b.is_empty() {
    ///         b.push(a);
    ///     } else {
    ///         b[0] = a;
    ///     }
    /// })
    /// .perm();
    /// ```
    ///
    /// In the example above the variable `b` first element is set to the updated value of `a`.
    ///
    /// [`bind`]: Self::bind
    pub fn bind_modify<O: VarValue>(&self, other: &Var<O>, mut modify: impl FnMut(&T, &mut VarModify<O>) + Send + 'static) -> VarHandle {
        self.any.bind_modify(other, move |v, m| modify(v.downcast_ref::<T>().unwrap(), m))
    }

    /// Bind `other` to receive the new values from this variable and this variable to receive new values from `other`.
    ///
    /// # Capabilities
    ///
    /// This has the same capabilities as [`bind`], it is equivalent of setting two bindings.
    ///
    /// The bindings are protected against cyclic updates, like all other mappings and bindings.
    ///
    /// [`bind`]: Self::bind
    pub fn bind_bidi(&self, other: &Var<T>) -> VarHandles {
        self.any.bind_bidi(other)
    }

    /// Bind `other` to receive the new mapped values from this variable and this variable to receive new mapped values from `other`.
    ///
    /// This has the same capabilities as [`bind_bidi`], but the `map` closure is called to produce the new value for `other`
    /// and `map_back` is called to produce the new value for this variable.
    ///
    /// [`bind_bidi`]: Self::bind_bidi
    pub fn bind_map_bidi<O: VarValue>(
        &self,
        other: &Var<O>,
        mut map: impl FnMut(&T) -> O + Send + 'static,
        mut map_back: impl FnMut(&O) -> T + Send + 'static,
    ) -> VarHandles {
        self.any.bind_map_bidi_any(
            other,
            move |v| BoxAnyVarValue::new(map(v.downcast_ref::<T>().unwrap())),
            move |v| BoxAnyVarValue::new(map_back(v.downcast_ref::<O>().unwrap())),
        )
    }

    /// Bind `other` to be modified when this variable updates and this variable to be modified when `other` updates.
    ///
    /// This has the same capabilities as [`bind_bidi`], but the `modify` closure is called to modify `other`
    /// and `modify_back` is called to modify this variable.
    ///
    /// [`bind_bidi`]: Self::bind_bidi
    pub fn bind_modify_bidi<O: VarValue>(
        &self,
        other: &Var<O>,
        mut modify: impl FnMut(&T, &mut VarModify<O>) + Send + 'static,
        mut modify_back: impl FnMut(&O, &mut VarModify<T>) + Send + 'static,
    ) -> VarHandles {
        self.any.bind_modify_bidi(
            other,
            move |v, m| modify(v.downcast_ref::<T>().unwrap(), m),
            move |v, m| modify_back(v, &mut m.downcast::<T>().unwrap()),
        )
    }

    /// Bind `other` to receive the new values filtered mapped from this variable.
    ///
    /// This has the same capabilities as [`bind_map`], except that `other` will only receive a new value if `map` returns a value.
    ///
    /// [`bind_map`]: Self::bind_map
    pub fn bind_filter_map<O: VarValue>(&self, other: &Var<O>, mut map: impl FnMut(&T) -> Option<O> + Send + 'static) -> VarHandle {
        self.any.bind_filter_map(other, move |v| map(v.downcast_ref::<T>().unwrap()))
    }

    /// Bind `other` to receive the new filtered mapped values from this variable and this variable to receive
    /// new filtered mapped values from `other`.
    pub fn bind_filter_map_bidi<O: VarValue>(
        &self,
        other: &Var<O>,
        mut map: impl FnMut(&T) -> Option<O> + Send + 'static,
        mut map_back: impl FnMut(&O) -> Option<T> + Send + 'static,
    ) -> VarHandles {
        self.any.bind_filter_map_bidi_any(
            other,
            move |v| map(v.downcast_ref::<T>().unwrap()).map(BoxAnyVarValue::new),
            move |v| map_back(v.downcast_ref::<O>().unwrap()).map(BoxAnyVarValue::new),
        )
    }
}
/// Animation
impl<T: VarValue> Var<T> {
    /// Schedule a custom animation that targets this variable.
    ///
    /// The `animate` closure is called every frame, starting after next frame, the closure inputs are
    /// the [`Animation`] args and *modify* access to the variable value, the args
    /// can be used to calculate the new variable value and to control or stop the animation.
    ///
    /// # Examples
    ///
    /// Customs animation that displays the animation elapsed time:
    ///
    /// ```
    /// # fn demo() {
    /// # use zng_var::*;
    /// # use zng_txt::*;
    /// # use zng_unit::*;
    /// let status = var(Txt::from("not animating"));
    ///
    /// status
    ///     .animate(|animation, value| {
    ///         let elapsed = animation.elapsed_dur();
    ///         if elapsed < 5.secs() {
    ///             value.set(formatx!("animating: elapsed {}ms", elapsed.as_millis()));
    ///         } else {
    ///             animation.stop();
    ///             value.set("not animating");
    ///         }
    ///     })
    ///     .perm();
    /// # }
    /// ```
    ///
    /// # Capabilities
    ///
    /// If the variable is always read-only no animation is created and a dummy handle returned.
    ///
    /// If this var is contextual the animation targets the current context var.
    ///
    /// The animation is stopped if this variable is dropped.
    ///
    /// [`Animation`]: Animation
    pub fn animate(&self, mut animate: impl FnMut(&Animation, &mut VarModify<T>) + Send + 'static) -> AnimationHandle {
        self.any.animate(move |a, v| animate(a, &mut v.downcast::<T>().unwrap()))
    }

    /// Schedule animations started by `animate`, the closure is called once at the start to begin, then again every time
    /// the variable stops animating.
    ///
    /// This can be used to create a sequence of animations or to repeat an animation.
    ///
    /// # Examples
    ///
    /// Running multiple animations in sequence:
    ///
    /// ```
    /// # fn demo() {
    /// # use zng_var::{*, animation::*};
    /// # use zng_txt::*;
    /// # use zng_unit::*;
    /// let status = var(Txt::from("not animating"));
    ///
    /// let mut stage = 0;
    /// status
    ///     .sequence(move |status| {
    ///         stage += 1;
    ///         if stage < 5 {
    ///             status.animate(move |animation, value| {
    ///                 let elapsed = animation.elapsed_stop(5.secs());
    ///                 value.set(formatx!("animation {stage}: {}", elapsed.pct()));
    ///             })
    ///         } else {
    ///             status.set("not animating");
    ///             AnimationHandle::dummy()
    ///         }
    ///     })
    ///     .perm();
    /// # }
    /// ```
    ///
    /// # Capabilities
    ///
    /// The sequence stops when `animate` returns a dummy handle, or the variable is modified outside of `animate`,
    /// or animations are disabled, or the returned handle is dropped.
    pub fn sequence(&self, mut animate: impl FnMut(Var<T>) -> AnimationHandle + Send + 'static) -> VarHandle {
        self.any.sequence(move |v| animate(Var::new_any(v)))
    }

    /// Schedule an easing transition from the `start_value` to `end_value`.
    ///
    /// The variable updates every time the [`EasingStep`] for each frame changes and a different value is sampled.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # fn demo() {
    /// # use zng_var::{*, animation::easing};
    /// # use zng_unit::*;
    /// let progress = var(0.pct());
    ///
    /// progress.set_ease(0.pct(), 100.pct(), 5.secs(), easing::linear).perm();
    /// # }
    /// ```
    ///
    /// Variable is reset to 0% at the start and them transition to 100% in 5 seconds with linear progression.
    ///
    /// # Capabilities
    ///
    /// See [`animate`] for details about animation capabilities.
    ///
    /// [`animate`]: Self::animate
    pub fn set_ease(
        &self,
        start_value: impl Into<T>,
        end_value: impl Into<T>,
        duration: Duration,
        easing: impl Fn(EasingTime) -> EasingStep + Send + 'static,
    ) -> AnimationHandle
    where
        T: Transitionable,
    {
        self.set_ease_with(start_value, end_value, duration, easing, Transition::sample)
    }

    /// Oscillate between `start_value` to `end_value` with an easing transition.
    ///
    /// The `duration` defines the easing duration between the two values. The animation will continue running
    /// until the handle or the variable is dropped.
    ///
    /// Note that you can use [`sequence`] to create more complex looping animations.
    ///
    /// [`sequence`]: Var::sequence
    pub fn set_ease_oci(
        &self,
        start_value: impl Into<T>,
        end_value: impl Into<T>,
        duration: Duration,
        easing: impl Fn(EasingTime) -> EasingStep + Send + 'static,
    ) -> AnimationHandle
    where
        T: Transitionable,
    {
        self.set_ease_oci_with(start_value, end_value, duration, easing, Transition::sample)
    }

    /// Schedule an easing transition from the `start_value` to `end_value` using a custom value sampler.
    ///
    /// The variable updates every time the [`EasingStep`] for each frame changes and a different value is sampled.
    ///
    /// See [`animate`] for details about animation capabilities.
    ///
    /// [`animate`]: Self::animate
    pub fn set_ease_with(
        &self,
        start_value: impl Into<T>,
        end_value: impl Into<T>,
        duration: Duration,
        easing: impl Fn(EasingTime) -> EasingStep + Send + 'static,
        sampler: impl Fn(&Transition<T>, EasingStep) -> T + Send + 'static,
    ) -> AnimationHandle
    where
        T: Transitionable,
    {
        self.ease_impl(start_value.into(), end_value.into(), duration, easing, 999.fct(), sampler)
    }

    /// Oscillate between `start_value` to `end_value` with an easing transition using a custom value sampler.
    ///
    /// The `duration` defines the easing duration between the two values.
    ///
    /// Note that you can use [`sequence`] to create more complex looping animations.
    ///
    /// [`sequence`]: Self::sequence
    pub fn set_ease_oci_with(
        &self,
        start_value: impl Into<T>,
        end_value: impl Into<T>,
        duration: Duration,
        easing: impl Fn(EasingTime) -> EasingStep + Send + 'static,
        sampler: impl Fn(&Transition<T>, EasingStep) -> T + Send + 'static,
    ) -> AnimationHandle
    where
        T: Transitionable,
    {
        self.ease_oci_impl(start_value.into(), end_value.into(), duration, easing, 999.fct(), sampler)
    }

    /// Schedule an easing transition from the current value to `new_value`.
    ///
    /// The variable updates every time the [`EasingStep`] for each frame changes and a different value is sampled.
    ///
    /// See [`animate`] for details about animation capabilities.
    ///
    /// [`animate`]: Var::animate
    pub fn ease(
        &self,
        new_value: impl Into<T>,
        duration: Duration,
        easing: impl Fn(EasingTime) -> EasingStep + Send + 'static,
    ) -> AnimationHandle
    where
        T: Transitionable,
    {
        self.ease_with(new_value, duration, easing, Transition::sample)
    }

    /// Oscillate between the current value and `new_value` with an easing transition.
    ///
    /// The `duration` defines the easing duration between the two values.
    ///
    /// Note that you can use [`sequence`] to create more complex looping animations.
    ///
    /// [`sequence`]: Var::sequence
    pub fn ease_oci(
        &self,
        new_value: impl Into<T>,
        duration: Duration,
        easing: impl Fn(EasingTime) -> EasingStep + Send + 'static,
    ) -> AnimationHandle
    where
        T: Transitionable,
    {
        self.ease_oci_with(new_value, duration, easing, Transition::sample)
    }

    /// Schedule an easing transition from the current value to `new_value` using a custom value sampler.
    ///
    /// The variable updates every time the [`EasingStep`] for each frame changes and a different value is sampled.
    ///
    /// See [`animate`] for details about animation capabilities.
    ///
    /// [`animate`]: Var::animate
    pub fn ease_with(
        &self,
        new_value: impl Into<T>,
        duration: Duration,
        easing: impl Fn(EasingTime) -> EasingStep + Send + 'static,
        sampler: impl Fn(&Transition<T>, EasingStep) -> T + Send + 'static,
    ) -> AnimationHandle
    where
        T: Transitionable,
    {
        self.ease_impl(self.get(), new_value.into(), duration, easing, 0.fct(), sampler)
    }

    /// Oscillate between the current value and `new_value` with an easing transition and a custom value sampler.
    ///
    /// The `duration` defines the easing duration between the two values.
    ///
    /// Note that you can use [`sequence`] to create more complex looping animations.
    ///
    /// [`sequence`]: Self::sequence
    pub fn ease_oci_with(
        &self,
        new_value: impl Into<T>,
        duration: Duration,
        easing: impl Fn(EasingTime) -> EasingStep + Send + 'static,
        sampler: impl Fn(&Transition<T>, EasingStep) -> T + Send + 'static,
    ) -> AnimationHandle
    where
        T: Transitionable,
    {
        self.ease_oci_impl(self.get(), new_value.into(), duration, easing, 0.fct(), sampler)
    }

    /// Schedule a keyframed transition animation for the variable, starting from the first key.
    ///
    /// The variable will be set to the first keyframe, then animated across all other keys.
    ///
    /// See [`animate`] for details about animations.
    ///
    /// [`animate`]: Self::animate
    pub fn set_ease_keyed(
        &self,
        keys: Vec<(Factor, T)>,
        duration: Duration,
        easing: impl Fn(EasingTime) -> EasingStep + Send + 'static,
    ) -> AnimationHandle
    where
        T: Transitionable,
    {
        self.set_ease_keyed_with(keys, duration, easing, TransitionKeyed::sample)
    }

    /// Schedule a keyframed transition animation for the variable, starting from the first key, using a custom value sampler.
    ///
    /// The variable will be set to the first keyframe, then animated across all other keys.
    ///
    /// See [`animate`] for details about animations.
    ///
    /// [`animate`]: Self::animate
    pub fn set_ease_keyed_with(
        &self,
        keys: Vec<(Factor, T)>,
        duration: Duration,
        easing: impl Fn(EasingTime) -> EasingStep + Send + 'static,
        sampler: impl Fn(&TransitionKeyed<T>, EasingStep) -> T + Send + 'static,
    ) -> AnimationHandle
    where
        T: Transitionable,
    {
        if let Some(transition) = TransitionKeyed::new(keys) {
            self.ease_keyed_impl(transition, duration, easing, 999.fct(), sampler)
        } else {
            AnimationHandle::dummy()
        }
    }

    /// Schedule a keyframed transition animation for the variable, starting from the current value.
    ///
    /// The variable will be set to the first keyframe, then animated across all other keys.
    ///
    /// See [`animate`] for details about animations.
    ///
    /// [`animate`]: Self::animate
    pub fn ease_keyed(
        &self,
        keys: Vec<(Factor, T)>,
        duration: Duration,
        easing: impl Fn(EasingTime) -> EasingStep + Send + 'static,
    ) -> AnimationHandle
    where
        T: Transitionable,
    {
        self.ease_keyed_with(keys, duration, easing, TransitionKeyed::sample)
    }

    /// Schedule a keyframed transition animation for the variable, starting from the current value, using a custom value sampler.
    ///
    /// The variable will be set to the first keyframe, then animated across all other keys.
    ///
    /// See [`animate`] for details about animations.
    ///
    /// [`animate`]: Self::animate
    pub fn ease_keyed_with(
        &self,
        mut keys: Vec<(Factor, T)>,
        duration: Duration,
        easing: impl Fn(EasingTime) -> EasingStep + Send + 'static,
        sampler: impl Fn(&TransitionKeyed<T>, EasingStep) -> T + Send + 'static,
    ) -> AnimationHandle
    where
        T: Transitionable,
    {
        keys.insert(0, (0.fct(), self.get()));

        let transition = TransitionKeyed::new(keys).unwrap();
        self.ease_keyed_impl(transition, duration, easing, 0.fct(), sampler)
    }

    /// Set the variable to `new_value` after a `delay`.
    ///
    /// The variable [`is_animating`] until the delay elapses and the value is set.
    ///
    /// See [`animate`] for details about animations.
    ///
    /// [`is_animating`]: AnyVar::is_animating
    /// [`animate`]: Self::animate
    pub fn step(&self, new_value: impl Into<T>, delay: Duration) -> AnimationHandle {
        self.step_impl(new_value.into(), delay)
    }

    /// Oscillate between the current value and `new_value`, every time the `delay` elapses the variable is set to the next value.
    pub fn step_oci(&self, new_value: impl Into<T>, delay: Duration) -> AnimationHandle {
        self.step_oci_impl([self.get(), new_value.into()], delay, false)
    }

    /// Oscillate between `from` and `to`, the variable is set to `from` to start and every time the `delay` elapses
    /// the variable is set to the next value.
    pub fn set_step_oci(&self, from: impl Into<T>, to: impl Into<T>, delay: Duration) -> AnimationHandle {
        self.step_oci_impl([from.into(), to.into()], delay, true)
    }

    /// Set the variable to a sequence of values as a time `duration` elapses.
    ///
    /// An animation curve is used to find the first factor in `steps` above or at the curve line at the current time,
    /// the variable is set to this step value, continuing animating across the next steps until the last or the animation end.
    /// The variable [`is_animating`] from the start, even if no step applies and stays *animating* until the last *step* applies
    /// or the duration is reached.
    ///
    /// # Examples
    ///
    /// Creates a variable that outputs text every 5% of a 5 seconds animation, advanced linearly.
    ///
    /// ```
    /// # use zng_var::{*, animation::easing};
    /// # use zng_txt::*;
    /// # use zng_unit::*;
    /// # fn demo(text_var: Var<Txt>) {
    /// let steps = (0..=100).step_by(5).map(|i| (i.pct().fct(), formatx!("{i}%"))).collect();
    /// # let _ =
    /// text_var.steps(steps, 5.secs(), easing::linear)
    /// # ;}
    /// ```
    ///
    /// The variable is set to `"0%"`, after 5% of the `duration` elapses it is set to `"5%"` and so on
    /// until the value is set to `"100%` at the end of the animation.
    ///
    /// Returns an [`AnimationHandle`]. See [`Var::animate`] for details about animations.
    ///
    /// [`is_animating`]: AnyVar::is_animating
    /// [`AnimationHandle`]: crate::animation::AnimationHandle
    pub fn steps(
        &self,
        steps: Vec<(Factor, T)>,
        duration: Duration,
        easing: impl Fn(EasingTime) -> EasingStep + Send + 'static,
    ) -> AnimationHandle {
        let mut prev_step = 999.fct();
        self.animate(move |a, vm| {
            let step = easing(a.elapsed_stop(duration));
            if step != prev_step {
                prev_step = step;
                if let Some(val) = steps.iter().find(|(f, _)| *f >= step).map(|(_, step)| step.clone()) {
                    vm.set(val);
                }
            }
        })
    }

    /// Starts an easing animation that *chases* a target value that can be changed using the [`ChaseAnimation<T>`] handle.
    pub fn chase(
        &self,
        first_target: impl Into<T>,
        duration: Duration,
        easing: impl Fn(EasingTime) -> EasingStep + Send + 'static,
    ) -> ChaseAnimation<T>
    where
        T: Transitionable,
    {
        self.chase_impl(first_target.into(), duration, easing)
    }
    fn chase_impl(
        &self,
        first_target: T,
        duration: Duration,
        easing: impl Fn(EasingTime) -> EasingStep + Send + 'static,
    ) -> ChaseAnimation<T>
    where
        T: Transitionable,
    {
        ChaseAnimation {
            handle: self.ease(first_target.clone(), duration, easing),
            target: first_target,
            var: self.current_context(),
        }
    }

    /// Start a [`chase`] animation without a first target.
    ///
    /// Use [`ChaseAnimation<T>::set`] to set the first chase target.
    ///
    /// [`chase`]: Self::chase
    pub fn chase_begin(&self) -> ChaseAnimation<T>
    where
        T: Transitionable,
    {
        ChaseAnimation {
            handle: AnimationHandle::dummy(),
            target: self.get(),
            var: self.current_context(),
        }
    }

    /// Create a vars that [`ease`] to each new value of `self`.
    ///
    /// Note that the mapping var can be [contextualized], see [`map`] for more details.
    ///
    /// If `self` can change the output variable will keep it alive.
    ///
    /// [contextualized]: crate::contextual_var
    /// [`ease`]: Var::ease
    /// [`map`]: Var::map
    pub fn easing(&self, duration: Duration, easing: impl Fn(EasingTime) -> EasingStep + Send + Sync + 'static) -> Var<T>
    where
        T: Transitionable,
    {
        self.easing_with(duration, easing, Transition::sample)
    }

    /// Create a vars that [`ease_with`] to each new value of `self`.
    ///
    /// Note that the mapping var can be contextualized, see [`map`] for more details.
    /// If `self` is shared the output variable will hold a strong reference to it.
    ///
    /// [`ease_with`]: Var::ease_with
    /// [`map`]: Var::map
    pub fn easing_with(
        &self,
        duration: Duration,
        easing: impl Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
        sampler: impl Fn(&Transition<T>, EasingStep) -> T + Send + Sync + 'static,
    ) -> Var<T>
    where
        T: Transitionable,
    {
        let caps = self.capabilities();
        if caps.is_const() {
            return self.clone();
        }

        let fns = Arc::new((easing, sampler));

        if caps.is_contextual() {
            let me = self.clone();
            return contextual_var(move || me.clone().easing_with_tail(duration, fns.clone()));
        }

        self.easing_with_tail(duration, fns)
    }
    // to avoid infinite closure type (contextual case)
    fn easing_with_tail(
        &self,
        duration: Duration,
        fns: Arc<(
            impl Fn(EasingTime) -> Factor + Send + Sync + 'static,
            impl Fn(&Transition<T>, Factor) -> T + Send + Sync + 'static,
        )>,
    ) -> Var<T>
    where
        T: Transitionable,
    {
        let me = self.current_context();

        let output = crate::var(me.get());

        let weak_output = output.downgrade();
        let mut _ease_handle = AnimationHandle::dummy();
        me.hook(move |args| {
            if let Some(output) = weak_output.upgrade() {
                _ease_handle = output.ease_with(
                    args.value().clone(),
                    duration,
                    clmv!(fns, |t| fns.0(t)),
                    clmv!(fns, |t, s| fns.1(t, s)),
                );
                true
            } else {
                false
            }
        })
        .perm();
        output.hold(me).perm();

        output.read_only()
    }

    fn ease_impl(
        &self,
        start_value: T,
        end_value: T,
        duration: Duration,
        easing: impl Fn(EasingTime) -> EasingStep + Send + 'static,
        init_step: Factor, // set to 0 skips first frame, set to 999 includes first frame.
        sampler: impl Fn(&Transition<T>, EasingStep) -> T + Send + 'static,
    ) -> AnimationHandle
    where
        T: Transitionable,
    {
        let transition = Transition::new(start_value, end_value);
        let mut prev_step = init_step;
        self.animate(move |a, vm| {
            let step = easing(a.elapsed_stop(duration));

            if prev_step != step {
                vm.set(sampler(&transition, step));
                prev_step = step;
            }
        })
    }

    fn ease_oci_impl(
        &self,
        start_value: T,
        end_value: T,
        duration: Duration,
        easing: impl Fn(EasingTime) -> EasingStep + Send + 'static,
        init_step: EasingStep, // set to 0 skips first frame, set to 999 includes first frame.
        sampler: impl Fn(&Transition<T>, EasingStep) -> T + Send + 'static,
    ) -> AnimationHandle
    where
        T: Transitionable,
    {
        let transition = Transition::new(start_value, end_value);
        let mut prev_step = init_step;
        self.animate(move |a, vm| {
            let t = a.elapsed(duration);
            let mut step = easing(t);
            if a.count() % 2 != 0 {
                step = step.flip()
            }
            if t.is_end() {
                a.restart();
            }

            if prev_step != step {
                vm.set(sampler(&transition, step));
                prev_step = step;
            }
        })
    }

    fn ease_keyed_impl(
        &self,
        transition: TransitionKeyed<T>,
        duration: Duration,
        easing: impl Fn(EasingTime) -> EasingStep + Send + 'static,
        init_step: EasingStep,
        sampler: impl Fn(&TransitionKeyed<T>, EasingStep) -> T + Send + 'static,
    ) -> AnimationHandle
    where
        T: Transitionable,
    {
        let mut prev_step = init_step;
        self.animate(move |a, value| {
            let step = easing(a.elapsed_stop(duration));

            if prev_step != step {
                value.set(sampler(&transition, step));
                prev_step = step;
            }
        })
    }

    fn step_impl(&self, new_value: T, delay: Duration) -> AnimationHandle {
        let mut new_value = Some(new_value);
        self.animate(move |a, vm| {
            if !a.animations_enabled() || a.elapsed_dur() >= delay {
                a.stop();
                if let Some(nv) = new_value.take() {
                    vm.set(nv);
                }
            } else {
                a.sleep(delay, false);
            }
        })
    }

    fn step_oci_impl(&self, values: [T; 2], delay: Duration, mut set: bool) -> AnimationHandle {
        let mut first = false;
        self.animate(move |a, vm| {
            if !a.animations_enabled() || std::mem::take(&mut set) {
                vm.set(values[0].clone());
            } else if a.elapsed_dur() >= delay {
                if first {
                    vm.set(values[0].clone());
                } else {
                    vm.set(values[1].clone());
                }
                first = !first;
            }
            a.sleep(delay, false);
        })
    }
}
/// Transition animations
impl<T: VarValue + Transitionable> Var<T> {}
/// Value type.
impl<T: VarValue> Var<T> {
    /// Reference the variable without the strong value type.
    pub fn as_any(&self) -> &AnyVar {
        &self.any
    }
}
/// Variable type.
impl<T: VarValue> Var<T> {
    /// Create a weak reference to this variable.
    pub fn downgrade(&self) -> WeakVar<T> {
        WeakVar {
            any: self.any.downgrade(),
            _t: PhantomData,
        }
    }

    /// Gets a clone of the var that is always read-only.
    ///
    /// The returned variable can still update if `self` is modified, but it does not have the `MODIFY` capability.
    pub fn read_only(&self) -> Var<T> {
        Var::new_any(self.any.read_only())
    }

    /// Create a var that redirects to this variable until the first value update, then it disconnects as a separate variable.
    ///
    /// The return variable is *clone-on-write* and has the `MODIFY` capability independent of the source capabilities, when
    /// a modify request is made the source value is cloned and offered for modification, if modified the source variable is dropped,
    /// if the modify closure does not update the source variable is retained.
    pub fn cow(&self) -> Var<T> {
        Var::new_any(self.any.cow())
    }

    /// Gets the underlying var in the current calling context.
    ///
    /// If this variable is [`CONTEXT`] returns a clone of the inner variable,
    /// otherwise returns a clone of this variable.
    ///
    /// [`CONTEXT`]: crate::VarCapability::CONTEXT
    pub fn current_context(&self) -> Var<T> {
        Var::new_any(self.any.current_context())
    }

    /// Gets if this variable is the same as `other`.
    ///
    /// If this variable is [`SHARE`] compares the *pointer*. If this variable is local this is always `false`.
    ///
    /// [`SHARE`]: crate::VarCapability::SHARE
    pub fn var_eq(&self, other: &Self) -> bool {
        self.any.var_eq(&other.any)
    }
}

/// Weak reference to a [`Var<T>`].
pub struct WeakVar<T: VarValue> {
    any: WeakAnyVar,
    _t: PhantomData<T>,
}
impl<T: VarValue> fmt::Debug for WeakVar<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("WeakVar").field(&self.any.0).finish()
    }
}
impl<T: VarValue> Clone for WeakVar<T> {
    fn clone(&self) -> Self {
        Self {
            any: self.any.clone(),
            _t: PhantomData,
        }
    }
}
impl<T: VarValue> From<WeakVar<T>> for WeakAnyVar {
    fn from(var: WeakVar<T>) -> Self {
        var.any
    }
}
impl<T: VarValue> ops::Deref for WeakVar<T> {
    type Target = WeakAnyVar;

    fn deref(&self) -> &Self::Target {
        self.as_any()
    }
}
impl<T: VarValue> WeakVar<T> {
    /// Reference the weak variable without the strong value type.
    pub fn as_any(&self) -> &WeakAnyVar {
        &self.any
    }

    /// Attempt to create a strong reference to the variable.
    pub fn upgrade(&self) -> Option<Var<T>> {
        self.any.upgrade().map(Var::new_any)
    }
}

/// New read/write shared reference variable from any type that can convert into it.
pub fn var_from<T: VarValue>(initial_value: impl Into<T>) -> Var<T> {
    crate::var(initial_value.into())
}

/// New read/write shared reference variable with default initial value.
pub fn var_default<T: VarValue + Default>() -> Var<T> {
    crate::var(T::default())
}

/// New immutable variable that stores the `value` directly.
///
/// Cloning this variable clones the value.
pub fn const_var<T: VarValue>(value: T) -> Var<T> {
    Var::new_any(any_const_var(BoxAnyVarValue::new(value)))
}

/// Type erased [`const_var`].
pub fn any_const_var(value: BoxAnyVarValue) -> AnyVar {
    AnyVar(crate::DynAnyVar::Const(crate::var_impl::const_var::ConstVar::new(value)))
}

/// Weak variable that never upgrades.
pub fn weak_var<T: VarValue>() -> WeakVar<T> {
    WeakVar {
        any: weak_var_any(),
        _t: PhantomData,
    }
}

/// Weak variable that never upgrades.
pub fn weak_var_any() -> WeakAnyVar {
    WeakAnyVar(crate::DynWeakAnyVar::Const(crate::var_impl::const_var::WeakConstVar))
}

/// Arguments for [`Var::hook`].
pub struct VarHookArgs<'a, T: VarValue> {
    pub(super) any: &'a AnyVarHookArgs<'a>,
    pub(super) _t: PhantomData<&'a T>,
}
impl<'a, T: VarValue> VarHookArgs<'a, T> {
    /// Reference the updated value.
    pub fn value(&self) -> &'a T {
        self.any.value.downcast_ref::<T>().unwrap()
    }
}
impl<'a, T: VarValue> ops::Deref for VarHookArgs<'a, T> {
    type Target = AnyVarHookArgs<'a>;

    fn deref(&self) -> &Self::Target {
        self.any
    }
}
