use core::fmt;
use std::{
    any::{Any, TypeId},
    marker::PhantomData,
    mem,
    sync::{Arc, atomic::AtomicBool},
};

use parking_lot::Mutex;
use smallbox::{SmallBox, smallbox};
use zng_clone_move::clmv;
use zng_txt::{Txt, formatx};

use crate::{
    BoxedVarValueAny, VARS, Var, VarCapability, VarHandle, VarHandles, VarImpl, VarIsReadOnlyError, VarModifyAny, VarUpdateId, VarValue,
    VarValueAny, WeakVarImpl,
    animation::{Animation, AnimationController, AnimationHandle, AnimationStopFn},
    any_contextual_var,
};

/// Variable of any type.
pub struct VarAny(pub(crate) SmallBox<dyn VarImpl, smallbox::space::S2>);
impl Clone for VarAny {
    fn clone(&self) -> Self {
        Self(self.0.clone_boxed())
    }
}
/// Value.
impl VarAny {
    /// Visit a reference to the current value.
    pub fn with<O>(&self, visitor: impl FnOnce(&dyn VarValueAny) -> O) -> O { // TODO try a ArcWap based read
        let mut once = Some(visitor);
        let mut output = None;
        self.0.with(&mut |v| {
            output = Some(once.take().unwrap()(v));
        });
        output.unwrap()
    }

    /// Get a clone of the current value.
    pub fn get(&self) -> BoxedVarValueAny {
        self.0.get()
    }

    /// Debug format the current value.
    pub fn get_debug(&self, alternate: bool) -> Txt {
        let mut r = Txt::default();
        self.0.with(&mut |v| {
            r = if alternate { formatx!("{v:#?}") } else { formatx!("{v:?}") };
        });
        r
    }

    /// Gets if the value updated.
    ///
    /// Returns `true` if the [`last_update`] is the current one. Note that this will only work reliably in
    /// UI code that is synchronized with app updates, prefer [`wait_update`] in async code.
    ///
    /// [`last_update`]: Self::last_update
    pub fn is_new(&self) -> bool {
        self.last_update() == VARS.update_id()
    }

    /// Gets a clone of the current value if it [`is_new`].
    ///
    /// [`is_new`]: Self::is_new
    pub fn get_new(&self) -> Option<BoxedVarValueAny> {
        if self.is_new() { Some(self.get()) } else { None }
    }

    /// Visit a reference to the current value if it [`is_new`].
    ///
    /// [`is_new`]: Self::is_new
    pub fn with_new<O>(&self, visitor: impl FnOnce(&dyn VarValueAny) -> O) -> Option<O> {
        if self.is_new() { Some(self.with(visitor)) } else { None }
    }

    /// Schedule `new_value` to be assigned next update, if the variable is not read-only.
    ///
    /// Panics if the value type does not match.
    pub fn try_set(&self, new_value: BoxedVarValueAny) -> Result<(), VarIsReadOnlyError> {
        assert_eq!(new_value.type_id(), self.value_type());
        self.handle_modify(self.0.set(new_value))
    }

    /// Schedule `new_value` to be assigned next update.
    ///
    /// If the variable is read-only this is ignored and a DEBUG level log is recorded.
    /// Use [`try_set`] to get an error for read-only vars.
    ///
    /// [`try_set`]: Self::try_set
    pub fn set(&self, new_value: BoxedVarValueAny) {
        trace_debug_error!(self.try_set(new_value))
    }

    /// Schedule an update notification, without actually changing the value, if the variable is not read-only.
    pub fn try_update(&self) -> Result<(), VarIsReadOnlyError> {
        self.handle_modify(self.0.update())
    }

    /// Show variable value as new next update, without actually changing the value.
    ///
    /// If the variable is read-only this is ignored and a DEBUG level log is recorded.
    /// Use [`try_update`] to get an error for read-only vars.
    ///
    /// [`try_update`]: Self::try_set
    pub fn update(&self) {
        trace_debug_error!(self.try_update())
    }

    /// Schedule `modify` to be called on the value for the next update, if the variable is not read-only.
    ///
    /// If the [`VarModifyAny`] closure input is deref_mut the variable will notify an update.
    pub fn try_modify(&self, modify: impl FnOnce(&mut VarModifyAny) + Send + 'static) -> Result<(), VarIsReadOnlyError> {
        // can't have a SmallBox<dyn FnOnce> because Rust has special compiler magic for Box<dyn FnOnce>,
        // so we wrap in an Option and FnMut that is only called once.
        let mut modify = Some(modify);
        let modify = move |value: &mut VarModifyAny| {
            #[cfg(debug_assertions)]
            let type_id = (&*value.value as &dyn Any).type_id();

            modify.take().unwrap()(value);

            #[cfg(debug_assertions)]
            if !value.update.is_empty() {
                assert_eq!((&*value.value as &dyn Any).type_id(), type_id, "VarAny::modify changed value type");
            }
        };

        self.handle_modify(self.0.modify(smallbox!(modify)))
    }

    /// Schedule `modify` to be called on the value for the next update, if the variable is not read-only.
    ///
    /// If the [`VarModifyAny`] closure input is deref_mut the variable will notify an update.
    ///
    /// If the variable is read-only this is ignored and a DEBUG level log is recorded.
    /// Use [`try_modify`] to get an error for read-only vars.
    ///
    /// [`try_modify`]: Self::try_modify
    pub fn modify(&self, modify: impl FnOnce(&mut VarModifyAny) + Send + 'static) {
        trace_debug_error!(self.try_modify(modify))
    }

