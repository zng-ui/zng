use std::{
    cell::Cell,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{self, Poll},
};

use super::*;

/// A future awaits for a copy of a variable's [new value](Var::copy_new) after the current update.
///
/// Use [`Var::wait_copy`] to create this future.
///
/// You can `.await` this in UI thread bound async code, like in async event handlers. The future
/// will unblock once for every time [`Var::copy_new`] returns `Some(T)` in a different update.
///
/// Note that if [`Var::can_update`] is `false` this will never awake.
///
/// # Example
///
/// ```
/// # use zero_ui_core::var::*;
/// # use zero_ui_core::handler::async_hn;
/// # fn __() -> impl zero_ui_core::handler::WidgetHandler<()> {
/// # let foo_var = var(10u32);
/// async_hn!(foo_var, |ctx, _| {
///     let value = foo_var.wait_copy(&ctx).await;
///     assert_eq!(Some(value), foo_var.copy_new(&ctx));
///
///     let value = foo_var.wait_copy(&ctx).await;
///     assert_eq!(Some(value), foo_var.copy_new(&ctx));
/// })
/// # }
/// ```
///
/// In the example the handler awaits for the variable to have a new value, the code immediately after
/// runs in the app update where the variable is new, the second `.await` does not poll immediately it awaits
/// for the variable to be new again but in a different update.
///
/// You can also reuse the future, but it is very cheap to just create a new one.
pub struct VarCopyNewFut<'a, C, T, V>
where
    C: WithVars,
    T: VarValue + Copy,
    V: Var<T>,
{
    _t: PhantomData<T>,
    ctx: &'a C,
    var: &'a V,
    update_id: Cell<u32>,
}
impl<'a, C, T, V> VarCopyNewFut<'a, C, T, V>
where
    C: WithVars,
    T: VarValue + Copy,
    V: Var<T>,
{
    #[allow(missing_docs)]
    pub fn new(ctx: &'a C, var: &'a V) -> Self {
        VarCopyNewFut {
            _t: PhantomData,
            update_id: Cell::new(ctx.with_vars(|vars| vars.update_id())),
            ctx,
            var,
        }
    }
}
impl<'a, C, T, V> Future for VarCopyNewFut<'a, C, T, V>
where
    C: WithVars,
    T: VarValue + Copy,
    V: Var<T>,
{
    type Output = T;

    fn poll(self: Pin<&mut Self>, _: &mut task::Context<'_>) -> Poll<Self::Output> {
        let update_id = self.ctx.with_vars(|vars| vars.update_id());
        if update_id != self.update_id.get() {
            self.update_id.set(update_id);
            if let Some(copy) = self.var.copy_new(self.ctx) {
                return Poll::Ready(copy);
            }
        }
        Poll::Pending
    }
}

