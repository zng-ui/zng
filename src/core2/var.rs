use super::{AppContext, AppContextId, AppExtension, AppRegister, EventContext, Service};
use std::any::type_name;
use std::cell::{Cell, RefCell, UnsafeCell};
use std::rc::{Rc, Weak};

/// A variable value that is set by the ancestors of an UiNode.
pub trait ContextVar: Clone + Copy + 'static {
    /// The variable type.
    type Type: 'static;

    /// Default value, used when the variable is not set in the context.
    fn default() -> &'static Self::Type;
}

/// A variable value that is set by the previously visited UiNodes during the call.
pub trait VisitedVar: 'static {
    /// The variable type.
    type Type: 'static;
}

pub(crate) mod protected {
    use super::AppContext;
    use std::any::TypeId;

    /// Infor for context var binding.
    pub enum BindInfo<'a, T: 'static> {
        /// Owned or SharedVar.
        ///
        /// * `&'a T` is a reference to the value borrowed in the context.
        /// * `bool` is the is_new flag.
        Var(&'a T, bool),
        /// ContextVar.
        ///
        /// * `TypeId` of self.
        /// * `&'static T` is the ContextVar::default value of self.
        ContextVar(TypeId, &'static T),
    }

    /// pub(crate) part of Var.
    pub trait Var<T: 'static> {
        fn bind_info<'a, 'b>(&'a self, ctx: &'b AppContext) -> BindInfo<'a, T>;
    }
}

/// Part of [Var] that can be boxed.
pub trait SizedVar<T: 'static>: protected::Var<T> + 'static {
    /// The current value.
    fn get<'a>(&'a self, ctx: &'a AppContext) -> &'a T;

    /// [get] if [is_new] or none.
    fn update<'a>(&'a self, ctx: &'a AppContext) -> Option<&'a T>;

    /// If the value changed this update.
    fn is_new(&self, ctx: &AppContext) -> bool;

    /// Box the variable. This disables mapping.
    fn into_box(self) -> BoxVar<T>
    where
        Self: std::marker::Sized,
    {
        Box::new(self)
    }
}

/// Boxed [Var].
pub type BoxVar<T> = Box<dyn SizedVar<T>>;

/// Abstraction over [ContextVar], [SharedVar] or [OwnedVar].
///
/// This is the complete generic trait, the non-generic methods are defined in [SizedVar]
/// to support boxing.
///
/// Cannot be implemented outside of zero-ui crate. Use this together with [IntoVar] to
/// support dinamic values in property definitions.
pub trait Var<T: 'static>: SizedVar<T> {
    fn map<O: 'static, M: FnMut(&T) -> O + 'static>(&self, ctx: &AppContext, f: M) -> MapVar<T, O, M, Self>
    where
        Self: Sized;

    //TODO merge, switch
}

impl<T: 'static, V: ContextVar<Type = T>> protected::Var<T> for V {
    fn bind_info<'a, 'b>(&'a self, _: &'b AppContext) -> protected::BindInfo<'a, T> {
        protected::BindInfo::ContextVar(std::any::TypeId::of::<V>(), V::default())
    }
}

impl<T: 'static, V: ContextVar<Type = T>> SizedVar<T> for V {
    fn get<'a>(&'a self, ctx: &'a AppContext) -> &'a T {
        ctx.get::<V>()
    }

    fn update<'a>(&'a self, ctx: &'a AppContext) -> Option<&'a T> {
        ctx.get_new::<V>()
    }

    fn is_new(&self, ctx: &AppContext) -> bool {
        ctx.get_is_new::<V>()
    }
}

impl<T: 'static, V: ContextVar<Type = T>> Var<T> for V {
    fn map<O: 'static, M: FnMut(&T) -> O + 'static>(&self, ctx: &AppContext, mut f: M) -> MapVar<T, O, M, Self> {
        MapVar::new(self.clone(), f, ctx)
    }
}

/// [Var] implementer that owns the value.
pub struct OwnedVar<T: 'static>(pub T);

impl<T: 'static> protected::Var<T> for OwnedVar<T> {
    fn bind_info<'a, 'b>(&'a self, _: &'b AppContext) -> protected::BindInfo<'a, T> {
        protected::BindInfo::Var(&self.0, false)
    }
}

