use std::{marker::PhantomData, thread::LocalKey};

use super::*;

/// A [`Var`] that represents a [`ContextVar`].
///
/// Context var types don't implement [`Var`] directly, to avoid problems with overlapping generics
/// this *proxy* zero-sized type is used.
///
/// The context var can have different values when read in different contexts, also the [`VarVersion`] is always different
/// in different contexts. Context vars are mostly read-only but can be settable if bound to a read/write variable.
#[derive(Clone)]
pub struct ContextVarProxy<C: ContextVar>(PhantomData<C>);
impl<C: ContextVar> ContextVarProxy<C> {
    /// New context var proxy.
    ///
    /// Prefer using [`ContextVar::new`] or the `new` generated using the [`context_var!`] macro.
    #[inline]
    pub fn new() -> Self {
        ContextVarProxy(PhantomData)
    }

    #[doc(hidden)]
    #[inline]
    pub fn static_ref() -> &'static Self {
        &ContextVarProxy(PhantomData)
    }
}

impl<C: ContextVar> Default for ContextVarProxy<C> {
    fn default() -> Self {
        ContextVarProxy(PhantomData)
    }
}

impl<C: ContextVar> crate::private::Sealed for ContextVarProxy<C> {}
impl<C: ContextVar> Var<C::Type> for ContextVarProxy<C> {
    type AsReadOnly = Self;

    #[inline]
    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a C::Type {
        let _vars = vars.as_ref();
        let ptr = C::thread_local_value().value();
        // SAFETY: this is safe because the pointer is either 'static or a reference held by
        // Vars::with_context_var.
        unsafe { &*ptr }
    }

    #[inline]
    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a C::Type> {
        let vars = vars.as_ref();
        let key = C::thread_local_value();
        if key.is_new() {
            Some(self.get(vars))
        } else {
            None
        }
    }

    #[inline]
    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|_v| C::thread_local_value().is_new())
    }

    #[inline]
    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> C::Type {
        self.get_clone(vars)
    }

    #[inline]
    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> VarVersion {
        vars.with_vars_read(|_v| C::thread_local_value().version())
    }

    #[inline]
    fn is_read_only<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|_v| C::thread_local_value().is_read_only())
    }

    #[inline]
    fn is_animating<Vr: WithVarsRead>(&self, vars: &Vr) -> bool {
        vars.with_vars_read(|_v| C::thread_local_value().is_animating())
    }

    #[inline]
    fn always_read_only(&self) -> bool {
        false
    }

    #[inline]
    fn is_contextual(&self) -> bool {
        true
    }

    #[inline]
    fn can_update(&self) -> bool {
        true
    }

    #[inline]
    fn strong_count(&self) -> usize {
        0
    }

    #[inline]
    fn modify<Vw, M>(&self, vars: &Vw, modify: M) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        M: FnOnce(VarModify<C::Type>) + 'static,
    {
        vars.with_vars(|vars| {
            if let Some(actual) = C::thread_local_value().actual_var() {
                // SAFETY, we hold a ref to this closure box in the context.
                let actual = unsafe { &*actual };
                actual(vars).modify(vars, modify)
            } else {
                Err(VarIsReadOnly)
            }
        })
    }

    #[inline]
    fn into_read_only(self) -> Self::AsReadOnly {
        self
    }

    #[inline]
    fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
        vars.with_vars_read(|_v| C::thread_local_value().update_mask())
    }

    type Weak = NoneWeakVar<C::Type>;

    #[inline]
    fn is_rc(&self) -> bool {
        false
    }

    #[inline]
    fn downgrade(&self) -> Option<Self::Weak> {
        None
    }

    #[inline]
    fn weak_count(&self) -> usize {
        0
    }

    #[inline]
    fn as_ptr(&self) -> *const () {
        std::ptr::null()
    }

    #[inline]
    fn actual_var<Vw: WithVars>(&self, vars: &Vw) -> BoxedVar<C::Type> {
        vars.with_vars(|vars| {
            if let Some(actual) = C::thread_local_value().actual_var() {
                // SAFETY, we hold a ref to this closure box in the context.
                let actual = unsafe { &*actual };
                actual(vars)
            } else {
                let value = self.get_clone(vars);
                LocalVar(value).boxed()
            }
        })
    }
}

