use std::{fmt, ops, sync::Arc};

use crate::prelude::*;

#[doc(hidden)]
pub use zng_wgt::prelude::clmv as __clmv;

type BoxedWgtFn<D> = Box<dyn Fn(D) -> BoxedUiNode + Send + Sync>;

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
    pub fn new<U: UiNode>(func: impl Fn(D) -> U + Send + Sync + 'static) -> Self {
        WidgetFn(Some(Arc::new(Box::new(move |data| func(data).boxed()))))
    }

    /// Function that always produces the [`NilUiNode`].
    ///
    /// No heap allocation happens to create this value.
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
    pub fn call(&self, data: D) -> BoxedUiNode {
        if let Some(g) = &self.0 {
            g(data)
        } else {
            NilUiNode.boxed()
        }
    }

    /// Calls the function with `data` argument and only returns a node if is not nil.
    ///
    /// Returns `None` if [`is_nil`] or [`UiNode::is_nil`].
    ///
    /// [`is_nil`]: Self::is_nil
    pub fn call_checked(&self, data: D) -> Option<BoxedUiNode> {
        let r = self.0.as_ref()?(data);
        if r.is_nil() {
            None
        } else {
            Some(r)
        }
    }

    /// New widget function that returns the same `widget` for every call.
    ///
    /// The `widget` is wrapped in an [`ArcNode`] and every function call returns an [`ArcNode::take_on_init`] node.
    /// Note that `take_on_init` is not always the `widget` on init as it needs to wait for it to deinit first if
    /// it is already in use, this could have an effect if the widget function caller always expects a full widget.
    pub fn singleton(widget: impl UiNode) -> Self {
        let widget = ArcNode::new(widget);
        Self::new(move |_| widget.take_on_init())
    }
}
impl<D: 'static> ops::Deref for WidgetFn<D> {
    type Target = dyn Fn(D) -> BoxedUiNode;

    fn deref(&self) -> &Self::Target {
        match self.0.as_ref() {
            Some(f) => &**f,
            None => &nil_call::<D>,
        }
    }
}
fn nil_call<D>(_: D) -> BoxedUiNode {
    NilUiNode.boxed()
}

/// <span data-del-macro-root></span> Declares a widget function closure.
///
/// The output type is a [`WidgetFn`], the closure is [`clmv!`].
///
/// # Syntax
///
/// * `wgt_fn!(cloned, |_args| Wgt!())` - Clone-move closure, the same syntax as [`clmv!`] you can
/// list the cloned values before the closure.
/// * `wgt_fn!(path::to::func)` - The macro also accepts unction, the signature must receive the args and return
/// a widget.
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
