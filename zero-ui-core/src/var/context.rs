use std::{cell::Cell, marker::PhantomData, thread::LocalKey};

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
    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> u32 {
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

/// Value bound to context var at a context.
pub struct ContextVarData<'a, T: VarValue> {
    /// Value for [`Var::get`].
    pub value: &'a T,
    /// Value for [`Var::is_new`].
    pub is_new: bool,
    /// Value for [`Var::version`].
    pub version: u32,
    /// Value for [`Var::update_mask`].
    ///
    /// Note that the node that owns `self` must not subscribe to this update mask, only
    /// inner nodes that subscribe to the context var mapped to `self` subscribe to this mask.
    pub update_mask: UpdateMask,
}
impl<'a, T: VarValue> ContextVarData<'a, T> {
    /// Value that does change or update.
    pub fn fixed(value: &'a T) -> Self {
        Self {
            value,
            is_new: false,
            version: 0,
            update_mask: UpdateMask::none(),
        }
    }

    /// Binds the context var to another `var`.
    pub fn var(vars: &'a Vars, var: &'a impl Var<T>) -> Self {
        Self {
            value: var.get(vars),
            is_new: var.is_new(vars),
            version: var.version(vars),
            update_mask: var.update_mask(vars),
        }
    }

    /// Binds the context var to another `var` in a read-only context.
    pub fn var_read(vars: &'a VarsRead, var: &'a impl Var<T>) -> Self {
        Self {
            value: var.get(vars),
            is_new: false,
            version: var.version(vars),
            update_mask: var.update_mask(vars),
        }
    }

    /// Binds the context var to a `value` that is always derived from another `var` such that
    /// they update at the same time.
    pub fn map<'b, S: VarValue>(vars: &'b Vars, var: &'b impl Var<S>, value: &'a T) -> Self {
        Self {
            value,
            is_new: var.is_new(vars),
            version: var.version(vars),
            update_mask: var.update_mask(vars),
        }
    }

    /// Binds the context var to a `value` that is always derived from another `var`.
    pub fn map_read<'b, S: VarValue>(vars: &'b VarsRead, var: &'b impl Var<S>, value: &'a T) -> Self {
        Self {
            value,
            is_new: false,
            version: var.version(vars),
            update_mask: var.update_mask(vars),
        }
    }

    pub(crate) fn to_raw(self) -> ContextVarDataRaw<T> {
        ContextVarDataRaw {
            value: self.value,
            is_new: self.is_new,
            version: self.version,
            update_mask: self.update_mask,
        }
    }
}
impl<T: VarValue> ContextVarDataRaw<T> {
    /// SAFETY: Only [`VarsRead`] can call this safely.
    pub(crate) unsafe fn to_safe(self, _vars: &VarsRead) -> ContextVarData<T> {
        ContextVarData {
            value: &*self.value,
            is_new: self.is_new,
            version: self.version,
            update_mask: self.update_mask,
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
        }
    }
}
impl<'a, T: VarValue> Copy for ContextVarData<'a, T> {}

pub(crate) struct ContextVarDataRaw<T: VarValue> {
    value: *const T,
    is_new: bool,
    version: u32,
    update_mask: UpdateMask,
}
impl<T: VarValue> Clone for ContextVarDataRaw<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            is_new: self.is_new,
            version: self.version,
            update_mask: self.update_mask,
        }
    }
}
impl<T: VarValue> Copy for ContextVarDataRaw<T> {}

/// See `ContextVar::thread_local_value`.
#[doc(hidden)]
pub struct ContextVarValue<V: ContextVar> {
    _var: PhantomData<V>,
    _default_value: Box<V::Type>,
    value: Cell<ContextVarDataRaw<V::Type>>,
}