impl<C: ContextVar> IntoVar<C::Type> for ContextVarProxy<C> {
    type Var = Self;

    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}

/// Value bound to a [`ContextVar`] at a context.
///
/// # Examples
///
/// The example manually sets a context flag for all descendants of a node.
///
/// ```
/// use zero_ui_core::{*, var::*, context::*};
///
/// context_var! {
///     /// Foo context variable.
///     pub struct FooVar: bool = false;
/// }
///
/// struct FooNode<C> { child: C }
///
/// #[impl_ui_node(child)]
/// impl<C: UiNode> UiNode for FooNode<C> {
///     fn init(&mut self, ctx: &mut WidgetContext) {
///         // `FooVar` is `true` inside `FooNode::init`.
///         ctx.vars.with_context_var(FooVar, ContextVarData::fixed(&true), || {
///             self.child.init(ctx);
///         })
///     }
/// }
/// ```
///
/// Note that this is the lowest level of [`ContextVar`] manipulation, usually you can just use the [`with_context_var`]
/// helper function to bind another variable to a context variable, internally these
/// functions use the [`ContextVarData::in_vars`] and [`ContextVarData::in_vars_read`] in all [`UiNode`] methods.
///
/// [`UiNode`]: crate::UiNode
pub struct ContextVarData<'a, T: VarValue> {
    /// Value for [`Var::get`].
    pub value: &'a T,
    /// Value for [`Var::is_new`].
    pub is_new: bool,
    /// Value for [`Var::version`].
    pub version: VarVersion,

    /// Value for [`Var::is_animating`].
    pub is_animating: bool,

    /// Value for [`Var::is_read_only`].
    ///
    /// If [`actual_var`] is `None` the context var is read-only, independent of this value.
    ///
    /// [`actual_var`]: Self::actual_var
    pub is_read_only: bool,

    /// Value for [`Var::update_mask`].
    ///
    /// Note that the node that owns `self` must not subscribe to this update mask, only
    /// inner nodes that subscribe to the context var mapped to `self` subscribe to this mask.
    pub update_mask: UpdateMask,

    /// Delegate for [`Var::modify`] and  [`Var::actual_var`].
    ///
    /// The context var [`Var::is_read_only`] if this is `None`, otherwise this closure is called when any of the
    /// assign, touch or modify methods are called and the operation applied to the variable.
    ///
    /// If this is `None`, [`Var::actual_var`] returns a [`LocalVar<T>`] clone of the value.
    pub actual_var: Option<Box<DynActualVarFn<T>>>,
}

impl<'a, T: VarValue> ContextVarData<'a, T> {
    /// Value that does not change or update.
    ///
    /// The context var is never new, the version is always zero and there is no update mask.
    pub fn fixed(value: &'a T) -> Self {
        Self {
            value,
            is_new: false,
            is_animating: false,
            version: VarVersion::normal(0),
            update_mask: UpdateMask::none(),
            is_read_only: true,
            actual_var: None,
        }
    }

    /// Binds the context var to another `var` in a read-write context.
    ///
    /// If the `var` is not read-only the context var can be set to modify it, or the [`Var::actual_var`] can
    /// be used to retrieve the `var` and then modify it later, you can avoid this by setting `force_read_only`.
    pub fn in_vars(vars: &'a Vars, var: &'a impl Var<T>, force_read_only: bool) -> Self {
        let is_read_only = force_read_only || var.is_read_only(vars);
        Self {
            value: var.get(vars),
            is_new: var.is_new(vars),
            is_animating: var.is_animating(vars),
            version: var.version(vars),
            update_mask: var.update_mask(vars),
            is_read_only,
            actual_var: if var.is_rc() || var.is_contextual() {
                Some(if is_read_only {
                    // ensure we stay read-only.
                    Box::new(clone_move!(var, |vars| var.actual_var(vars).into_read_only()))
                } else {
                    Box::new(clone_move!(var, |vars| var.actual_var(vars)))
                })
            } else {
                debug_assert!(is_read_only);
                None
            },
        }
    }

