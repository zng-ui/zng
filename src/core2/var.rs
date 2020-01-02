use super::{AppContext, AppContextId};
use std::any::type_name;
use std::cell::{Cell, RefCell, UnsafeCell};
use std::rc::Rc;

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
        Var(&'a T, bool, u32),
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

    /// Current value version. Version changes every time the value changes.
    fn version(&self, ctx: &AppContext) -> u32;

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

    fn version(&self, ctx: &AppContext) -> u32 {
        ctx.get_version::<V>()
    }
}

impl<T: 'static, V: ContextVar<Type = T>> Var<T> for V {
    fn map<O: 'static, M: FnMut(&T) -> O + 'static>(&self, ctx: &AppContext, f: M) -> MapVar<T, O, M, Self> {
        MapVar::new(*self, f, ctx)
    }
}

/// [Var] implementer that owns the value.
pub struct OwnedVar<T: 'static>(pub T);

impl<T: 'static> protected::Var<T> for OwnedVar<T> {
    fn bind_info<'a, 'b>(&'a self, _: &'b AppContext) -> protected::BindInfo<'a, T> {
        protected::BindInfo::Var(&self.0, false, 0)
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

    fn version(&self, _: &AppContext) -> u32 {
        0
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
    version: Cell<u32>,
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
        self.r.version.set(self.r.version.get().wrapping_add(1));

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
        protected::BindInfo::Var(self.borrow(ctx.id()), self.r.is_new.get(), self.r.version.get())
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

    fn version(&self, _: &AppContext) -> u32 {
        self.r.version.get()
    }
}

impl<T: 'static> Var<T> for SharedVar<T> {
    fn map<O: 'static, M: FnMut(&T) -> O + 'static>(&self, ctx: &AppContext, f: M) -> MapVar<T, O, M, Self> {
        MapVar::new(self.clone(), f, ctx)
    }
}

struct MapVarSource<M, S> {
    is_contextual: bool,
    source: S,
    map: RefCell<M>,
    borrowed: Cell<Option<AppContextId>>,
    output_version: Cell<u32>,
}

struct MapVarData<O, M, S> {
    source: Option<Box<MapVarSource<M, S>>>,
    output: UnsafeCell<O>,
}

/// [Var] that maps other vars.
pub struct MapVar<T: 'static, O: 'static, M: FnMut(&T) -> O + 'static, S: SizedVar<T>> {
    _t: std::marker::PhantomData<T>,
    r: Rc<MapVarData<O, M, S>>,
}

impl<T: 'static, O: 'static, M: FnMut(&T) -> O + 'static, S: SizedVar<T>> MapVar<T, O, M, S> {
    fn owned(output: O) -> Self {
        MapVar {
            _t: std::marker::PhantomData,
            r: Rc::new(MapVarData {
                source: None,
                output: UnsafeCell::new(output),
            }),
        }
    }

    fn new(source: S, mut map: M, ctx: &AppContext) -> Self {
        let output = UnsafeCell::new(map(source.get(ctx)));
        let output_version = Cell::new(source.version(ctx));
        let map = RefCell::new(map);
        let inner = Rc::new(MapVarData {
            source: Some(Box::new(MapVarSource {
                is_contextual: false,
                source,
                map,
                borrowed: Cell::default(),
                output_version,
            })),
            output,
        });
        MapVar {
            _t: std::marker::PhantomData,
            r: inner,
        }
    }

    fn borrow(&self, ctx: &AppContext) -> &O {
        if let Some(s) = &self.r.source {
            // 1 - borrow
            let ctx_id = ctx.id();
            if let Some(borrowed_id) = s.borrowed.get() {
                if ctx_id != borrowed_id {
                    panic!(
                        "`MapVar<{}>` is already borrowed in a different `AppContext`",
                        type_name::<T>()
                    )
                }
            } else {
                s.borrowed.set(Some(ctx_id));
            }

            // 2 - update output
            let source_version = s.source.version(ctx);
            if s.output_version.get() != source_version {
                let value = (&mut *s.map.borrow_mut())(s.source.get(ctx));
                unsafe { *self.r.output.get() = value }
                s.output_version.set(source_version);
            }
        }

        // SAFETY:
        // If we don't have a source we own the output and it is imutable.
        // If we have a source borrow validation was done in the `if` above.
        unsafe { &*self.r.output.get() }
    }
}

impl<T: 'static, O: 'static, M: FnMut(&T) -> O + 'static, S: SizedVar<T>> protected::Var<O> for MapVar<T, O, M, S> {
    fn bind_info<'a, 'b>(&'a self, ctx: &'b AppContext) -> protected::BindInfo<'a, O> {
        if let Some(s) = &self.r.source {
            if s.is_contextual {
                todo!()
            } else {
                protected::BindInfo::Var(self.borrow(ctx), s.source.is_new(ctx), s.source.version(ctx))
            }
        } else {
            // SAFETY: safe because without a source we are owned.
            protected::BindInfo::Var(unsafe { &*self.r.output.get() }, false, 0)
        }
    }
}

impl<T: 'static, O: 'static, M: FnMut(&T) -> O + 'static, S: SizedVar<T>> SizedVar<O> for MapVar<T, O, M, S> {
    fn get<'a>(&'a self, ctx: &'a AppContext) -> &'a O {
        self.borrow(ctx)
    }

    fn update<'a>(&'a self, ctx: &'a AppContext) -> Option<&'a O> {
        if self.is_new(ctx) {
            Some(self.borrow(ctx))
        } else {
            None
        }
    }

    fn is_new(&self, ctx: &AppContext) -> bool {
        if let Some(s) = &self.r.source {
            s.source.is_new(ctx)
        } else {
            false
        }
    }

    fn version(&self, ctx: &AppContext) -> u32 {
        if let Some(s) = &self.r.source {
            s.source.version(ctx)
        } else {
            0
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
        if let Some(s) = &self.r.source {
            if s.is_contextual {
                todo!()
            } else {
                MapVar::new(self.clone(), f, ctx)
            }
        } else {
            // SAFETY: safe because without a source we are owned.
            MapVar::owned(f(unsafe { &*self.r.output.get() }))
        }
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
