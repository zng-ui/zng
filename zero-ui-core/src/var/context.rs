use std::thread::LocalKey;

use super::*;

///<span data-del-macro-root></span> Declares new [`ContextVar`] keys.
///
/// # Examples
///
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
///     // A var that *inherits* from another.
///     pub static DERIVED_VAR: u8 = FOO_VAR;
/// }
/// ```
///
/// # Default Value
///
/// All context variable have a default fallback value that is used when the variable is not setted in the context.
///
/// The default value is instantiated once per app thread and is the value of the variable when it is not set in the context,
/// any value [`IntoVar<T>`] is allowed, meaning other variables are supported, you can use this to *inherit* from another
/// context variable, when the context fallback to default the other context var is used, it can have a value or fallback to
/// it's default too.
///
/// # Naming Convention
///
/// It is recommended that the type name ends with the `_VAR` suffix.
///
/// # Context Only
///
/// Note that if you are only interested in sharing a contextual value you can use the [`context_value!`] macro instead.
#[macro_export]
macro_rules! context_var {
    ($(
        $(#[$attr:meta])*
        $vis:vis static $NAME:ident: $Type:ty = $default:expr;
    )+) => {$(
        $crate::paste! {
            std::thread_local! {
                #[doc(hidden)]
                static [<$NAME _LOCAL>]: $crate::var::types::ContextData<$Type> = $crate::var::types::ContextData::init($default);
            }
        }

        $(#[$attr])*
        $vis static $NAME: $crate::var::ContextVar<$Type> = paste::paste! { $crate::var::ContextVar::new(&[<$NAME _LOCAL>]) };
    )+}
}
#[doc(inline)]
pub use crate::context_var;

struct ContextEntry<T> {
    var: BoxedVar<T>,
    busy: Cell<bool>,
}

#[doc(hidden)]
pub struct ContextData<T: VarValue> {
    stack: RefCell<Vec<ContextEntry<T>>>,
}
impl<T: VarValue> ContextData<T> {
    pub fn init(default: impl IntoVar<T>) -> Self {
        Self {
            stack: RefCell::new(vec![ContextEntry {
                var: default.into_var().boxed(),
                busy: Cell::new(false),
            }]),
        }
    }
}

/// Represents another variable in a context.
///
/// Context variables are [`Var<T>`] implementers that represent a contextual value, unlike other variables it does not own
/// the value it represents.
///
/// See [`context_var!`] for more details.
pub struct ContextVar<T: VarValue> {
    local: &'static LocalKey<ContextData<T>>,
}

impl<T: VarValue> ContextVar<T> {
    #[doc(hidden)]
    pub const fn new(local: &'static LocalKey<ContextData<T>>) -> Self {
        ContextVar { local }
    }

    /// Runs `action` with this context var representing the other `var` in the current thread.
    ///
    /// Returns the actual var that was used and the result of `action`.
    ///
    /// Note that the `var` must be the same for subsequent calls in the same *context*, otherwise [contextualized]
    /// variables may not update their binding, in widgets you must re-init the descendants if you replace the `var`.
    ///
    /// [contextualized]: types::ContextualizedVar
    pub fn with_context<R>(self, var: impl IntoVar<T>, action: impl FnOnce() -> R) -> (BoxedVar<T>, R) {
        self.push_context(var.into_var().actual_var().boxed());
        let r = action();
        let var = self.pop_context();
        (var, r)
    }

    fn push_context(self, var: BoxedVar<T>) {
        self.local.with(move |l| {
            l.stack.borrow_mut().push(ContextEntry {
                var,
                busy: Cell::new(false),
            })
        })
    }

    fn pop_context(self) -> BoxedVar<T> {
        self.local.with(move |l| l.stack.borrow_mut().pop()).unwrap().var
    }

    fn with_var<R>(self, f: impl FnOnce(&BoxedVar<T>) -> R) -> R {
        let i = self.lock_idx();
        let r = self.local.with(move |l| f(&l.stack.borrow()[i].var));
        self.free_idx(i);
        r
    }

    fn lock_idx(self) -> usize {
        let i = self.local.with(|l| {
            let stack = l.stack.borrow();
            stack.iter().rposition(|f| !f.busy.replace(true))
        });
        i.expect("context var recursion in default value")
    }

    fn free_idx(self, i: usize) {
        self.local.with(|l| l.stack.borrow()[i].busy.set(false));
    }
}

impl<T: VarValue> Clone for ContextVar<T> {
    fn clone(&self) -> Self {
        Self { local: self.local }
    }
}
impl<T: VarValue> Copy for ContextVar<T> {}

impl<T: VarValue> crate::private::Sealed for ContextVar<T> {}

impl<T: VarValue> AnyVar for ContextVar<T> {
    fn clone_any(&self) -> BoxedAnyVar {
        Box::new(*self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn double_boxed_any(self: Box<Self>) -> Box<dyn Any> {
        let me: BoxedVar<T> = self;
        Box::new(me)
    }

    fn var_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn get_any(&self) -> Box<dyn AnyVarValue> {
        Box::new(self.get())
    }

    fn set_any(&self, vars: &Vars, value: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        self.modify(vars, var_set_any(value))
    }

    fn last_update(&self) -> VarUpdateId {
        self.with_var(AnyVar::last_update)
    }

    fn capabilities(&self) -> VarCapabilities {
        self.with_var(AnyVar::capabilities) | VarCapabilities::CAPS_CHANGE
    }

    fn hook(&self, pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool>) -> VarHandle {
        self.with_var(|v| v.hook(pos_modify_action))
    }

    fn subscribe(&self, widget_id: WidgetId) -> VarHandle {
        self.with_var(|v| v.subscribe(widget_id))
    }

    fn strong_count(&self) -> usize {
        self.with_var(AnyVar::strong_count)
    }

    fn weak_count(&self) -> usize {
        self.with_var(AnyVar::weak_count)
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        self.with_var(AnyVar::actual_var_any)
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        self.with_var(AnyVar::downgrade_any)
    }

    fn is_animating(&self) -> bool {
        self.with_var(AnyVar::is_animating)
    }

    fn var_ptr(&self) -> VarPtr {
        VarPtr::new_thread_local(self.local)
    }
}

impl<T: VarValue> IntoVar<T> for ContextVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue> Var<T> for ContextVar<T> {
    type ReadOnly = types::ReadOnlyVar<T, Self>;

    type ActualVar = BoxedVar<T>;

    type Downgrade = BoxedWeakVar<T>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.with_var(move |v| v.with(read))
    }

    fn modify<V, F>(&self, vars: &V, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        V: WithVars,
        F: FnOnce(&mut VarModifyValue<T>) + 'static,
    {
        self.with_var(move |v| v.modify(vars, modify))
    }

    fn actual_var(self) -> BoxedVar<T> {
        self.with_var(|v| v.clone().actual_var())
    }

    fn downgrade(&self) -> BoxedWeakVar<T> {
        self.with_var(Var::downgrade)
    }

    fn into_value(self) -> T {
        self.get()
    }

    fn read_only(&self) -> Self::ReadOnly {
        types::ReadOnlyVar::new(*self)
    }
}

/// Context var that is always read-only, even if it is representing a read-write var.
pub type ReadOnlyContextVar<T> = types::ReadOnlyVar<T, ContextVar<T>>;

pub use helpers::*;
mod helpers {
    use std::cell::RefCell;

    use crate::{context::*, event::*, render::*, var::*, widget_info::*, widget_instance::*, *};

    /// Helper for declaring properties that sets a context var.
    ///
    /// The method presents the `value` as the [`ContextVar<T>`] in the widget and widget descendants.
    /// The context var [`is_new`] and [`read_only`] status are always equal to the `value` var status. Users
    /// of the context var can also retrieve the `value` var using [`actual_var`].
    ///
    /// The generated [`UiNode`] delegates each method to `child` inside a call to [`ContextVar::with_context`].
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
    /// variable the `FOO_VAR` will also reflect its [`is_new`] and [`read_only`]. If the `value` var is not read-only inner nodes
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
    /// [`read_only`]: Var::read_only
    /// [`actual_var`]: Var::actual_var
    /// [`default`]: crate::property#default
    pub fn with_context_var<T: VarValue>(child: impl UiNode, context_var: ContextVar<T>, value: impl IntoVar<T>) -> impl UiNode {
        #[ui_node(struct WithContextVarNode<T: VarValue> {
            child: impl UiNode,
            context_var: ContextVar<T>,
            value: RefCell<Option<BoxedVar<T>>>,
        })]
        impl WithContextVarNode {
            fn with<R>(&self, mtd: impl FnOnce(&T_child) -> R) -> R {
                let mut value = self.value.borrow_mut();
                let var = value.take().unwrap();
                let (var, r) = self.context_var.with_context(var, move || mtd(&self.child));
                *value = Some(var);
                r
            }

            fn with_mut<R>(&mut self, mtd: impl FnOnce(&mut T_child) -> R) -> R {
                let var = self.value.get_mut().take().unwrap();
                let (var, r) = self.context_var.with_context(var, || mtd(&mut self.child));
                *self.value.get_mut() = Some(var);
                r
            }

            #[UiNode]
            fn init(&mut self, ctx: &mut WidgetContext) {
                self.with_mut(|c| c.init(ctx))
            }

            #[UiNode]
            fn deinit(&mut self, ctx: &mut WidgetContext) {
                self.with_mut(|c| c.deinit(ctx))
            }

            #[UiNode]
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                self.with(|c| c.info(ctx, info))
            }

            #[UiNode]
            fn event(&mut self, ctx: &mut WidgetContext, update: &mut crate::event::EventUpdate) {
                self.with_mut(|c| c.event(ctx, update))
            }

            #[UiNode]
            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                self.with_mut(|c| c.update(ctx, updates))
            }

            #[UiNode]
            fn measure(&self, ctx: &mut MeasureContext) -> units::PxSize {
                self.with(|c| c.measure(ctx))
            }

            #[UiNode]
            fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> units::PxSize {
                self.with_mut(|c| c.layout(ctx, wl))
            }

            #[UiNode]
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                self.with(|c| c.render(ctx, frame))
            }

            #[UiNode]
            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                self.with(|c| c.render_update(ctx, update))
            }
        }
        WithContextVarNode {
            child: child.cfg_boxed(),
            context_var,
            value: RefCell::new(Some(value.into_var().boxed())),
        }
    }

    /// Helper for declaring properties that sets a context var to a value generated on init.
    ///
    /// The method calls the `init_value` closure on init to produce a *value* var that is presented as the [`ContextVar<T>`]
    /// in the widget and widget descendants. The closure can be called more than once if the returned node is reinited.
    ///
    /// Apart from the value initialization this behaves just like [`with_context_var`].
    pub fn with_context_var_init<T: VarValue>(
        child: impl UiNode,
        var: ContextVar<T>,
        init_value: impl FnMut(&mut WidgetContext) -> BoxedVar<T> + 'static,
    ) -> impl UiNode {
        #[ui_node(struct WithContextVarInitNode<T: VarValue> {
            child: impl UiNode,
            context_var: ContextVar<T>,
            init_value: impl FnMut(&mut WidgetContext) -> BoxedVar<T> + 'static,
            value: RefCell<Option<BoxedVar<T>>>,
        })]
        impl WithContextVarInitNode {
            fn with<R>(&self, mtd: impl FnOnce(&T_child) -> R) -> R {
                let mut value = self.value.borrow_mut();
                let var = value.take().unwrap();
                let (var, r) = self.context_var.with_context(var, move || mtd(&self.child));
                *value = Some(var);
                r
            }

            fn with_mut<R>(&mut self, mtd: impl FnOnce(&mut T_child) -> R) -> R {
                let var = self.value.get_mut().take().unwrap();
                let (var, r) = self.context_var.with_context(var, || mtd(&mut self.child));
                *self.value.get_mut() = Some(var);
                r
            }

            #[UiNode]
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                self.with(|c| c.info(ctx, info));
            }

            #[UiNode]
            fn init(&mut self, ctx: &mut WidgetContext) {
                *self.value.get_mut() = Some((self.init_value)(ctx));
                self.with_mut(|c| c.init(ctx));
            }

            #[UiNode]
            fn deinit(&mut self, ctx: &mut WidgetContext) {
                self.with_mut(|c| c.deinit(ctx));
                *self.value.get_mut() = None;
            }

            #[UiNode]
            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                self.with_mut(|c| c.update(ctx, updates));
            }

            #[UiNode]
            fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
                self.with_mut(|c| c.event(ctx, update));
            }

            #[UiNode]
            fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
                self.with(|c| c.measure(ctx))
            }

            #[UiNode]
            fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                self.with_mut(|c| c.layout(ctx, wl))
            }

            #[UiNode]
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                self.with(|c| c.render(ctx, frame));
            }

            #[UiNode]
            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                self.with(|c| c.render_update(ctx, update));
            }
        }
        WithContextVarInitNode {
            child: child.cfg_boxed(),
            context_var: var,
            init_value,
            value: RefCell::new(None),
        }
        .cfg_boxed()
    }
}
