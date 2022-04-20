use std::{
    cell::Cell,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{self, Poll},
};

use super::*;

#[doc(hidden)]
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

#[doc(hidden)]
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

#[doc(hidden)]
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

#[doc(hidden)]
pub struct VarIsNotAnimatingFut<'a, C, T, V>
where
    C: WithVars,
    T: VarValue,
    V: Var<T>,
{
    is_new: VarIsNewFut<'a, C, T, V>,
    is_animating: bool,
}
impl<'a, C, T, V> VarIsNotAnimatingFut<'a, C, T, V>
where
    C: WithVars,
    T: VarValue,
    V: Var<T>,
{
    #[allow(missing_docs)]
    pub fn new(ctx: &'a C, var: &'a V) -> Self {
        VarIsNotAnimatingFut {
            is_animating: ctx.with_vars(|v| var.is_animating(v)),
            is_new: VarIsNewFut::new(ctx, var),
        }
    }
}
impl<'a, C, T, V> Future for VarIsNotAnimatingFut<'a, C, T, V>
where
    C: WithVars,
    T: VarValue,
    V: Var<T>,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        // SAFETY: we don't move anything
        let this = unsafe { self.get_unchecked_mut() };
        let is_new = unsafe { Pin::new_unchecked(&mut this.is_new) };

        match is_new.poll(cx) {
            Poll::Ready(()) => {
                let is_animating = this.is_new.ctx.with_vars(|v| this.is_new.var.is_animating(v));
                let r = if this.is_animating && !is_animating {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                };
                this.is_animating = is_animating;
                r
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
