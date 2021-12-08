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
        vars.context_var::<C>().value(vars)
    }

    #[inline]
    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a C::Type> {
        let vars = vars.as_ref();
        let info = vars.context_var::<C>();
        if info.is_new(vars) {
            Some(info.value(vars))
        } else {
            None
        }
    }

    #[inline]
    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|v| v.context_var::<C>().is_new(v))
    }

    #[inline]
    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> C::Type {
        self.get_clone(vars)
    }

    #[inline]
    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> u32 {
        vars.with_vars_read(|v| v.context_var::<C>().version(v))
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
        vars.with_vars_read(|v| v.context_var::<C>().update_mask(v))
    }
}

impl<C: ContextVar> IntoVar<C::Type> for ContextVarProxy<C> {
    type Var = Self;

    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}

/// Backing data for a [`ContextVar`] in a context.
///
/// Use [`ContextVarSourceVar`] to bind to another variable, use [`ContextVarSourceMap`]
/// to bind to a value generated from another source.
pub trait ContextVarSource<T: VarValue>: 'static {
    /// Value for [`Var::get`].
    fn value<'a>(&'a self, vars: &'a VarsRead) -> &'a T;

    /// Value for [`Var::is_new`].
    fn is_new(&self, vars: &Vars) -> bool;

    /// Value for [`Var::version`].
    fn version(&self, vars: &VarsRead) -> u32;

    /// Value for [`Var::update_mask`].
    ///
    /// Note that the node that owns `self` must not subscribe to this update mask, only
    /// inner nodes that subscribe to the context var mapped to `self` subscribe to this mask.
    fn update_mask(&self, vars: &VarsRead) -> UpdateMask;
}

/// Represents a context var source that is a `Var<T>`.
pub struct ContextVarSourceVar<V>(pub V);
impl<T, V> ContextVarSource<T> for ContextVarSourceVar<V>
where
    T: VarValue,
    V: Var<T>,
{
    fn value<'a>(&'a self, vars: &'a VarsRead) -> &'a T {
        self.0.get(vars)
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.0.is_new(vars)
    }

    fn version(&self, vars: &VarsRead) -> u32 {
        self.0.version(vars)
    }

    fn update_mask(&self, vars: &VarsRead) -> UpdateMask {
        self.0.update_mask(vars)
    }
}

/// Represents a context var source that is a value generated from a different context var source
/// such that all the update tracking metadata is the same.
pub struct ContextVarSourceMap<T: VarValue, ST: VarValue, SV: ContextVarSource<ST>> {
    /// Value.
    pub value: T,

    /// Metadata source.
    pub source: SV,

    _st: PhantomData<ST>,
}

impl<T: VarValue, ST: VarValue, SV: ContextVarSource<ST>> ContextVarSourceMap<T, ST, SV> {
    /// New with initial value.
    pub fn new(value: T, source: SV) -> Self {
        Self {
            value,
            source,
            _st: PhantomData,
        }
    }
}

impl<T, ST, SV> ContextVarSource<T> for ContextVarSourceMap<T, ST, SV>
where
    T: VarValue,
    ST: VarValue,
    SV: ContextVarSource<ST>,
{
    fn value<'a>(&'a self, _: &'a VarsRead) -> &'a T {
        &self.value
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.source.is_new(vars)
    }

    fn version(&self, vars: &VarsRead) -> u32 {
        self.source.version(vars)
    }

    fn update_mask(&self, vars: &VarsRead) -> UpdateMask {
        self.source.update_mask(vars)
    }
}

/// Represents a context var source that is an unchanging value.
pub struct ContextVarSourceValue<T: VarValue>(T);
impl<T: VarValue> ContextVarSourceValue<T> {
    /// New with  
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Reference the value.
    pub fn get(&self) -> &T {
        &self.0
    }
}
impl<T: VarValue> ContextVarSource<T> for ContextVarSourceValue<T> {
    fn value<'a>(&'a self, _: &'a VarsRead) -> &'a T {
        &self.0
    }

    fn is_new(&self, _: &Vars) -> bool {
        false
    }

    fn version(&self, _: &VarsRead) -> u32 {
        0
    }

    fn update_mask(&self, _: &VarsRead) -> UpdateMask {
        UpdateMask::none()
    }
}

