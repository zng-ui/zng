use super::*;

use std::{future::*, marker::PhantomData, pin::Pin, task::Poll};

/// See [`Var::wait_new`].
pub struct WaitNewFut<'a, C: WithVars, T: VarValue, V: Var<T>> {
    vars: &'a C,
    var: &'a V,
    wakers: RefCell<Vec<VarHandle>>,
    _value: PhantomData<T>,
}
impl<'a, C: WithVars, T: VarValue, V: Var<T>> WaitNewFut<'a, C, T, V> {
    pub(super) fn new(vars: &'a C, var: &'a V) -> Self {
        Self {
            vars,
            var,
            wakers: RefCell::new(vec![]),
            _value: PhantomData,
        }
    }
}
impl<'a, C: WithVars, T: VarValue, V: Var<T>> Future for WaitNewFut<'a, C, T, V> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<T> {
        match self.var.get_new(self.vars) {
            Some(value) => {
                self.wakers.borrow_mut().clear();
                Poll::Ready(value)
            }
            None => {
                let waker = cx.waker().clone();
                self.wakers.borrow_mut().push(self.var.hook(Box::new(move |_, _, _| {
                    waker.wake_by_ref();
                    false
                })));
                Poll::Pending
            }
        }
    }
}

/// See [`Var::wait_is_new`].
pub struct WaitIsNewFut<'a, C: WithVars, T: VarValue, V: Var<T>> {
    vars: &'a C,
    var: &'a V,
    wakers: RefCell<Vec<VarHandle>>,
    _value: PhantomData<T>,
}
impl<'a, C: WithVars, T: VarValue, V: Var<T>> WaitIsNewFut<'a, C, T, V> {
    pub(super) fn new(vars: &'a C, var: &'a V) -> Self {
        Self {
            vars,
            var,
            wakers: RefCell::new(vec![]),
            _value: PhantomData,
        }
    }
}
impl<'a, C: WithVars, T: VarValue, V: Var<T>> Future for WaitIsNewFut<'a, C, T, V> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<()> {
        match self.var.is_new(self.vars) {
            true => {
                self.wakers.borrow_mut().clear();
                Poll::Ready(())
            }
            false => {
                let waker = cx.waker().clone();
                self.wakers.borrow_mut().push(self.var.hook(Box::new(move |_, _, _| {
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
    wakers: RefCell<Vec<VarHandle>>,
    _value: PhantomData<T>,
}
impl<'a, T: VarValue, V: Var<T>> WaitIsNotAnimatingFut<'a, T, V> {
    pub(super) fn new(var: &'a V) -> Self {
        Self {
            var,
            wakers: RefCell::new(vec![]),
            _value: PhantomData,
        }
    }
}
impl<'a, T: VarValue, V: Var<T>> Future for WaitIsNotAnimatingFut<'a, T, V> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<()> {
        match self.var.is_animating() {
            true => {
                self.wakers.borrow_mut().clear();
                Poll::Ready(())
            }
            false => {
                let waker = cx.waker().clone();
                self.wakers.borrow_mut().push(self.var.hook(Box::new(move |_, _, _| {
                    waker.wake_by_ref();
                    false
                })));
                Poll::Pending
            }
        }
    }
}
