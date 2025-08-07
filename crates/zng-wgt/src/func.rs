use std::{fmt, ops, sync::Arc};

use crate::prelude::*;

use zng_app::event::{CommandMetaVar, CommandMetaVarId};
use zng_var::AnyVar;
#[doc(hidden)]
pub use zng_wgt::prelude::clmv as __clmv;

type BoxedWgtFn<D> = Box<dyn Fn(D) -> UiNode + Send + Sync>;

/// Boxed shared closure that generates a widget for a given data.
///
/// You can also use the [`wgt_fn!`] macro do instantiate.
///
/// See `presenter` for a way to quickly use the widget function in the UI.
pub struct WidgetFn<D: ?Sized>(Option<Arc<BoxedWgtFn<D>>>);
impl<D> Clone for WidgetFn<D> {
    fn clone(&self) -> Self {
        WidgetFn(self.0.clone())
    }
}
impl<D> fmt::Debug for WidgetFn<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WidgetFn<{}>", pretty_type_name::pretty_type_name::<D>())
    }
}
impl<D> PartialEq for WidgetFn<D> {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}
impl<D> Default for WidgetFn<D> {
    /// `nil`.
    fn default() -> Self {
        Self::nil()
    }
}
impl<D> WidgetFn<D> {
    /// New from a closure that generates a node from data.
    pub fn new(func: impl Fn(D) -> UiNode + Send + Sync + 'static) -> Self {
        WidgetFn(Some(Arc::new(Box::new(func))))
    }

    /// Function that always produces the [`NilUiNode`].
    ///
    /// No heap allocation happens to create this value.
    ///
    /// [`NilUiNode`]: zng_app::widget::node::NilUiNode
    pub const fn nil() -> Self {
        WidgetFn(None)
    }

    /// If this is the [`nil`] function.
    ///
    /// If `true` the function always generates a node that is [`UiNode::is_nil`], if
    /// `false` the function may still return a nil node some of the time.
    ///
    /// See [`call_checked`] for more details.
    ///
    /// [`nil`]: WidgetFn::nil
    /// [`call_checked`]: Self::call_checked
    /// [`UiNode::is_nil`]: zng_app::widget::node::UiNode::is_nil
    pub fn is_nil(&self) -> bool {
        self.0.is_none()
    }

    /// Calls the function with `data` argument.
    ///
    /// Note that you can call the widget function directly where `D: 'static`:
    ///
    /// ```
    /// # use zng_wgt::WidgetFn;
    /// fn foo(func: &WidgetFn<bool>) {
    ///     let a = func.call(true);
    ///     let b = func(true);
    /// }
    /// ```
    ///
    /// In the example above `a` and `b` are both calls to the widget function.
    pub fn call(&self, data: D) -> UiNode {
        if let Some(g) = &self.0 { g(data) } else { UiNode::nil() }
    }

    /// Calls the function with `data` argument and only returns a node if is not nil.
    ///
    /// Returns `None` if [`is_nil`] or [`UiNode::is_nil`].
    ///
    /// [`is_nil`]: Self::is_nil
    /// [`UiNode::is_nil`]: zng_app::widget::node::UiNode::is_nil
    pub fn call_checked(&self, data: D) -> Option<UiNode> {
        let r = self.0.as_ref()?(data);
        if r.is_nil() { None } else { Some(r) }
    }

    /// New widget function that returns the same `widget` for every call.
    ///
    /// The `widget` is wrapped in an [`ArcNode`] and every function call returns an [`ArcNode::take_on_init`] node.
    /// Note that `take_on_init` is not always the `widget` on init as it needs to wait for it to deinit first if
    /// it is already in use, this could have an effect if the widget function caller always expects a full widget.
    ///
    /// [`ArcNode`]: zng_app::widget::node::ArcNode
    /// [`ArcNode::take_on_init`]: zng_app::widget::node::ArcNode::take_on_init
    pub fn singleton(widget: impl IntoUiNode) -> Self {
        let widget = ArcNode::new(widget);
        Self::new(move |_| widget.take_on_init())
    }