/// Represents a context var source that is a value that can change.
pub struct ContextVarSourceCustom<T: VarValue> {
    /// [`ContextVarSource::value`]
    pub value: T,
    /// [`ContextVarSource::is_new`]
    pub is_new: bool,
    /// [`ContextVarSource::version`]
    pub version: u32,
    /// [`ContextVarSource::update_mask`]
    pub update_mask: Cell<UpdateMask>,
}
impl<T: VarValue> ContextVarSource<T> for ContextVarSourceCustom<T> {
    fn value<'a>(&'a self, _: &'a VarsRead) -> &'a T {
        &self.value
    }

    fn is_new(&self, _: &Vars) -> bool {
        self.is_new
    }

    fn version(&self, _: &VarsRead) -> u32 {
        self.version
    }

    fn update_mask(&self, _: &VarsRead) -> UpdateMask {
        self.update_mask.get()
    }
}

/// See `ContextVar::thread_local_value`.
#[doc(hidden)]
pub struct ContextVarValue<V: ContextVar> {
    _var: PhantomData<V>,
    _default_value: Box<dyn ContextVarSource<V::Type>>,
    value: Cell<*const dyn ContextVarSource<V::Type>>,
}

#[allow(missing_docs)]
impl<V: ContextVar> ContextVarValue<V> {
    #[inline]
    pub fn init() -> Self {
        let default_value = Box::new(ContextVarSourceValue(V::default_value()));
        ContextVarValue {
            _var: PhantomData,
            value: Cell::new(&*default_value as _),
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

    pub(super) fn get(&self) -> *const dyn ContextVarSource<V::Type> {
        self.local.with(|l| l.value.get())
    }

    pub(super) fn set(&self, value: *const dyn ContextVarSource<V::Type>) {
        self.local.with(|l| l.value.set(value))
    }

    pub(super) fn replace(&self, value: *const dyn ContextVarSource<V::Type>) -> *const dyn ContextVarSource<V::Type> {
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
            value: ContextVarSourceVar<V>,
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
                let child = &mut self.child;
                ctx.vars.with_context_var(self.var, &self.value, || child.init(ctx));
            }
            #[inline(always)]
            fn deinit(&mut self, ctx: &mut WidgetContext) {
                let child = &mut self.child;
                ctx.vars.with_context_var(self.var, &self.value, || child.deinit(ctx));
            }
            #[inline(always)]
            fn update(&mut self, ctx: &mut WidgetContext) {
                let child = &mut self.child;
                ctx.vars.with_context_var(self.var, &self.value, || child.update(ctx));
            }
            #[inline(always)]
            fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
                let child = &mut self.child;
                ctx.vars.with_context_var(self.var, &self.value, || child.event(ctx, args));
            }
            #[inline(always)]
            fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                let child = &mut self.child;
                ctx.vars
                    .with_context_var(self.var, &self.value, || child.measure(ctx, available_size))
            }
            #[inline(always)]
            fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
                let child = &mut self.child;
                ctx.vars
                    .with_context_var(self.var, &self.value, || child.arrange(ctx, widget_offset, final_size));
            }
            #[inline(always)]
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                let child = &self.child;
                ctx.vars.with_context_var(self.var, &self.value, || child.info(ctx, info));
            }
            #[inline(always)]
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                let child = &self.child;
                ctx.vars.with_context_var(self.var, &self.value, || child.render(ctx, frame));
            }
            #[inline(always)]
            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                let child = &self.child;
                ctx.vars
                    .with_context_var(self.var, &self.value, || child.render_update(ctx, update));
            }
        }
        WithContextVarNode {
            child,
            var,
            value: ContextVarSourceVar(value.into_var()),
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
            value: ContextVarSourceVar<V>,
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
                let child = &mut self.child;
                ctx.vars.with_context_var_wgt_only(self.var, &self.value, || child.init(ctx));
            }
            #[inline(always)]
            fn deinit(&mut self, ctx: &mut WidgetContext) {
                let child = &mut self.child;
                ctx.vars.with_context_var_wgt_only(self.var, &self.value, || child.deinit(ctx));
            }
            #[inline(always)]
            fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
                let child = &self.child;
                ctx.vars
                    .with_context_var_wgt_only(self.var, &self.value, || child.info(ctx, widget));
            }
            #[inline(always)]
            fn update(&mut self, ctx: &mut WidgetContext) {
                let child = &mut self.child;
                ctx.vars.with_context_var_wgt_only(self.var, &self.value, || child.update(ctx));
            }
            #[inline(always)]
            fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                let child = &mut self.child;
                ctx.vars.with_context_var_wgt_only(self.var, &self.value, || child.event(ctx, args));
            }
            #[inline(always)]
            fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                let child = &mut self.child;
                ctx.vars
                    .with_context_var_wgt_only(self.var, &self.value, || child.measure(ctx, available_size))
            }
            #[inline(always)]
            fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
                let child = &mut self.child;
                ctx.vars
                    .with_context_var_wgt_only(self.var, &self.value, || child.arrange(ctx, widget_offset, final_size))
            }
            #[inline(always)]
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                ctx.vars
                    .with_context_var_wgt_only(self.var, &self.value, || self.child.render(ctx, frame));
            }
            #[inline(always)]
            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                ctx.vars
                    .with_context_var_wgt_only(self.var, &self.value, || self.child.render_update(ctx, update));
            }
        }
        WithContextVarWidgetOnlyNode {
            child,
            var,
            value: ContextVarSourceVar(value.into_var()),
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
            item: V,
            fold: F,
            value: ContextVarSourceCustom<C::Type>,
            var_ver: u32,
            item_ver: u32,
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
                self.value.update_mask.set(C::new().update_mask(ctx) | self.item.update_mask(ctx));
                ctx.vars.with_context_var(self.var, &self.value, || self.child.info(ctx, info));
            }
            #[inline(always)]
            fn init(&mut self, ctx: &mut WidgetContext) {
                let var = C::new();
                self.value.value = (self.fold)(var.get_clone(ctx), self.item.get(ctx));
                self.value.version = 0;
                self.item_ver = self.item.version(ctx);
                self.var_ver = var.version(ctx);

                ctx.vars.with_context_var(self.var, &self.value, || self.child.init(ctx));
            }
            #[inline(always)]
            fn deinit(&mut self, ctx: &mut WidgetContext) {
                let child = &mut self.child;
                ctx.vars.with_context_var(self.var, &self.value, || child.deinit(ctx));
            }
            #[inline(always)]
            fn update(&mut self, ctx: &mut WidgetContext) {
                let child = &mut self.child;

                let var = C::new();
                let var_ver = var.version(ctx);
                let item_ver = self.item.version(ctx);
                self.value.is_new = var.is_new(ctx) || self.item.is_new(ctx);

                if self.value.is_new || self.var_ver != var_ver || self.item_ver != item_ver {
                    self.var_ver = var_ver;
                    self.item_ver = item_ver;
                    self.value.value = (self.fold)(var.get_clone(ctx), self.item.get(ctx));
                    self.value.version = self.value.version.wrapping_add(1);
                }

                ctx.vars.with_context_var(self.var, &self.value, || child.update(ctx));

                self.value.is_new = false;
            }
            #[inline(always)]
            fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
                let child = &mut self.child;
                ctx.vars.with_context_var(self.var, &self.value, || child.event(ctx, args));
            }
            #[inline(always)]
            fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                let child = &mut self.child;
                ctx.vars
                    .with_context_var(self.var, &self.value, || child.measure(ctx, available_size))
            }
            #[inline(always)]
            fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
                let child = &mut self.child;
                ctx.vars
                    .with_context_var(self.var, &self.value, || child.arrange(ctx, widget_offset, final_size));
            }

            #[inline(always)]
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                let child = &self.child;
                ctx.vars.with_context_var(self.var, &self.value, || child.render(ctx, frame));
            }
            #[inline(always)]
            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                let child = &self.child;
                ctx.vars
                    .with_context_var(self.var, &self.value, || child.render_update(ctx, update));
            }
        }
        WithContextVarFoldNode {
            child,
            var,
            item: item.into_var(),
            value: ContextVarSourceCustom {
                value: C::default_value(),
                is_new: false,
                version: 0,
                update_mask: Cell::new(UpdateMask::none()),
            },
            fold,
            var_ver: 0,
            item_ver: 0,
            _t: PhantomData,
        }
    }
}
#[doc(inline)]
pub use properties::*;
