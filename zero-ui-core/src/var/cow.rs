use std::{
    mem,
    sync::{Arc, Weak},
};

use parking_lot::RwLock;

use super::{animation::AnimateModifyInfo, *};

enum Data<T: VarValue, S> {
    Source {
        source: S,
        source_handle: VarHandle,
        hooks: Vec<VarHook>,
    },
    Owned {
        value: T,
        last_update: VarUpdateId,
        hooks: Vec<VarHook>,
        animation: AnimateModifyInfo,
    },
}

/// See [`Var::cow`].
pub struct RcCowVar<T: VarValue, S>(Arc<RwLock<Data<T, S>>>);

/// Weak reference to a [`RcCowVar<T>`].
pub struct WeakCowVar<T: VarValue, S>(Weak<RwLock<Data<T, S>>>);

impl<T: VarValue, S: Var<T>> RcCowVar<T, S> {
    pub(super) fn new(source: S) -> Self {
        let cow = Arc::new(RwLock::new(Data::Source {
            source,
            source_handle: VarHandle::dummy(),
            hooks: vec![],
        }));
        {
            let mut data = cow.write();
            if let Data::Source { source, source_handle, .. } = &mut *data {
                let weak_cow = Arc::downgrade(&cow);
                *source_handle = source.hook(Box::new(move |vars, updates, value| {
                    if let Some(cow) = weak_cow.upgrade() {
                        match &mut *cow.write() {
                            Data::Source { hooks, .. } => {
                                hooks.retain(|h| h.call(vars, updates, value));
                                true
                            }
                            Data::Owned { .. } => false,
                        }
                    } else {
                        false
                    }
                }));
            }
        }
        Self(cow)
    }

    fn modify_impl(&self, vars: &Vars, modify: impl FnOnce(&mut Cow<T>) + 'static) -> Result<(), VarIsReadOnlyError> {
        let me = self.clone();
        vars.schedule_update(Box::new(move |vars, updates| {
            let mut data = me.0.write();
            let data = &mut *data;

            match data {
                Data::Source { source, hooks, .. } => {
                    let modified = source.with(|val| {
                        let mut r = Cow::Borrowed(val);
                        modify(&mut r);
                        match r {
                            Cow::Owned(r) => Some(r),
                            Cow::Borrowed(_) => None,
                        }
                    });
                    if let Some(value) = modified {
                        *data = Data::Owned {
                            value,
                            last_update: vars.update_id(),
                            hooks: mem::take(hooks),
                            animation: vars.current_animation(),
                        };
                    }
                }
                Data::Owned {
                    value,
                    last_update,
                    hooks,
                    animation,
                } => {
                    {
                        let curr_anim = vars.current_animation();
                        if curr_anim.importance() < animation.importance() {
                            return;
                        }
                        *animation = curr_anim;
                    }

                    let new_value = {
                        let mut value = Cow::Borrowed(value);
                        modify(&mut value);
                        match value {
                            Cow::Owned(v) => Some(v),
                            Cow::Borrowed(_) => None,
                        }
                    };

                    if let Some(new_value) = new_value {
                        *value = new_value;
                        *last_update = vars.update_id();
                        hooks.retain(|h| h.call(vars, updates, value));
                        updates.update_ext();
                    }
                }
            }
        }));
        Ok(())
    }

    impl_infallible_write! {
        for<T>
    }
}

impl<T: VarValue, S> Clone for RcCowVar<T, S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T: VarValue, S> Clone for WeakCowVar<T, S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: VarValue, S: Var<T>> crate::private::Sealed for RcCowVar<T, S> {}
impl<T: VarValue, S: Var<T>> crate::private::Sealed for WeakCowVar<T, S> {}

impl<T: VarValue, S: Var<T>> AnyVar for RcCowVar<T, S> {
    fn clone_any(&self) -> BoxedAnyVar {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn double_boxed_any(self: Box<Self>) -> Box<dyn Any> {
        let me: BoxedVar<T> = self;
        Box::new(me)
    }

    fn var_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn get_any(&self) -> Box<dyn AnyVarValue> {
        Box::new(self.get())
    }

    fn set_any(&self, vars: &Vars, value: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        self.modify(vars, var_set_any(value));
        Ok(())
    }

    fn last_update(&self) -> VarUpdateId {
        match &*self.0.read_recursive() {
            Data::Source { source, .. } => source.last_update(),
            Data::Owned { last_update, .. } => *last_update,
        }
    }

    fn capabilities(&self) -> VarCapabilities {
        VarCapabilities::MODIFY
    }

    fn hook(&self, pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool + Send + Sync>) -> VarHandle {
        let mut data = self.0.write();
        match &mut *data {
            Data::Source { hooks, .. } => {
                let (hook, weak) = VarHandle::new(pos_modify_action);
                hooks.push(weak);
                hook
            }
            Data::Owned { hooks, .. } => {
                let (hook, weak) = VarHandle::new(pos_modify_action);
                hooks.push(weak);
                hook
            }
        }
    }

    fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    fn weak_count(&self) -> usize {
        Arc::weak_count(&self.0)
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        self.clone_any()
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(WeakCowVar(Arc::downgrade(&self.0)))
    }

    fn is_animating(&self) -> bool {
        match &*self.0.read_recursive() {
            Data::Source { source, .. } => source.is_animating(),
            Data::Owned { animation, .. } => animation.is_animating(),
        }
    }

    fn var_ptr(&self) -> VarPtr {
        VarPtr::new_arc(&self.0)
    }
}

impl<T: VarValue, S: Var<T>> AnyWeakVar for WeakCowVar<T, S> {
    fn clone_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.clone())
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.0.weak_count()
    }

    fn upgrade_any(&self) -> Option<BoxedAnyVar> {
        self.0.upgrade().map(|rc| Box::new(RcCowVar(rc)) as _)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: VarValue, S: Var<T>> IntoVar<T> for RcCowVar<T, S> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue, S: Var<T>> Var<T> for RcCowVar<T, S> {
    type ReadOnly = types::ReadOnlyVar<T, Self>;

    type ActualVar = Self;

    type Downgrade = WeakCowVar<T, S>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        match &*self.0.read_recursive() {
            Data::Source { source, .. } => source.with(read),
            Data::Owned { value, .. } => read(value),
        }
    }

    fn modify<V, F>(&self, vars: &V, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        V: WithVars,
        F: FnOnce(&mut Cow<T>) + 'static,
    {
        vars.with_vars(|vars| self.modify_impl(vars, modify))
    }

    fn actual_var(self) -> Self {
        self
    }

    fn downgrade(&self) -> Self::Downgrade {
        WeakCowVar(Arc::downgrade(&self.0))
    }

    fn into_value(self) -> T {
        match Arc::try_unwrap(self.0) {
            Ok(state) => match state.into_inner() {
                Data::Source { source, .. } => source.into_value(),
                Data::Owned { value, .. } => value,
            },
            Err(rc) => Self(rc).get(),
        }
    }

    fn read_only(&self) -> Self::ReadOnly {
        types::ReadOnlyVar::new(self.clone())
    }
}

impl<T: VarValue, S: Var<T>> WeakVar<T> for WeakCowVar<T, S> {
    type Upgrade = RcCowVar<T, S>;

    fn upgrade(&self) -> Option<Self::Upgrade> {
        self.0.upgrade().map(|rc| RcCowVar(rc))
    }
}