    /// Creates a [`WeakWidgetFn<D>`] reference to this function.
    pub fn downgrade(&self) -> WeakWidgetFn<D> {
        match &self.0 {
            Some(f) => WeakWidgetFn(Arc::downgrade(f)),
            None => WeakWidgetFn::nil(),
        }
    }
}
impl<D: 'static> ops::Deref for WidgetFn<D> {
    type Target = dyn Fn(D) -> UiNode;

    fn deref(&self) -> &Self::Target {
        match self.0.as_ref() {
            Some(f) => &**f,
            None => &nil_call::<D>,
        }
    }
}
fn nil_call<D>(_: D) -> UiNode {
    UiNode::nil()
}

/// Weak reference to a [`WidgetFn<D>`].
pub struct WeakWidgetFn<D>(std::sync::Weak<BoxedWgtFn<D>>);
impl<D> WeakWidgetFn<D> {
    /// New weak reference to nil.
    pub const fn nil() -> Self {
        WeakWidgetFn(std::sync::Weak::new())
    }

    /// If this weak reference only upgrades to a nil function.
    pub fn is_nil(&self) -> bool {
        self.0.strong_count() == 0
    }

    /// Upgrade to strong reference if it still exists or nil.
    pub fn upgrade(&self) -> WidgetFn<D> {
        match self.0.upgrade() {
            Some(f) => WidgetFn(Some(f)),
            None => WidgetFn::nil(),
        }
    }
}

/// <span data-del-macro-root></span> Declares a widget function closure.
///
/// The output type is a [`WidgetFn`], the closure is [`clmv!`].
///
/// # Syntax
///
/// * `wgt_fn!(cloned, |_args| Wgt!())` - Clone-move closure, the same syntax as [`clmv!`] you can
///   list the cloned values before the closure.
/// * `wgt_fn!(path::to::func)` - The macro also accepts unction, the signature must receive the args and return
///   a widget.
/// * `wgt_fn!()` - An empty call generates the [`WidgetFn::nil()`] value.
///
/// # Examples
///
/// Declares a basic widget function that ignores the argument and does not capture any value:
///
/// ```
/// # zng_wgt::enable_widget_macros!();
/// # use zng_wgt::{prelude::*, Wgt, on_init};
/// #
/// # fn main() {
/// # let wgt: WidgetFn<bool> =
/// wgt_fn!(|_| Wgt! {
///     on_init = hn!(|_| println!("generated widget init"));
/// });
/// # ; }
/// ```
///
/// The macro is clone-move, meaning you can use the same syntax as [`clmv!`] to capture clones of values:
///
/// ```
/// # zng_wgt::enable_widget_macros!();
/// # use zng_wgt::{prelude::*, Wgt};
/// # fn main() {
/// let moved_var = var('a');
/// let cloned_var = var('b');
///
/// # let wgt: WidgetFn<bool> =
/// wgt_fn!(cloned_var, |args| {
///     println!(
///         "wgt_fn, args: {:?}, moved_var: {}, cloned_var: {}",
///         args,
///         moved_var.get(),
///         cloned_var.get()
///     );
///     Wgt!()
/// });
/// # ; }
/// ```
///
/// [`clmv!`]: zng_clone_move::clmv
#[macro_export]
macro_rules! wgt_fn {
    ($fn:path) => {
        $crate::WidgetFn::new($fn)
    };
    ($($tt:tt)+) => {
        $crate::WidgetFn::new($crate::__clmv! {
            $($tt)+
        })
    };
    () => {
        $crate::WidgetFn::nil()
    };
}

/// Service that provides editor widgets for a given variable.
///
/// Auto generating widgets such as a settings list or a properties list can use this
/// service to instantiate widgets for each item.
///
/// The main crate registers some common editors.
pub struct EDITORS;
impl EDITORS {
    /// Register an `editor` handler.
    ///
    /// The handler must return [`NilUiNode`] if it cannot handle the request. Later added handlers are called first.
    pub fn register(&self, editor: WidgetFn<EditorRequestArgs>) {
        if !editor.is_nil() {
            UPDATES
                .run(async move {
                    EDITORS_SV.write().push(editor);
                })
                .perm();
        }
    }

