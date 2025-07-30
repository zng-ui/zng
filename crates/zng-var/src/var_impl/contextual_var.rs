//! Context dependent unwrapping mapping var

use std::{
    any::Any,
    fmt,
    sync::{Arc, Weak},
};

use parking_lot::{Mutex, RwLock};
use smallbox::{SmallBox, smallbox};
use zng_app_context::context_local;

use crate::{AnyVar, Var, VarImpl, VarInstanceTag, VarValue, WeakVarImpl};

use super::VarCapability;

/// Create a type erased contextualized variable.
///
/// The `context_init` closure must produce variables of the same value type.
///
/// See [`contextual_var`] for more details about contextualized variables.
pub fn any_contextual_var(context_init: impl FnMut() -> AnyVar + Send + 'static) -> AnyVar {
    any_contextual_var_impl(smallbox!(ContextInitFnMut(context_init)))
}
pub(super) fn any_contextual_var_impl(context_init: ContextInitFn) -> AnyVar {
    AnyVar(smallbox!(ContextualVar::new(context_init)))
}

/// Create a contextualized variable.
///
/// This is useful for declaring variables that depend on the contextual state on first usage to actually determinate the value.
///
/// # Examples
///
/// Basic usage:
///
/// ```rust
/// # macro_rules! fake{($($tt:tt)*) => {}}
/// # fake! {
/// widget_set! {
///     self;
///     padding = contextual_var(|| WINDOW.vars().safe_padding().map(|p| SideOffsets::from(*p)));
/// };
/// # }
/// ```
///
/// The example above shows the declaration of a default widget property `padding` that depends on the contextual `WINDOW.vars` value.
/// When the padding property reads the variable for the first time (on `UiNode::init`) the contextual may be different from the
/// declaration, so closure will eval to produce a contextualized inner variable. If the widget is moved to another window the closure
/// will be called again to get a new contextualized inner variable.
///
/// This variable is for advanced usage like this, where you need a contextual value and there is no *CONTEXT_VAR* that provides the value.
/// Note that you **do not need this** to contextualize context vars, they already are context aware.
///
/// # Capabilities
///
/// When the returned variable is used in a new context for the first time the `context_init` closure is called
/// to produce the actual variable in that context.
///
/// If a clone of the returned variable is moved to another context the `context_init` closure is called again
/// to init that clone.
///
/// If the return variable is *mapped* the mapping var is also context aware and will also delay init until first usage.
pub fn contextual_var<T: VarValue>(mut context_init: impl FnMut() -> Var<T> + Send + 'static) -> Var<T> {
    Var::new_any(any_contextual_var(move || context_init().into()))
}

pub(super) type ContextInitFn = SmallBox<dyn ContextInitFnImpl, smallbox::space::S8>;
// not using a FnMut here so that the source can be inspected (WhenVarBuilder does this)
pub(crate) trait ContextInitFnImpl: Send + Any {
    fn init(&mut self) -> AnyVar;
}
struct ContextInitFnMut<F>(F);
impl<F: FnMut() -> AnyVar + Send + 'static> ContextInitFnImpl for ContextInitFnMut<F> {
    fn init(&mut self) -> AnyVar {
        self.0()
    }
}

pub(crate) struct ContextualVar {
    pub(super) init: Arc<Mutex<ContextInitFn>>,
    ctx: RwLock<(AnyVar, ContextInitHandle)>,
}
impl Clone for ContextualVar {
    fn clone(&self) -> Self {
        Self {
            init: self.init.clone(),
            ctx: RwLock::new((no_ctx_var(), ContextInitHandle::no_context())),
        }
    }
}
impl ContextualVar {
    pub fn new(init: ContextInitFn) -> Self {
        ContextualVar {
            init: Arc::new(Mutex::new(init)),
            ctx: RwLock::new((no_ctx_var(), ContextInitHandle::no_context())),
        }
    }

    fn load(&self) -> parking_lot::MappedRwLockReadGuard<AnyVar> {
        let ctx = self.ctx.read();
        let id = ContextInitHandle::current();
        if ctx.1 == id {
            parking_lot::RwLockReadGuard::map(ctx, |f| &f.0)
        } else {
            drop(ctx);
            let mut ctx = self.ctx.write();
            if ctx.1 != id {
                ctx.0 = self.init.lock().init();
            }
            let ctx = parking_lot::RwLockWriteGuard::downgrade(ctx);
            parking_lot::RwLockReadGuard::map(ctx, |f| &f.0)
        }
    }
}
impl VarImpl for ContextualVar {
    fn clone_boxed(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        smallbox!(self.clone())
    }

    fn current_context(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        self.load().0.clone_boxed()
    }

    fn value_type(&self) -> std::any::TypeId {
        self.load().0.value_type()
    }

