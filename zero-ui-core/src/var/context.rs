use std::{marker::PhantomData, thread::LocalKey, rc::Rc};

use super::*;

/// A [`Var`] that represents a [`ContextVar`].
///
/// Context var types don't implement [`Var`] directly, to avoid problems with overlapping generics
/// this *proxy* zero-sized type is used.
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
        let vars = vars.as_ref();
        vars.context_var::<C>().value
    }

    #[inline]
    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a C::Type> {
        let vars = vars.as_ref();
        let info = vars.context_var::<C>();
        if info.is_new {
            Some(info.value)
        } else {
            None
        }
    }

    #[inline]
    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|v| v.context_var::<C>().is_new)
    }

    #[inline]
    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> C::Type {
        self.get_clone(vars)
    }

    #[inline]
    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> VarVersion {
        vars.with_vars_read(|v| v.context_var::<C>().version)
    }

    #[inline]
    fn is_read_only<Vr: WithVars>(&self, _: &Vr) -> bool {
        true
    }

    #[inline]
    fn always_read_only(&self) -> bool {
        true
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
    fn modify<Vw, M>(&self, _: &Vw, _: M) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        M: FnOnce(&mut VarModify<C::Type>) + 'static,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn set<Vw, N>(&self, _: &Vw, _: N) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<C::Type>,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn set_ne<Vw, N>(&self, _: &Vw, _: N) -> Result<bool, VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<C::Type>,
        C::Type: PartialEq,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn into_read_only(self) -> Self::AsReadOnly {
        self
    }

    #[inline]
    fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
        vars.with_vars_read(|v| v.context_var::<C>().update_mask)
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
/// or [`with_context_var_wgt_only`] helper functions to bind another variable to a context variable, internally these
/// functions use the [`ContextVarData::var`] and [`ContextVarData::var_read`] in all [`UiNode`] methods.
///
/// [`UiNode`]: crate::UiNode
pub struct ContextVarData<'a, T: VarValue> {
    /// Value for [`Var::get`].
    pub value: &'a T,
    /// Value for [`Var::is_new`].
    pub is_new: bool,
    /// Value for [`Var::version`].
    pub version: VarVersion,
    /// Value for [`Var::update_mask`].
    ///
    /// Note that the node that owns `self` must not subscribe to this update mask, only
    /// inner nodes that subscribe to the context var mapped to `self` subscribe to this mask.
    pub update_mask: UpdateMask,

    /// Delegate for [`Var::modify`].
    /// 
    /// The context var [`Var::is_read_only`] if this is `None`, otherwise this closure is called when any of the
    /// assign, touch or modify methods are called for the context var.
    /// 
    /// Note that this closure is called immediately, but it should only affect the value for the next update cycle,
    /// like any other variable. The best way to implement this properly is using the [`ContextVarData::var`] constructor.
    pub modify: ContextVarModify<T>,
}
type BoxedVarModify<T> = Box<dyn FnOnce(&mut VarModify<T>)>;
type ContextVarModify<T> = Option<Rc<dyn Fn(&Vars, BoxedVarModify<T>)>>;
impl<'a, T: VarValue> ContextVarData<'a, T> {
    /// Value that does not change or update.
    ///
    /// The context var is never new, the version is always zero and there is no update mask.
    pub fn fixed(value: &'a T) -> Self {
        Self {
            value,
            is_new: false,
            version: VarVersion::normal(0),
            update_mask: UpdateMask::none(),
            modify: None,
        }
    }

    /// Binds the context var to another `var`.
    pub fn var(vars: &'a Vars, var: &'a impl Var<T>) -> Self {
        Self {
            value: var.get(vars),
            is_new: var.is_new(vars),
            version: var.version(vars),
            update_mask: var.update_mask(vars),
            modify: if var.is_read_only(vars) {
                None
            } else {
                Some(Rc::new(clone_move!(var, |vars, m| var.modify(vars, m).unwrap())))
            },
        }
    }

    /// Binds the context var to another `var` in a read-only context.
    pub fn var_read(vars: &'a VarsRead, var: &'a impl Var<T>) -> Self {
        Self {
            value: var.get(vars),
            is_new: false,
            version: var.version(vars),
            update_mask: var.update_mask(vars),
            modify: None,
        }
    }