    /// Register an `editor` handler to be called if none of the `register` editors can handle the value.
    ///
    /// The handler must return [`NilUiNode`] if it cannot handle the request. Later added handlers are called last.
    pub fn register_fallback(&self, editor: WidgetFn<EditorRequestArgs>) {
        if !editor.is_nil() {
            UPDATES
                .run(async move {
                    EDITORS_SV.write().push_fallback(editor);
                })
                .perm();
        }
    }

    /// Instantiate an editor for the `value`.
    ///
    /// Returns [`NilUiNode`] if no registered editor can handle the value type.
    pub fn get(&self, value: AnyVar) -> UiNode {
        EDITORS_SV.read().get(EditorRequestArgs { value })
    }

    /// Same as [`get`], but also logs an error is there are no available editor for the type.
    ///
    /// [`get`]: Self::get
    pub fn req<T: VarValue>(&self, value: Var<T>) -> UiNode {
        let e = self.get(value.into());
        if e.is_nil() {
            tracing::error!("no editor available for `{}`", std::any::type_name::<T>())
        }
        e
    }
}

/// Service that provides icon drawing widgets.
///
/// This service enables widgets to use icons in an optional way, without needing to bundle icon resources. It
/// also enables app wide icon theming.
pub struct ICONS;
impl ICONS {
    /// Register an `icon` handler.
    ///
    /// The handler must return [`NilUiNode`] if it cannot handle the request. Later added handlers are called first.
    pub fn register(&self, icon: WidgetFn<IconRequestArgs>) {
        if !icon.is_nil() {
            UPDATES
                .run(async move {
                    ICONS_SV.write().push(icon);
                })
                .perm();
        }
    }

    /// Register an `icon` handler to be called if none of the `register` handlers can handle request.
    ///
    /// The handler must return [`NilUiNode`] if it cannot handle the request. Later added handlers are called last.
    pub fn register_fallback(&self, icon: WidgetFn<IconRequestArgs>) {
        if !icon.is_nil() {
            UPDATES
                .run(async move {
                    ICONS_SV.write().push_fallback(icon);
                })
                .perm();
        }
    }

    /// Instantiate an icon drawing widget for the `icon_name`.
    ///
    /// Returns [`NilUiNode`] if no registered handler can provide an icon.
    pub fn get(&self, icon_name: impl IconNames) -> UiNode {
        self.get_impl(&mut icon_name.names())
    }
    fn get_impl(&self, names: &mut dyn Iterator<Item = Txt>) -> UiNode {
        let sv = ICONS_SV.read();
        for name in names {
            let node = sv.get(IconRequestArgs { name });
            if !node.is_nil() {
                return node;
            }
        }
        UiNode::nil()
    }

    /// Instantiate an icon drawing widget for the `icon_name` or call `fallback` to do it
    /// if no handler can handle the request.
    pub fn get_or(&self, icon_name: impl IconNames, fallback: impl FnOnce() -> UiNode) -> UiNode {
        let i = self.get(icon_name);
        if i.is_nil() { fallback() } else { i }
    }

    /// Same as [`get`], but also logs an error is there are no available icon for any of the names.
    ///
    /// [`get`]: Self::get
    pub fn req(&self, icon_name: impl IconNames) -> UiNode {
        self.req_impl(&mut icon_name.names())
    }
    fn req_impl(&self, names: &mut dyn Iterator<Item = Txt>) -> UiNode {
        let sv = ICONS_SV.read();
        let mut missing = vec![];
        for name in names {
            let node = sv.get(IconRequestArgs { name: name.clone() });
            if !node.is_nil() {
                return node;
            } else {
                missing.push(name);
            }
        }
        tracing::error!("no icon available for {missing:?}");
        UiNode::nil()
    }

    //// Same as [`get_or`], but also logs an error is there are no available icon for any of the names.
    ///
    /// [`get_or`]: Self::get_or
    pub fn req_or(&self, icon_name: impl IconNames, fallback: impl FnOnce() -> UiNode) -> UiNode {
        let i = self.req(icon_name);
        if i.is_nil() { fallback() } else { i }
    }
}