    #[cfg(feature = "value_type_name")]
    fn value_type_name(&self) -> &'static str {
        self.load().0.value_type_name()
    }

    fn strong_count(&self) -> usize {
        self.load().0.strong_count()
    }

    fn var_eq(&self, other: &dyn std::any::Any) -> bool {
        match other.downcast_ref::<Self>() {
            Some(o) => {
                Arc::ptr_eq(&self.init, &o.init) && {
                    let a = self.ctx.read_recursive();
                    let b = o.ctx.read_recursive();
                    a.1 == b.1 && a.0.var_eq(&b.0)
                }
            }
            None => false,
        }
    }

    fn var_instance_tag(&self) -> VarInstanceTag {
        self.load().0.var_instance_tag()
    }

    fn downgrade(&self) -> SmallBox<dyn super::WeakVarImpl, smallbox::space::S2> {
        smallbox!(WeakContextualVar {
            init: Arc::downgrade(&self.init)
        })
    }

    fn capabilities(&self) -> VarCapability {
        self.load().0.capabilities() | VarCapability::CONTEXT
    }

    fn with(&self, visitor: &mut dyn FnMut(&dyn crate::AnyVarValue)) {
        self.load().0.with(visitor);
    }

    fn get(&self) -> crate::BoxAnyVarValue {
        self.load().0.get()
    }

    fn set(&self, new_value: crate::BoxAnyVarValue) -> bool {
        self.load().0.set(new_value)
    }

    fn update(&self) -> bool {
        self.load().0.update()
    }

    fn modify(&self, modify: SmallBox<dyn FnMut(&mut super::AnyVarModify) + Send + 'static, smallbox::space::S4>) -> bool {
        self.load().0.modify(modify)
    }

    fn hook(&self, on_new: SmallBox<dyn FnMut(&crate::AnyVarHookArgs) -> bool + Send + 'static, smallbox::space::S4>) -> super::VarHandle {
        self.load().0.hook(on_new)
    }

    fn last_update(&self) -> crate::VarUpdateId {
        self.load().0.last_update()
    }

    fn modify_importance(&self) -> usize {
        self.load().0.modify_importance()
    }

    fn is_animating(&self) -> bool {
        self.load().0.is_animating()
    }

    fn hook_animation_stop(&self, handler: crate::animation::AnimationStopFn) -> Result<(), crate::animation::AnimationStopFn> {
        self.load().0.hook_animation_stop(handler)
    }
}

#[derive(Clone)]
struct WeakContextualVar {
    init: Weak<Mutex<ContextInitFn>>,
}
impl WeakVarImpl for WeakContextualVar {
    fn clone_boxed(&self) -> SmallBox<dyn WeakVarImpl, smallbox::space::S2> {
        smallbox!(self.clone())
    }

    fn strong_count(&self) -> usize {
        self.init.strong_count()
    }

    fn upgrade(&self) -> Option<SmallBox<dyn VarImpl, smallbox::space::S2>> {
        match self.init.upgrade() {
            Some(init) => Some(smallbox!(ContextualVar {
                init,
                ctx: RwLock::new((no_ctx_var(), ContextInitHandle::no_context()))
            })),
            None => None,
        }
    }
}

fn no_ctx_var() -> AnyVar {
    crate::const_var(()).into()
}

#[derive(Default)]
struct ContextInitHandleMarker;

/// Identifies the unique context a [`contextual_var`] is in.
///
/// Each node that sets context-vars have an unique ID, it is different after each (re)init. The contextual var
/// records this ID, and rebuilds when it has changed. The contextualized inner vars are retained locally to the clone
/// of the contextual var.
#[derive(Clone, Default)]
pub struct ContextInitHandle(Option<Arc<ContextInitHandleMarker>>);
context_local! {
    static CONTEXT_INIT_ID: ContextInitHandleMarker = ContextInitHandleMarker;
}
impl ContextInitHandle {
    /// Generates a new unique handle.
    pub fn new() -> Self {
        Self(Some(Arc::new(ContextInitHandleMarker)))
    }

    /// Identifies the state before first contextualization.
    ///
    /// This is the default value.
    pub fn no_context() -> Self {
        Self::default()
    }

    /// Gets the current context handle.
    ///
    /// # Panics
    ///
    /// Panics is not called in an app context at least, never returns [`no_context`].
    ///
    /// [`no_context`]: Self::no_context
    pub fn current() -> Self {
        Self(Some(CONTEXT_INIT_ID.get()))
    }

    /// Handle represents the state before first contextualization.
    pub fn is_no_context(&self) -> bool {
        self.0.is_none()
    }

    /// Runs `action` with `self` as the current context ID.
    ///
    /// Note that [`ContextVar::with_context`] already calls this method.
    ///
    /// # Panics
    ///
    /// Panics if the handle [`is_no_context`].
    ///
    /// [`is_no_context`]: Self::is_no_context
    pub fn with_context<R>(&self, action: impl FnOnce() -> R) -> R {
        let mut opt = self.0.clone();
        CONTEXT_INIT_ID.with_context(&mut opt, action)
    }

    /// Create a weak handle that can be used to monitor this handle without holding it.
    pub fn downgrade(&self) -> WeakContextInitHandle {
        match &self.0 {
            Some(a) => WeakContextInitHandle(Arc::downgrade(a)),
            None => WeakContextInitHandle(std::sync::Weak::new()),
        }
    }
}
impl fmt::Debug for ContextInitHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ContextInitHandle").finish_non_exhaustive()
    }
}
impl PartialEq for ContextInitHandle {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }
}
impl Eq for ContextInitHandle {}

/// Weak [`ContextInitHandle`].
#[derive(Clone, Default)]
pub struct WeakContextInitHandle(std::sync::Weak<ContextInitHandleMarker>);
impl WeakContextInitHandle {
    /// Returns `true` if the strong handle still exists.
    pub fn is_alive(&self) -> bool {
        self.0.strong_count() > 0
    }
}
impl fmt::Debug for WeakContextInitHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WeakContextInitHandle")
            .field(&std::sync::Weak::as_ptr(&self.0))
            .finish()
    }
}
impl PartialEq for WeakContextInitHandle {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Weak::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for WeakContextInitHandle {}
impl std::hash::Hash for WeakContextInitHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let i = std::sync::Weak::as_ptr(&self.0) as usize;
        std::hash::Hash::hash(&i, state)
    }
}
