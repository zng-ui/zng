use super::*;

/// See [`Var::map_ref`].
pub struct MapRef<I, O, S> {
    source: S,
    map: Arc<dyn Fn(&I) -> &O + Send + Sync>,
}

/// Weak var that can upgrade to [`MapRef<I, O, S>`] if the source is not dropped.
pub struct WeakMapRef<I, O, S> {
    source: S,
    map: Arc<dyn Fn(&I) -> &O + Send + Sync>,
}

impl<I: VarValue, O: VarValue, S: Var<I>> MapRef<I, O, S> {
    pub(super) fn new(source: S, map: Arc<dyn Fn(&I) -> &O + Send + Sync>) -> Self {
        MapRef { source, map }
    }
}

impl<I: VarValue, O: VarValue, S: Var<I>> crate::private::Sealed for MapRef<I, O, S> {}
impl<I: VarValue, O: VarValue, S: WeakVar<I>> crate::private::Sealed for WeakMapRef<I, O, S> {}

impl<I: VarValue, O: VarValue, S: Var<I>> Clone for MapRef<I, O, S> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            map: self.map.clone(),
        }
    }
}
impl<I: VarValue, O: VarValue, S: WeakVar<I>> Clone for WeakMapRef<I, O, S> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            map: self.map.clone(),
        }
    }
}

impl<I: VarValue, O: VarValue, S: Var<I>> AnyVar for MapRef<I, O, S> {
    fn clone_any(&self) -> BoxedAnyVar {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn double_boxed_any(self: Box<Self>) -> Box<dyn Any> {
        let me: BoxedVar<O> = self;
        Box::new(me)
    }

    fn var_type_id(&self) -> TypeId {
        TypeId::of::<O>()
    }

    fn get_any(&self) -> Box<dyn AnyVarValue> {
        Box::new(self.get())
    }

    fn set_any(&self, _: &Vars, _: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        Err(VarIsReadOnlyError {
            capabilities: self.capabilities(),
        })
    }

    fn last_update(&self) -> VarUpdateId {
        self.source.last_update()
    }

    fn capabilities(&self) -> VarCapabilities {
        self.source.capabilities().as_read_only()
    }

    fn hook(&self, pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool + Send + Sync>) -> VarHandle {
        let map = self.map.clone();
        self.source.hook(Box::new(move |vars, updates, value| {
            if let Some(value) = value.as_any().downcast_ref() {
                let value = map(value);
                pos_modify_action(vars, updates, value)
            } else {
                true
            }
        }))
    }

    fn strong_count(&self) -> usize {
        self.source.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.source.weak_count()
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        Box::new(self.clone().actual_var())
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.downgrade())
    }

    fn is_animating(&self) -> bool {
        self.source.is_animating()
    }

    fn modify_importance(&self) -> usize {
        self.source.modify_importance()
    }

    fn var_ptr(&self) -> VarPtr {
        self.source.var_ptr()
    }
}
impl<I: VarValue, O: VarValue, S: WeakVar<I>> AnyWeakVar for WeakMapRef<I, O, S> {
    fn clone_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.clone())
    }

    fn strong_count(&self) -> usize {
        self.source.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.source.weak_count()
    }

    fn upgrade_any(&self) -> Option<BoxedAnyVar> {
        self.upgrade().map(|m| Box::new(m) as _)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<I: VarValue, O: VarValue, S: Var<I>> IntoVar<O> for MapRef<I, O, S> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<I: VarValue, O: VarValue, S: Var<I>> Var<O> for MapRef<I, O, S> {
    type ReadOnly = Self;

    type ActualVar = MapRef<I, O, S::ActualVar>;

    type Downgrade = WeakMapRef<I, O, S::Downgrade>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&O) -> R,
    {
        self.source.with(|val| {
            let val = (self.map)(val);
            read(val)
        })
    }

    fn modify<V, F>(&self, _: &V, _: F) -> Result<(), VarIsReadOnlyError>
    where
        V: WithVars,
        F: FnOnce(&mut Cow<O>) + 'static,
    {
        Err(VarIsReadOnlyError {
            capabilities: self.capabilities(),
        })
    }

    fn actual_var(self) -> Self::ActualVar {
        MapRef {
            source: self.source.actual_var(),
            map: self.map,
        }
    }

    fn downgrade(&self) -> Self::Downgrade {
        WeakMapRef {
            source: self.source.downgrade(),
            map: self.map.clone(),
        }
    }

    fn into_value(self) -> O {
        self.get()
    }

    fn read_only(&self) -> Self::ReadOnly {
        self.clone()
    }
}

impl<I: VarValue, O: VarValue, S: WeakVar<I>> WeakVar<O> for WeakMapRef<I, O, S> {
    type Upgrade = MapRef<I, O, S::Upgrade>;

    fn upgrade(&self) -> Option<Self::Upgrade> {
        self.source.upgrade().map(|s| MapRef {
            source: s,
            map: self.map.clone(),
        })
    }
}

/// See [`Var::map_ref_bidi`].
pub struct MapRefBidi<I, O, S> {
    source: S,
    map: Arc<dyn Fn(&I) -> &O + Send + Sync>,
    map_mut: Arc<dyn Fn(&mut I) -> &mut O + Send + Sync>,
}