impl<T: 'static> SizedVar<T> for OwnedVar<T> {
    fn get(&self, _: &AppContext) -> &T {
        &self.0
    }

    fn update<'a>(&'a self, _: &'a AppContext) -> Option<&'a T> {
        None
    }

    fn is_new(&self, _: &AppContext) -> bool {
        false
    }
}

impl<T: 'static> Var<T> for OwnedVar<T> {
    fn map<O: 'static, M: FnMut(&T) -> O + 'static>(&self, ctx: &AppContext, mut f: M) -> MapVar<T, O, M, Self> {
        MapVar::owned(f(self.get(ctx)))
    }
}

struct SharedVarInner<T> {
    data: UnsafeCell<T>,
    borrowed: Cell<Option<AppContextId>>,
    is_new: Cell<bool>,
}

/// [Var] Rc implementer.
pub struct SharedVar<T: 'static> {
    r: Rc<SharedVarInner<T>>,
}

impl<T: 'static> SharedVar<T> {
    pub(crate) fn modify(
        self,
        mut_ctx_id: AppContextId,
        modify: impl FnOnce(&mut T) + 'static,
        cleanup: &mut Vec<Box<dyn FnOnce()>>,
    ) {
        if let Some(ctx_id) = self.r.borrowed.get() {
            if ctx_id != mut_ctx_id {
                panic!(
                    "cannot set `SharedVar<{}>` because it is borrowed in a different context",
                    type_name::<T>()
                )
            }
            self.r.borrowed.set(None);
        }

        // SAFETY: This is safe because borrows are bound to a context that
        // is the only place where the value can be changed and this change is
        // only applied when the context is mut.
        modify(unsafe { &mut *self.r.data.get() });

        cleanup.push(Box::new(move || self.r.is_new.set(false)));
    }

    fn borrow(&self, ctx_id: AppContextId) -> &T {
        if let Some(borrowed_id) = self.r.borrowed.get() {
            if ctx_id != borrowed_id {
                panic!(
                    "`SharedVar<{}>` is already borrowed in a different `AppContext`",
                    type_name::<T>()
                )
            }
        } else {
            self.r.borrowed.set(Some(ctx_id));
        }

        // SAFETY: This is safe because borrows are bound to a context that
        // is the only place where the value can be changed and this change is
        // only applied when the context is mut.
        unsafe { &*self.r.data.get() }
    }
}

impl<T: 'static> Clone for SharedVar<T> {
    fn clone(&self) -> Self {
        SharedVar { r: Rc::clone(&self.r) }
    }
}

impl<T: 'static> protected::Var<T> for SharedVar<T> {
    fn bind_info<'a, 'b>(&'a self, ctx: &'b AppContext) -> protected::BindInfo<'a, T> {
        protected::BindInfo::Var(self.borrow(ctx.id()), self.r.is_new.get())
    }
}

impl<T: 'static> SizedVar<T> for SharedVar<T> {
    fn get(&self, ctx: &AppContext) -> &T {
        self.borrow(ctx.id())
    }

    fn update<'a>(&'a self, ctx: &'a AppContext) -> Option<&'a T> {
        if self.r.is_new.get() {
            Some(self.get(ctx))
        } else {
            None
        }
    }

    fn is_new(&self, _: &AppContext) -> bool {
        self.r.is_new.get()
    }
}

impl<T: 'static> Var<T> for SharedVar<T> {
    fn map<O: 'static, M: FnMut(&T) -> O + 'static>(&self, ctx: &AppContext, f: M) -> MapVar<T, O, M, Self> {
        MapVar::new(self.clone(), f, ctx)
    }
}

enum MapVarInner<O, M, S> {
    Owned(O),
    Full(S, M, Cell<O>),
}

/// [Var] that maps other vars.
pub struct MapVar<T: 'static, O: 'static, M: FnMut(&T) -> O + 'static, S: SizedVar<T>> {
    _t: std::marker::PhantomData<T>,
    r: Rc<MapVarInner<O, M, S>>,
}

impl<T: 'static, O: 'static, M: FnMut(&T) -> O + 'static, S: SizedVar<T>> MapVar<T, O, M, S> {
    fn owned(output: O) -> Self {
        MapVar {
            _t: std::marker::PhantomData,
            r: Rc::new(MapVarInner::Owned(output)),
        }
    }

    fn new(source: S, mut map: M, ctx: &AppContext) -> Self {
        let output = map(source.get(ctx));
        let inner = Rc::new(MapVarInner::Full(source, map, Cell::new(output)));
        ctx.service::<MapVarUpdate>().register(Rc::downgrade(&inner));
        MapVar {
            _t: std::marker::PhantomData,
            r: inner,
        }
    }
}

