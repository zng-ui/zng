use super::*;

use std::{future::*, marker::PhantomData, pin::Pin, task::Poll};

/// See [`Var::wait_new`].
pub struct WaitNewFut<'a, T: VarValue, V: Var<T>> {
    is_new: WaitIsNewFut<'a, V>,
    _value: PhantomData<&'a T>,
}
impl<'a, T: VarValue, V: Var<T>> WaitNewFut<'a, T, V> {
    pub(super) fn new(var: &'a V) -> Self {
        Self {
            is_new: WaitIsNewFut::new(var),
            _value: PhantomData,
        }
    }
}
impl<'a, T: VarValue, V: Var<T>> Future for WaitNewFut<'a, T, V> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<T> {
        match self.is_new.poll_impl(cx) {
            Poll::Ready(()) => Poll::Ready(self.is_new.var.get()),
            Poll::Pending => Poll::Pending,
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

    fn poll_impl(&mut self, cx: &mut std::task::Context<'_>) -> Poll<()> {
        let update_id = self.var.last_update();
        if update_id != self.update_id {
            // has changed since init or last poll
            self.update_id = update_id;
            Poll::Ready(())
        } else {
            // has not changed since init or last poll, register hook
            let waker = cx.waker().clone();
            let handle = self.var.hook(Box::new(move |_| {
                waker.wake_by_ref();
                false
            }));

            // check if changed in parallel while was registering hook
            let update_id = self.var.last_update();
            if update_id != self.update_id {
                // changed in parallel
                // the hook will be dropped (handle not perm), it may wake in parallel too, but poll checks again.
                self.update_id = update_id;
                Poll::Ready(())
            } else {
                // really not ready yet
                handle.perm();
                Poll::Pending
            }
        }
    }
}
impl<'a, V: AnyVar> Future for WaitIsNewFut<'a, V> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        self.poll_impl(cx)
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
                let handle = self.var.hook(Box::new(move |_| {
                    waker.wake_by_ref();
                    false
                }));
                if self.var.is_animating() {
                    handle.perm();
                    Poll::Pending
                } else {
                    Poll::Ready(())
                }
            }
        }
    }
}