    /// Binds the context var to a `value` that is always derived from another `var` such that
    /// they update at the same time.
    ///
    /// # Examples
    ///
    /// The example maps boolean source variable to a text context variable.
    ///
    /// ```
    /// use zero_ui_core::{*, var::*, text::*, context::*};
    ///
    /// context_var! {
    ///     /// Foo context variable.
    ///     pub struct FooVar: Text = Text::empty();
    /// }
    ///
    /// struct FooNode<C, V> {
    ///     child: C,
    ///     data_source: V,
    /// }
    ///
    /// #[impl_ui_node(child)]
    /// impl<C: UiNode, V: Var<bool>> FooNode<C, V> {
    ///     fn foo_value(&self, vars: &VarsRead) -> Text {
    ///         if self.data_source.copy(vars) {
    ///             Text::from_static("Enabled")
    ///         } else {
    ///             Text::from_static("Disabled")
    ///         }
    ///     }
    ///
    ///     #[UiNode]
    ///     fn update(&mut self, ctx: &mut WidgetContext) {
    ///         let value = self.foo_value(ctx.vars);
    ///         ctx.vars.with_context_var(FooVar, ContextVarData::map(ctx.vars, &self.data_source, &value), || {
    ///             self.child.update(ctx);
    ///         })
    ///     }
    ///     
    ///     // .. all other UiNode methods.
    /// }
    /// ```
    ///
    /// The context variable will have the same `is_new`, `version` and `update_mask` from the source variable, but it
    /// has a different value.
    ///
    /// The example only demonstrates one [`UiNode`] method but usually all methods must do the same, and you must
    /// use [`map_read`] in the methods that only expose the [`VarsRead`] accessor.
    ///
    /// [`UiNode`]: crate::UiNode
    /// [`map_read`]: Self::map_read
    pub fn map<'b, S: VarValue>(vars: &'b Vars, var: &'b impl Var<S>, value: &'a T) -> Self {
        Self {
            value,
            is_new: var.is_new(vars),
            version: var.version(vars),
            update_mask: var.update_mask(vars),
            modify: None,
        }
    }

    /// Binds the context var to a `value` that is always derived from another `var`.
    pub fn map_read<'b, S: VarValue>(vars: &'b VarsRead, var: &'b impl Var<S>, value: &'a T) -> Self {
        Self {
            value,
            is_new: false,
            version: var.version(vars),
            update_mask: var.update_mask(vars),
            modify: None,
        }
    }

    pub(crate) fn into_raw(self) -> ContextVarDataRaw<T> {
        ContextVarDataRaw {
            value: self.value,
            is_new: self.is_new,
            version: self.version,
            update_mask: self.update_mask,
            modify: self.modify,
        }
    }
}
impl<T: VarValue> ContextVarDataRaw<T> {
    /// SAFETY: Only [`VarsRead`] can call this safely.
    pub(crate) unsafe fn into_safe(self, _vars: &VarsRead) -> ContextVarData<T> {
        ContextVarData {
            value: &*self.value,
            is_new: self.is_new,
            version: self.version,
            update_mask: self.update_mask,
            modify: self.modify,
        }
    }
}
impl<'a, T: VarValue> Clone for ContextVarData<'a, T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            is_new: self.is_new,
            version: self.version,
            update_mask: self.update_mask,
            modify: self.modify.clone(),
        }
    }
}
struct ContextVarDataCell<T: VarValue>(UnsafeCell<ContextVarDataRaw<T>>);
impl<T: VarValue> ContextVarDataCell<T> {
    pub fn new(data: ContextVarDataRaw<T>) -> Self {
        ContextVarDataCell(UnsafeCell::new(data))
    }

    pub fn get(&self) -> ContextVarDataRaw<T> {
        // SAFETY: this is safe because VarVersion has no cyclical references.
        unsafe { &*self.0.get() }.clone()
    }

    pub fn get_version(&self) -> VarVersion {
        // SAFETY: this is safe because VarVersion has no cyclical references.
        unsafe { &*self.0.get() }.version
    }

    pub fn set(&self, data: ContextVarDataRaw<T>) {
        // SAFETY: this is safe because `Self` is not Sync and we do not share references, so this deref is unique.
        unsafe {
            *self.0.get() = data;
        }
    }

