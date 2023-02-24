use super::*;

use std::{future::*, marker::PhantomData, pin::Pin, task::Poll};

/// See [`Var::wait_new`].
pub struct WaitNewFut<'a, T: VarValue, V: Var<T>> {
    var: &'a V,
    wakers: Mutex<Vec<VarHandle>>,
    _value: PhantomData<T>,
}
impl<'a, T: VarValue, V: Var<T>> WaitNewFut<'a, T, V> {
    pub(super) fn new(var: &'a V) -> Self {
        Self {
            var,
            wakers: Mutex::new(vec![]),
            _value: PhantomData,
        }
    }
}
impl<'a, T: VarValue, V: Var<T>> Future for WaitNewFut<'a, T, V> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<T> {
        match self.var.get_new() {
            Some(value) => {
                self.wakers.lock().clear();
                Poll::Ready(value)
            }
            None => {
                let waker = cx.waker().clone();
                self.wakers.lock().push(self.var.hook(Box::new(move |_| {
                    waker.wake_by_ref();
                    false
                })));
                Poll::Pending
            }
        }
    }
}

/// See [`Var::wait_is_new`].
pub struct WaitIsNewFut<'a, T: VarValue, V: Var<T>> {
    var: &'a V,
    wakers: Mutex<Vec<VarHandle>>,
    _value: PhantomData<T>,
}
impl<'a, T: VarValue, V: Var<T>> WaitIsNewFut<'a, T, V> {
    pub(super) fn new(var: &'a V) -> Self {
        Self {
            var,
            wakers: Mutex::new(vec![]),
            _value: PhantomData,
        }
    }
}
impl<'a, T: VarValue, V: Var<T>> Future for WaitIsNewFut<'a, T, V> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<()> {
        match self.var.is_new() {
            true => {
                self.wakers.lock().clear();
                Poll::Ready(())
            }
            false => {
                let waker = cx.waker().clone();
                self.wakers.lock().push(self.var.hook(Box::new(move |_| {
                    waker.wake_by_ref();
                    false
                })));
                Poll::Pending
            }
        }
    }
}

/// See [`Var::wait_animation`].
pub struct WaitIsNotAnimatingFut<'a, T: VarValue, V: Var<T>> {
    var: &'a V,
    wakers: Mutex<Vec<VarHandle>>,
    _value: PhantomData<T>,
}
impl<'a, T: VarValue, V: Var<T>> WaitIsNotAnimatingFut<'a, T, V> {
    pub(super) fn new(var: &'a V) -> Self {
        Self {
            var,
            wakers: Mutex::new(vec![]),
            _value: PhantomData,
        }
    }
}
impl<'a, T: VarValue, V: Var<T>> Future for WaitIsNotAnimatingFut<'a, T, V> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<()> {
        match self.var.is_animating() {
            false => {
                self.wakers.lock().clear();
                Poll::Ready(())
            }
            true => {
                let waker = cx.waker().clone();
                self.wakers.lock().push(self.var.hook(Box::new(move |_| {
                    waker.wake_by_ref();
                    false
                })));
                Poll::Pending
            }
        }
    }
}
