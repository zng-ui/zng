use crate::var::types;
use std::thread::LocalKey;

use super::*;

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
///     pub static FOO_VAR: bool = false;
/// }
///
/// struct FooNode<C> { child: C }
///
/// #[impl_ui_node(child)]
/// impl<C: UiNode> UiNode for FooNode<C> {
///     fn init(&mut self, ctx: &mut WidgetContext) {
///         // `FOO_VAR` is `true` inside `FooNode::init`.
///         ctx.vars.with_context_var(FOO_VAR, ContextVarData::fixed(&true), || {
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
            parent: None,
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
    parent: Option<&'static LocalKey<ContextVarValue<T>>>,
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
pub struct ContextVarValue<T: VarValue> {
    parent: Cell<Option<&'static LocalKey<Self>>>,

    _default_value: Box<T>,
    default_value_fn: fn() -> T,
    value: Cell<*const T>,
    is_new: Cell<bool>,
    version: Cell<VarVersion>,
    is_read_only: Cell<bool>,
    is_animating: Cell<bool>,
    update_mask: Cell<UpdateMask>,
    actual_var: Cell<Option<*mut DynActualVarFn<T>>>,
}

type DynActualVarFn<T> = dyn Fn(&Vars) -> BoxedVar<T>;

#[allow(missing_docs)]
impl<T: VarValue> ContextVarValue<T> {
    pub fn init(default_value_fn: fn() -> T) -> Self {
        let default_value = Box::new(default_value_fn());
        ContextVarValue {
            parent: Cell::new(None),

            value: Cell::new(default_value.as_ref()),
            _default_value: default_value,
            default_value_fn,

            is_new: Cell::new(false),
            version: Cell::new(VarVersion::normal(0)),
            is_read_only: Cell::new(true),
            is_animating: Cell::new(false),
            update_mask: Cell::new(UpdateMask::none()),
            actual_var: Cell::new(None),
        }
    }

    pub fn derive(parent: &'static LocalKey<Self>) -> Self {
        let r = Self::init(parent.with(|l| l.default_value_fn));
        r.parent.set(Some(parent));
        r
    }

    fn default_value(&self) -> T {
        (self.default_value_fn)()
    }

    fn current_value_ptr(&self) -> *const T {
        if let Some(parent) = self.parent.get() {
            parent.with(Self::current_value_ptr)
        } else {
            self.value.get()
        }
    }

    fn current_is_new(&self) -> bool {
        if let Some(parent) = self.parent.get() {
            parent.with(Self::current_is_new)
        } else {
            self.is_new.get()
        }
    }

    fn current_is_animating(&self) -> bool {
        if let Some(parent) = self.parent.get() {
            parent.with(Self::current_is_animating)
        } else {
            self.is_animating.get()
        }
    }

    fn current_is_read_only(&self) -> bool {
        if let Some(parent) = self.parent.get() {
            parent.with(Self::current_is_read_only)
        } else {
            self.is_read_only.get()
        }
    }

    fn current_version(&self) -> VarVersion {
        if let Some(parent) = self.parent.get() {
            parent.with(Self::current_version)
        } else {
            self.version.get()
        }
    }

    fn current_actual_var_fn(&self) -> Option<*mut DynActualVarFn<T>> {
        if let Some(parent) = self.parent.get() {
            parent.with(Self::current_actual_var_fn)
        } else {
            self.actual_var.get()
        }
    }

    fn current_update_mask(&self) -> UpdateMask {
        if let Some(parent) = self.parent.get() {
            parent.with(Self::current_update_mask)
        } else {
            self.update_mask.get()
        }
    }
}

