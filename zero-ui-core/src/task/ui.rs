//! UI-thread bound tasks.

use std::{
    fmt,
    future::Future,
    pin::Pin,
    task::{Poll, Waker},
};

use crate::{app::AppEventSender, context::*, widget_instance::WidgetId};

impl<'a> AppContext<'a> {
    /// Create an app thread bound future executor that executes in the app context.
    ///
    /// The `task` closure is called immediately with the [`AppContextMut`] that is paired with the task, it
    /// should return the task future `F` in an inert state. Calls to [`AppTask::update`] exclusive borrow the
    /// [`AppContext`] that is made available inside `F` using the [`AppContextMut::with`] method.
    pub fn async_task<R, F, T>(&mut self, task: T) -> AppTask<R>
    where
        R: 'static,
        F: Future<Output = R> + 'static,
        T: FnOnce(AppContextMut) -> F,
    {
        AppTask::new(self, task)
    }
}
impl<'a> WidgetContext<'a> {
    /// Create an app thread bound future executor that executes in the context of a widget.
    ///
    /// The `task` closure is called immediately with the [`WidgetContextMut`] that is paired with the task, it
    /// should return the task future `F` in an inert state. Calls to [`WidgetTask::update`] exclusive borrow a
    /// [`WidgetContext`] that is made available inside `F` using the [`WidgetContextMut::with`] method.
    pub fn async_task<R, F, T>(&mut self, task: T) -> WidgetTask<R>
    where
        R: 'static,
        F: Future<Output = R> + 'static,
        T: FnOnce(WidgetContextMut) -> F,
    {
        WidgetTask::new(self, task)
    }
}