/// Adapter for [`ICONS`] queries.
///
/// Can be `"name"` or `["name", "fallback-name1"]` names.
pub trait IconNames {
    /// Iterate over names, from most wanted to least.
    fn names(self) -> impl Iterator<Item = Txt>;
}
impl IconNames for &'static str {
    fn names(self) -> impl Iterator<Item = Txt> {
        [Txt::from(self)].into_iter()
    }
}
impl IconNames for Txt {
    fn names(self) -> impl Iterator<Item = Txt> {
        [self].into_iter()
    }
}
impl IconNames for Vec<Txt> {
    fn names(self) -> impl Iterator<Item = Txt> {
        self.into_iter()
    }
}
impl IconNames for &[Txt] {
    fn names(self) -> impl Iterator<Item = Txt> {
        self.iter().cloned()
    }
}
impl IconNames for &[&'static str] {
    fn names(self) -> impl Iterator<Item = Txt> {
        self.iter().copied().map(Txt::from)
    }
}
impl<const N: usize> IconNames for [&'static str; N] {
    fn names(self) -> impl Iterator<Item = Txt> {
        self.into_iter().map(Txt::from)
    }
}

/// Adds the [`icon`](CommandIconExt::icon) command metadata.
///
/// The value is an [`WidgetFn<()>`] that can generate any icon widget, the [`ICONS`] service is recommended.
///
/// [`WidgetFn<()>`]: WidgetFn
pub trait CommandIconExt {
    /// Gets a read-write variable that is the icon for the command.
    fn icon(self) -> CommandMetaVar<WidgetFn<()>>;

    /// Sets the initial icon if it is not set.
    fn init_icon(self, icon: WidgetFn<()>) -> Self;
}
static_id! {
    static ref COMMAND_ICON_ID: CommandMetaVarId<WidgetFn<()>>;
}
impl CommandIconExt for Command {
    fn icon(self) -> CommandMetaVar<WidgetFn<()>> {
        self.with_meta(|m| m.get_var_or_default(*COMMAND_ICON_ID))
    }

    fn init_icon(self, icon: WidgetFn<()>) -> Self {
        self.with_meta(|m| m.init_var(*COMMAND_ICON_ID, icon));
        self
    }
}

/// Arguments for [`EDITORS.register`].
///
/// Note that the handler is usually called in the widget context that will host the editor, so context
/// variables and services my also be available to inform the editor preferences.
///
/// [`EDITORS.register`]: EDITORS::register
#[derive(Clone)]
pub struct EditorRequestArgs {
    value: AnyVar,
}
impl EditorRequestArgs {
    /// The value variable.
    pub fn value_any(&self) -> &AnyVar {
        &self.value
    }

    /// Try to downcast the value variable to `T`.
    pub fn value<T: VarValue>(&self) -> Option<Var<T>> {
        self.value_any().clone().downcast::<T>().ok()
    }
}

/// Arguments for [`ICONS.register`].
///
/// Note that the handler is usually called in the widget context that will host the editor, so context
/// variables and services my also be available to inform the editor preferences.
///
/// [`ICONS.register`]: ICONS::register
#[derive(Clone)]
pub struct IconRequestArgs {
    name: Txt,
}
impl IconRequestArgs {
    /// Icon unique name,
    pub fn name(&self) -> &str {
        &self.name
    }
}

app_local! {
    static EDITORS_SV: WidgetProviderService<EditorRequestArgs> = const { WidgetProviderService::new() };
    static ICONS_SV: WidgetProviderService<IconRequestArgs> = const { WidgetProviderService::new() };
}
struct WidgetProviderService<A> {
    handlers: Vec<WidgetFn<A>>,
    fallback: Vec<WidgetFn<A>>,
}
impl<A: Clone + 'static> WidgetProviderService<A> {
    const fn new() -> Self {
        Self {
            handlers: vec![],
            fallback: vec![],
        }
    }

    fn push(&mut self, handler: WidgetFn<A>) {
        self.handlers.push(handler);
    }

    fn push_fallback(&mut self, handler: WidgetFn<A>) {
        self.fallback.push(handler);
    }

    fn get(&self, args: A) -> UiNode {
        for handler in self.handlers.iter().rev() {
            let editor = handler(args.clone());
            if !editor.is_nil() {
                return editor;
            }
        }
        for handler in self.fallback.iter() {
            let editor = handler(args.clone());
            if !editor.is_nil() {
                return editor;
            }
        }
        UiNode::nil()
    }
}