    /// Schedule a new `value` for the variable, it will be set in the end of the current app update to the updated
    /// value of `other`, so if the other var has already scheduled an update, the updated value will be used.
    ///  
    /// This can be used just before creating a binding to start with synchronized values.
    pub fn try_set_from(&self, other: &VarAny) -> Result<(), VarIsReadOnlyError> {
        if other.capabilities().is_const() {
            self.try_set(other.get())
        } else if self.capabilities().is_read_only() {
            Err(VarIsReadOnlyError {})
        } else {
            let weak_other = other.downgrade();
            self.try_modify(move |v| {
                if let Some(other) = weak_other.upgrade() {
                    other.with(|ov| {
                        if *ov != **v {
                            // only clone if really changed
                            let mut new_value = ov.clone_boxed();
                            assert!(v.try_swap(&mut *new_value), "set_from other var not of the same type");

                            // tag for bidi bindings
                            v.push_tag(other.var_instance_tag());
                        }
                        // don't break animation of this if other just started animating after the `set_from` request was scheduled
                        v.set_modify_importance(other.modify_importance());
                    });
                }
            })
        }
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
    pub fn set_from(&self, other: &VarAny) {
        trace_debug_error!(self.try_set_from(other))
    }

    /// Like [`try_set_from`], but uses `map` to produce the new value from the updated value of `other`.
    ///
    /// [`try_set_from`]: Self::try_set_from
    pub fn try_set_from_map(
        &self,
        other: &VarAny,
        map: impl FnOnce(&dyn VarValueAny) -> BoxedVarValueAny + Send + 'static,
    ) -> Result<(), VarIsReadOnlyError> {
        if other.capabilities().is_const() {
            self.try_set(other.get())
        } else if self.capabilities().is_read_only() {
            Err(VarIsReadOnlyError {})
        } else {
            let weak_other = other.downgrade();
            self.try_modify(move |v| {
                if let Some(other) = weak_other.upgrade() {
                    other.with(|ov| {
                        let new_value = map(ov);
                        if v.set(new_value) {
                            // tag for bidi bindings
                            v.push_tag(other.var_instance_tag());
                        }
                        // don't break animation of this if other just started animating after the `set_from` request was scheduled
                        v.set_modify_importance(other.modify_importance());
                    });
                }
            })
        }
    }

    /// Like [`set_from`], but uses `map` to produce the new value from the updated value of `other`.
    ///
    /// If the variable is read-only this is ignored and a DEBUG level log is recorded.
    /// Use [`try_set_from_map`] to get an error for read-only vars.
    ///
    /// [`try_set_from_map`]: Self::try_set_from_map
    /// [`set_from`]: Self::set_from
    pub fn set_from_map(&self, other: &VarAny, map: impl FnOnce(&dyn VarValueAny) -> BoxedVarValueAny + Send + 'static) {
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
    pub fn hook(&self, on_update: impl FnMut(&VarAnyHookArgs) -> bool + Send + 'static) -> VarHandle {
        self.0.hook(smallbox!(on_update))
    }

    ///Awaits for a value that passes the `predicate`, including the current value.
    #[allow(clippy::manual_async_fn)] // false positive, async fn futures are not Send + Sync
    pub fn wait_match(&self, predicate: impl Fn(&dyn VarValueAny) -> bool + Send + Sync) -> impl Future<Output = ()> + Send + Sync {
        async move {
            while !self.with(&predicate) {
                let future = self.wait_update();
                if self.with(&predicate) {
                    break;
                }
                future.await;
            }
        }
    }

    /// Awaits for an update them [`get`] the value.
    ///
    /// [`get`]: Self::get
    #[allow(clippy::manual_async_fn)] // false positive, async fn futures are not Send + Sync
    pub fn wait_next(&self) -> impl Future<Output = BoxedVarValueAny> + Send + Sync {
        async {
            self.wait_update().await;
            self.get()
        }
    }

    /// Last update ID a variable was modified.
    ///
    /// If the ID equals [`VARS.update_id`] the variable [`is_new`].
    ///
    /// [`is_new`]: Self::is_new
    /// [`VARS.update_id`]: VARS::update_id
    pub fn last_update(&self) -> VarUpdateId {
        self.0.last_update()
    }

    /// Awaits for the [`last_update`] to change.
    ///
    /// Note that [`is_new`] will be `true` when the future elapses only when polled
    /// in sync with the UI, but it will elapse in any thread when the variable updates after the future is instantiated.
    ///
    /// Note that outside of the UI tree there is no variable synchronization across multiple var method calls, so
    /// a sequence of `get(); wait_update().await; get();` can miss a value between `get` and `wait_update`. The returned
    /// future captures the [`last_update`] at the moment this method is called, this can be leveraged by double-checking to
    /// avoid race conditions, see the [`wait_match`] default implementation for more details.
    ///
    /// [`wait_match`]: Self::wait_match
    /// [`last_update`]: Self::last_update
    /// [`is_new`]: Self::is_new
    pub fn wait_update(&self) -> impl Future<Output = VarUpdateId> + Send + Sync {
        crate::future::WaitUpdateFut::new(self)
    }

    /// Debug helper for tracing the lifetime of a value in this variable.
    ///
    /// See [`trace_value`] for more details.
    ///
    /// [`trace_value`]: Var::trace_value
    pub fn trace_value<S: Send + 'static>(&self, mut enter_value: impl FnMut(&VarAnyHookArgs) -> S + Send + 'static) -> VarHandle {
        let span = self.with(|v| {
            enter_value(&VarAnyHookArgs {
                value: v,
                update: false,
                tags: &[],
            })
        });
        let mut span = Some(span);
        self.hook(move |v| {
            let _ = span.take();
            span = Some(enter_value(v));
            true
        })
    }

    fn handle_modify(&self, scheduled: bool) -> Result<(), VarIsReadOnlyError> {
        match scheduled {
            true => Ok(()),
            false => Err(VarIsReadOnlyError {}),
        }
    }
}
/// Value mapping.
impl VarAny {
    /// Create a mapping variable from any to any.
    ///
    /// The mapping variable type is defined by the first call to `map`,
    /// after that it will panic if the mapped value changes type.
    ///
    /// See [`map`] for more details about mapping variables.
    ///
    /// [`map`]: Var::map
    pub fn map_any(&self, map: impl FnMut(&dyn VarValueAny) -> BoxedVarValueAny + Send + 'static) -> VarAny {
        let caps = self.capabilities();

        if caps.is_contextual() {
            let me = self.clone();
            let map = Arc::new(Mutex::new(map));
            return any_contextual_var(move || me.map_any_tail(clmv!(map, |v| map.lock()(v)), me.capabilities()));
        }
        self.map_any_tail(map, caps)
    }
    // to avoid infinite closure type (contextual case)
    fn map_any_tail(&self, mut map: impl FnMut(&dyn VarValueAny) -> BoxedVarValueAny + Send + 'static, caps: VarCapability) -> VarAny {
        let mut init_value = None;
        self.with(&mut |v: &dyn VarValueAny| init_value = Some(map(v)));
        let init_value = init_value.unwrap();

        if caps.is_const() {
            return crate::any_const_var(init_value);
        }

        let output = crate::var_any(init_value);
        self.bind_impl(&output, map).perm();
        output.hold(self.clone()).perm();

        output.read_only()
    }