/// Weak var that can upgrade to [`MapRefBidi<I, O, S>`] if the source is not dropped.
pub struct WeakMapRefBidi<I, O, S> {
    source: S,
    map: Arc<dyn Fn(&I) -> &O + Send + Sync>,
    map_mut: Arc<dyn Fn(&mut I) -> &mut O + Send + Sync>,
}

impl<I: VarValue, O: VarValue, S: Var<I>> MapRefBidi<I, O, S> {
    pub(super) fn new(source: S, map: Arc<dyn Fn(&I) -> &O + Send + Sync>, map_mut: Arc<dyn Fn(&mut I) -> &mut O + Send + Sync>) -> Self {
        MapRefBidi { source, map, map_mut }
    }
}

impl<I: VarValue, O: VarValue, S: Var<I>> crate::private::Sealed for MapRefBidi<I, O, S> {}
impl<I: VarValue, O: VarValue, S: WeakVar<I>> crate::private::Sealed for WeakMapRefBidi<I, O, S> {}

impl<I: VarValue, O: VarValue, S: Var<I>> Clone for MapRefBidi<I, O, S> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            map: self.map.clone(),
            map_mut: self.map_mut.clone(),
        }
    }
}
impl<I: VarValue, O: VarValue, S: WeakVar<I>> Clone for WeakMapRefBidi<I, O, S> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            map: self.map.clone(),
            map_mut: self.map_mut.clone(),
        }
    }
}

impl<I: VarValue, O: VarValue, S: Var<I>> AnyVar for MapRefBidi<I, O, S> {
    fn clone_any(&self) -> BoxedAnyVar {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn double_boxed_any(self: Box<Self>) -> Box<dyn Any> {
        let me: BoxedVar<O> = self;
        Box::new(me)
    }

    fn var_type_id(&self) -> TypeId {
        TypeId::of::<O>()
    }

    fn get_any(&self) -> Box<dyn AnyVarValue> {
        Box::new(self.get())
    }

    fn set_any(&self, vars: &Vars, value: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        self.modify(vars, var_set_any(value))
    }

    fn last_update(&self) -> VarUpdateId {
        self.source.last_update()
    }

    fn capabilities(&self) -> VarCapabilities {
        self.source.capabilities()
    }

    fn hook(&self, pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool + Send + Sync>) -> VarHandle {
        let map = self.map.clone();
        self.source.hook(Box::new(move |vars, updates, value| {
            if let Some(value) = value.as_any().downcast_ref() {
                let value = map(value);
                pos_modify_action(vars, updates, value)
            } else {
                true
            }
        }))
    }

    fn strong_count(&self) -> usize {
        self.source.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.source.weak_count()
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        Box::new(self.clone().actual_var())
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.downgrade())
    }

    fn is_animating(&self) -> bool {
        self.source.is_animating()
    }

    fn modify_importance(&self) -> usize {
        self.source.modify_importance()
    }

    fn var_ptr(&self) -> VarPtr {
        self.source.var_ptr()
    }
}
impl<I: VarValue, O: VarValue, S: WeakVar<I>> AnyWeakVar for WeakMapRefBidi<I, O, S> {
    fn clone_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.clone())
    }

    fn strong_count(&self) -> usize {
        self.source.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.source.weak_count()
    }

    fn upgrade_any(&self) -> Option<BoxedAnyVar> {
        self.upgrade().map(|m| Box::new(m) as _)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<I: VarValue, O: VarValue, S: Var<I>> IntoVar<O> for MapRefBidi<I, O, S> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<I: VarValue, O: VarValue, S: Var<I>> Var<O> for MapRefBidi<I, O, S> {
    type ReadOnly = Self;

    type ActualVar = MapRefBidi<I, O, S::ActualVar>;

    type Downgrade = WeakMapRefBidi<I, O, S::Downgrade>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&O) -> R,
    {
        self.source.with(|val| {
            let val = (self.map)(val);
            read(val)
        })
    }

    fn modify<V, F>(&self, vars: &V, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        V: WithVars,
        F: FnOnce(&mut Cow<O>) + 'static,
    {
        let map = self.map.clone();
        let map_mut = self.map_mut.clone();
        self.source.modify(vars, move |value| {
            let mut inner = Cow::Borrowed(map(value.as_ref()));
            modify(&mut inner);
            if let Cow::Owned(inner) = inner {
                *map_mut(value.to_mut()) = inner;
            }
        })
    }

    fn actual_var(self) -> Self::ActualVar {
        MapRefBidi {
            source: self.source.actual_var(),
            map: self.map,
            map_mut: self.map_mut,
        }
    }

    fn downgrade(&self) -> Self::Downgrade {
        WeakMapRefBidi {
            source: self.source.downgrade(),
            map: self.map.clone(),
            map_mut: self.map_mut.clone(),
        }
    }

    fn into_value(self) -> O {
        self.get()
    }

    fn read_only(&self) -> Self::ReadOnly {
        self.clone()
    }
}

impl<I: VarValue, O: VarValue, S: WeakVar<I>> WeakVar<O> for WeakMapRefBidi<I, O, S> {
    type Upgrade = MapRefBidi<I, O, S::Upgrade>;

    fn upgrade(&self) -> Option<Self::Upgrade> {
        self.source.upgrade().map(|s| MapRefBidi {
            source: s,
            map: self.map.clone(),
            map_mut: self.map_mut.clone(),
        })
    }
}