#[allow(missing_docs)]
impl<V: ContextVar> ContextVarValue<V> {
    #[inline]
    pub fn init() -> Self {
        let default_value = Box::new(V::default_value());
        ContextVarValue {
            _var: PhantomData,
            value: Cell::new(ContextVarData::fixed(&*default_value).to_raw()),
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
            pub fn version<Vr: $crate::var::WithVarsRead>(vars: &Vr) -> u32 {
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
    use std::{cell::Cell, marker::PhantomData};

    use crate::{context::*, event::*, render::*, units::*, var::*, widget_info::*, *};

    /// Helper for declaring properties that sets a context var.
    ///
    /// The method presents the `value` as the [`ContextVar<Type=T>`] in the widget and widget descendants.
    /// The context var [`version`] and [`is_new`] status are always equal to the `value` var status.
    ///
    /// The generated [`UiNode`] delegates each method to `child` inside a call to [`Vars::with_context_bind`].
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
            fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
                ctx.vars.with_context_var(self.var, ContextVarData::var(ctx.vars, &self.value), || {
                    self.child.arrange(ctx, widget_offset, final_size)
                });
            }
            #[inline(always)]
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                ctx.vars
                    .with_context_var(self.var, ContextVarData::var_read(ctx.vars, &self.value), || {
                        self.child.info(ctx, info)
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
    /// # use zero_ui_core::{*, var::*, border::BorderRadius};
    /// context_var! {
    ///     pub struct CornersClipVar: BorderRadius = BorderRadius::zero();
    /// }
    ///
    /// /// Sets widget content clip corner radius.
    /// #[property(context, default(CornersClipVar))]
    /// pub fn corners_clip(child: impl UiNode, radius: impl IntoVar<BorderRadius>) -> impl UiNode {
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
            fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
                ctx.vars
                    .with_context_var_wgt_only(self.var, ContextVarData::var_read(ctx.vars, &self.value), || {
                        self.child.info(ctx, widget)
                    });
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
            fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
                ctx.vars
                    .with_context_var_wgt_only(self.var, ContextVarData::var(ctx.vars, &self.value), || {
                        self.child.arrange(ctx, widget_offset, final_size)
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

    /// Helper for declaring properties that affects a context var value but does not fully replace it.
    pub fn with_context_var_fold<C: ContextVar, I: VarValue>(
        child: impl UiNode,
        var: C,
        item: impl IntoVar<I>,
        fold: impl Fn(C::Type, &I) -> C::Type + 'static,
    ) -> impl UiNode {
        struct WithContextVarFoldNode<U, C: ContextVar, T, V, F> {
            child: U,

            var: C,
            var_ver: u32,
            item: V,
            item_ver: u32,

            fold: F,

            value: C::Type,
            version: u32,
            update_mask: Cell<UpdateMask>,

            _t: PhantomData<T>,
        }
        impl<U, C: ContextVar, T, V, F> WithContextVarFoldNode<U, C, T, V, F>
        where
            U: UiNode,
            T: VarValue,
            C: ContextVar,
            V: Var<T>,
            F: Fn(C::Type, &T) -> C::Type + 'static,
        {
        }
        impl<U, T, C, V, F> UiNode for WithContextVarFoldNode<U, C, T, V, F>
        where
            U: UiNode,
            T: VarValue,
            C: ContextVar,
            V: Var<T>,
            F: Fn(C::Type, &T) -> C::Type + 'static,
        {
            #[inline(always)]
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                self.update_mask.set(C::new().update_mask(ctx) | self.item.update_mask(ctx));
                ctx.vars.with_context_var(
                    self.var,
                    ContextVarData {
                        value: &self.value,
                        is_new: false,
                        version: self.version,
                        update_mask: self.update_mask.get(),
                    },
                    || self.child.info(ctx, info),
                );
            }
            #[inline(always)]
            fn init(&mut self, ctx: &mut WidgetContext) {
                let var = C::new();
                self.value = (self.fold)(var.get_clone(ctx), self.item.get(ctx));
                self.version = 0;
                self.item_ver = self.item.version(ctx);
                self.var_ver = var.version(ctx);

                ctx.vars.with_context_var(
                    self.var,
                    ContextVarData {
                        value: &self.value,
                        is_new: false,
                        version: self.version,
                        update_mask: self.update_mask.get(),
                    },
                    || self.child.init(ctx),
                );
            }
            #[inline(always)]
            fn deinit(&mut self, ctx: &mut WidgetContext) {
                ctx.vars.with_context_var(
                    self.var,
                    ContextVarData {
                        value: &self.value,
                        is_new: false,
                        version: self.version,
                        update_mask: self.update_mask.get(),
                    },
                    || self.child.deinit(ctx),
                );
            }
            #[inline(always)]
            fn update(&mut self, ctx: &mut WidgetContext) {
                let var = C::new();
                let var_ver = var.version(ctx);
                let item_ver = self.item.version(ctx);
                let is_new = var.is_new(ctx) || self.item.is_new(ctx);

                if is_new || self.var_ver != var_ver || self.item_ver != item_ver {
                    self.var_ver = var_ver;
                    self.item_ver = item_ver;
                    self.value = (self.fold)(var.get_clone(ctx), self.item.get(ctx));
                    self.version = self.version.wrapping_add(1);
                }

                ctx.vars.with_context_var(
                    self.var,
                    ContextVarData {
                        value: &self.value,
                        is_new,
                        version: self.version,
                        update_mask: self.update_mask.get(),
                    },
                    || self.child.update(ctx),
                );
            }
            #[inline(always)]
            fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
                ctx.vars.with_context_var(
                    self.var,
                    ContextVarData {
                        value: &self.value,
                        is_new: false,
                        version: self.version,
                        update_mask: self.update_mask.get(),
                    },
                    || self.child.event(ctx, args),
                );
            }
            #[inline(always)]
            fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                ctx.vars.with_context_var(
                    self.var,
                    ContextVarData {
                        value: &self.value,
                        is_new: false,
                        version: self.version,
                        update_mask: self.update_mask.get(),
                    },
                    || self.child.measure(ctx, available_size),
                )
            }
            #[inline(always)]
            fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
                ctx.vars.with_context_var(
                    self.var,
                    ContextVarData {
                        value: &self.value,
                        is_new: false,
                        version: self.version,
                        update_mask: self.update_mask.get(),
                    },
                    || self.child.arrange(ctx, widget_offset, final_size),
                );
            }

            #[inline(always)]
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                ctx.vars.with_context_var(
                    self.var,
                    ContextVarData {
                        value: &self.value,
                        is_new: false,
                        version: self.version,
                        update_mask: self.update_mask.get(),
                    },
                    || self.child.render(ctx, frame),
                );
            }
            #[inline(always)]
            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                ctx.vars.with_context_var(
                    self.var,
                    ContextVarData {
                        value: &self.value,
                        is_new: false,
                        version: self.version,
                        update_mask: self.update_mask.get(),
                    },
                    || self.child.render_update(ctx, update),
                );
            }
        }
        WithContextVarFoldNode {
            child,

            var,
            var_ver: 0,

            item: item.into_var(),
            item_ver: 0,

            fold,

            value: C::default_value(),
            version: 0,
            update_mask: Cell::new(UpdateMask::none()),

            _t: PhantomData,
        }
    }
}
#[doc(inline)]
pub use properties::*;