///<span data-del-macro-root></span> Declares new [`ContextVar`] keys.
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
///     pub static FOO_VAR: u8 = 10;
///
///     // A private context var.
///     static BAR_VAR: NotConst = init_val();
///
///     // A var that inherits from another.
///     pub static DERIVED_VAR: u8 => FOO_VAR;
/// }
/// ```
///
/// # Default Value
///
/// All context variable have a default fallback value that is used when the variable is not setted in the context.
///
/// The default value is instantiated once per app thread and is the value of the variable when it is not set in the context.
/// Other instances of the default value can be created by calls to [`ContextVar::default_value`], so the code after the `=` should
/// always generate a value equal to the first value generated, the value is automatically converted `Into<T>` the variable `T`, so
/// you can use the same conversions available when initializing properties to define the default.
///
/// # Inherit
///
/// Instead of setting a default value you can *inherit* from another context var of the same type using the syntax `=> OTHER;`, the
/// generated context var will represent the `OTHER` context var in contexts it is not set directly.
///
/// # Naming Convention
///
/// It is recommended that the type name ends with the `_VAR` suffix.
#[macro_export]
macro_rules! context_var {
    ($(
        $(#[$attr:meta])*
        $vis:vis static $NAME:ident: $Type:ty $(=> $PARENT:path)? $(= $default:expr)?;
    )+) => {$(
        $crate::__context_var! {
            $(#[$attr])*
            $vis static $NAME: $Type $(=> $PARENT)? $(= $default)?;
        }
    )+}
}
#[doc(inline)]
pub use crate::context_var;

#[doc(hidden)]
#[macro_export]
macro_rules! __context_var {
    (
        $(#[$attr:meta])*
        $vis:vis static $NAME:ident : $Type:ty => $PARENT:path;
    ) => {
        paste::paste! {
            std::thread_local! {
                #[doc(hidden)]
                static [<$NAME _LOCAL>]: $crate::var::ContextVarValue<$Type> = $PARENT.derive_local();
            }
        }

        $(#[$attr])*
        $vis static $NAME: $crate::var::ContextVar<$Type> = paste::paste! { $crate::var::ContextVar::new(&[<$NAME _LOCAL>]) };
    };
    (
        $(#[$attr:meta])*
        $vis:vis static $NAME:ident : $Type:ty = $default:expr;
    ) => {
        paste::paste! {
            std::thread_local! {
                #[doc(hidden)]
                static [<$NAME _LOCAL>]: $crate::var::ContextVarValue<$Type> = $crate::var::ContextVarValue::init([<$NAME:lower _default>]);
            }

            fn [<$NAME:lower _default>]() -> $Type {
                std::convert::Into::into($default)
            }
        }

        $(#[$attr])*
        $vis static $NAME: $crate::var::ContextVar<$Type> = paste::paste! { $crate::var::ContextVar::new(&[<$NAME _LOCAL>]) };
    };
}

/// The [`ContextVar<T>`] into read-only var type.
///
/// Ensures that even if the context is set to a read-write variable it cannot be set.
pub type ReadOnlyContextVar<T> = types::ReadOnlyVar<T, ContextVar<T>>;

/// Represents a context var.
///
/// Context variables are [`Var<T>`] implementers that represent a contextual value, unlike other variables it does not own
/// the value it represents.
///
/// The context var can have different values when read in different contexts, also the [`VarVersion`] is always different
/// in different contexts. Context vars are mostly read-only but can be settable if bound to a read/write variable.
///
/// Use [`context_var!`] do declare.
pub struct ContextVar<T: VarValue> {
    local: &'static LocalKey<ContextVarValue<T>>,
}
impl<T: VarValue> Clone for ContextVar<T> {
    fn clone(&self) -> Self {
        Self { local: self.local }
    }
}
impl<T: VarValue> Copy for ContextVar<T> {}
impl<T: VarValue> ContextVar<T> {
    #[doc(hidden)]
    pub const fn new(local: &'static LocalKey<ContextVarValue<T>>) -> Self {
        Self { local }
    }

    #[doc(hidden)]
    pub fn derive_local(&self) -> ContextVarValue<T> {
        ContextVarValue::derive(self.local)
    }

    /// New default value.
    pub fn default_value(&self) -> T {
        self.local.with(ContextVarValue::default_value)
    }

    pub(super) fn current_version(&self) -> VarVersion {
        self.local.with(ContextVarValue::current_version)
    }

    pub(super) fn enter_context(&self, new: ContextVarDataRaw<T>) -> ContextVarDataRaw<T> {
        self.local.with(|l| ContextVarDataRaw {
            parent: l.parent.take(),
            value: l.value.replace(new.value),
            is_new: l.is_new.replace(new.is_new),
            is_read_only: l.is_read_only.replace(new.is_read_only),
            is_animating: l.is_animating.replace(new.is_animating),
            version: l.version.replace(new.version),
            update_mask: l.update_mask.replace(new.update_mask),
            actual_var: l.actual_var.replace(new.actual_var),
        })
    }

    pub(super) fn exit_context(&self, prev: ContextVarDataRaw<T>) {
        self.local.with(|l| {
            l.parent.set(prev.parent);
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
impl<T: VarValue> crate::private::Sealed for ContextVar<T> {}
impl<T: VarValue> any::AnyVar for ContextVar<T> {
    any_var_impls!(Var);
}
impl<T: VarValue> Var<T> for ContextVar<T> {
    type AsReadOnly = types::ReadOnlyVar<T, Self>;

    type Weak = NoneWeakVar<T>;

    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a T {
        let _vars = vars.as_ref();
        let ptr = self.local.with(ContextVarValue::current_value_ptr);
        // SAFETY: this is safe because the pointer is either 'static or a reference held by
        // Vars::with_context_var.
        unsafe { &*ptr }
    }

    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a T> {
        let vars = vars.as_ref();
        if self.is_new(vars) {
            Some(self.get(vars))
        } else {
            None
        }
    }

    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|_vars| self.local.with(ContextVarValue::current_is_new))
    }

    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> VarVersion {
        vars.with_vars_read(|_vars| self.local.with(ContextVarValue::current_version))
    }

    fn is_read_only<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|_vars| self.local.with(ContextVarValue::current_is_read_only))
    }

    fn always_read_only(&self) -> bool {
        false
    }

    fn is_contextual(&self) -> bool {
        true
    }

    fn actual_var<Vw: WithVars>(&self, vars: &Vw) -> BoxedVar<T> {
        vars.with_vars(|vars| {
            if let Some(actual) = self.local.with(ContextVarValue::current_actual_var_fn) {
                // SAFETY, we hold a ref to this closure box in the context.
                let actual = unsafe { &*actual };
                actual(vars)
            } else {
                let value = self.get_clone(vars);
                LocalVar(value).boxed()
            }
        })
    }

    fn is_rc(&self) -> bool {
        false
    }

    fn can_update(&self) -> bool {
        true
    }

    fn is_animating<Vr: WithVarsRead>(&self, vars: &Vr) -> bool {
        vars.with_vars_read(|_vars| self.local.with(ContextVarValue::current_is_animating))
    }

    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> T {
        self.get_clone(vars)
    }

    fn downgrade(&self) -> Option<Self::Weak> {
        None
    }

    fn strong_count(&self) -> usize {
        0
    }

    fn weak_count(&self) -> usize {
        0
    }

    fn as_ptr(&self) -> *const () {
        self.local as *const _ as _
    }

    fn modify<Vw, M>(&self, vars: &Vw, modify: M) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        M: FnOnce(VarModify<T>) + 'static,
    {
        vars.with_vars(|vars| {
            if let Some(actual) = self.local.with(ContextVarValue::current_actual_var_fn) {
                // SAFETY, we hold a ref to this closure box in the context.
                let actual = unsafe { &*actual };
                actual(vars).modify(vars, modify)
            } else {
                Err(VarIsReadOnly)
            }
        })
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        types::ReadOnlyVar::new(self)
    }

    fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
        vars.with_vars_read(|_vars| self.local.with(ContextVarValue::current_update_mask))
    }
}
impl<T: VarValue> IntoVar<T> for ContextVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

mod properties {
    use crate::{context::*, event::*, render::*, units::*, var::*, widget_info::*, *};

    /// Helper for declaring properties that sets a context var.
    ///
    /// The method presents the `value` as the [`ContextVar<T>`] in the widget and widget descendants.
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
    ///     pub static FOO_VAR: u32 = 0u32;
    /// }
    ///
    /// /// Sets the [`FooVar`] in the widgets and its content.
    /// #[property(context, default(FOO_VAR))]
    /// pub fn foo(child: impl UiNode, value: impl IntoVar<u32>) -> impl UiNode {
    ///     with_context_var(child, FOO_VAR, value)
    /// }
    /// ```
    ///
    /// When set in a widget, the `value` is accessible in all inner nodes of the widget, using `FOO_VAR.get`, and if `value` is set to a
    /// variable the `FOO_VAR` will also reflect its [`is_new`] and [`is_read_only`]. If the `value` var is not read-only inner nodes
    /// can modify it using `FOO_VAR.set` or `FOO_VAR.modify`.
    ///
    /// Also note that the property [`default`] is set to the same `FOO_VAR`, this causes the property to *pass-through* the outer context
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
    ///     pub static CONFIG_VAR: Config = Config::default();
    /// }
    ///
    /// /// Sets the *foo* config.
    /// #[property(context, default(false))]
    /// pub fn foo(child: impl UiNode, value: impl IntoVar<bool>) -> impl UiNode {
    ///     with_context_var(child, CONFIG_VAR, merge_var!(CONFIG_VAR, value.into_var(), |c, &v| {
    ///         let mut c = c.clone();
    ///         c.foo = v;
    ///         c
    ///     }))
    /// }
    ///
    /// /// Sets the *bar* config.
    /// #[property(context, default(false))]
    /// pub fn bar(child: impl UiNode, value: impl IntoVar<bool>) -> impl UiNode {
    ///     with_context_var(child, CONFIG_VAR, merge_var!(CONFIG_VAR, value.into_var(), |c, &v| {
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
    pub fn with_context_var<T: VarValue>(child: impl UiNode, var: ContextVar<T>, value: impl IntoVar<T>) -> impl UiNode {
        struct WithContextVarNode<U, T: VarValue, V> {
            child: U,
            var: ContextVar<T>,
            value: V,
        }
        impl<U, T, V> WithContextVarNode<U, T, V>
        where
            U: UiNode,
            T: VarValue,
            V: Var<T>,
        {
            fn with<R>(&self, vars: &VarsRead, f: impl FnOnce(&U) -> R) -> R {
                vars.with_context_var(self.var, ContextVarData::in_vars_read(vars, &self.value), || f(&self.child))
            }

            fn with_mut<R>(&mut self, vars: &Vars, f: impl FnOnce(&mut U) -> R) -> R {
                vars.with_context_var(self.var, ContextVarData::in_vars(vars, &self.value, false), || f(&mut self.child))
            }
        }
        impl<U, T, V> UiNode for WithContextVarNode<U, T, V>
        where
            U: UiNode,
            T: VarValue,
            V: Var<T>,
        {
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                self.with(ctx.vars, |c| c.info(ctx, info));
            }

            fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
                self.with(ctx.vars, |c| c.subscriptions(ctx, subs));
            }

            fn init(&mut self, ctx: &mut WidgetContext) {
                self.with_mut(ctx.vars, |c| c.init(ctx));
            }

            fn deinit(&mut self, ctx: &mut WidgetContext) {
                self.with_mut(ctx.vars, |c| c.deinit(ctx));
            }

            fn update(&mut self, ctx: &mut WidgetContext) {
                self.with_mut(ctx.vars, |c| c.update(ctx));
            }

            fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
                self.with_mut(ctx.vars, |c| c.event(ctx, args));
            }

            fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
                self.with(ctx.vars, |c| c.measure(ctx))
            }

            fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                self.with_mut(ctx.vars, |c| c.layout(ctx, wl))
            }

            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                self.with(ctx.vars, |c| c.render(ctx, frame));
            }

            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                self.with(ctx.vars, |c| c.render_update(ctx, update));
            }
        }
        WithContextVarNode {
            child: child.cfg_boxed(),
            var,
            value: value.into_var(),
        }
        .cfg_boxed()
    }

    /// Helper for declaring properties that sets a context var to a value generated on init.
    ///
    /// The method calls the `init_value` closure on init to produce a *value* var that is presented as the [`ContextVar<T>`]
    /// in the widget and widget descendants. The closure can be called more than once if the returned node is reinited.
    ///
    /// Apart from the value initialization this behaves just like [`with_context_var`].
    pub fn with_context_var_init<T: VarValue, V: Var<T>>(
        child: impl UiNode,
        var: ContextVar<T>,
        init_value: impl FnMut(&mut WidgetContext) -> V + 'static,
    ) -> impl UiNode {
        struct WithContextVarInitNode<U, T: VarValue, I, V> {
            child: U,
            var: ContextVar<T>,
            init_value: I,
            value: Option<V>,
        }
        impl<U, T, I, V> WithContextVarInitNode<U, T, I, V>
        where
            U: UiNode,
            T: VarValue,
            I: FnMut(&mut WidgetContext) -> V + 'static,
            V: Var<T>,
        {
            fn with<R>(&self, vars: &VarsRead, f: impl FnOnce(&U) -> R) -> R {
                let value = self.value.as_ref().expect("with_context_var_init not inited");
                vars.with_context_var(self.var, ContextVarData::in_vars_read(vars, value), || f(&self.child))
            }

            fn with_mut<R>(&mut self, vars: &Vars, f: impl FnOnce(&mut U) -> R) -> R {
                let value = self.value.as_ref().expect("with_context_var_init not inited");
                vars.with_context_var(self.var, ContextVarData::in_vars(vars, value, false), || f(&mut self.child))
            }
        }
        impl<U, T, I, V> UiNode for WithContextVarInitNode<U, T, I, V>
        where
            U: UiNode,
            T: VarValue,
            I: FnMut(&mut WidgetContext) -> V + 'static,
            V: Var<T>,
        {
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                self.with(ctx.vars, |c| c.info(ctx, info));
            }

            fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
                self.with(ctx.vars, |c| c.subscriptions(ctx, subs));
            }

            fn init(&mut self, ctx: &mut WidgetContext) {
                self.value = Some((self.init_value)(ctx));
                self.with_mut(ctx.vars, |c| c.init(ctx));
            }

            fn deinit(&mut self, ctx: &mut WidgetContext) {
                self.with_mut(ctx.vars, |c| c.deinit(ctx));
                self.value = None;
            }

            fn update(&mut self, ctx: &mut WidgetContext) {
                self.with_mut(ctx.vars, |c| c.update(ctx));
            }

            fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
                self.with_mut(ctx.vars, |c| c.event(ctx, args));
            }

            fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
                self.with(ctx.vars, |c| c.measure(ctx))
            }

            fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                self.with_mut(ctx.vars, |c| c.layout(ctx, wl))
            }

            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                self.with(ctx.vars, |c| c.render(ctx, frame));
            }

            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                self.with(ctx.vars, |c| c.render_update(ctx, update));
            }
        }
        WithContextVarInitNode {
            child: child.cfg_boxed(),
            var,
            init_value,
            value: None,
        }
        .cfg_boxed()
    }
}
#[doc(inline)]
pub use properties::*;

