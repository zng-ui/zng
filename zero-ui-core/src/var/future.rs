use super::*;

use std::{future::*, marker::PhantomData, pin::Pin, task::Poll};

/// See [`Var::wait_new`].
pub struct WaitNewFut<'a, T: VarValue, V: Var<T>> {
    var: &'a V,
    update_id: VarUpdateId,
    _value: PhantomData<&'a T>,
}
impl<'a, T: VarValue, V: Var<T>> WaitNewFut<'a, T, V> {
    pub(super) fn new(var: &'a V) -> Self {
        Self {
            update_id: var.last_update(),
            var,
            _value: PhantomData,
        }
    }
}
impl<'a, T: VarValue, V: Var<T>> Future for WaitNewFut<'a, T, V> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<T> {
        let update_id = self.var.last_update();
        if update_id != self.update_id {
            self.update_id = update_id;
            Poll::Ready(self.var.get())
        } else {
            let waker = cx.waker().clone();
            self.var
                .hook(Box::new(move |_| {
                    waker.wake_by_ref();
                    false
                }))
                .perm();
            Poll::Pending
        }
    }
}

/// See [`Var::wait_is_new`].
pub struct WaitIsNewFut<'a, V: AnyVar> {
    var: &'a V,
    update_id: VarUpdateId,
}
impl<'a, V: AnyVar> WaitIsNewFut<'a, V> {
    pub(super) fn new(var: &'a V) -> Self {
        Self {
            update_id: var.last_update(),
            var,
        }
    }
}
impl<'a, V: AnyVar> Future for WaitIsNewFut<'a, V> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let update_id = self.var.last_update();
        if update_id != self.update_id {
            self.update_id = update_id;
            Poll::Ready(())
        } else {
            let waker = cx.waker().clone();
            self.var
                .hook(Box::new(move |_| {
                    waker.wake_by_ref();
                    false
                }))
                .perm();
            Poll::Pending
        }
    }
}

/// See [`Var::wait_animation`].
pub struct WaitIsNotAnimatingFut<'a, V: AnyVar> {
    var: &'a V,
}
impl<'a, V: AnyVar> WaitIsNotAnimatingFut<'a, V> {
    pub(super) fn new(var: &'a V) -> Self {
        Self { var }
    }
}
impl<'a, V: AnyVar> Future for WaitIsNotAnimatingFut<'a, V> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<()> {
        match self.var.is_animating() {
            false => Poll::Ready(()),
            true => {
                let waker = cx.waker().clone();
                self.var
                    .hook(Box::new(move |_| {
                        waker.wake_by_ref();
                        false
                    }))
                    .perm();
                Poll::Pending
            }
        }
    }
}