    /// Binds the context var to another `var` in a read-only context.
    ///
    /// Note that the API does not forbid using this in a full [`Vars`] context, but doing so is probably a logic
    /// error, [`Var::is_new`] is always `false`, and [`Var::actual_var`] always returns a [`LocalVar`]
    /// clone of the current value instead of `var`.
    pub fn in_vars_read(vars: &'a VarsRead, var: &'a impl Var<T>) -> Self {
        Self {
            value: var.get(vars),
            is_new: false,
            is_animating: var.is_animating(vars),
            version: var.version(vars),
            update_mask: var.update_mask(vars),
            is_read_only: true,
            actual_var: None,
        }
    }

    pub(crate) fn into_raw(self) -> ContextVarDataRaw<T> {
        ContextVarDataRaw {
            value: self.value,
            is_new: self.is_new,
            is_read_only: self.is_read_only,
            is_animating: self.is_animating,
            version: self.version,
            update_mask: self.update_mask,
            actual_var: self.actual_var.map(Box::into_raw),
        }
    }
}

pub(crate) struct ContextVarDataRaw<T: VarValue> {
    value: *const T,
    is_new: bool,
    version: VarVersion,
    is_read_only: bool,
    is_animating: bool,
    update_mask: UpdateMask,
    actual_var: Option<*mut DynActualVarFn<T>>,
}

/// See `ContextVar::thread_local_value`.
#[doc(hidden)]
pub struct ContextVarValue<V: ContextVar> {
    _var: PhantomData<V>,
    _default_value: Box<V::Type>,
    value: Cell<*const V::Type>,
    is_new: Cell<bool>,
    version: Cell<VarVersion>,
    is_read_only: Cell<bool>,
    is_animating: Cell<bool>,
    update_mask: Cell<UpdateMask>,
    actual_var: Cell<Option<*mut DynActualVarFn<V::Type>>>,
}

type DynActualVarFn<T> = dyn Fn(&Vars) -> BoxedVar<T>;

#[allow(missing_docs)]
impl<V: ContextVar> ContextVarValue<V> {
    #[inline]
    pub fn init() -> Self {
        let default_value = Box::new(V::default_value());
        ContextVarValue {
            _var: PhantomData,
            value: Cell::new(default_value.as_ref()),
            _default_value: default_value,

            is_new: Cell::new(false),
            version: Cell::new(VarVersion::normal(0)),
            is_read_only: Cell::new(true),
            is_animating: Cell::new(false),
            update_mask: Cell::new(UpdateMask::none()),
            actual_var: Cell::new(None),
        }
    }
}

/// See `ContextVar::thread_local_value`.
#[doc(hidden)]
pub struct ContextVarLocalKey<V: ContextVar> {
    local: &'static LocalKey<ContextVarValue<V>>,
}
#[allow(missing_docs)]
impl<V: ContextVar> ContextVarLocalKey<V> {
    #[inline]
    pub fn new(local: &'static LocalKey<ContextVarValue<V>>) -> Self {
        ContextVarLocalKey { local }
    }

    pub(super) fn value(&self) -> *const V::Type {
        self.local.with(|l| l.value.get())
    }

    pub(super) fn is_new(&self) -> bool {
        self.local.with(|l| l.is_new.get())
    }

    pub(super) fn version(&self) -> VarVersion {
        self.local.with(|l| l.version.get())
    }

    pub(super) fn update_mask(&self) -> UpdateMask {
        self.local.with(|l| l.update_mask.get())
    }

    pub(super) fn is_read_only(&self) -> bool {
        self.local.with(|l| l.is_read_only.get())
    }

    pub(super) fn is_animating(&self) -> bool {
        self.local.with(|l| l.is_animating.get())
    }

    pub(super) fn actual_var(&self) -> Option<*mut DynActualVarFn<V::Type>> {
        self.local.with(|l| l.actual_var.get())
    }

    pub(super) fn enter_context(&self, new: ContextVarDataRaw<V::Type>) -> ContextVarDataRaw<V::Type> {
        self.local.with(|l| ContextVarDataRaw {
            value: l.value.replace(new.value),
            is_new: l.is_new.replace(new.is_new),
            is_read_only: l.is_read_only.replace(new.is_read_only),
            is_animating: l.is_animating.replace(new.is_animating),
            version: l.version.replace(new.version),
            update_mask: l.update_mask.replace(new.update_mask),
            actual_var: l.actual_var.replace(new.actual_var),
        })
    }