impl<T: 'static, O: 'static, M: FnMut(&T) -> O + 'static, S: SizedVar<T>> protected::Var<O> for MapVar<T, O, M, S> {
    fn bind_info<'a, 'b>(&'a self, ctx: &'b AppContext) -> protected::BindInfo<'a, O> {
        match &*self.r {
            MapVarInner::Owned(o) => protected::BindInfo::Var(o, false),
            MapVarInner::Full(_, _, o) => todo!(),
        }
    }
}

impl<T: 'static, O: 'static, M: FnMut(&T) -> O + 'static, S: SizedVar<T>> SizedVar<O> for MapVar<T, O, M, S> {
    fn get<'a>(&'a self, ctx: &'a AppContext) -> &'a O {
        match &*self.r {
            MapVarInner::Owned(o) => o,
            MapVarInner::Full(_, _, o) => todo!(),
        }
    }

    fn update<'a>(&'a self, ctx: &'a AppContext) -> Option<&'a O> {
        match &*self.r {
            MapVarInner::Owned(o) => None,
            MapVarInner::Full(_, _, o) => todo!(),
        }
    }

    fn is_new(&self, ctx: &AppContext) -> bool {
        match &*self.r {
            MapVarInner::Owned(_) => false,
            MapVarInner::Full(source, _, _) => source.is_new(ctx),
        }
    }
}

impl<T: 'static, O: 'static, M: FnMut(&T) -> O + 'static, S: SizedVar<T>> Clone for MapVar<T, O, M, S> {
    fn clone(&self) -> Self {
        MapVar {
            _t: std::marker::PhantomData,
            r: Rc::clone(&self.r),
        }
    }
}

impl<T: 'static, O: 'static, M: FnMut(&T) -> O + 'static, S: SizedVar<T>> Var<O> for MapVar<T, O, M, S> {
    fn map<O2: 'static, M2: FnMut(&O) -> O2 + 'static>(&self, ctx: &AppContext, mut f: M2) -> MapVar<O, O2, M2, Self> {
        match &*self.r {
            MapVarInner::Owned(o) => MapVar::owned(f(o)),
            MapVarInner::Full(_, _, o) => MapVar::new(self.clone(), f, ctx),
        }
    }
}

/// Updates the MapVar is required, returns if should retain the eval function.
type MapVarEval = Box<dyn Fn(&AppContext) -> bool>;

/// [MapVar] management app extension.
#[derive(Default, Clone)]
pub(crate) struct MapVarUpdate {
    r: Rc<RefCell<Vec<MapVarEval>>>,
}

impl AppExtension for MapVarUpdate {
    fn register(&mut self, r: &mut AppRegister) {
        r.register_service(self.clone());
    }

    fn respond(&mut self, r: &mut EventContext) {
        self.r.borrow_mut().retain(|f| f(r.app_ctx()));
    }
}

impl Service for MapVarUpdate {}

impl MapVarUpdate {
    fn register<T: 'static, O: 'static, M: FnMut(&T) -> O + 'static, S: SizedVar<T>>(
        &self,
        map: Weak<MapVarInner<O, M, S>>,
    ) {
        let eval = move |ctx: &AppContext| {
            if let Some(map) = map.upgrade() {
                match &*map {
                    MapVarInner::Full(source, map, output) => {
                        if let Some(source) = source.update(ctx) {
                            todo!()
                        }
                    }
                    _ => unreachable!(),
                }
                true
            } else {
                false
            }
        };

        self.r.borrow_mut().push(Box::new(eval));
    }
}

pub trait IntoVar<T: 'static> {
    type Var: Var<T> + 'static;

    fn into_var(self) -> Self::Var;
}

/// Does nothing. `[Var]<T>` already implements `Value<T>`.
impl<T: 'static> IntoVar<T> for SharedVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

/// Wraps the value in an `[Owned]<T>` value.
impl<T: 'static> IntoVar<T> for T {
    type Var = OwnedVar<T>;

    fn into_var(self) -> OwnedVar<T> {
        OwnedVar(self)
    }
}