enum UiTaskState<R> {
    Pending {
        future: Pin<Box<dyn Future<Output = R>>>,
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

/// Represents a [`Future`] running in the UI thread.
///
/// The future [`Waker`], wakes the app event loop and causes an update, in an update handler
/// [`update`] must be called, if this task waked the app the future is polled once, in widgets [`subscribe`] also
/// must be called to register the waker update slot.
///
/// [`Waker`]: std::task::Waker
/// [`update`]: UiTask::update
/// [`subscribe`]: UiTask::update
#[derive(Debug)]
pub struct UiTask<R>(UiTaskState<R>);
impl<R> UiTask<R> {
    /// Create a app thread bound future executor.
    ///
    /// The `task` is inert and must be polled using [`update`] to start, and it must be polled every
    /// [`UiNode::update`] after that, in widgets the `target` can be set so that the update requests are received.
    ///
    /// [`update`]: UiTask::update
    /// [`UiNode::update`]: crate::UiNode::update
    /// [`UiNode::info`]: crate::UiNode::info
    /// [`subscribe`]: Self::subscribe
    pub fn new<F: Future<Output = R> + 'static>(updates: &AppEventSender, target: Option<WidgetId>, task: F) -> Self {
        UiTask(UiTaskState::Pending {
            future: Box::pin(task),
            event_loop_waker: updates.waker(target.into_iter().collect()),
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

/// Represents a [`Future`] running in the UI thread in a widget context.
///
/// The future [`Waker`], wakes the app event loop and causes an update, the widget that is running this task
/// calls [`update`] and if this task waked the app the future is polled once.
///
/// [`Waker`]: std::task::Waker
/// [`update`]: Self::update
/// [`UiNode::info`]: crate::UiNode::info
/// [`subscribe`]: Self::subscribe
pub struct WidgetTask<R> {
    task: UiTask<R>,
    scope: WidgetContextScope,
}
impl<R> WidgetTask<R> {
    /// Create an app thread bound future executor that executes in the context of a widget.
    ///
    /// The `task` closure is called immediately with the [`WidgetContextMut`] that is paired with the task, it
    /// should return the task future `F` in an inert state. Calls to [`WidgetTask::update`] exclusive borrow a
    /// [`WidgetContext`] that is made available inside `F` using the [`WidgetContextMut::with`] method.
    pub fn new<F, T>(ctx: &mut WidgetContext, task: T) -> WidgetTask<R>
    where
        R: 'static,
        F: Future<Output = R> + 'static,
        T: FnOnce(WidgetContextMut) -> F,
    {
        let (scope, mut_) = WidgetContextScope::new();

        let task = scope.with(ctx, move || task(mut_));

        WidgetTask {
            task: UiTask::new(&ctx.updates.sender(), Some(ctx.path.widget_id()), task),
            scope,
        }
    }

    /// Polls the future if needed, returns a reference to the result if the task is done.
    ///
    /// This does not poll the future if the task is done, it also only polls the future if it requested poll.
    pub fn update(&mut self, ctx: &mut WidgetContext) -> Option<&R> {
        let task = &mut self.task;
        self.scope.with(ctx, move || task.update())
    }

    /// Returns `true` if the task is done.
    ///
    /// This does not poll the future.
    pub fn is_ready(&self) -> bool {
        self.task.is_ready()
    }

    /// Returns the result if the task is completed.
    ///
    /// This does not poll the future, you must call [`update`](Self::update) to poll until a result is available,
    /// then call this method to take ownership of the result.
    pub fn into_result(self) -> Result<R, Self> {
        match self.task.into_result() {
            Ok(r) => Ok(r),
            Err(task) => Err(Self { task, scope: self.scope }),
        }
    }
}
impl<T: fmt::Debug> fmt::Debug for WidgetTask<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WidgetTask").field(&self.task.0).finish()
    }
}

/// Represents a [`Future`] running in the UI thread in the app context.
///
/// The future [`Waker`](std::task::Waker), wakes the app event loop and causes an update, a update handler
/// then calls [`update`](Self::update) and if this task waked the app the future is polled once.
pub struct AppTask<R> {
    task: UiTask<R>,
    scope: AppContextScope,
}
impl<R> AppTask<R> {
    /// Create an app thread bound future executor that executes in the app context.
    ///
    /// The `task` closure is called immediately with the [`AppContextMut`] that is paired with the task, it
    /// should return the task future `F` in an inert state. Calls to [`AppTask::update`] exclusive borrow the
    /// [`AppContext`] that is made available inside `F` using the [`AppContextMut::with`] method.
    pub fn new<F, T>(ctx: &mut AppContext, task: T) -> AppTask<R>
    where
        R: 'static,
        F: Future<Output = R> + 'static,
        T: FnOnce(AppContextMut) -> F,
    {
        let (scope, mut_) = AppContextScope::new();

        let task = scope.with(ctx, move || task(mut_));

        AppTask {
            task: UiTask::new(&ctx.updates.sender(), None, task),
            scope,
        }
    }

    /// Polls the future if needed, returns a reference to the result if the task is done.
    ///
    /// This does not poll the future if the task is done, it also only polls the future if it requested poll.
    pub fn update(&mut self, ctx: &mut AppContext) -> Option<&R> {
        let task = &mut self.task;
        self.scope.with(ctx, move || task.update())
    }

    /// Returns `true` if the task is done.
    ///
    /// This does not poll the future.
    pub fn is_ready(&self) -> bool {
        self.task.is_ready()
    }

    /// Returns the result if the task is completed.
    ///
    /// This does not poll the future, you must call [`update`](Self::update) to poll until a result is available,
    /// then call this method to take ownership of the result.
    pub fn into_result(self) -> Result<R, Self> {
        match self.task.into_result() {
            Ok(r) => Ok(r),
            Err(task) => Err(Self { task, scope: self.scope }),
        }
    }
}
impl AppTask<()> {
    /// Schedule the app task to run to completion.
    pub fn run(mut self, updates: &mut Updates) {
        if !self.is_ready() {
            updates
                .on_pre_update(app_hn!(|ctx, _, handle| {
                    if self.update(ctx).is_some() {
                        handle.unsubscribe();
                    }
                }))
                .perm();
        }
    }
}
impl<T: fmt::Debug> fmt::Debug for AppTask<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AppTask").field(&self.task.0).finish()
    }
}