    /// Create a deref variable from any to any.
    ///
    /// See [`map_ref`] for more details about mapping variables.
    ///
    /// [`map_ref`]: Var::map_ref
    pub fn map_ref_any(&self, deref: impl Fn(&dyn VarValueAny) -> &(dyn VarValueAny) + Send + Sync + 'static) -> VarAny {
        let mapping = crate::var_impl::map_ref::MapRefVar::new(self.clone(), smallbox!(deref));
        VarAny(smallbox!(mapping))
    }

    /// Create a strongly typed mapping variable.
    ///
    /// The `map` closure must produce a strongly typed value for every update of this variable.
    ///
    /// See [`map`] for more details about mapping variables.
    ///
    /// [`map`]: Var::map
    pub fn map<O: VarValue>(&self, mut map: impl FnMut(&dyn VarValueAny) -> O + Send + 'static) -> Var<O> {
        let mapping = self.map_any(move |v| BoxedVarValueAny::new(map(v)));
        Var::new_any(mapping)
    }

    /// Create a strongly typed deref variable.
    ///
    /// The `deref` closure must produce a strongly typed value reference for every read this variable.
    ///
    /// See [`map_ref`] for more details about mapping variables.
    ///
    /// [`map_ref`]: Var::map_ref
    pub fn map_ref<O: VarValue>(&self, deref: impl Fn(&dyn VarValueAny) -> &O + Send + Sync + 'static) -> Var<O> {
        let mapping = self.map_ref_any(move |v| deref(v));
        Var::new_any(mapping)
    }

    /// Create a mapping variable that contains the debug formatted value from this variable.
    ///
    /// See [`map`] for more details about mapping variables.
    ///
    /// [`map`]: Var::map
    pub fn map_debug(&self, alternate: bool) -> Var<Txt> {
        if alternate {
            self.map(|v| formatx!("{v:#?}"))
        } else {
            self.map(|v| formatx!("{v:?}"))
        }
    }

    /// Create a mapping variable that can skip updates.
    ///
    /// The `map` closure is called for every update this variable and if it returns a new value the mapping variable updates.
    ///
    /// If the `map` closure does not produce a value on init the `fallback_init` closure is called.
    ///
    /// See [`filter_map`] for more details about mapping variables.
    ///
    /// [`filter_map`]: Var::filter_map
    pub fn filter_map_any(
        &self,
        map: impl FnMut(&dyn VarValueAny) -> Option<BoxedVarValueAny> + Send + 'static,
        fallback_init: impl Fn() -> BoxedVarValueAny + Send + 'static,
    ) -> VarAny {
        let caps = self.capabilities();

        if caps.is_contextual() {
            let me = self.clone();
            let fns = Arc::new(Mutex::new((map, fallback_init)));
            return any_contextual_var(move || {
                me.filter_map_any_tail(clmv!(fns, |v| fns.lock().0(v)), clmv!(fns, || fns.lock().1()), me.capabilities())
            });
        }

        self.filter_map_any_tail(map, fallback_init, caps)
    }
    // to avoid infinite closure type (contextual case)
    fn filter_map_any_tail(
        &self,
        mut map: impl FnMut(&dyn VarValueAny) -> Option<BoxedVarValueAny> + Send + 'static,
        fallback_init: impl Fn() -> BoxedVarValueAny + Send + 'static,
        caps: VarCapability,
    ) -> VarAny {
        let mut init_value = None;
        self.with(&mut |v: &dyn VarValueAny| init_value = map(v));
        let init_value = match init_value {
            Some(v) => v,
            None => fallback_init(),
        };

        if caps.is_const() {
            return crate::any_const_var(init_value);
        }

        let output = crate::var_any(init_value);
        let weak_output = output.downgrade();

        self.hook(move |args| {
            match weak_output.upgrade() {
                Some(o) => {
                    if let Some(new_value) = map(args.value) {
                        o.set(new_value);
                    }
                    true
                }
                None => {
                    // don't retain, output var dropped
                    false
                }
            }
        })
        .perm();
        output.hold(self.clone()).perm();

        output.read_only()
    }

    /// Create a deref variable that can reset for some updates.
    ///
    /// The `deref` closure is called read of this variable, if it does not return a reference the `fallback_ref` is returned instead.
    ///
    /// See [`filter_ref`] for more details about deref variables.
    ///
    /// [`filter_ref`]: Var::filter_ref
    pub fn filter_ref_any(
        &self,
        deref: impl Fn(&dyn VarValueAny) -> Option<&dyn VarValueAny> + Send + Sync + 'static,
        fallback_ref: &'static dyn VarValueAny,
    ) -> VarAny {
        self.map_ref_any(move |v| deref(v).unwrap_or(fallback_ref))
    }

