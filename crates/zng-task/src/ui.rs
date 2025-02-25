//! UI-thread bound tasks.

use std::{
    fmt, mem,
    pin::Pin,
    task::{Poll, Waker},
};

enum UiTaskState<R> {
    Pending {
        future: Pin<Box<dyn Future<Output = R> + Send>>,
        event_loop_waker: Waker,
        #[cfg(debug_assertions)]
        last_update: Option<zng_var::VarUpdateId>,
    },
    Ready(R),
    Cancelled,
}
impl<R: fmt::Debug> fmt::Debug for UiTaskState<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending { .. } => write!(f, "Pending"),
            Self::Ready(arg0) => f.debug_tuple("Ready").field(arg0).finish(),
            Self::Cancelled => unreachable!(),
        }
    }
}

/// Represents a [`Future`] running in sync with the UI.
///
/// The future [`Waker`], wakes the app event loop and causes an update, in an update handler
/// of the task owner [`update`] is called, if this task waked the app the future is polled once.
///
/// [`Waker`]: std::task::Waker
/// [`update`]: UiTask::update
#[derive(Debug)]
pub struct UiTask<R>(UiTaskState<R>);
impl<R> UiTask<R> {
    /// New task with already build event-loop waker.
    ///
    /// App crate provides an integrated `UiTaskWidget::new` that creates the waker for widgets.
    pub fn new_raw<F>(event_loop_waker: Waker, task: impl IntoFuture<IntoFuture = F>) -> Self
    where
        F: Future<Output = R> + Send + 'static,
    {
        UiTask(UiTaskState::Pending {
            future: Box::pin(task.into_future()),
            event_loop_waker,
            #[cfg(debug_assertions)]
            last_update: None,
        })
    }

    /// Polls the future if needed, returns a reference to the result if the task is done.
    ///
    /// This does not poll the future if the task is done.
    ///
    /// # App Update
    ///
    /// This method must be called only once per app update, if it is called more than once it will cause **execution bugs**,
    /// futures like [`task::yield_now`] will not work correctly, variables will have old values when a new one
    /// is expected and any other number of hard to debug issues will crop-up.
    ///
    /// In debug builds this is validated and an error message is logged if incorrect updates are detected.
    ///
    /// [`task::yield_now`]: crate::yield_now
    pub fn update(&mut self) -> Option<&R> {
        if let UiTaskState::Pending {
            future,
            event_loop_waker,
            #[cfg(debug_assertions)]
            last_update,
            ..
        } = &mut self.0
        {
            #[cfg(debug_assertions)]
            {
                let update = Some(zng_var::VARS.update_id());
                if *last_update == update {
                    tracing::error!("UiTask::update called twice in the same update");
                }
                *last_update = update;
            }

            if let Poll::Ready(r) = future.as_mut().poll(&mut std::task::Context::from_waker(event_loop_waker)) {
                self.0 = UiTaskState::Ready(r);
            }
        }

        if let UiTaskState::Ready(r) = &self.0 { Some(r) } else { None }
    }

    /// Returns `true` if the task is done.
    ///
    /// This does not poll the future.
    pub fn is_ready(&self) -> bool {
        matches!(&self.0, UiTaskState::Ready(_))
    }

    /// Returns the result if the task is completed.
    ///
    /// This does not poll the future, you must call [`update`] to poll until a result is available,
    /// then call this method to take ownership of the result.
    ///
    /// [`update`]: Self::update
    pub fn into_result(mut self) -> Result<R, Self> {
        match mem::replace(&mut self.0, UiTaskState::Cancelled) {
            UiTaskState::Ready(r) => Ok(r),
            p @ UiTaskState::Pending { .. } => Err(Self(p)),
            UiTaskState::Cancelled => unreachable!(),
        }
    }

    /// Drop the task without logging a warning if it is pending.
    pub fn cancel(mut self) {
        self.0 = UiTaskState::Cancelled;
    }
}
impl<R> Drop for UiTask<R> {
    fn drop(&mut self) {
        if let UiTaskState::Pending { .. } = &self.0 {
            #[cfg(debug_assertions)]
            {
                tracing::warn!("pending UiTask<{}> dropped", std::any::type_name::<R>());
            }
            #[cfg(not(debug_assertions))]
            {
                tracing::warn!("pending UiTask dropped");
            }
        }
    }
}