/// A future awaits for a copy of a variable's [new value](Var::clone_new) after the current update.
///
/// Use [`Var::wait_clone`] to create this future.
///
/// You can `.await` this in UI thread bound async code, like in async event handlers. The future
/// will unblock once for every time [`Var::clone_new`] returns `Some(T)` in a different update.
///
/// Note that if [`Var::can_update`] is `false` this will never awake.
///
/// # Example
///
/// ```
/// # use zero_ui_core::var::*;
/// # use zero_ui_core::handler::async_hn;
/// # fn __() -> impl zero_ui_core::handler::WidgetHandler<()> {
/// # let foo_var = var(10u32);
/// async_hn!(foo_var, |ctx, _| {
///     let value = foo_var.wait_clone(&ctx).await;
///     assert_eq!(Some(value), foo_var.clone_new(&ctx));
///
///     let value = foo_var.wait_clone(&ctx).await;
///     assert_eq!(Some(value), foo_var.clone_new(&ctx));
/// })
/// # }
/// ```
///
/// In the example the handler awaits for the variable to have a new value, the code immediately after
/// runs in the app update where the variable is new, the second `.await` does not poll immediately it awaits
/// for the variable to be new again but in a different update.
///
/// You can also reuse the future, but it is very cheap to just create a new one.
pub struct VarCloneNewFut<'a, C, T, V>
where
    C: WithVars,
    T: VarValue,
    V: Var<T>,
{
    _t: PhantomData<T>,
    ctx: &'a C,
    var: &'a V,
    update_id: Cell<u32>,
}
impl<'a, C, T, V> VarCloneNewFut<'a, C, T, V>
where
    C: WithVars,
    T: VarValue,
    V: Var<T>,
{
    #[allow(missing_docs)]
    pub fn new(ctx: &'a C, var: &'a V) -> Self {
        VarCloneNewFut {
            _t: PhantomData,
            update_id: Cell::new(ctx.with_vars(|vars| vars.update_id())),
            ctx,
            var,
        }
    }
}
impl<'a, C, T, V> Future for VarCloneNewFut<'a, C, T, V>
where
    C: WithVars,
    T: VarValue,
    V: Var<T>,
{
    type Output = T;

    fn poll(self: Pin<&mut Self>, _: &mut task::Context<'_>) -> Poll<Self::Output> {
        let update_id = self.ctx.with_vars(|vars| vars.update_id());
        if update_id != self.update_id.get() {
            self.update_id.set(update_id);
            if let Some(copy) = self.var.clone_new(self.ctx) {
                return Poll::Ready(copy);
            }
        }
        Poll::Pending
    }
}

/// A future awaits for when a variable [is new](Var::is_new) after the current update.
///
/// Use [`Var::wait_new`] to create this future.
///
/// You can `.await` this in UI thread bound async code, like in async event handlers. The future
/// will unblock once for every time [`Var::is_new`] returns `true` in a different update.
///
/// Note that if [`Var::can_update`] is `false` this will never awake.
///
/// # Example
///
/// ```
/// # use zero_ui_core::var::*;
/// # use zero_ui_core::handler::async_hn;
/// # fn __() -> impl zero_ui_core::handler::WidgetHandler<()> {
/// # let foo_var = var(10u32);
/// async_hn!(foo_var, |ctx, _| {
///     foo_var.wait_new(&ctx).await;
///     assert!(foo_var.is_new(&ctx));
///
///     foo_var.wait_new(&ctx).await;
///     assert!(foo_var.is_new(&ctx));
/// })
/// # }
/// ```
///
/// In the example the handler awaits for the variable to have a new value, the code immediately after
/// runs in the app update where the variable is new, the second `.await` does not poll immediately it awaits
/// for the variable to be new again but in a different update.
///
/// You can also reuse the future, but it is very cheap to just create a new one.
pub struct VarIsNewFut<'a, C, T, V>
where
    C: WithVars,
    T: VarValue,
    V: Var<T>,
{
    _t: PhantomData<T>,
    ctx: &'a C,
    var: &'a V,
    update_id: Cell<u32>,
}
impl<'a, C, T, V> VarIsNewFut<'a, C, T, V>
where
    C: WithVars,
    T: VarValue,
    V: Var<T>,
{
    #[allow(missing_docs)]
    pub fn new(ctx: &'a C, var: &'a V) -> Self {
        VarIsNewFut {
            _t: PhantomData,
            update_id: Cell::new(ctx.with_vars(|vars| vars.update_id())),
            ctx,
            var,
        }
    }
}
impl<'a, C, T, V> Future for VarIsNewFut<'a, C, T, V>
where
    C: WithVars,
    T: VarValue,
    V: Var<T>,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, _: &mut task::Context<'_>) -> Poll<Self::Output> {
        let update_id = self.ctx.with_vars(|vars| vars.update_id());
        if update_id != self.update_id.get() {
            self.update_id.set(update_id);
            if self.var.is_new(self.ctx) {
                return Poll::Ready(());
            }
        }
        Poll::Pending
    }
}