    /// Create a strongly typed mapping variable that can skip updates.
    ///
    /// The `map` closure is called for every update this variable and if it returns a new value the mapping variable updates.
    ///
    /// If the `map` closure does not produce a value on init the `fallback_init` closure is called.
    ///
    /// See [`filter_map`] for more details about mapping variables.
    ///
    /// [`filter_map`]: Var::filter_map
    pub fn filter_map<O: VarValue>(
        &self,
        mut map: impl FnMut(&dyn VarValueAny) -> Option<O> + Send + 'static,
        fallback_init: impl Fn() -> O + Send + 'static,
    ) -> Var<O> {
        let mapping = self.filter_map_any(
            move |v| map(v).map(BoxedVarValueAny::new),
            move || BoxedVarValueAny::new(fallback_init()),
        );
        Var::new_any(mapping)
    }

    /// Create a strongly typed deref variable that can reset for some updates.
    ///
    /// The `deref` closure is called read of this variable, if it does not return a reference the `fallback_ref` is returned instead.
    ///
    /// See [`filter_ref`] for more details about deref variables.
    ///
    /// [`filter_ref`]: Var::filter_ref
    pub fn filter_ref<O: VarValue>(
        &self,
        deref: impl Fn(&dyn VarValueAny) -> Option<&O> + Send + Sync + 'static,
        fallback_ref: &'static O,
    ) -> Var<O> {
        self.map_ref(move |v| deref(v).unwrap_or(fallback_ref))
    }

    /// Create a bidirectional mapping variable.
    ///
    /// The mapping variable type is defined by the first call to `map`,
    /// after that it will panic if the mapped value changes type.
    ///
    /// The `map_back` closure must produce values of the same type as this variable, this variable will panic
    /// if map back value is not the same.
    ///
    /// See [`map_bidi`] for more details about bidirectional mapping variables.
    ///
    /// [`map_bidi`]: Var::map_bidi
    pub fn map_bidi_any(
        &self,
        mut map: impl FnMut(&dyn VarValueAny) -> BoxedVarValueAny + Send + 'static,
        map_back: impl FnMut(&dyn VarValueAny) -> BoxedVarValueAny + Send + 'static,
    ) -> VarAny {
        let caps = self.capabilities();

        if caps.is_contextual() {
            let me = self.clone();
            let fns = Arc::new(Mutex::new((map, map_back)));
            return any_contextual_var(move || me.map_bidi_any(clmv!(fns, |v| fns.lock().0(v)), clmv!(fns, |v| fns.lock().1(v))));
        }

        let mut init_value = None;
        self.with(&mut |v: &dyn VarValueAny| init_value = Some(map(v)));
        let init_value = init_value.unwrap();

        if caps.is_const() {
            return crate::any_const_var(init_value);
        }

        let output = crate::var_any(init_value);

        self.bind_map_bidi_any(&output, map, map_back).perm();
        output.hold(self.clone()).perm();

        output
    }

    /// Create a bidirectional referencing variable.
    ///
    /// See [`map_ref_bidi`] for more details about mapping variables.
    ///
    /// [`map_ref_bidi`]: Var::map_ref_bidi
    pub fn map_ref_bidi_any(
        &self,
        deref: impl Fn(&dyn VarValueAny) -> &(dyn VarValueAny) + Send + Sync + 'static,
        deref_mut: impl Fn(&mut dyn VarValueAny) -> &mut (dyn VarValueAny) + Send + Sync + 'static,
    ) -> VarAny {
        let mapping = crate::var_impl::map_ref_bidi::MapBidiRefVar::new(self.clone(), smallbox!(deref), smallbox!(deref_mut));
        VarAny(smallbox!(mapping))
    }

    /// Create a bidirectional mapping variable that can skip updates.
    ///
    /// The mapping variable type is defined by the first call to `map`,
    /// after that it will panic if the mapped value changes type.
    ///
    /// The `map_back` closure must produce values of the same type as this variable, this variable will panic
    /// if map back value is not the same.
    ///
    /// See [`filter_map_bidi`] for more details about bidirectional mapping variables.
    ///
    /// [`filter_map_bidi`]: Var::filter_map_bidi
    pub fn filter_map_bidi_any(
        &self,
        mut map: impl FnMut(&dyn VarValueAny) -> Option<BoxedVarValueAny> + Send + 'static,
        map_back: impl FnMut(&dyn VarValueAny) -> Option<BoxedVarValueAny> + Send + 'static,
        fallback_init: impl Fn() -> BoxedVarValueAny + Send + 'static,
    ) -> VarAny {
        let caps = self.capabilities();

        if caps.is_contextual() {
            let me = self.clone();
            let fns = Arc::new(Mutex::new((map, map_back, fallback_init)));
            return any_contextual_var(move || {
                me.filter_map_bidi_any(
                    clmv!(fns, |v| fns.lock().0(v)),
                    clmv!(fns, |v| fns.lock().1(v)),
                    clmv!(fns, || fns.lock().2()),
                )
            });
        }

        let mut init_value = None;
        self.with(&mut |v: &dyn VarValueAny| init_value = map(v));
        let init_value = init_value.unwrap_or_else(&fallback_init);

        if caps.is_const() {
            return crate::any_const_var(init_value);
        }

        let output = crate::var_any(init_value);

        self.bind_filter_map_bidi_any(&output, map, map_back).perm();
        output.hold(self.clone()).perm();

        output
    }

    /// Create a mapping variable from any to any that *unwraps* an inner variable.
    ///
    /// See [`flat_map`] for more details about flat mapping variables.
    ///
    /// [`flat_map`]: Var::flat_map
    pub fn flat_map_any(&self, map: impl FnMut(&dyn VarValueAny) -> VarAny + Send + 'static) -> VarAny {
        let mapping = crate::var_impl::flat_map::FlatMapVar::new(self.clone(), smallbox!(map));
        VarAny(smallbox!(mapping))
    }