    pub(super) fn exit_context(&self, prev: ContextVarDataRaw<V::Type>) {
        self.local.with(|l| {
            l.value.set(prev.value);
            l.is_new.set(prev.is_new);
            l.version.set(prev.version);
            l.is_read_only.set(prev.is_read_only);
            l.is_animating.set(prev.is_animating);
            l.update_mask.set(prev.update_mask);
            if let Some(m) = l.actual_var.replace(prev.actual_var) {
                let _ = unsafe { Box::from_raw(m) };
            }
        })
    }
}

///<span data-del-macro-root></span> Declares new [`ContextVar`](crate::var::ContextVar) types.
///
/// # Examples
/// ```
/// # use zero_ui_core::var::context_var;
/// # #[derive(Debug, Clone)]
/// # struct NotConst(u8);
/// # fn init_val() -> NotConst { NotConst(10) }
/// #
/// context_var! {
///     /// A public documented context var.
///     pub struct FooVar: u8 = 10;
///
///     // A private context var.
///     struct BarVar: NotConst = init_val();
/// }
/// ```
///
/// # Default Value
///
/// All context variable have a default fallback value that is used when the variable is not setted in the context.
///
/// The default value is instantiated once per app thread and is the value of the variable when it is not set in the context.
/// Other instances of the default value can be created by calls to [`ContextVar::default_value`], so the code after the `=` should
/// always generate a value equal to the first value generated.
///
/// # Naming Convention
///
/// It is recommended that the type name ends with the `Var` suffix.
#[macro_export]
macro_rules! context_var {
    ($($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = $default:expr;)+) => {$(
        $(#[$outer])*
        ///
        /// # ContextVar
        ///
        /// This `struct` is a [`ContextVar`](crate::var::ContextVar).
        #[derive(Debug, Clone, Copy)]
        $vis struct $ident;

        impl $ident {
            std::thread_local! {
                static THREAD_LOCAL_VALUE: $crate::var::ContextVarValue<$ident> = $crate::var::ContextVarValue::init();
            }

            /// [`Var`](crate::var::Var) implementer that represents this context var.
            #[inline]
            #[allow(unused)]
            pub fn new() -> $crate::var::ContextVarProxy<Self> {
                $crate::var::ContextVarProxy::new()
            }

            /// New default value.
            ///
            /// Returns a value that is equal to the variable value when it is not set in any context.
            #[inline]
            pub fn default_value() -> $type {
                $default
            }

            /// References the value in the current `vars` context.
            #[inline]
            #[allow(unused)]
            pub fn get<'a, Vr: AsRef<$crate::var::VarsRead>>(vars: &'a Vr) -> &'a $type {
                $crate::var::Var::get($crate::var::ContextVarProxy::<Self>::static_ref(), vars)
            }

            /// Returns a clone of the value in the current `vars` context.
            #[inline]
            #[allow(unused)]
            pub fn get_clone<Vr: $crate::var::WithVarsRead>(vars: &Vr) -> $type {
                $crate::var::Var::get_clone($crate::var::ContextVarProxy::<Self>::static_ref(), vars)
            }

            /// References the value in the current `vars` context if it is marked as new.
            #[inline]
            #[allow(unused)]
            pub fn get_new<'a, Vw: AsRef<$crate::var::Vars>>(vars: &'a Vw) -> Option<&'a $type> {
                $crate::var::Var::get_new($crate::var::ContextVarProxy::<Self>::static_ref(), vars)
            }

            /// Returns a clone of the value in the current `vars` context if it is marked as new.
            #[inline]
            #[allow(unused)]
            pub fn clone_new<Vw: $crate::var::WithVars>(vars: &Vw) -> Option<$type> {
                $crate::var::Var::clone_new($crate::var::ContextVarProxy::<Self>::static_ref(), vars)
            }

            // TODO generate copy and set_ne fns when https://github.com/rust-lang/rust/issues/48214 is stable

            /// If the value in the current `vars` context is marked as new.
            #[inline]
            #[allow(unused)]
            pub fn is_new<Vw: $crate::var::WithVars>(vars: &Vw) -> bool {
                $crate::var::Var::is_new($crate::var::ContextVarProxy::<Self>::static_ref(), vars)
            }

            /// Gets the version of the value in the current `vars` context.
            #[inline]
            #[allow(unused)]
            pub fn version<Vr: $crate::var::WithVarsRead>(vars: &Vr) -> $crate::var::VarVersion {
                $crate::var::Var::version($crate::var::ContextVarProxy::<Self>::static_ref(), vars)
            }

            /// If the value in the current `vars` context cannot be set or modified right now.
            ///
            /// If this is `false` the context var can [`set`] or [`modify`], the value is change
            /// is applied in the backing source of the current value.
            ///
            /// [`set`]: Self::set
            /// [`modify`]: Self::modify
            #[inline]
            #[allow(unused)]
            pub fn is_read_only<Vw: $crate::var::WithVars>(vars: &Vw) -> bool {
                $crate::var::Var::is_read_only($crate::var::ContextVarProxy::<Self>::static_ref(), vars)
            }

            /// Schedule a modification of the variable value source in the current `vars` context.
            ///
            /// If the backing source [`is_read_only`] returns an error.
            ///
            /// [`is_read_only`]: Self::is_read_only
            #[inline]
            #[allow(unused)]
            pub fn modify<Vw, M>(vars: &Vw, modify: M) -> std::result::Result<(), $crate::var::VarIsReadOnly>
            where
                Vw: $crate::var::WithVars,
                M: std::ops::FnOnce($crate::var::VarModify<$type>) + 'static
            {
                $crate::var::Var::modify($crate::var::ContextVarProxy::<Self>::static_ref(), vars, modify)
            }

            /// Schedule a new value for the variable value source in the current `vars` context.
            ///
            /// If the backing source [`is_read_only`] returns an error.
            ///
            /// [`is_read_only`]: Self::is_read_only
            #[inline]
            #[allow(unused)]
            pub fn set<Vw, N>(vars: &Vw, new_value: N) -> std::result::Result<(), $crate::var::VarIsReadOnly>
            where
                Vw: $crate::var::WithVars,
                N: std::convert::Into<$type>,
            {
                $crate::var::Var::set($crate::var::ContextVarProxy::<Self>::static_ref(), vars, new_value)
            }
        }

        impl $crate::var::ContextVar for $ident {
            type Type = $type;

            #[inline]
            fn default_value() -> Self::Type {
               Self::default_value()
            }

            #[inline]
            fn thread_local_value() -> $crate::var::ContextVarLocalKey<Self> {
                $crate::var::ContextVarLocalKey::new(&Self::THREAD_LOCAL_VALUE)
            }
        }

        impl $crate::var::IntoVar<$type> for $ident {
            type Var = $crate::var::ContextVarProxy<Self>;
            #[inline]
            fn into_var(self) -> Self::Var {
                $crate::var::ContextVarProxy::default()
            }
        }
    )+};
}
#[doc(inline)]
pub use crate::context_var;

