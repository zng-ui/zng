use super::*;

/// A [`Var`] that locally owns the value.
///
/// This is [`always read-only`](VarObj::always_read_only), [cannot update](VarObj::can_update) and
/// is a [`VarLocal`].
///
/// This is the variable type used when a property is set to a fixed value.
/// All types that are [`VarValue`] implement [`IntoVar`] to this var type so you
/// rarely need to manually construct this variable.
#[derive(Clone, Default)]
pub struct OwnedVar<T: VarValue>(pub T);
impl<T: VarValue> protected::Var for OwnedVar<T> {}
#[cfg(debug_assertions)]
impl<T: VarValue> VarDebug for OwnedVar<T> {
    fn debug_var(&self) -> BoxedVar<crate::debug::ValueInfo> {
        OwnedVar(crate::debug::ValueInfo::new_debug_only(&self.0)).boxed()
    }
}
#[cfg(debug_assertions)]
impl<T: VarValue + std::fmt::Display> VarDisplay for OwnedVar<T> {
    fn display_var(&self) -> BoxedVar<crate::debug::ValueInfo> {
        OwnedVar(crate::debug::ValueInfo::new_display(&self.0)).boxed()
    }
}
impl<T: VarValue> VarObj<T> for OwnedVar<T> {
    fn get<'a>(&'a self, _: &'a VarsRead) -> &'a T {
        &self.0
    }

    fn get_new<'a>(&'a self, _: &'a Vars) -> Option<&'a T> {
        None
    }

    fn is_new(&self, _: &Vars) -> bool {
        false
    }

    fn version(&self, _: &VarsRead) -> u32 {
        0
    }

    fn is_read_only(&self, _: &Vars) -> bool {
        true
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        false
    }

    fn set(&self, _: &Vars, _: T) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn set_ne(&self, _: &Vars, _: T) -> Result<bool, VarIsReadOnly>
    where
        T: PartialEq,
    {
        Err(VarIsReadOnly)
    }

    fn modify_boxed(&self, _: &Vars, _: Box<dyn FnOnce(&mut T)>) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }
}
impl<T: VarValue> VarLocal<T> for OwnedVar<T> {
    fn get_local(&self) -> &T {
        &self.0
    }
    fn init_local(&mut self, _: &Vars) -> &T {
        &self.0
    }

    fn update_local(&mut self, _: &Vars) -> Option<&T> {
        None
    }
}
impl<T: VarValue> Var<T> for OwnedVar<T> {
    type AsReadOnly = Self;

    type AsLocal = Self;
    fn into_local(self) -> Self::AsLocal {
        self
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        self
    }

    fn modify<F: FnOnce(&mut T) + 'static>(&self, _: &Vars, _: F) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn map<O: VarValue, F: FnMut(&T) -> O + 'static>(&self, map: F) -> RcMapVar<T, O, Self, F> {
        self.clone().into_map(map)
    }

    fn map_ref<O: VarValue, F: Fn(&T) -> &O + Clone + 'static>(&self, map: F) -> MapRefVar<T, O, Self, F> {
        self.clone().into_map_ref(map)
    }

    fn map_bidi<O: VarValue, F: FnMut(&T) -> O + 'static, G: FnMut(O) -> T + 'static>(
        &self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<T, O, Self, F, G> {
        self.clone().into_map_bidi(map, map_back)
    }

    fn into_map<O: VarValue, F: FnMut(&T) -> O + 'static>(self, map: F) -> RcMapVar<T, O, Self, F> {
        RcMapVar::new(self, map)
    }

    fn into_map_bidi<O: VarValue, F: FnMut(&T) -> O + 'static, G: FnMut(O) -> T + 'static>(
        self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<T, O, Self, F, G> {
        RcMapBidiVar::new(self, map, map_back)
    }

    fn into_map_ref<O: VarValue, F: Fn(&T) -> &O + Clone + 'static>(self, map: F) -> MapRefVar<T, O, Self, F> {
        MapRefVar::new(self, map)
    }

    fn map_bidi_ref<O: VarValue, F: Fn(&T) -> &O + Clone + 'static, G: Fn(&mut T) -> &mut O + Clone + 'static>(
        &self,
        map: F,
        map_mut: G,
    ) -> MapBidiRefVar<T, O, Self, F, G> {
        self.clone().into_map_bidi_ref(map, map_mut)
    }

    fn into_map_bidi_ref<O: VarValue, F: Fn(&T) -> &O + Clone + 'static, G: Fn(&mut T) -> &mut O + Clone + 'static>(
        self,
        map: F,
        map_mut: G,
    ) -> MapBidiRefVar<T, O, Self, F, G> {
        MapBidiRefVar::new(self, map, map_mut)
    }
}

/// Wraps the value in an [`OwnedVar`] value.
impl<T: VarValue> IntoVar<T> for T {
    type Var = OwnedVar<T>;

    fn into_var(self) -> OwnedVar<T> {
        OwnedVar(self)
    }
}

impl<T: VarValue> IntoVar<T> for OwnedVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}
