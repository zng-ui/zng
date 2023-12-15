use std::{fmt, ops, sync::Arc};

use crate::prelude::*;

#[doc(hidden)]
pub use zero_ui_wgt::prelude::clmv as __clmv;

type BoxedWgtFn<D> = Box<dyn Fn(D) -> BoxedUiNode + Send + Sync>;

/// Boxed shared closure that generates an widget for a given data.
///
/// # Examples
///
/// Define the content that is shown when an image fails to load:
///
/// ```
/// # macro_rules! _demo { () => {
/// Image! {
///     source = "not_found.png";
///     img_error_fn = WidgetFn::new(|e: image::ImgErrorArgs| Text! {
///         txt = e.error.clone();
///         font_color = colors::RED;
///     });
/// }
/// # }}
/// ```
///
/// You can also use the [`wgt_fn!`] macro, it has the advantage of being clone move.
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

    /// If this is  the [`nil`] function.
    ///
    /// [`nil`]: WidgetFn::nil
    pub fn is_nil(&self) -> bool {
        self.0.is_none()
    }

    /// Calls the function with `data` argument.
    ///
    /// Note that you can call the widget function directly where `D: 'static`:
    ///
    /// ```
    /// # use zero_ui_wgt_view::*;
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

    /// New widget function that returns the same `widget` for every call.
    ///
    /// The `widget` is wrapped in an [`ArcNode`] and every function call returns an [`ArcNode::take_on_init`] node.
    /// Note that `take_on_init` is not always the `widget` on init as it needs to wait for it to deinit first if
    /// it is already in use, this could have an effect if the the widget function caller always expects a full widget.
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
/// # Examples
///
/// Define the content that is shown when an image fails to load, capturing another variable too.
///
/// ```
/// # macro_rules! _demo { () => {
/// Image! {
///     source = "not_found.png";
///     img_error_fn = wgt_fn!(img_error_vis, |e: image::ImgErrorArgs| Text! {
///         txt = e.error.clone();
///         font_color = colors::RED;
///         visibility = img_error_vis.clone();
///     });
/// }
/// # }}
/// ```
#[macro_export]
macro_rules! wgt_fn {
    ($($tt:tt)+) => {
        $crate::WidgetFn::new($crate::__clmv! {
            $($tt)+
        })
    };
    () => {
        $crate::WidgetFn::nil()
    };
}