    /// Create a strongly typed flat mapping variable.
    ///
    /// See [`flat_map`] for more details about mapping variables.
    ///
    /// [`map`]: Var::map
    pub fn flat_map<O: VarValue>(&self, mut map: impl FnMut(&dyn VarValueAny) -> Var<O> + Send + 'static) -> Var<O> {
        let mapping = self.flat_map_any(move |v| {
            let typed = map(v);
            typed.into()
        });
        Var::new_any(mapping)
    }
}
/// Binding
impl VarAny {
    /// Bind `other` to receive the new values from this variable.
    ///
    /// See [`bind`] for more details about variable bindings.
    ///
    /// [`bind`]: Var::bind
    pub fn bind(&self, other: &VarAny) -> VarHandle {
        self.bind_map_any(other, |v| v.clone_boxed())
    }

    /// Like [`bind`] but also sets `other` to the current value.
    ///
    /// See [`set_bind`] for more details.
    ///
    /// [`bind`]: Self::bind
    /// [`set_bind`]: Var::set_bind
    pub fn set_bind(&self, other: &VarAny) -> VarHandle {
        other.set_from(self);
        self.bind(other)
    }

    /// Bind `other` to receive the new values mapped from this variable.
    ///
    /// See [`bind_map`] for more details about variable bindings.
    ///
    /// [`bind_map`]: Var::bind_map
    pub fn bind_map_any(&self, other: &VarAny, map: impl FnMut(&dyn VarValueAny) -> BoxedVarValueAny + Send + 'static) -> VarHandle {
        if self.capabilities().is_const() || other.capabilities().is_always_read_only() {
            return VarHandle::dummy();
        }

        self.bind_impl(other, map)
    }

    /// Like [`bind_map_any`] but also sets `other` to the current value.
    ///
    /// See [`set_bind_map`] for more details.
    ///
    /// [`bind_map_any`]: Self::bind_map_any
    /// [`set_bind_map`]: Var::set_bind_map
    pub fn set_bind_map_any(&self, other: &VarAny, map: impl FnMut(&dyn VarValueAny) -> BoxedVarValueAny + Send + 'static) -> VarHandle {
        let map = Arc::new(Mutex::new(map));
        other.set_from_map(self, clmv!(map, |v| map.lock()(v)));

        enum MapFn<F> {
            Hot(F),
            Cold(Arc<Mutex<F>>),
            Taken,
        }
        let mut map = MapFn::Cold(map);
        self.bind_map_any(other, move |v| match mem::replace(&mut map, MapFn::Taken) {
            MapFn::Hot(mut f) => {
                let r = f(v);
                map = MapFn::Hot(f);
                r
            }
            MapFn::Cold(f) => match Arc::try_unwrap(f) {
                Ok(f) => {
                    let mut f = f.into_inner();
                    let r = f(v);
                    map = MapFn::Hot(f);
                    r
                }
                Err(f) => {
                    let r = f.lock()(v);
                    map = MapFn::Cold(f);
                    r
                }
            },
            MapFn::Taken => unreachable!(),
        })
    }

    /// Bind strongly typed `other` to receive the new values mapped from this variable.
    ///
    /// See [`bind_map`] for more details about variable bindings.
    ///
    /// [`bind_map`]: Var::bind_map
    pub fn bind_map<O: VarValue>(&self, other: &Var<O>, mut map: impl FnMut(&dyn VarValueAny) -> O + Send + 'static) -> VarHandle {
        self.bind_map_any(other, move |v| BoxedVarValueAny::new(map(v)))
    }

    /// Like [`bind_map_any`] but also sets `other` to the current value.
    ///
    /// See [`set_bind_map`] for more details.
    ///
    /// [`bind_map_any`]: Self::bind_map_any
    /// [`set_bind_map`]: Var::set_bind_map
    pub fn set_bind_map<O: VarValue>(&self, other: &Var<O>, mut map: impl FnMut(&dyn VarValueAny) -> O + Send + 'static) -> VarHandle {
        self.set_bind_map_any(other, move |v| BoxedVarValueAny::new(map(v)))
    }

    /// Bind `other` to receive the new values from this variable and this variable to receive new values from `other`.
    ///
    /// See [`bind_bidi`] for more details about variable bindings.
    ///
    /// [`bind_bidi`]: Var::bind_bidi
    pub fn bind_bidi(&self, other: &VarAny) -> VarHandles {
        self.bind_map_bidi_any(other, |v| v.clone_boxed(), |v| v.clone_boxed())
    }

    /// Bind `other` to receive the new mapped values from this variable and this variable to receive new mapped values from `other`.
    ///
    /// See [`bind_bidi`] for more details about variable bindings.
    ///
    /// [`bind_bidi`]: Var::bind_bidi
    pub fn bind_map_bidi_any(
        &self,
        other: &VarAny,
        map: impl FnMut(&dyn VarValueAny) -> BoxedVarValueAny + Send + 'static,
        map_back: impl FnMut(&dyn VarValueAny) -> BoxedVarValueAny + Send + 'static,
    ) -> VarHandles {
        assert!(!self.var_eq(other), "cannot bind var to itself");

        let self_cap = self.capabilities();
        let other_cap = other.capabilities();
        if self_cap.is_const() || other_cap.is_const() {
            return VarHandles::dummy();
        }
        if self_cap.is_always_read_only() {
            return self.bind_map_any(other, map).into();
        }
        if other_cap.is_always_read_only() {
            return other.bind_map_any(self, map_back).into();
        }

        let a = self.bind_impl(other, map);
        let b = other.bind_impl(self, map_back);

        a.with(b)
    }

    /// Bind `other` to receive the new values filtered mapped from this variable.
    ///
    /// See [`bind_filter_map`] for more details about variable bindings.
    ///
    /// [`bind_filter_map`]: Var::bind_filter_map
    pub fn bind_filter_map_any(
        &self,
        other: &VarAny,
        map: impl FnMut(&dyn VarValueAny) -> Option<BoxedVarValueAny> + Send + 'static,
    ) -> VarHandle {
        if self.capabilities().is_const() || other.capabilities().is_always_read_only() {
            return VarHandle::dummy();
        }

        self.bind_filter_map_impl(other, map)
    }