mod properties {
    use crate::{context::*, event::*, render::*, units::*, var::*, widget_info::*, *};

    /// Helper for declaring properties that sets a context var.
    ///
    /// The method presents the `value` as the [`ContextVar<Type=T>`] in the widget and widget descendants.
    /// The context var [`is_new`] and [`is_read_only`] status are always equal to the `value` var status. Users
    /// of the context var can also retrieve the `value` var using [`actual_var`].
    ///
    /// The generated [`UiNode`] delegates each method to `child` inside a call to [`VarsRead::with_context_var`].
    ///
    /// # Examples
    ///
    /// A simple context property declaration:
    ///
    /// ```
    /// # fn main() -> () { }
    /// # use zero_ui_core::{*, var::*};
    /// context_var! {
    ///     pub struct FooVar: u32 = 0;
    /// }
    ///
    /// /// Sets the [`FooVar`] in the widgets and its content.
    /// #[property(context, default(FooVar))]
    /// pub fn foo(child: impl UiNode, value: impl IntoVar<u32>) -> impl UiNode {
    ///     with_context_var(child, FooVar, value)
    /// }
    /// ```
    ///
    /// When set in a widget, the `value` is accessible in all inner nodes of the widget, using `FooVar::get`, and if `value` is set to a
    /// variable the `FooVar` will also reflect its [`is_new`] and [`is_read_only`]. If the `value` var is not read-only inner nodes
    /// can modify it using `FooVar::set` or `FooVar::modify`.
    ///
    /// Also note that the property [`default`] is set to the same `FooVar`, this causes the property to *pass-through* the outer context
    /// value, as if it was not set.
    ///
    /// **Tip:** You can use a [`merge_var!`] to merge a new value to the previous context value:
    ///
    /// ```
    /// # fn main() -> () { }
    /// # use zero_ui_core::{*, var::*};
    ///
    /// #[derive(Debug, Clone, Default)]
    /// pub struct Config {
    ///     pub foo: bool,
    ///     pub bar: bool,
    /// }
    ///
    /// context_var! {
    ///     pub struct ConfigVar: Config = Config::default();
    /// }
    ///
    /// /// Sets the *foo* config.
    /// #[property(context, default(false))]
    /// pub fn foo(child: impl UiNode, value: impl IntoVar<bool>) -> impl UiNode {
    ///     with_context_var(child, ConfigVar, merge_var!(ConfigVar::new(), value.into_var(), |c, &v| {
    ///         let mut c = c.clone();
    ///         c.foo = v;
    ///         c
    ///     }))
    /// }
    ///
    /// /// Sets the *bar* config.
    /// #[property(context, default(false))]
    /// pub fn bar(child: impl UiNode, value: impl IntoVar<bool>) -> impl UiNode {
    ///     with_context_var(child, ConfigVar, merge_var!(ConfigVar::new(), value.into_var(), |c, &v| {
    ///         let mut c = c.clone();
    ///         c.bar = v;
    ///         c
    ///     }))
    /// }
    /// ```
    ///
    /// When set in a widget, the [`merge_var!`] will read the context value of the parent properties, modify a clone of the value and
    /// the result will be accessible to the inner properties, the widget user can then set with the composed value in steps and
    /// the final consumer of the composed value only need to monitor to a single context variable.
    ///
    /// [`is_new`]: Var::is_new
    /// [`is_read_only`]: Var::is_read_only
    /// [`actual_var`]: Var::actual_var
    /// [`default`]: crate::property#default
    pub fn with_context_var<T: VarValue>(child: impl UiNode, var: impl ContextVar<Type = T>, value: impl IntoVar<T>) -> impl UiNode {
        struct WithContextVarNode<U, C, V> {
            child: U,
            var: C,
            value: V,
        }
        impl<U, T, C, V> UiNode for WithContextVarNode<U, C, V>
        where
            U: UiNode,
            T: VarValue,
            C: ContextVar<Type = T>,
            V: Var<T>,
        {
            #[inline(always)]
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                ctx.vars
                    .with_context_var(self.var, ContextVarData::in_vars_read(ctx.vars, &self.value), || {
                        self.child.info(ctx, info)
                    });
            }
            #[inline(always)]
            fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                ctx.vars
                    .with_context_var(self.var, ContextVarData::in_vars_read(ctx.vars, &self.value), || {
                        self.child.subscriptions(ctx, subscriptions)
                    });
            }
            #[inline(always)]
            fn init(&mut self, ctx: &mut WidgetContext) {
                ctx.vars
                    .with_context_var(self.var, ContextVarData::in_vars(ctx.vars, &self.value, false), || {
                        self.child.init(ctx)
                    });
            }
            #[inline(always)]
            fn deinit(&mut self, ctx: &mut WidgetContext) {
                ctx.vars
                    .with_context_var(self.var, ContextVarData::in_vars(ctx.vars, &self.value, false), || {
                        self.child.deinit(ctx)
                    });
            }
            #[inline(always)]
            fn update(&mut self, ctx: &mut WidgetContext) {
                ctx.vars
                    .with_context_var(self.var, ContextVarData::in_vars(ctx.vars, &self.value, false), || {
                        self.child.update(ctx)
                    });
            }
            #[inline(always)]
            fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
                ctx.vars
                    .with_context_var(self.var, ContextVarData::in_vars(ctx.vars, &self.value, false), || {
                        self.child.event(ctx, args)
                    });
            }
            #[inline(always)]
            fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                ctx.vars
                    .with_context_var(self.var, ContextVarData::in_vars(ctx.vars, &self.value, false), || {
                        self.child.measure(ctx, available_size)
                    })
            }
            #[inline(always)]
            fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
                ctx.vars
                    .with_context_var(self.var, ContextVarData::in_vars(ctx.vars, &self.value, false), || {
                        self.child.arrange(ctx, widget_layout, final_size)
                    });
            }
            #[inline(always)]
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                ctx.vars
                    .with_context_var(self.var, ContextVarData::in_vars_read(ctx.vars, &self.value), || {
                        self.child.render(ctx, frame)
                    });
            }
            #[inline(always)]
            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                ctx.vars
                    .with_context_var(self.var, ContextVarData::in_vars_read(ctx.vars, &self.value), || {
                        self.child.render_update(ctx, update)
                    });
            }
        }
        WithContextVarNode {
            child,
            var,
            value: value.into_var(),
        }
    }
}
#[doc(inline)]
pub use properties::*;

