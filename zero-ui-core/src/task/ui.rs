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
            event_loop_waker: UPDATES.waker(target.into_iter().collect()),
        })
    }

    /// Polls the future if needed, returns a reference to the result if the task is done.
    ///
    /// This does not poll the future if the task is done.
    pub fn update(&mut self) -> Option<&R> {
        if let UiTaskState::Pending {
            future, event_loop_waker, ..
        } = &mut self.0
        {
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
