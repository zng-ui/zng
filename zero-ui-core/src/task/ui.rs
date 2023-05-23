//! UI-thread bound tasks.

use std::{
    fmt,
    future::Future,
    pin::Pin,
    task::{Poll, Waker},
};

use crate::{context::*, widget_instance::WidgetId};

enum UiTaskState<R> {
    Pending {
        future: Pin<Box<dyn Future<Output = R> + Send>>,
        event_loop_waker: Waker,
        #[cfg(debug_assertions)]
        last_update: Option<crate::var::VarUpdateId>,
    },
    Ready(R),
}
impl<R: fmt::Debug> fmt::Debug for UiTaskState<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending { .. } => write!(f, "Pending"),
            Self::Ready(arg0) => f.debug_tuple("Ready").field(arg0).finish(),
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
    /// Create a UI bound future executor.
    ///
    /// The `task` is inert and must be polled using [`update`] to start, and it must be polled every
    /// [`UiNode::update`] after that, in widgets the `target` can be set so that the update requests are received.
    ///
    /// [`update`]: UiTask::update
    /// [`UiNode::update`]: crate::widget_instance::UiNode::update
    /// [`UiNode::info`]: crate::widget_instance::UiNode::info
    pub fn new<F>(target: Option<WidgetId>, task: F) -> Self
    where
        F: Future<Output = R> + Send + 'static,
    {
        UiTask(UiTaskState::Pending {
            future: Box::pin(task),
            event_loop_waker: UPDATES.waker(target),
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
    /// [`task::yield_now`]: crate::task::yield_now
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
                let update = Some(crate::var::VARS.update_id());
                if *last_update == update {
                    tracing::error!("UiTask::update called twice in the same update");
                }
                *last_update = update;
            }

            if let Poll::Ready(r) = future.as_mut().poll(&mut std::task::Context::from_waker(event_loop_waker)) {
                self.0 = UiTaskState::Ready(r);
            }
        }

        if let UiTaskState::Ready(r) = &self.0 {
            Some(r)
        } else {
            None
        }
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
    pub fn into_result(self) -> Result<R, Self> {
        match self.0 {
            UiTaskState::Ready(r) => Ok(r),
            p @ UiTaskState::Pending { .. } => Err(Self(p)),
        }
    }
}
