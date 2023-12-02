//! Variables

use std::sync::Arc;

use crate::task::parking_lot::Mutex;
use zero_ui_layout::units::Px;
pub use zero_ui_var::*;

pub mod helpers;

use crate::{
    context::{UpdateOp, UPDATES},
    handler::{AppHandler, AppHandlerArgs},
    units::{Layout1d, Layout2d},
    widget_instance::WidgetId,
};

/// Extension method to subscribe any widget to a variable.
///
/// Also see [`WIDGET`] methods for the primary way to subscribe from inside an widget.
///
/// [`WIDGET`]: crate::context::WIDGET
pub trait AnyVarSubscribe: AnyVar {
    /// Register the widget to receive an [`UpdateOp`] when this variable is new.
    ///
    /// Variables without the [`NEW`] capability return [`VarHandle::dummy`].
    ///
    /// [`NEW`]: VarCapabilities::NEW
    fn subscribe(&self, op: UpdateOp, widget_id: WidgetId) -> VarHandle;
}
impl<V: AnyVar> AnyVarSubscribe for V {
    fn subscribe(&self, op: UpdateOp, widget_id: WidgetId) -> VarHandle {
        if !self.capabilities().is_always_static() {
            self.hook(var_subscribe(op, widget_id))
        } else {
            VarHandle::dummy()
        }
    }
}

/// Extension methods to subscribe any widget to a variable or app handlers to a variable.
///
/// Also see [`WIDGET`] methods for the primary way to subscribe from inside an widget.
///
/// [`WIDGET`]: crate::context::WIDGET
pub trait VarSubscribe<T: VarValue>: Var<T> + AnyVarSubscribe {
    /// Register the widget to receive an [`UpdateOp`] when this variable is new and the `predicate` approves the new value.
    ///
    /// Variables without the [`NEW`] capability return [`VarHandle::dummy`].
    ///
    /// [`NEW`]: VarCapabilities::NEW
    fn subscribe_when(&self, op: UpdateOp, widget_id: WidgetId, predicate: impl Fn(&T) -> bool + Send + Sync + 'static) -> VarHandle;

    /// Add a preview `handler` that is called every time this variable updates,
    /// the handler is called before all other UI updates.
    ///
    /// Note that the handler runs on the app context, all [`ContextVar<T>`] read inside read the default value.
    fn on_pre_new<H>(&self, handler: H) -> VarHandle
    where
        H: AppHandler<OnVarArgs<T>>,
    {
        var_on_new(self, handler, true)
    }

    /// Add a `handler` that is called every time this variable updates,
    /// the handler is called after all other UI updates.
    ///
    /// Note that the handler runs on the app context, all [`ContextVar<T>`] read inside read the default value.
    fn on_new<H>(&self, handler: H) -> VarHandle
    where
        H: AppHandler<OnVarArgs<T>>,
    {
        var_on_new(self, handler, false)
    }
}
impl<T: VarValue, V: Var<T>> VarSubscribe<T> for V {
    fn subscribe_when(&self, op: UpdateOp, widget_id: WidgetId, predicate: impl Fn(&T) -> bool + Send + Sync + 'static) -> VarHandle {
        self.hook(var_subscribe_when(op, widget_id, predicate))
    }
}

/// Extension methods to subscribe app handlers to a response variable.
pub trait ResponseVarSubscribe<T: VarValue> {
    /// Add a `handler` that is called once when the response is received,
    /// the handler is called before all other UI updates.
    ///
    /// The handle is not called if already [`is_done`], in this case a dummy handle is returned.
    ///
    /// [`is_done`]: ResponseVar::is_done
    fn on_pre_rsp<H>(&self, handler: H) -> VarHandle
    where
        H: AppHandler<OnVarArgs<T>>;

    /// Add a `handler` that is called once when the response is received,
    /// the handler is called after all other UI updates.
    ///
    /// The handle is not called if already [`is_done`], in this case a dummy handle is returned.
    ///
    /// [`is_done`]: ResponseVar::is_done
    fn on_rsp<H>(&self, handler: H) -> VarHandle
    where
        H: AppHandler<OnVarArgs<T>>;
}
impl<T: VarValue> ResponseVarSubscribe<T> for ResponseVar<T> {
    fn on_pre_rsp<H>(&self, mut handler: H) -> VarHandle
    where
        H: AppHandler<OnVarArgs<T>>,
    {
        if self.is_done() {
            return VarHandle::dummy();
        }

        self.on_pre_new(app_hn!(|args: &OnVarArgs<types::Response<T>>, handler_args| {
            if let types::Response::Done(value) = &args.value {
                handler.event(
                    &OnVarArgs::new(value.clone(), args.tags.iter().map(|t| (*t).clone_boxed()).collect()),
                    &crate::handler::AppHandlerArgs {
                        handle: handler_args,
                        is_preview: true,
                    },
                )
            }
        }))
    }