    /// Bind strongly typed `other` to receive the new values filtered mapped from this variable.
    ///
    /// See [`bind_filter_map`] for more details about variable bindings.
    ///
    /// [`bind_filter_map`]: Var::bind_filter_map
    pub fn bind_filter_map<O: VarValue>(
        &self,
        other: &VarAny,
        mut map: impl FnMut(&dyn VarValueAny) -> Option<O> + Send + 'static,
    ) -> VarHandle {
        self.bind_filter_map_any(other, move |v| map(v).map(BoxedVarValueAny::new))
    }

    /// Bind `other` to receive the new filtered mapped values from this variable and this variable to receive
    /// new filtered mapped values from `other`.
    ///
    /// See [`bind_filter_map_bidi`] for more details about variable bindings.
    ///
    /// [`bind_filter_map_bidi`]: Var::bind_filter_map_bidi
    pub fn bind_filter_map_bidi_any(
        &self,
        other: &VarAny,
        map: impl FnMut(&dyn VarValueAny) -> Option<BoxedVarValueAny> + Send + 'static,
        map_back: impl FnMut(&dyn VarValueAny) -> Option<BoxedVarValueAny> + Send + 'static,
    ) -> VarHandles {
        let self_cap = self.capabilities();
        let other_cap = other.capabilities();
        if self_cap.is_const() || other_cap.is_const() {
            return VarHandles::dummy();
        }
        if self_cap.is_always_read_only() {
            return self.bind_filter_map_any(other, map).into();
        }
        if other_cap.is_always_read_only() {
            return other.bind_filter_map_any(self, map_back).into();
        }

        let a = self.bind_filter_map_impl(other, map);
        let b = other.bind_filter_map_impl(self, map_back);

        a.with(b)
    }

    fn bind_impl(&self, other: &VarAny, mut map: impl FnMut(&dyn VarValueAny) -> BoxedVarValueAny + Send + 'static) -> VarHandle {
        let weak_other = other.downgrade();
        let self_tag = self.var_instance_tag();
        self.hook(move |args| {
            if let Some(other) = weak_other.upgrade() {
                if args.contains_tag(&other.var_instance_tag()) {
                    // skip circular update
                    return true;
                }

                let new_value = map(args.value());
                let update = args.update();
                other.modify(move |v| {
                    if v.set(new_value) || update {
                        // tag to avoid circular update
                        v.push_tag(self_tag);
                    }
                    if update {
                        // propagate explicit update requests
                        v.update();
                    }
                });
                true
            } else {
                false
            }
        })
    }

    fn bind_filter_map_impl(
        &self,
        other: &VarAny,
        mut map: impl FnMut(&dyn VarValueAny) -> Option<BoxedVarValueAny> + Send + 'static,
    ) -> VarHandle {
        let weak_other = other.downgrade();
        let self_tag = self.var_instance_tag();
        self.hook(move |args| {
            if let Some(other) = weak_other.upgrade() {
                if args.contains_tag(&other.var_instance_tag()) {
                    // skip circular update
                    return true;
                }

                let update = args.update();
                if let Some(new_value) = map(args.value()) {
                    other.modify(move |v| {
                        if v.set(new_value) || update {
                            // tag to avoid circular update
                            v.push_tag(self_tag);
                        }
                        if update {
                            // propagate explicit update requests
                            v.update();
                        }
                    });
                } else if update {
                    other.modify(move |v| {
                        v.update();
                        v.push_tag(self_tag);
                    });
                }

                true
            } else {
                false
            }
        })
    }
}
/// Animation
impl VarAny {
    /// Schedule an animation that targets this variable.
    ///
    /// See [`animate`] for more details.
    ///
    /// [`animate`]: Var::animate
    pub fn animate(&self, animate: impl FnMut(&Animation, &mut VarModifyAny) + Send + 'static) -> AnimationHandle {
        if !self.capabilities().is_always_read_only() {
            let target = self.current_context();
            if !target.capabilities().is_always_read_only() {
                // target var can be animated.

                let wk_target = target.downgrade();
                let animate = Arc::new(Mutex::new(animate));

                return VARS.animate(move |args| {
                    // animation

                    if let Some(target) = wk_target.upgrade() {
                        // target still exists

                        if target.modify_importance() > VARS.current_modify().importance {
                            // var modified by a more recent animation or directly, this animation cannot
                            // affect it anymore.
                            args.stop();
                            return;
                        }

                        // try update
                        let r = target.try_modify(clmv!(animate, args, |value| {
                            (animate.lock())(&args, value);
                        }));

                        if let Err(VarIsReadOnlyError { .. }) = r {
                            // var can maybe change to allow write again, but we wipe all animations anyway.
                            args.stop();
                        }
                    } else {
                        // target dropped.
                        args.stop();
                    }
                });
            }
        }
        AnimationHandle::dummy()
    }

