//! Asynchronous task runner

use std::future::Future;

/// Asynchronous task runner.
pub struct Tasks {}
impl Tasks {
    pub(super) fn new() -> Self {
        Tasks {}
    }

    /// Run a CPU bound task.
    ///
    /// The task runs in a [`rayon`] thread-pool, this function is not blocking.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::{context::WidgetContext, var::{ResponseVar, response_channel}};
    /// # struct SomeStruct { sum_response: ResponseVar<usize> }
    /// # impl SomeStruct {
    /// fn on_event(&mut self, ctx: &mut WidgetContext) {
    ///     let (sender, response) = response_channel(ctx.vars);
    ///     self.sum_response = response;
    ///     ctx.tasks.run(move ||{
    ///         let r = (0..1000).sum();
    ///         sender.send_response(r);
    ///     });
    /// }
    ///
    /// fn on_update(&mut self, ctx: &mut WidgetContext) {
    ///     if let Some(result) = self.sum_response.response_new(ctx.vars) {
    ///         println!("sum of 0..1000: {}", result);   
    ///     }
    /// }
    /// # }
    /// ```
    pub fn run<T: FnOnce() + Send + 'static>(&mut self, task: T) {
        rayon::spawn(task);
    }

    /// Run an IO bound task.
    ///
    /// The task runs in an [`async-global-executor`] thread-pool, this function is not blocking.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::{context::WidgetContext, var::{ResponseVar, response_channel}};
    /// # struct SomeStruct { file_response: ResponseVar<Vec<u8>> }
    /// # impl SomeStruct {
    /// fn on_event(&mut self, ctx: &mut WidgetContext) {
    ///     let (sender, response) = response_channel(ctx.vars);
    ///     self.file_response = response;
    ///     ctx.tasks.run_async(async move {
    ///         todo!("use async_std to read a file");
    ///         let file = vec![];
    ///         sender.send_response(file);    
    ///     });
    /// }
    ///
    /// fn on_update(&mut self, ctx: &mut WidgetContext) {
    ///     if let Some(result) = self.file_response.response_new(ctx.vars) {
    ///         println!("file loaded: {} bytes", result.len());   
    ///     }
    /// }
    /// # }
    /// ```
    pub fn run_async<T: Future<Output = ()> + Send + 'static>(&mut self, task: T) {
        // TODO run block-on?
        async_global_executor::spawn(task).detach();
    }

    ///// Run a task in the UI thread.
    //pub fn run_async_ui<R, T: Future<Output = R> + 'static>(&mut self, task: T) -> UiTaskExecutor {
    //    todo!()
    //}
}

//pub struct UiTaskExecutor {}