#[cfg(test)]
mod tests {
    use crate::{app::*, context::*, text::*, var::*, *};

    context_var! {
        static TEST_VAR: Text = "";
    }

    static PROBE_ID: StaticStateId<Text> = StaticStateId::new_unique();

    #[property(context, default(TEST_VAR))]
    fn test_prop(child: impl UiNode, value: impl IntoVar<Text>) -> impl UiNode {
        with_context_var(child, TEST_VAR, value)
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
                ctx.app_state.set(&PROBE_ID, self.var.get_clone(ctx.vars));
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

            fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut widget_info::WidgetSubscriptions) {
                subs.handler(&self.handler);
                self.child.subscriptions(ctx, subs);
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
        Windows::req(app.ctx().services).open(move |_| crate::window::Window::new_test(root));
        let _ = app.update(false);
        app
    }

    #[test]
    fn context_var_basic() {
        let mut test = test_app(test_wgt! {
            test_prop = "test!";

            child = test_wgt! {
                probe = TEST_VAR;
            }
        });

        assert_eq!(test.ctx().app_state.get(&PROBE_ID), Some(&Text::from("test!")));
    }

    #[test]
    fn context_var_map() {
        let mut test = test_app(test_wgt! {
            test_prop = "test!";

            child = test_wgt! {
                probe = TEST_VAR.map(|t| formatx!("map {t}"));
            }
        });

        assert_eq!(test.ctx().app_state.get(&PROBE_ID), Some(&Text::from("map test!")));
    }