#[cfg(test)]
mod tests {
    use crate::{app::*, context::*, text::*, var::*, *};

    context_var! {
        struct TestVar: Text = "".into();
    }

    state_key! {
        pub struct ProbeKey: Text;
    }

    #[property(context, default(TestVar))]
    fn test_prop(child: impl UiNode, value: impl IntoVar<Text>) -> impl UiNode {
        with_context_var(child, TestVar, value)
    }

    #[property(context)]
    fn probe(child: impl UiNode, var: impl IntoVar<Text>) -> impl UiNode {
        struct ProbeNode<C, V> {
            child: C,
            var: V,
        }
        #[impl_ui_node(child)]
        impl<C: UiNode, V: Var<Text>> UiNode for ProbeNode<C, V> {
            fn init(&mut self, ctx: &mut WidgetContext) {
                ctx.app_state.set(ProbeKey, self.var.get_clone(ctx.vars));
                self.child.init(ctx);
            }
        }
        ProbeNode {
            child,
            var: var.into_var(),
        }
    }

    #[property(event)]
    fn on_init(child: impl UiNode, handler: impl handler::WidgetHandler<()>) -> impl UiNode {
        struct OnInitNode<C, H> {
            child: C,
            handler: H,
        }
        #[impl_ui_node(child)]
        impl<C, H> UiNode for OnInitNode<C, H>
        where
            C: UiNode,
            H: handler::WidgetHandler<()>,
        {
            fn init(&mut self, ctx: &mut WidgetContext) {
                self.child.init(ctx);
                self.handler.event(ctx, &());
            }

            fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut widget_info::WidgetSubscriptions) {
                subscriptions.handler(&self.handler);
                self.child.subscriptions(ctx, subscriptions);
            }

            fn update(&mut self, ctx: &mut WidgetContext) {
                self.child.update(ctx);
                self.handler.update(ctx);
            }
        }
        OnInitNode { child, handler }
    }

    #[widget($crate::var::context::tests::test_wgt)]
    mod test_wgt {
        use super::*;

        properties! {
            #[allowed_in_when = false]
            child(impl UiNode) = NilUiNode;
        }

        fn new_child(child: impl UiNode) -> impl UiNode {
            child
        }
    }

    fn test_app(root: impl UiNode) -> HeadlessApp {
        test_log();

        use crate::window::*;
        let mut app = App::default().run_headless(false);
        app.ctx().services.windows().open(move |_| crate::window::Window::new_test(root));
        let _ = app.update(false);
        app
    }

    #[test]
    fn context_var_basic() {
        let mut test = test_app(test_wgt! {
            test_prop = "test!";

            child = test_wgt! {
                probe = TestVar;
            }
        });

        assert_eq!(test.ctx().app_state.get(ProbeKey), Some(&Text::from("test!")));
    }

    #[test]
    fn context_var_map() {
        let mut test = test_app(test_wgt! {
            test_prop = "test!";

            child = test_wgt! {
                probe = TestVar::new().map(|t| formatx!("map {t}"));
            }
        });

        assert_eq!(test.ctx().app_state.get(ProbeKey), Some(&Text::from("map test!")));
    }

    #[test]
    fn context_var_map_cloned() {
        // mapped context var should depend on the context.

        let mapped = TestVar::new().map(|t| formatx!("map {t}"));
        use self::test_prop as test_prop_a;
        use self::test_prop as test_prop_b;

        let mut test = test_app(test_wgt! {
            test_prop_a = "A!";

            child = test_wgt! {
                probe = mapped.clone();
                test_prop_b = "B!";

                child = test_wgt! {
                    probe = mapped;
                }
            }
        });

        assert_eq!(test.ctx().app_state.get(ProbeKey), Some(&Text::from("map B!")));
    }

    #[test]
    fn context_var_map_cloned3() {
        // mapped context var should depend on the context.

        let mapped = TestVar::new().map(|t| formatx!("map {t}"));
        let mut test = test_app(test_wgt! {
            test_prop = "A!";

            child = test_wgt! {
                probe = mapped.clone();
                test_prop = "B!";

                child = test_wgt! {
                    probe = mapped.clone();
                    test_prop = "C!";

                    child = test_wgt! {
                        probe = mapped;
                        test_prop = "D!";
                    }
                }
            }
        });

        assert_eq!(test.ctx().app_state.get(ProbeKey), Some(&Text::from("map C!")));
    }

    #[test]
    fn context_var_map_not_cloned() {
        // sanity check for `context_var_map_cloned`

        use self::test_prop as test_prop_a;
        use self::test_prop as test_prop_b;

        let mut test = test_app(test_wgt! {
            test_prop_a = "A!";

            child = test_wgt! {
                probe = TestVar::new().map(|t| formatx!("map {t}"));
                test_prop_b = "B!";

                child = test_wgt! {
                    probe = TestVar::new().map(|t| formatx!("map {t}"));
                }
            }
        });

        assert_eq!(test.ctx().app_state.get(ProbeKey), Some(&Text::from("map B!")));
    }

    #[test]
    fn context_var_map_moved_app_ctx() {
        // need to support different value using the same variable instance too.

        let mapped = TestVar::new().map(|t| formatx!("map {t}"));

        let mut app = test_app(NilUiNode);
        let ctx = app.ctx();

        let a = ctx
            .vars
            .with_context_var(TestVar, ContextVarData::fixed(&"A".into()), || mapped.get_clone(ctx.vars));
        let b = ctx
            .vars
            .with_context_var(TestVar, ContextVarData::fixed(&"B".into()), || mapped.get_clone(ctx.vars));

        assert_ne!(a, b);
    }

    #[test]
    fn context_var_cloned_same_widget() {
        let mapped = TestVar::new().map(|t| formatx!("map {t}"));
        use self::probe as probe_a;
        use self::probe as probe_b;
        use self::test_prop as test_prop_a;
        use self::test_prop as test_prop_b;

        let mut test = test_app(test_wgt! {
            test_prop_a = "A!";
            probe_a = mapped.clone();
            test_prop_b = "B!";
            probe_b = mapped;
        });

        assert_eq!(test.ctx().app_state.get(ProbeKey), Some(&Text::from("map B!")));
    }

    #[test]
    fn context_var_set() {
        let mut app = test_app(NilUiNode);

        let backing_var = var(Text::from(""));

        let ctx = app.ctx();
        ctx.vars
            .with_context_var(TestVar, ContextVarData::in_vars(ctx.vars, &backing_var, false), || {
                let t = TestVar::new();
                assert!(!t.is_read_only(ctx.vars));
                t.set(ctx.vars, "set!").unwrap();
            });

        let _ = app.update(false);
        let ctx = app.ctx();
        assert_eq!(backing_var.get(ctx.vars), "set!");
    }

    #[test]
    fn context_var_binding() {
        let input_var = var("Input!".to_text());
        let other_var = var(".".to_text());

        let mut test = test_app(test_wgt! {
            test_prop = input_var.clone();
            on_init = hn_once!(other_var, |ctx, _| {
                TestVar::new().bind(ctx, &other_var).perm();
            });
            child = NilUiNode;
        });

        test.update(false).assert_wait();

        assert_eq!(".", other_var.get(test.ctx().vars));

        input_var.set(test.ctx().vars, "Update!");

        test.update(false).assert_wait();

        assert_eq!("Update!", input_var.get(test.ctx().vars));
        assert_eq!("Update!", other_var.get(test.ctx().vars));
    }
}