    fn on_rsp<H>(&self, mut handler: H) -> VarHandle
    where
        H: AppHandler<OnVarArgs<T>>,
    {
        if self.is_done() {
            return VarHandle::dummy();
        }

        self.on_new(app_hn!(|args: &OnVarArgs<types::Response<T>>, handler_args| {
            if let types::Response::Done(value) = &args.value {
                handler.event(
                    &OnVarArgs::new(value.clone(), args.tags.iter().map(|t| (*t).clone_boxed()).collect()),
                    &crate::handler::AppHandlerArgs {
                        handle: handler_args,
                        is_preview: false,
                    },
                )
            }
        }))
    }
}

fn var_subscribe(op: UpdateOp, widget_id: WidgetId) -> Box<dyn Fn(&VarHookArgs) -> bool + Send + Sync> {
    Box::new(move |_| {
        UPDATES.update_op(op, widget_id);
        true
    })
}

fn var_subscribe_when<T: VarValue>(
    op: UpdateOp,
    widget_id: WidgetId,
    when: impl Fn(&T) -> bool + Send + Sync + 'static,
) -> Box<dyn Fn(&VarHookArgs) -> bool + Send + Sync> {
    Box::new(move |a| {
        if let Some(a) = a.downcast_value::<T>() {
            if when(a) {
                UPDATES.update_op(op, widget_id);
            }
            true
        } else {
            false
        }
    })
}

fn var_on_new<T>(var: &impl Var<T>, handler: impl AppHandler<OnVarArgs<T>>, is_preview: bool) -> VarHandle
where
    T: VarValue,
{
    if var.capabilities().is_always_static() {
        return VarHandle::dummy();
    }

    let handler = Arc::new(Mutex::new(handler));
    let (inner_handle_owner, inner_handle) = crate::crate_util::Handle::new(());
    var.hook(Box::new(move |args| {
        if inner_handle_owner.is_dropped() {
            return false;
        }

        if let Some(value) = args.downcast_value::<T>() {
            let handle = inner_handle.downgrade();
            let value = value.clone();
            let tags = args.tags().iter().map(|t| (*t).clone_boxed()).collect();
            let update_once = app_hn_once!(handler, value, |_| {
                handler.lock().event(
                    &OnVarArgs::new(value, tags),
                    &AppHandlerArgs {
                        handle: &handle,
                        is_preview,
                    },
                );
            });

            if is_preview {
                UPDATES.on_pre_update(update_once).perm();
            } else {
                UPDATES.on_update(update_once).perm();
            }
        }
        true
    }))
}

/// Arguments for a var event handler.
pub struct OnVarArgs<T: VarValue> {
    /// The new value.
    pub value: T,
    /// Custom tag objects that where set when the value was modified.
    pub tags: Vec<Box<dyn AnyVarValue>>,
}
impl<T: VarValue> OnVarArgs<T> {
    /// New from value and custom modify tags.
    pub fn new(value: T, tags: Vec<Box<dyn AnyVarValue>>) -> Self {
        Self { value, tags }
    }

    /// Reference all custom tag values of type `T`.
    pub fn downcast_tags<Ta: VarValue>(&self) -> impl Iterator<Item = &Ta> + '_ {
        self.tags.iter().filter_map(|t| (*t).as_any().downcast_ref::<Ta>())
    }
}
impl<T: VarValue> Clone for OnVarArgs<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            tags: self.tags.iter().map(|t| (*t).clone_boxed()).collect(),
        }
    }
}

/// Extension methods to layout var values.
pub trait VarLayout<T: VarValue>: Var<T> {
    /// Compute the pixel value in the current [`LAYOUT`] context.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout(&self) -> T::Px
    where
        T: Layout2d,
    {
        self.with(|s| s.layout())
    }

    /// Compute the pixel value in the current [`LAYOUT`] context with `default`.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_dft(&self, default: T::Px) -> T::Px
    where
        T: Layout2d,
    {
        self.with(move |s| s.layout_dft(default))
    }

    /// Compute the pixel value in the current [`LAYOUT`] context ***x*** axis.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_x(&self) -> Px
    where
        T: Layout1d,
    {
        self.with(|s| s.layout_x())
    }

    /// Compute the pixel value in the current [`LAYOUT`] context ***y*** axis.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_y(&self) -> Px
    where
        T: Layout1d,
    {
        self.with(|s| s.layout_y())
    }

    /// Compute the pixel value in the current [`LAYOUT`] context ***z*** axis.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_z(&self) -> Px
    where
        T: Layout1d,
    {
        self.with(|s| s.layout_z())
    }

    /// Compute the pixel value in the current [`LAYOUT`] context ***x*** axis with `default`.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_dft_x(&self, default: Px) -> Px
    where
        T: Layout1d,
    {
        self.with(move |s| s.layout_dft_x(default))
    }

    /// Compute the pixel value in the current [`LAYOUT`] context ***y*** axis with `default`.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_dft_y(&self, default: Px) -> Px
    where
        T: Layout1d,
    {
        self.with(move |s| s.layout_dft_y(default))
    }

    /// Compute the pixel value in the current [`LAYOUT`] context ***z*** axis with `default`.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    fn layout_dft_z(&self, default: Px) -> Px
    where
        T: Layout1d,
    {
        self.with(move |s| s.layout_dft_z(default))
    }
}
impl<T: VarValue, V: Var<T>> VarLayout<T> for V {}