    /// Schedule animations started by `animate`, the closure is called once at the start to begin, then again every time
    /// the variable stops animating.
    ///
    /// See [`sequence`] for more details.
    ///
    /// [`sequence`]: Var::sequence
    pub fn sequence(&self, animate: impl FnMut(VarAny) -> AnimationHandle + Send + 'static) -> VarHandle {
        if !self.capabilities().is_always_read_only() {
            let target = self.current_context();
            if !target.capabilities().is_always_read_only() {
                // target var can be animated.

                let handle_hook = Arc::new(AtomicBool::new(false));
                let handle = VarHandle::new(handle_hook.clone());

                let wk_target = target.downgrade();

                #[derive(Clone)]
                struct SequenceController(Arc<dyn Fn() + Send + Sync + 'static>);
                impl AnimationController for SequenceController {
                    fn on_stop(&self, _: &Animation) {
                        let ctrl = self.clone();
                        VARS.with_animation_controller(ctrl, || (self.0)());
                    }
                }
                let animate = Mutex::new(animate);
                let animate = Arc::new(move || {
                    if let Some(target) = wk_target.upgrade() {
                        if target.modify_importance() <= VARS.current_modify().importance()
                            && (Arc::strong_count(&handle_hook) > 1 || handle_hook.load(std::sync::atomic::Ordering::Relaxed))
                            && VARS.animations_enabled().get()
                        {
                            (animate.lock())(target).perm();
                        }
                    }
                });
                VARS.with_animation_controller(SequenceController(animate.clone()), || {
                    animate();
                });

                return handle;
            }
        }
        VarHandle::dummy()
    }

    /*
       !!: TODO
       This entire setup of no update on animation stop does not work?
       If the variable is overridden it is cut out from the animation (so `is_animating`) changes to `false`, but
       the `hook_animation_stop` and `wait_animation` continue linked to the underlying animation
    */

    /// If the variable current value was set by an active animation.
    ///
    /// The variable [`is_new`] when this changes to `true`, but it **may not be new** when the value changes to `false`.
    /// If the variable is not updated at the last frame of the animation that has last set it, it will not update
    /// just because that animation has ended. You can use [`hook_animation_stop`] to get a notification when the
    /// last animation stops, or use [`wait_animation`] to get a future that is ready when `is_animating` changes
    /// from `true` to `false`.
    ///
    /// [`is_new`]: AnyVar::is_new
    /// [`hook_animation_stop`]: AnyVar::hook_animation_stop
    /// [`wait_animation`]: Var::wait_animation
    pub fn is_animating(&self) -> bool {
        self.0.is_animating()
    }

    /// Gets the minimum *importance* clearance that is needed to modify this variable.
    ///
    /// Direct modify/set requests always apply, but requests made from inside an animation only apply if
    /// the animation *importance* is greater or equal this value.This is the mechanism that ensures that only
    /// the latest animation has *control* of the variable value.
    ///
    /// [`MODIFY`]: VarCapability::MODIFY
    /// [`VARS.current_modify`]: VARS::current_modify
    /// [`VARS.animate`]: VARS::animate
    pub fn modify_importance(&self) -> usize {
        self.0.modify_importance()
    }

    /// Register a `handler` to be called when the current animation stops.
    ///
    /// Note that the `handler` is owned by the animation, not the variable, it will only be called/dropped when the
    /// animation stops.
    ///
    /// Returns the `handler` as an error if the variable is not animating. Note that if you are interacting
    /// with the variable from a non-UI thread the variable can stops animating between checking [`is_animating`]
    /// and registering the hook, in this case the `handler` will be returned as an error as well.
    ///
    /// [`modify_importance`]: AnyVar::modify_importance
    /// [`is_animating`]: AnyVar::is_animating
    pub fn hook_animation_stop(&self, handler: impl FnOnce() + Send + 'static) -> Result<(), AnimationStopFn> {
        let mut once = Some(handler);
        let handler: AnimationStopFn = smallbox!(move || { once.take().unwrap()() });
        self.0.hook_animation_stop(handler)
    }

    /// Awaits for [`is_animating`] to change from `true` to `false`.
    ///
    /// If the variable is not animating at the moment of this call the future will await until the animation starts and stops.
    ///
    /// [`is_animating`]: Self::is_animating
    pub fn wait_animation(&self) -> impl Future<Output = ()> + Send + Sync {
        crate::future::WaitIsNotAnimatingFut::new(self)
    }
}
/// Value type.
impl VarAny {
    /// Returns the strongly typed variable, if its of of value type `T`.
    pub fn downcast<T: VarValue>(self) -> Result<Var<T>, VarAny> {
        if self.value_is::<T>() { Ok(Var::new_any(self)) } else { Err(self) }
    }

    /// Returns [`downcast`] or `fallback_var`.
    ///
    /// [`downcast`]: Self::downcast
    pub fn downcast_or<T: VarValue, F: Into<Var<T>>>(self, fallback_var: impl FnOnce(VarAny) -> F) -> Var<T> {
        match self.downcast() {
            Ok(tv) => tv,
            Err(av) => fallback_var(av).into(),
        }
    }

    /// Gets the value type.
    pub fn value_type(&self) -> TypeId {
        self.0.value_type()
    }

    /// Gets the value type name.
    #[cfg(feature = "value_type_name")]
    pub fn value_type_name(&self) -> &'static str {
        self.0.value_type_name()
    }

    /// Gets if the value type is `T`.
    pub fn value_is<T: VarValue>(&self) -> bool {
        self.value_type() == TypeId::of::<T>()
    }
}
/// Variable type.
impl VarAny {
    /// Flags that indicate what operations the variable is capable of in this update.
    pub fn capabilities(&self) -> VarCapability {
        self.0.capabilities()
    }

    /// Current count of strong references to this variable.
    ///
    /// If this variable is [`SHARE`] cloning the variable only clones a reference to the variable.
    /// If this variable is local this is always `1` as it clones the value.
    ///
    /// [`SHARE`]: VarCapability::SHARE
    pub fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    /// Create a weak reference to this variable.
    ///
    /// If this variable is [`SHARE`] returns a weak reference to the variable that can be upgraded to the variable it
    /// it is still alive. If this variable is local returns a dummy weak reference that cannot upgrade.
    ///
    /// [`SHARE`]: VarCapability::SHARE
    pub fn downgrade(&self) -> WeakVarAny {
        WeakVarAny(self.0.downgrade())
    }

    /// Gets if this variable is the same as `other`.
    ///
    /// If this variable is [`SHARE`] compares the *pointer*. If this variable is local this is always `false`.
    ///
    /// [`SHARE`]: VarCapability::SHARE
    pub fn var_eq(&self, other: &VarAny) -> bool {
        self.0.var_eq(&*other.0)
    }

    /// Copy ID that identifies this variable instance.
    ///
    /// The ID is only unique if this variable is [`SHARE`] and only while the variable is alive.
    /// This can be used with [`VarModify::push_tag`] and [`VarAnyHookArgs::contains_tag`] to avoid cyclic updates in custom
    /// bidirectional bindings.
    ///
    /// [`SHARE`]: VarCapability::SHARE
    pub fn var_instance_tag(&self) -> VarInstanceTag {
        self.0.var_instance_tag()
    }

