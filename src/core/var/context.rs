use super::{protected, CloningLocalVar, ContextVar, IntoVar, MapVar, MapVarBiDi, MapVarBiDiInner, MapVarInner, ObjVar, Var, VarValue};
use crate::core::context::{ContextVarStageId, Vars};
use fnv::FnvHashMap;
use std::{
    cell::{RefCell, UnsafeCell},
    marker::PhantomData,
    rc::Rc,
};

/// [`ContextVar`](ContextVar) var. Use [`context_var!`](context_var!) to generate context variables.
pub struct ContextVarImpl<V: ContextVar>(PhantomData<V>);

impl<T: VarValue, V: ContextVar<Type = T>> protected::Var<T> for ContextVarImpl<V> {
    fn bind_info<'a, 'b>(&'a self, _: &'b Vars) -> protected::BindInfo<'a, T> {
        protected::BindInfo::ContextVar(std::any::TypeId::of::<V>(), V::default(), None)
    }

    fn is_context_var(&self) -> bool {
        true
    }
}

impl<T: VarValue, V: ContextVar<Type = T>> Clone for ContextVarImpl<V> {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl<T: VarValue, V: ContextVar<Type = T>> Copy for ContextVarImpl<V> {}

impl<T: VarValue, V: ContextVar<Type = T>> Default for ContextVarImpl<V> {
    fn default() -> Self {
        ContextVarImpl(PhantomData)
    }
}

impl<T: VarValue, V: ContextVar<Type = T>> ObjVar<T> for ContextVarImpl<V> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
        vars.context::<V>()
    }

    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        vars.context_update::<V>()
    }

    fn is_new(&self, vars: &Vars) -> bool {
        vars.context_is_new::<V>()
    }

    fn version(&self, vars: &Vars) -> u32 {
        vars.context_version::<V>()
    }
}

impl<T: VarValue, V: ContextVar<Type = T>> Var<T> for ContextVarImpl<V> {
    type AsReadOnly = Self;
    type AsLocal = CloningLocalVar<T, Self>;

    fn map<O, M>(&self, map: M) -> MapVar<T, Self, O, M>
    where
        M: FnMut(&T) -> O + 'static,
        O: VarValue,
    {
        MapVar::new(MapVarInner::Context(MapContextVar::new(*self, map)))
    }

    fn map_bidi<O, M, N>(&self, map: M, _: N) -> MapVarBiDi<T, Self, O, M, N>
    where
        M: FnMut(&T) -> O + 'static,
        N: FnMut(&O) -> T + 'static,
        O: VarValue,
    {
        MapVarBiDi::new(MapVarBiDiInner::Context(MapContextVar::new(*self, map)))
    }

    fn as_read_only(self) -> Self {
        self
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }
}

impl<T: VarValue, V: ContextVar<Type = T>> IntoVar<T> for ContextVarImpl<V> {
    type Var = Self;

    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}

/* MAP */

type MapContextVarOutputs<O> = FnvHashMap<ContextVarStageId, (UnsafeCell<O>, u32)>;

struct MapContextVarInner<T: VarValue, S: ObjVar<T>, O: VarValue, M: FnMut(&T) -> O> {
    _t: PhantomData<T>,
    source: S,
    map: RefCell<M>,
    outputs: RefCell<MapContextVarOutputs<O>>,
}

/// A variable that maps the value of a context variable.
pub(crate) struct MapContextVar<T: VarValue, S: ObjVar<T>, O: VarValue, M: FnMut(&T) -> O> {
    r: Rc<MapContextVarInner<T, S, O, M>>,
}

impl<T: VarValue, S: ObjVar<T>, O: VarValue, M: FnMut(&T) -> O> MapContextVar<T, S, O, M> {
    pub(crate) fn new(source: S, map: M) -> Self {
        MapContextVar {
            r: Rc::new(MapContextVarInner {
                _t: PhantomData,
                source,
                map: RefCell::new(map),
                outputs: RefCell::default(),
            }),
        }
    }

    fn borrow<'a>(&'a self, vars: &'a Vars) -> &'a O {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        let mut outputs = self.r.outputs.borrow_mut();
        let context_id = vars.context_id();
        let source_version = self.r.source.version(vars);

        let output = match outputs.entry(context_id) {
            Occupied(entry) => {
                let (output, output_version) = entry.into_mut();
                if *output_version != source_version {
                    let value = (&mut *self.r.map.borrow_mut())(self.r.source.get(vars));
                    // SAFETY: This is safe because it only happens before the first borrow
                    // of this update.
                    unsafe { *output.get() = value }
                    *output_version = source_version;
                }
                output
            }
            Vacant(entry) => {
                let value = (&mut *self.r.map.borrow_mut())(self.r.source.get(vars));
                let (output, _) = entry.insert((UnsafeCell::new(value), source_version));
                output
            }
        };

        // SAFETY:
        // This is safe because a mutable reference to `Vars` is required
        // for changing values.
        unsafe { &*output.get() }
    }
}