    pub fn replace(&self, data: ContextVarDataRaw<T>) -> ContextVarDataRaw<T> {
        // SAFETY: this is safe because `Self` is not Sync and we do not share references, so this borrow is unique.
        std::mem::replace(unsafe { &mut *self.0.get() }, data)
    }
}
pub(crate) struct ContextVarDataRaw<T: VarValue> {
    value: *const T,
    is_new: bool,
    version: VarVersion,
    update_mask: UpdateMask,
    modify: ContextVarModify<T>,
}
impl<T: VarValue> Clone for ContextVarDataRaw<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            is_new: self.is_new,
            version: self.version,
            update_mask: self.update_mask,
            modify: self.modify.clone(),
        }
    }
}

/// See `ContextVar::thread_local_value`.
#[doc(hidden)]
pub struct ContextVarValue<V: ContextVar> {
    _var: PhantomData<V>,
    _default_value: Box<V::Type>,
    value: ContextVarDataCell<V::Type>,
}

#[allow(missing_docs)]
impl<V: ContextVar> ContextVarValue<V> {
    #[inline]
    pub fn init() -> Self {
        let default_value = Box::new(V::default_value());
        ContextVarValue {
            _var: PhantomData,
            value: ContextVarDataCell::new(ContextVarData::fixed(&*default_value).into_raw()),
            _default_value: default_value,
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

    pub(super) fn get(&self) -> ContextVarDataRaw<V::Type> {
        self.local.with(|l| l.value.get())
    }

    pub(super) fn version(&self) -> VarVersion {
        self.local.with(|l| l.value.get_version())
    }

    pub(super) fn set(&self, value: ContextVarDataRaw<V::Type>) {
        self.local.with(|l| l.value.set(value))
    }

    pub(super) fn replace(&self, value: ContextVarDataRaw<V::Type>) -> ContextVarDataRaw<V::Type> {
        self.local.with(|l| l.value.replace(value))
    }
}

///<span data-inline></span> Declares new [`ContextVar`](crate::var::ContextVar) types.
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

            // TODO generate copy fns when https://github.com/rust-lang/rust/issues/48214 is stable

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
    /// The context var [`version`] and [`is_new`] status are always equal to the `value` var status.
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
    /// When set in a widget, the `value` is accessible in all inner nodes of the widget, using `FooVar.get`, and if `value` is set to a
    /// variable the `FooVar` will also reflect its [`is_new`] and [`version`].
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
    /// [`version`]: Var::version
    /// [`is_new`]: Var::is_new
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
                    .with_context_var(self.var, ContextVarData::var_read(ctx.vars, &self.value), || {
                        self.child.info(ctx, info)
                    });
            }
            #[inline(always)]
            fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                ctx.vars
                    .with_context_var(self.var, ContextVarData::var_read(ctx.vars, &self.value), || {
                        self.child.subscriptions(ctx, subscriptions)
                    });
            }
            #[inline(always)]
            fn init(&mut self, ctx: &mut WidgetContext) {
                ctx.vars
                    .with_context_var(self.var, ContextVarData::var(ctx.vars, &self.value), || self.child.init(ctx));
            }
            #[inline(always)]
            fn deinit(&mut self, ctx: &mut WidgetContext) {
                ctx.vars
                    .with_context_var(self.var, ContextVarData::var(ctx.vars, &self.value), || self.child.deinit(ctx));
            }
            #[inline(always)]
            fn update(&mut self, ctx: &mut WidgetContext) {
                ctx.vars
                    .with_context_var(self.var, ContextVarData::var(ctx.vars, &self.value), || self.child.update(ctx));
            }
            #[inline(always)]
            fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
                ctx.vars
                    .with_context_var(self.var, ContextVarData::var(ctx.vars, &self.value), || self.child.event(ctx, args));
            }
            #[inline(always)]
            fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                ctx.vars.with_context_var(self.var, ContextVarData::var(ctx.vars, &self.value), || {
                    self.child.measure(ctx, available_size)
                })
            }
            #[inline(always)]
            fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
                ctx.vars.with_context_var(self.var, ContextVarData::var(ctx.vars, &self.value), || {
                    self.child.arrange(ctx, widget_layout, final_size)
                });
            }
            #[inline(always)]
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                ctx.vars
                    .with_context_var(self.var, ContextVarData::var_read(ctx.vars, &self.value), || {
                        self.child.render(ctx, frame)
                    });
            }
            #[inline(always)]
            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                ctx.vars
                    .with_context_var(self.var, ContextVarData::var_read(ctx.vars, &self.value), || {
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

    /// Helper for declaring properties that sets a context var for the widget only.
    ///
    /// This is similar to [`with_context_var`] except the context var value is visible only inside
    /// the `child` nodes that are part of the same widget that is the parent of the return node.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> () { }
    /// # use zero_ui_core::{*, var::*, border::CornerRadius};
    /// context_var! {
    ///     pub struct CornersClipVar: CornerRadius = CornerRadius::zero();
    /// }
    ///
    /// /// Sets widget content clip corner radius.
    /// #[property(context, default(CornersClipVar))]
    /// pub fn corners_clip(child: impl UiNode, radius: impl IntoVar<CornerRadius>) -> impl UiNode {
    ///     with_context_var_wgt_only(child, CornersClipVar, radius)
    /// }
    /// ```
    pub fn with_context_var_wgt_only<T: VarValue>(
        child: impl UiNode,
        var: impl ContextVar<Type = T>,
        value: impl IntoVar<T>,
    ) -> impl UiNode {
        struct WithContextVarWidgetOnlyNode<U, C, V> {
            child: U,
            var: C,
            value: V,
        }
        impl<U, T, C, V> UiNode for WithContextVarWidgetOnlyNode<U, C, V>
        where
            U: UiNode,
            T: VarValue,
            C: ContextVar<Type = T>,
            V: Var<T>,
        {
            #[inline(always)]
            fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
                ctx.vars
                    .with_context_var_wgt_only(self.var, ContextVarData::var_read(ctx.vars, &self.value), || {
                        self.child.info(ctx, widget)
                    });
            }
            #[inline(always)]
            fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                ctx.vars
                    .with_context_var_wgt_only(self.var, ContextVarData::var_read(ctx.vars, &self.value), || {
                        self.child.subscriptions(ctx, subscriptions)
                    });
            }
            #[inline(always)]
            fn init(&mut self, ctx: &mut WidgetContext) {
                ctx.vars
                    .with_context_var_wgt_only(self.var, ContextVarData::var(ctx.vars, &self.value), || self.child.init(ctx));
            }
            #[inline(always)]
            fn deinit(&mut self, ctx: &mut WidgetContext) {
                ctx.vars
                    .with_context_var_wgt_only(self.var, ContextVarData::var(ctx.vars, &self.value), || self.child.deinit(ctx));
            }
            #[inline(always)]
            fn update(&mut self, ctx: &mut WidgetContext) {
                ctx.vars
                    .with_context_var_wgt_only(self.var, ContextVarData::var(ctx.vars, &self.value), || self.child.update(ctx));
            }
            #[inline(always)]
            fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                ctx.vars
                    .with_context_var_wgt_only(self.var, ContextVarData::var(ctx.vars, &self.value), || self.child.event(ctx, args));
            }
            #[inline(always)]
            fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                ctx.vars
                    .with_context_var_wgt_only(self.var, ContextVarData::var(ctx.vars, &self.value), || {
                        self.child.measure(ctx, available_size)
                    })
            }
            #[inline(always)]
            fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
                ctx.vars
                    .with_context_var_wgt_only(self.var, ContextVarData::var(ctx.vars, &self.value), || {
                        self.child.arrange(ctx, widget_layout, final_size)
                    })
            }
            #[inline(always)]
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                ctx.vars
                    .with_context_var_wgt_only(self.var, ContextVarData::var_read(ctx.vars, &self.value), || {
                        self.child.render(ctx, frame)
                    });
            }
            #[inline(always)]
            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                ctx.vars
                    .with_context_var_wgt_only(self.var, ContextVarData::var_read(ctx.vars, &self.value), || {
                        self.child.render_update(ctx, update)
                    });
            }
        }
        WithContextVarWidgetOnlyNode {
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
        app.ctx().services.windows().open(move |_| crate::window::Window::test(root));
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
        ctx.vars.with_context_var(TestVar, ContextVarData::var(ctx.vars, &backing_var), || {
            let t = TestVar::new();
            assert!(!t.is_read_only(ctx.vars));
            t.set(ctx.vars, "set!").unwrap();
        });

        let _ = app.update(false);
        let ctx = app.ctx();
        assert_eq!(backing_var.get(ctx.vars), "set!");
    }
}