    #[test]
    fn context_var_map_cloned() {
        // mapped context var should depend on the context.

        let mapped = TEST_VAR.map(|t| formatx!("map {t}"));
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

        assert_eq!(test.ctx().app_state.get(&PROBE_ID), Some(&Text::from("map B!")));
    }

    #[test]
    fn context_var_map_cloned3() {
        // mapped context var should depend on the context.

        let mapped = TEST_VAR.map(|t| formatx!("map {t}"));
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

        assert_eq!(test.ctx().app_state.get(&PROBE_ID), Some(&Text::from("map C!")));
    }

    #[test]
    fn context_var_map_not_cloned() {
        // sanity check for `context_var_map_cloned`

        use self::test_prop as test_prop_a;
        use self::test_prop as test_prop_b;

        let mut test = test_app(test_wgt! {
            test_prop_a = "A!";

            child = test_wgt! {
                probe = TEST_VAR.map(|t| formatx!("map {t}"));
                test_prop_b = "B!";

                child = test_wgt! {
                    probe = TEST_VAR.map(|t| formatx!("map {t}"));
                }
            }
        });

        assert_eq!(test.ctx().app_state.get(&PROBE_ID), Some(&Text::from("map B!")));
    }

    #[test]
    fn context_var_map_moved_app_ctx() {
        // need to support different value using the same variable instance too.

        let mapped = TEST_VAR.map(|t| formatx!("map {t}"));

        let mut app = test_app(NilUiNode);
        let ctx = app.ctx();

        let a = ctx
            .vars
            .with_context_var(TEST_VAR, ContextVarData::fixed(&"A".into()), || mapped.get_clone(ctx.vars));
        let b = ctx
            .vars
            .with_context_var(TEST_VAR, ContextVarData::fixed(&"B".into()), || mapped.get_clone(ctx.vars));

        assert_ne!(a, b);
    }

    #[test]
    fn context_var_cloned_same_widget() {
        let mapped = TEST_VAR.map(|t| formatx!("map {t}"));
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

        assert_eq!(test.ctx().app_state.get(&PROBE_ID), Some(&Text::from("map B!")));
    }

    #[test]
    fn context_var_set() {
        let mut app = test_app(NilUiNode);

        let backing_var = var(Text::from(""));

        let ctx = app.ctx();
        ctx.vars
            .with_context_var(TEST_VAR, ContextVarData::in_vars(ctx.vars, &backing_var, false), || {
                let t = TEST_VAR;
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
                TEST_VAR.bind(ctx, &other_var).perm();
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