impl<T, S, O, M> protected::Var<O> for MapContextVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, O> {
        protected::BindInfo::Var(self.borrow(vars), self.r.source.is_new(vars), self.r.source.version(vars))
    }
}

impl<T, S, O, M> ObjVar<O> for MapContextVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
        self.borrow(vars)
    }

    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
        if self.is_new(vars) {
            Some(self.borrow(vars))
        } else {
            None
        }
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.r.source.is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        self.r.source.version(vars)
    }
}

impl<T, S, O, M> Clone for MapContextVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    fn clone(&self) -> Self {
        MapContextVar { r: Rc::clone(&self.r) }
    }
}

impl<T, S, O, M> Var<O> for MapContextVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    type AsReadOnly = Self;
    type AsLocal = CloningLocalVar<O, Self>;

    fn map<O2, M2>(&self, _map: M2) -> MapVar<O, Self, O2, M2>
    where
        O2: VarValue,
        M2: FnMut(&O) -> O2,
    {
        todo!("when GATs are stable")
    }

    fn map_bidi<O2, M2, N2>(&self, _map: M2, _map_back: N2) -> MapVarBiDi<O, Self, O2, M2, N2>
    where
        O2: VarValue,
        M2: FnMut(&O) -> O2,
        N2: FnMut(&O2) -> O,
    {
        todo!("when GATs are stable")
    }

    fn as_read_only(self) -> Self {
        self
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }
}

impl<T, S, O, M> IntoVar<O> for MapContextVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T, S, O, M> IntoVar<O> for MapVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __context_var {
    ($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = const $default:expr;) => {
        __context_var!(gen => $(#[$outer])* $vis struct $ident: $type = {

            static DEFAULT: $type = $default;
            &DEFAULT

        };);
    };

    ($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = once $default:expr;) => {
        __context_var!(gen => $(#[$outer])* $vis struct $ident: $type = {

            static DEFAULT: once_cell::sync::OnceCell<$type> = once_cell::sync::OnceCell::new();
            DEFAULT.get_or_init(||{
                $default
            })

        };);
    };

    (gen => $(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = $DEFAULT:expr;) => {
        $(#[$outer])*
        /// # ContextVar
        /// This `struct` is a [`ContextVar`](zero_ui::core::var::ContextVar).
        #[derive(Clone, Copy)]
        $vis struct $ident;

        impl $ident {
            /// Context var as [`Var`](zero_ui::core::var::Var).
            #[inline]
            pub fn as_var(self) -> $crate::core::var::ContextVarImpl<Self> {
                $crate::core::var::ContextVarImpl::<Self>::default()
            }

            /// [`Var`](zero_ui::core::var::Var) that represents this context var.
            #[inline]
            pub fn var() -> $crate::core::var::ContextVarImpl<Self> {
                Self.as_var()
            }
        }

        impl $crate::core::var::ContextVar for $ident {
            type Type = $type;

            fn default() -> &'static Self::Type {
               $DEFAULT
            }
        }

        impl $crate::core::var::IntoVar<$type> for $ident {
            type Var = $crate::core::var::ContextVarImpl<Self>;
            #[inline]
            fn into_var(self) -> Self::Var {
                self.as_var()
            }
        }
    };
}

/// Declares new [`ContextVar`](crate::core::context::ContextVar) types.
///
/// # Examples
/// ```
/// # #[macro_use] extern crate zero_ui;
/// # fn main() {
/// # #[derive(Debug, Clone)]
/// # struct NotConst(u8);
/// # fn init_val() -> NotConst { NotConst(10) }
/// #
/// context_var! {
///     /// A public documented property with default value initialization that is `const`.
///     /// Will use a static variable.
///     pub struct Property1: u8 = const 10;
///
///     // A private property with default value that is not `const`. Will evaluate
///     // and cache the default on the first usage.
///     struct Property2: NotConst = once init_val();
/// }
/// # }
/// ```
#[macro_export]
macro_rules! context_var {
    ($($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = $mode:ident $default:expr;)+) => {$(
        __context_var!($(#[$outer])* $vis struct $ident: $type = $mode $default;);
    )+};
}
