use super::*;

use std::{pin::Pin, task::Poll};

/// See [`Var::wait_update`].
pub(crate) struct WaitUpdateFut<'a> {
    var: &'a AnyVar,
    update_id: VarUpdateId,
}
impl<'a> WaitUpdateFut<'a> {
    pub(super) fn new(var: &'a AnyVar) -> Self {
        Self {
            update_id: var.last_update(),
            var,
        }
    }

    fn poll_impl(&mut self, cx: &mut std::task::Context<'_>) -> Poll<VarUpdateId> {
        let update_id = self.var.last_update();
        if update_id != self.update_id {
            // has changed since init or last poll
            self.update_id = update_id;
            Poll::Ready(update_id)
        } else {
            // has not changed since init or last poll, register hook
            let waker = cx.waker().clone();
            let handle = self.var.hook(move |_| {
                waker.wake_by_ref();
                false
            });

            // check if changed in parallel while was registering hook
            let update_id = self.var.last_update();
            if update_id != self.update_id {
                // changed in parallel
                // the hook will be dropped (handle not perm), it may wake in parallel too, but poll checks again.
                self.update_id = update_id;
                Poll::Ready(update_id)
            } else {
                // really not ready yet
                handle.perm();
                Poll::Pending
            }
        }
    }
}
impl Future for WaitUpdateFut<'_> {
    type Output = VarUpdateId;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        self.poll_impl(cx)
    }
}

/// See [`Var::wait_animation`].
pub(crate) struct WaitIsNotAnimatingFut<'a> {
    var: &'a AnyVar,
    observed_animation_start: bool,
}
impl<'a> WaitIsNotAnimatingFut<'a> {
    pub(super) fn new(var: &'a AnyVar) -> Self {
        Self {
            observed_animation_start: var.is_animating(),
            var,
        }
    }
}
impl Future for WaitIsNotAnimatingFut<'_> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<()> {
        if !self.var.capabilities().contains(VarCapability::NEW) {
            // var cannot have new value, ready to avoid deadlock.
            self.observed_animation_start = false;
            return Poll::Ready(());
        }
        if self.observed_animation_start {
            // already observed `is_animating` in a previous poll.

            if self.var.is_animating() {
                // still animating, but received poll so an animation was overridden and stopped.
                // try hook with new animation.

                while self.var.capabilities().contains(VarCapability::NEW) {
                    let waker = cx.waker().clone();
                    let r = self.var.hook_animation_stop(move || {
                        waker.wake_by_ref();
                    });
                    if r.is_dummy() {
                        // failed to hook with new animation too.
                        if self.var.is_animating() {
                            // but has yet another animation, try again.
                            continue;
                        } else {
                            // observed `is_animating` changing to `false`.
                            self.observed_animation_start = false;
                            return Poll::Ready(());
                        }
                    } else {
                        // new animation hook setup ok, break loop.
                        return Poll::Pending;
                    }
                }

                // var no longer has the `NEW` capability.
                self.observed_animation_start = false;
                Poll::Ready(())
            } else {
                // now observed change to `false`.
                self.observed_animation_start = false;
                Poll::Ready(())
            }
        } else {
            // have not observed `is_animating` yet.

            // hook with normal var updates, `is_animating && is_new` is always `true`.
            let waker = cx.waker().clone();
            let start_hook = self.var.hook(move |_| {
                waker.wake_by_ref();
                false
            });

            if self.var.is_animating() {
                // observed `is_animating` already, changed in other thread during the `hook` setup.
                self.observed_animation_start = true;

                while self.var.capabilities().contains(VarCapability::NEW) {
                    // hook with animation stop.
                    let waker = cx.waker().clone();
                    let r = self.var.hook_animation_stop(Box::new(move || {
                        waker.wake_by_ref();
                    }));
                    if r.is_dummy() {
                        // failed to hook, animation already stopped during hook setup.
                        if self.var.is_animating() {
                            // but var is still animating, reason a new animation replaced the previous one (that stopped).
                            // continue to hook with new animation.
                            continue;
                        } else {
                            // we have observed `is_animating` changing to `false` in one poll call.
                            self.observed_animation_start = false;
                            return Poll::Ready(());
                        }
                    } else {
                        r.perm();
                        // animation hook setup ok, break loop.
                        return Poll::Pending;
                    }
                }

                // var no longer has the `NEW` capability.
                self.observed_animation_start = false;
                Poll::Ready(())
            } else {
                // updates hook ok.
                start_hook.perm();
                Poll::Pending
            }
        }
    }
}