    /// Gets a clone of the var that is always read-only.
    ///
    /// The returned variable can still update if `self` is modified, but it does not have the [`MODIFY`] capability.
    ///
    /// [`MODIFY`]: VarCapability::MODIFY
    pub fn read_only(&self) -> VarAny {
        let cap = self.capabilities();
        if cap.is_always_read_only() {
            return self.clone();
        }

        let inner: &dyn Any = &*self.0;
        if inner.type_id() == TypeId::of::<crate::read_only::ReadOnlyVar>() {
            return self.clone();
        }

        VarAny(smallbox!(crate::read_only::ReadOnlyVar(self.clone())))
    }

    /// Create a var that redirects to this variable until the first value update, then it disconnects as a separate variable.
    ///
    /// The return variable is *clone-on-write* and has the `MODIFY` capability independent of the source capabilities, when
    /// a modify request is made the source value is cloned and offered for modification, if modified the source variable is dropped,
    /// if the modify closure does not update the source variable is retained.
    pub fn cow(&self) -> VarAny {
        VarAny(smallbox!(crate::clone_on_write::CowVar::new(self.clone())))
    }

    /// Hold the variable in memory until the app exit.
    ///
    /// Note that this is different from [`std::mem::forget`], if the app is compiled with `"multi_app"` feature
    /// the variable will be dropped before the new app instance in the same process.
    pub fn perm(&self) {
        VARS.perm(self.clone());
    }

    /// Hold arbitrary `thing` for the lifetime of this variable or the return handle.
    pub fn hold(&self, thing: impl Any + Send) -> VarHandle {
        self.hold_impl(smallbox!(thing))
    }
    fn hold_impl(&self, thing: SmallBox<dyn Any + Send, smallbox::space::S2>) -> VarHandle {
        self.hook(move |_| {
            let _hold = &thing;
            true
        })
    }

    /// Gets the underlying var in the current calling context.
    ///
    /// If this variable is [`CONTEXT`] returns a clone of the inner variable,
    /// otherwise returns a clone of this variable.
    ///
    /// [`CONTEXT`]: VarCapability::CONTEXT
    pub fn current_context(&self) -> VarAny {
        if self.capabilities().is_contextual() {
            VarAny(self.0.current_context())
        } else {
            self.clone()
        }
    }
}

/// Weak reference to a [`VarAny`].
pub struct WeakVarAny(pub(crate) SmallBox<dyn WeakVarImpl, smallbox::space::S2>);
impl Clone for WeakVarAny {
    fn clone(&self) -> Self {
        Self(self.0.clone_boxed())
    }
}
impl WeakVarAny {
    /// Current count of strong references to the variable.
    pub fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    /// Attempt to create a strong reference to the variable.
    pub fn upgrade(&self) -> Option<VarAny> {
        self.0.upgrade().map(VarAny)
    }
}

/// Arguments for [`VarAny::hook_any`].
pub struct VarAnyHookArgs<'a> {
    pub(super) value: &'a dyn VarValueAny,
    pub(super) update: bool,
    pub(super) tags: &'a [BoxedVarValueAny],
}
impl<'a> VarAnyHookArgs<'a> {
    /// New from updated value and custom tag.
    pub fn new(value: &'a dyn VarValueAny, update: bool, tags: &'a [BoxedVarValueAny]) -> Self {
        Self { value, update, tags }
    }

    /// Reference the updated value.
    pub fn value(&self) -> &'a dyn VarValueAny {
        self.value
    }

    /// If update was explicitly requested.
    ///
    /// Note that bindings/mappings propagate this update request.
    pub fn update(&self) -> bool {
        self.update
    }

    /// Value type ID.
    pub fn value_type(&self) -> TypeId {
        self.value.type_id()
    }

    /// Custom tag objects.
    pub fn tags(&self) -> &[BoxedVarValueAny] {
        self.tags
    }

    /// Clone the custom tag objects set by the code that updated the value.
    pub fn tags_vec(&self) -> Vec<BoxedVarValueAny> {
        self.tags.iter().map(|t| (*t).clone_boxed()).collect()
    }

    /// Reference the value, if it is of type `T`.
    pub fn downcast_value<T: VarValue>(&self) -> Option<&T> {
        self.value.downcast_ref()
    }

    /// Reference all custom tag values of type `T`.
    pub fn downcast_tags<T: VarValue>(&self) -> impl Iterator<Item = &T> + '_ {
        self.tags.iter().filter_map(|t| (*t).downcast_ref::<T>())
    }

    /// Gets if the `tag` is in [`tags`].
    ///
    /// [`tags`]: Self::tags
    pub fn contains_tag<T: VarValue>(&self, tag: &T) -> bool {
        self.downcast_tags::<T>().any(|t| t == tag)
    }

    /// Try cast to strongly typed args.
    pub fn downcast<T: VarValue>(&self) -> Option<crate::VarHookArgs<'_, T>> {
        if TypeId::of::<T>() == self.value_type() {
            Some(crate::VarHookArgs {
                any: self,
                _t: PhantomData,
            })
        } else {
            None
        }
    }
}

/// Unique identifier of a share variable, while it is alive.
///
/// See [`VarAny::var_instance_tag`] for more details
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VarInstanceTag(pub(crate) usize);
impl fmt::Debug for VarInstanceTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self == Self::NOT_SHARED {
            write!(f, "VarInstanceTag::NOT_SHARED")
        } else {
            write!(f, "VarInstanceTag(0x{:X})", self.0)
        }
    }
}
impl VarInstanceTag {
    /// ID for variables that are not [`SHARE`].
    ///
    /// [`SHARE`]: VarCapability::SHARE
    pub const NOT_SHARED: VarInstanceTag = VarInstanceTag(0);
}
