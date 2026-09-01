#![allow(clippy::result_large_err)]

use core::fmt;
use std::{any::Any, collections::VecDeque, pin::Pin, sync::Arc};

use zng_app::{event::app_local, update::UPDATES};
use zng_clone_move::clmv;
use zng_ext_config::RawConfigValue;
use zng_task::{Progress, parking_lot::Mutex};
use zng_txt::Txt;
use zng_var::{ResponderVar, ResponseVar, Var, VarEq, VarValue, const_var, response_var, var};

use crate::task::{SetupTask, SetupTaskError, SetupTaskType, TaskTypeId};

/// Setup service.
///
/// This service runs [install] and [uninstall] operations sequentially. Operations
/// start once the current app update finishes and there are no other running operations.
///
/// [install]:  Self::install
/// [uninstall]:  Self::uninstall
pub struct SETUP;

impl SETUP {
    /// Register a custom task type.
    pub fn register_task_type<T: SetupTask>(&self) {
        self.register_task_type_impl(SetupTaskType::new::<T>());
    }
    fn register_task_type_impl(&self, t: SetupTaskType) {
        UPDATES.once_update("register_task_type", move || {
            let mut sv = SETUP_SV.write();
            let id = (t.task_type_id)();
            if let Some(e) = sv.task_types.iter_mut().find(|t| (t.task_type_id)() == id) {
                *e = t;
            } else {
                sv.task_types.push(t);
            }
        });
    }

    /// Enqueue a new install operation.
    ///
    /// This will [prepare] and [commit] an install.
    ///
    /// Returns a response var that updates once with the result of the operation. If
    /// successful an [`UninstallConfig`], this data can be (de)serialized and used with
    /// to [`uninstall`].
    ///
    /// [prepare]: Self::prepare_install
    /// [commit]: Self::commit_install
    /// [`uninstall`]: Self::uninstall
    pub fn install(&self, config: InstallConfig, update: Option<UninstallConfig>) -> ResponseVar<Result<UninstallConfig, SetupError>> {
        let (r, rsp) = response_var();
        UPDATES.once_update("install", move || {
            SETUP_SV.write().run(async move { install(config, update).await }, r);
        });
        rsp
    }

    /// Enqueue a new prepare install operation.
    ///
    /// This will run all expensive install operations that can run without affecting the system or previous installs.
    /// One reason to run this step separate is to begin an update installation while the application is still running.
    ///
    /// If `update` is set the tasks will use it to find and patch/replace a previous install.
    ///
    /// Returns a response var that updates once with the result of the operation. If
    /// successful a [`PreparedInstallConfig`], this data can be (de)serialized and used with
    /// to [`commit_install`] or [`cancel_prepared`].
    ///
    /// [`commit_install`]: Self::commit_install
    /// [`cancel_prepared`]: Self::cancel_prepared
    pub fn prepare_install(
        &self,
        config: InstallConfig,
        update: Option<UninstallConfig>,
    ) -> ResponseVar<Result<PreparedInstallConfig, SetupError>> {
        let (r, rsp) = response_var();
        UPDATES.once_update("prepare_install", move || {
            SETUP_SV.write().run(async move { prepare_install(config, update).await }, r);
        });
        rsp
    }

    /// Enqueue a prepared install cancellation operation.
    ///
    /// Note that [`prepare_install`] will automatically cancel if requested. This method cancels
    /// a prepared install that already completed.
    ///
    /// [`prepare_install`]: Self::prepare_install
    pub fn cancel_prepared(&self, config: PreparedInstallConfig) -> ResponseVar<Result<(), SetupError>> {
        let (r, rsp) = response_var();
        UPDATES.once_update("cancel_prepared", move || {
            SETUP_SV.write().run(async move { cancel_prepared(config).await }, r);
        });
        rsp
    }

    /// Enqueue a new commit prepared install operation.
    ///
    /// This operation cannot be canceled once it starts, if cancel is requested while enqueued
    /// the [`cancel_prepared`] operation will run instead.
    ///
    /// Returns a response var that updates once with the result of the operation. If
    /// successful an [`UninstallConfig`], this data can be (de)serialized and used with
    /// to [`uninstall`].
    ///
    /// [`cancel_prepared`]: Self::cancel_prepared
    /// [`uninstall`]: Self::uninstall
    pub fn commit_install(&self, config: PreparedInstallConfig) -> ResponseVar<Result<UninstallConfig, SetupError>> {
        let (r, rsp) = response_var();
        UPDATES.once_update("commit_install", move || {
            SETUP_SV.write().run(async move { commit_install(config).await }, r);
        });
        rsp
    }

    /// Enqueue a new uninstall operation.
    ///
    /// This will quickly [validate] the installation and uninstall. During uninstall
    /// the operation cannot be canceled.
    ///
    /// Returns a response var that updates once with the result of the operation.
    ///
    /// [validate]: Self::validate_uninstall
    pub fn uninstall(&self, config: UninstallConfig) -> ResponseVar<Result<(), SetupError>> {
        let (r, rsp) = response_var();
        UPDATES.once_update("uninstall", move || {
            SETUP_SV.write().run(async move { uninstall(config).await }, r);
        });
        rsp
    }

    /// Enqueue a new validate uninstall config operation.
    ///
    /// This will verify that the uninstall config can still be used to [`uninstall`].
    ///
    /// Returns a response var that updates once with the result of the operation. If
    /// successful the config data is returned, potentially corrected if recoverable issues where found.
    ///
    /// [`uninstall`]: Self::uninstall
    pub fn validate_uninstall(&self, config: UninstallConfig) -> ResponseVar<Result<UninstallConfig, SetupError>> {
        let (r, rsp) = response_var();
        UPDATES.once_update("validate_uninstall", move || {
            SETUP_SV.write().run(async move { validate_uninstall(config).await }, r);
        });
        rsp
    }

    /// Status of running operation.
    pub fn status(&self) -> Var<SetupStatus> {
        SETUP_SV.read().status.read_only()
    }
}

type SetupOp = Pin<Box<dyn Future<Output = ()> + Send>>;

struct Setup {
    task_types: Vec<SetupTaskType>,
    queue: Mutex<VecDeque<SetupOp>>, // Mutex for +Sync only
    status: Var<SetupStatus>,
    cancel: Var<bool>,
}
app_local! {
    static SETUP_SV: Setup = Setup {
        task_types: vec![
            SetupTaskType::new::<crate::task::ExtractTar>(),
            SetupTaskType::new::<crate::task::CreateShortcut>(),
            SetupTaskType::new::<crate::task::RegisterUninstaller>(),
        ],
        queue: Mutex::default(),
        status: var(SetupStatus::Idle),
        cancel: var(false),
    };
}

/// Represents a list of tasks for a [`SETUP.install`] operation.
///
/// [`SETUP.install`]: SETUP::install
#[derive(Default)]
pub struct InstallConfig {
    cfg: Vec<(SetupTaskType, Box<dyn Any + Send>)>,
    tasks: Vec<(TaskTypeId, Txt)>,
}
impl InstallConfig {
    /// New empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a task to run.
    ///
    /// The `name` is used to identify the task instance in progress status.
    pub fn push<T: SetupTask>(&mut self, name: impl Into<Txt>, config: T::InstallConfig) {
        let t = SetupTaskType::new::<T>();
        self.tasks.push(((t.task_type_id)(), name.into()));
        self.cfg.push((t, Box::new(config)))
    }

    /// Task types and names in order they will execute.
    pub fn tasks(&self) -> &[(TaskTypeId, Txt)] {
        &self.tasks
    }

    /// Inspect the task config.
    pub fn config<T: SetupTask>(&self, index: usize) -> Option<&T::InstallConfig> {
        self.cfg.get(index)?.1.downcast_ref()
    }
}

/// Represents data generated  by [`SETUP.prepare_install`] that can be used to run a [`SETUP.commit_install`] operation.
///
/// [`SETUP.prepare_install`]: SETUP::prepare_install
/// [`SETUP.commit_install`]: SETUP::commit_install
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct PreparedInstallConfig {
    tasks: Vec<(TaskTypeId, Txt)>,
    cfg: Vec<RawConfigValue>,
}
impl PreparedInstallConfig {
    /// Task types and names in order they will execute.
    pub fn tasks(&self) -> &[(TaskTypeId, Txt)] {
        &self.tasks
    }
}

/// Represents data generated by [`SETUP.install`] that can be used to run a [`SETUP.uninstall`] operation.
///
/// [`SETUP.install`]: SETUP::install
/// [`SETUP.uninstall`]: SETUP::uninstall
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct UninstallConfig {
    tasks: Vec<(TaskTypeId, Txt)>,
    cfg: Vec<RawConfigValue>,
}
impl UninstallConfig {
    /// Task types and names in order they will execute.
    pub fn tasks(&self) -> &[(TaskTypeId, Txt)] {
        &self.tasks
    }
}

/// Represents a [`SETUP`] operation error.
#[derive(Clone, PartialEq, Debug)]
#[non_exhaustive]
pub struct SetupError {
    /// Error associated with operation itself that affects all tasks.
    ///
    /// This is often [`SetupTaskError::CorruptedTaskData`] detected
    pub op_error: Option<SetupTaskError>,

    /// Errors associated with a task in the operation.
    ///
    /// Each task is identified by index on the operation, type and name.
    pub task_errors: Vec<((usize, TaskTypeId, Txt), SetupTaskError)>,

    /// Operation state after not completing successfully.
    pub state: SetupErrorState,
}
impl SetupError {
    /// No actual error, canceled by request.
    pub fn canceled() -> Self {
        Self {
            op_error: None,
            task_errors: vec![],
            state: SetupErrorState::Canceled,
        }
    }

    /// One or more tasks failed.
    ///
    /// Each task is identified by index on the operation, type and name.
    ///
    /// If the tasks managed to reverse all changes before committing the `state` must be `Canceled`.
    pub fn task_errors(errors: Vec<((usize, TaskTypeId, Txt), SetupTaskError)>, state: SetupErrorState) -> Self {
        Self {
            op_error: None,
            task_errors: errors,
            state,
        }
    }

    /// Operation failed with error associated with operation itself that affects all tasks.
    ///
    /// If the error is detected before any task runs and any non-destructive change was made the `state` must be `Canceled`.
    pub fn op_error(error: SetupTaskError, state: SetupErrorState) -> Self {
        Self {
            op_error: Some(error),
            task_errors: vec![],
            state,
        }
    }

    /// Operation cannot start due to corrupted config data.
    pub fn corrupted_op_config(config_name: &'static str) -> Self {
        #[derive(Debug)]
        struct CorruptedOpConfig(&'static str);
        impl fmt::Display for CorruptedOpConfig {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{} is corrupted", self.0)
            }
        }
        impl std::error::Error for CorruptedOpConfig {}
        Self::op_error(
            SetupTaskError::CorruptedTaskData(Arc::new(CorruptedOpConfig(config_name))),
            SetupErrorState::Canceled,
        )
    }
}

/// Represents state of a setup operation that ended in a [`SetupError`].
#[derive(Clone, PartialEq, Debug)]
pub enum SetupErrorState {
    /// Canceled successfully, all changes where reverted.
    ///
    /// The operation is canceled by request or by error before it starts committing irreversible changes,
    /// either way the system and any previous installation is not affected when ended in this state.
    Canceled,

    /// Install operation failed before it started making irreversible changes and failed to cleanup
    /// temporary changes.
    ///
    /// When an install fails during the preparing phase it automatically attempts to *cancel*, this is
    /// the error when that cancel fails. If the install was an update the previous version will still be valid.
    PartialPrepareInstall,

    /// Install operation failed while making irreversible changes to the system.
    ///
    /// Operation attempts to complete as much of the install as possible, the associated `data`
    /// can be used to uninstall the committed changes. Well designed tasks will cleanup all temporary
    /// *prepared* data on error and generate uninstall data that cleanups even partial written files, but
    /// there is no guarantee that this data will fully uninstall every change.
    ///
    /// If the operation was replacing a previous install (update or repair) the `data` will also
    /// uninstall the previous installation.
    PartialInstall {
        /// Data that can uninstall all the successfully committed changes made during the failed install.
        data: UninstallConfig,
        /// Tasks that failed without generating uninstall data.
        ///
        /// If this is not empty uninstalling `data` will definitely not fully cleanup the broken install.
        ///
        /// Tasks are identified by index on the operation, type and name.
        no_data: Vec<(usize, TaskTypeId, Txt)>,
    },
    /// Uninstall operation failed while making irreversible changes to the system.
    ///
    /// Operation attempts to complete as much of the uninstall as possible, so all tasks without error
    /// have completed successfully.
    ///
    ///
    PartialUninstall,
}

/// Represents status of [`SETUP`].
#[derive(Debug, PartialEq, Clone)]
#[non_exhaustive]
pub enum SetupStatus {
    /// No setup operation is running.
    Idle,
    /// Prepare install is running or complete.
    PrepareInstall(SetupOpStatus),
    /// Commit install is running or complete.
    CommitInstall(SetupOpStatus),
    /// Validate uninstall is running or complete.
    ValidateUninstall(SetupOpStatus),
    /// Uninstall is running or complete.
    Uninstall(SetupOpStatus),
}
impl SetupStatus {
    /// If is `Idle` or op is complete.
    pub fn is_idle(&self) -> bool {
        match self {
            SetupStatus::Idle => true,
            SetupStatus::PrepareInstall(s)
            | SetupStatus::CommitInstall(s)
            | SetupStatus::ValidateUninstall(s)
            | SetupStatus::Uninstall(s) => s.is_complete(),
        }
    }

    /// Get operation status.
    pub fn op_status(&self) -> Option<&SetupOpStatus> {
        match self {
            SetupStatus::Idle => None,
            SetupStatus::PrepareInstall(s)
            | SetupStatus::CommitInstall(s)
            | SetupStatus::ValidateUninstall(s)
            | SetupStatus::Uninstall(s) => Some(s),
        }
    }
}

/// Represents status of a [`SETUP`] install or uninstall operation.
#[derive(Debug, PartialEq, Clone)]
#[non_exhaustive]
pub struct SetupOpStatus {
    /// Is cancelling.
    pub cancel: bool,
    /// Current task.
    pub task: (TaskTypeId, Txt),
    /// Task index of len.
    pub progress: (usize, usize),
    /// Progress report from task.
    pub task_progress: VarEq<Progress>,

    /// Errors.
    ///
    /// The task is identified by index, type and name.
    pub errors: Vec<((usize, TaskTypeId, Txt), SetupTaskError)>,
}
impl SetupOpStatus {
    /// If `progress` is last task and `task_progress` is complete.
    ///
    /// Note that tasks report completion on error.
    pub fn is_complete(&self) -> bool {
        self.progress.0 == self.progress.1.saturating_sub(1) && self.task_progress.with(|p| p.is_complete())
    }

    /// If `cancel` and `is_complete`.
    ///
    /// If this is `true` and `errors` is not empty the tasks managed to cleanup and the install is not corrupted.
    pub fn is_canceled(&self) -> bool {
        self.cancel && self.is_complete()
    }

    /// If `is_complete`, not `cancel` and has `errors`.
    ///
    /// If this is `true` the tasks did not manage to cleanup and the install is in a corrupted state.
    pub fn is_corrupted(&self) -> bool {
        !self.errors.is_empty() && !self.cancel && self.is_complete()
    }
}

impl Setup {
    fn run<R: VarValue>(
        &mut self,
        op: impl Future<Output = Result<R, SetupError>> + Send + 'static,
        r: ResponderVar<Result<R, SetupError>>,
    ) {
        self.run_impl(Box::pin(async move {
            let res = op.await;
            r.respond(res);
        }));
    }
    fn run_impl(&mut self, op: SetupOp) {
        let q = self.queue.get_mut();
        q.push_back(op);
        if q.len() == 1 {
            zng_task::spawn(async {
                fn next_op() -> Option<SetupOp> {
                    let mut sv = SETUP_SV.write();
                    let op = sv.queue.get_mut().pop_front();
                    if op.is_some() {
                        // ensure op will not retain errors from previous op
                        sv.status.set(SetupStatus::Idle);
                    }
                    op
                }
                while let Some(op) = next_op() {
                    op.await;
                }
            });
        }
    }

    fn task_type(&self, id: &TaskTypeId) -> Result<SetupTaskType, SetupTaskError> {
        for t in &self.task_types {
            if &(t.task_type_id)() == id {
                return Ok(t.clone());
            }
        }
        Err(SetupTaskError::UnknownType(id.clone()))
    }
}

async fn install(config: InstallConfig, update: Option<UninstallConfig>) -> Result<UninstallConfig, SetupError> {
    let config = prepare_install(config, update).await?;
    if SETUP_SV.read().cancel.get() {
        cancel_prepared(config).await?;
        Err(SetupError::canceled())
    } else {
        commit_install(config).await
    }
}

async fn prepare_install(config: InstallConfig, update: Option<UninstallConfig>) -> Result<PreparedInstallConfig, SetupError> {
    let (status, cancel) = {
        let sv = SETUP_SV.read();
        (sv.status.clone(), sv.cancel.clone())
    };

    if let Some(u) = &update
        && u.tasks.len() != u.cfg.len()
    {
        return Err(SetupError::corrupted_op_config("UninstallConfig"));
    }

    let tasks_len = config.tasks.len();
    if tasks_len != config.cfg.len() {
        return Err(SetupError::corrupted_op_config("InstallConfig"));
    }
    let mut prepared_cfg: Vec<RawConfigValue> = Vec::with_capacity(tasks_len);
    let mut error = None;
    for (i, (id, (task_ty, cfg))) in config.tasks.iter().zip(config.cfg).enumerate() {
        let task_progress = var(Progress::indeterminate());
        // notify new task started
        let task_progress_s = task_progress.read_only();
        status.modify(clmv!(id, |a| {
            match a.value_mut() {
                SetupStatus::PrepareInstall(s) => {
                    s.task = id;
                    s.progress.0 = i;
                    s.task_progress = VarEq(task_progress_s);
                }
                _ => {
                    **a = SetupStatus::PrepareInstall(SetupOpStatus {
                        cancel: false,
                        task: id,
                        progress: (i, tasks_len),
                        task_progress: VarEq(task_progress_s),
                        errors: vec![],
                    });
                }
            }
        }));

        // find previous install
        let mut uninstall_data = None;
        if let Some(u) = &update {
            // uninstall is reversed
            if let Some(i) = u.cfg.len().checked_sub(i + 1)
                && id == &u.tasks[i]
            {
                uninstall_data = Some(u.cfg[i].clone());
            }

            if uninstall_data.is_none() {
                #[derive(Debug)]
                struct TaskTypeMismatch;
                impl fmt::Display for TaskTypeMismatch {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(f, "expected different task type")
                    }
                }
                impl std::error::Error for TaskTypeMismatch {}
                error = Some((
                    (i, id.0.clone(), id.1.clone()),
                    SetupTaskError::CorruptedTaskData(Arc::new(TaskTypeMismatch)),
                ));
                break;
            }
        }

        // run task
        let r = (task_ty.prepare_install)(cfg, uninstall_data, task_progress.clone(), cancel.read_only()).await;
        task_progress.set(Progress::complete());
        match r {
            Ok(r) => prepared_cfg.push(r),
            Err(e) => {
                // go to cancel due to error
                error = Some(((i, id.0.clone(), id.1.clone()), e));
                break;
            }
        }
        if cancel.get() {
            break;
        }
    }

    let prepared_cfg = PreparedInstallConfig {
        tasks: config.tasks,
        cfg: prepared_cfg,
    };

    if let Some(e) = error {
        status.modify(clmv!(e, |a| {
            // add error, cancel_prepared will preserve it as it updates status
            if let SetupStatus::PrepareInstall(s) = a.value_mut() {
                s.errors.push(e);
                // immediately indicate cancelling too, to avoid notifying completion
                s.task_progress = VarEq(const_var(Progress::indeterminate()));
                s.cancel = true;
            }
        }));

        // cancel due to error
        if let Err(mut ce) = cancel_prepared(prepared_cfg).await {
            // did not cleanup prepared either.
            ce.state = SetupErrorState::PartialPrepareInstall;
            ce.task_errors.insert(0, e);
            Err(ce)
        } else {
            Err(SetupError::task_errors(vec![e], SetupErrorState::Canceled))
        }
    } else if cancel.get() {
        // cancel due to request
        cancel_prepared(prepared_cfg).await?;
        Err(SetupError::canceled())
    } else {
        // ensure general status updates to complete
        status.modify(clmv!(|a| {
            if let SetupStatus::PrepareInstall(s) = a.value_mut() {
                s.task_progress = VarEq(const_var(Progress::complete()));
            }
        }));
        Ok(prepared_cfg)
    }
}

async fn cancel_prepared(config: PreparedInstallConfig) -> Result<(), SetupError> {
    let status = SETUP_SV.read().status.clone();

    let tasks_len = config.cfg.len();
    if tasks_len > config.tasks.len() {
        return Err(SetupError::corrupted_op_config("PreparedInstallConfig"));
    }

    let mut errors = vec![];

    for (i, (id, cfg)) in config.tasks.into_iter().zip(config.cfg).enumerate() {
        let task_progress = var(Progress::indeterminate());
        // notify new task started
        let task_progress_s = task_progress.read_only();
        status.modify(clmv!(id, |a| {
            match a.value_mut() {
                SetupStatus::PrepareInstall(s) => {
                    s.cancel = true;
                    s.task = id;
                    s.progress = (i, tasks_len);
                    s.task_progress = VarEq(task_progress_s);
                }
                _ => {
                    **a = SetupStatus::PrepareInstall(SetupOpStatus {
                        cancel: true,
                        task: id,
                        progress: (i, tasks_len),
                        task_progress: VarEq(task_progress_s),
                        errors: vec![],
                    });
                }
            }
        }));

        // run task
        let task_ty = SETUP_SV.read().task_type(&id.0);
        let error = match task_ty {
            Ok(task_ty) => {
                let r = (task_ty.cancel_install)(cfg, task_progress.clone()).await;
                r.err()
            }
            Err(e) => Some(e),
        };

        if let Some(e) = error {
            let e = ((i, id.0, id.1), e);
            // notify error
            status.modify(clmv!(e, |a| {
                if let SetupStatus::PrepareInstall(s) = a.value_mut() {
                    s.errors.push(e);
                    s.cancel = false;
                    s.task_progress = VarEq(const_var(Progress::complete()));
                }
            }));
            errors.push(e);

            // continues trying to cancel other tasks for best effort cleanup
        } else {
            task_progress.set(Progress::complete());
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(SetupError::task_errors(errors, SetupErrorState::PartialPrepareInstall))
    }
}

async fn commit_install(config: PreparedInstallConfig) -> Result<UninstallConfig, SetupError> {
    let status = SETUP_SV.read().status.clone();

    let tasks_len = config.tasks.len();
    if tasks_len != config.cfg.len() {
        return Err(SetupError::corrupted_op_config("PreparedInstallConfig"));
    }

    let mut errors = vec![];
    let mut uninstall_cfg = vec![];
    let mut err_no_clean = vec![];

    for (i, (id, cfg)) in config.tasks.iter().zip(config.cfg).enumerate() {
        let task_progress = var(Progress::indeterminate());
        // notify new task started
        let task_progress_s = task_progress.read_only();
        status.modify(clmv!(id, |a| {
            match a.value_mut() {
                SetupStatus::CommitInstall(s) => {
                    s.task = id;
                    s.progress.0 = i;
                    s.task_progress = VarEq(task_progress_s);
                }
                _ => {
                    **a = SetupStatus::CommitInstall(SetupOpStatus {
                        cancel: false,
                        task: id,
                        progress: (i, tasks_len),
                        task_progress: VarEq(task_progress_s),
                        errors: vec![],
                    })
                }
            }
        }));

        // run task
        let task_ty = SETUP_SV.read().task_type(&id.0);
        let mut error = None;
        match task_ty {
            Ok(task_ty) => match (task_ty.install)(cfg, task_progress.clone()).await {
                Ok(c) => {
                    uninstall_cfg.push(c);
                }
                Err(e) => {
                    error = Some(e.error);
                    if let Some(d) = e.clean_data {
                        uninstall_cfg.push(d);
                    } else {
                        err_no_clean.push((i, id.0.clone(), id.1.clone()));
                    }
                }
            },
            Err(e) => error = Some(e),
        };

        if let Some(e) = error {
            let e = ((i, id.0.clone(), id.1.clone()), e);
            // notify error
            status.modify(clmv!(e, |a| {
                if let SetupStatus::CommitInstall(s) = a.value_mut() {
                    s.errors.push(e);
                    s.cancel = false;
                    s.task_progress = VarEq(const_var(Progress::complete()));
                }
            }));
            errors.push(e);

            // continues trying to commit other tasks, since cannot recover at this
            // point might as well try to deliver a partial install
        } else {
            task_progress.set(Progress::complete());
        }
    }

    let mut tasks = config.tasks;
    tasks.reverse();
    uninstall_cfg.reverse();
    let data = UninstallConfig { tasks, cfg: uninstall_cfg };

    if errors.is_empty() {
        // ensure general status updates to complete
        status.modify(clmv!(|a| {
            if let SetupStatus::CommitInstall(s) = a.value_mut() {
                s.task_progress = VarEq(const_var(Progress::complete()));
            }
        }));

        Ok(data)
    } else {
        Err(SetupError::task_errors(
            errors,
            SetupErrorState::PartialInstall {
                data,
                no_data: err_no_clean,
            },
        ))
    }
}

async fn uninstall(config: UninstallConfig) -> Result<(), SetupError> {
    let config = validate_uninstall(config).await?;

    let status = SETUP_SV.read().status.clone();

    let tasks_len = config.tasks.len();
    if tasks_len != config.cfg.len() {
        return Err(SetupError::corrupted_op_config("UninstallConfig"));
    }

    let mut errors = vec![];

    for (i, (id, cfg)) in config.tasks.into_iter().zip(config.cfg).enumerate() {
        let task_progress = var(Progress::indeterminate());
        // notify new task started
        let task_progress_s = task_progress.read_only();
        status.modify(clmv!(id, |a| {
            match a.value_mut() {
                SetupStatus::Uninstall(s) => {
                    s.task = id;
                    s.progress.0 = i;
                    s.task_progress = VarEq(task_progress_s);
                }
                _ => {
                    **a = SetupStatus::Uninstall(SetupOpStatus {
                        cancel: false,
                        task: id,
                        progress: (i, tasks_len),
                        task_progress: VarEq(task_progress_s),
                        errors: vec![],
                    })
                }
            }
        }));

        // run task
        let task_ty = SETUP_SV.read().task_type(&id.0);
        let error = match task_ty {
            Ok(task_ty) => (task_ty.uninstall)(cfg, task_progress.clone()).await.err(),
            Err(e) => Some(e),
        };

        if let Some(e) = error {
            let e = ((i, id.0.clone(), id.1.clone()), e);
            // notify error
            status.modify(clmv!(e, |a| {
                if let SetupStatus::Uninstall(s) = a.value_mut() {
                    s.errors.push(e);
                    s.cancel = false;
                    s.task_progress = VarEq(const_var(Progress::complete()));
                }
            }));
            errors.push(e);

            // continues trying to commit other tasks, since cannot recover at this
            // point might as well try to deliver a partial install
        } else {
            task_progress.set(Progress::complete());
        }
    }
    if errors.is_empty() {
        // ensure general status updates to complete
        status.modify(clmv!(|a| {
            if let SetupStatus::Uninstall(s) = a.value_mut() {
                s.task_progress = VarEq(const_var(Progress::complete()));
            }
        }));
        Ok(())
    } else {
        Err(SetupError::task_errors(errors, SetupErrorState::PartialUninstall))
    }
}

async fn validate_uninstall(config: UninstallConfig) -> Result<UninstallConfig, SetupError> {
    let (status, cancel) = {
        let sv = SETUP_SV.read();
        (sv.status.clone(), sv.cancel.clone())
    };

    let tasks_len = config.tasks.len();
    if tasks_len != config.cfg.len() {
        return Err(SetupError::corrupted_op_config("UninstallConfig"));
    }

    let UninstallConfig { tasks, mut cfg } = config;
    let empty_cfg = RawConfigValue::serialize(()).unwrap();

    let mut errors = vec![];

    for (i, (id, cfg)) in tasks.iter().zip(cfg.iter_mut()).enumerate() {
        let task_progress = var(Progress::indeterminate());
        // notify new task started
        let task_progress_s = task_progress.read_only();
        status.modify(clmv!(id, |a| {
            match a.value_mut() {
                SetupStatus::ValidateUninstall(s) => {
                    s.task = id;
                    s.progress.0 = i;
                    s.task_progress = VarEq(task_progress_s);
                }
                _ => {
                    **a = SetupStatus::ValidateUninstall(SetupOpStatus {
                        cancel: false,
                        task: id,
                        progress: (i, tasks_len),
                        task_progress: VarEq(task_progress_s),
                        errors: vec![],
                    })
                }
            }
        }));

        // run task
        let task_ty = SETUP_SV.read().task_type(&id.0);
        let error = match task_ty {
            Ok(task_ty) => {
                match (task_ty.validate_uninstall)(std::mem::replace(cfg, empty_cfg.clone()), task_progress.clone(), cancel.clone()).await {
                    Ok(c) => {
                        *cfg = c;
                        None
                    }
                    Err(e) => Some(e),
                }
            }
            Err(e) => Some(e),
        };

        if let Some(e) = error {
            let e = ((i, id.0.clone(), id.1.clone()), e);
            // notify error
            status.modify(clmv!(e, |a| {
                if let SetupStatus::ValidateUninstall(s) = a.value_mut() {
                    s.errors.push(e);
                    s.cancel = false;
                    s.task_progress = VarEq(const_var(Progress::complete()));
                }
            }));
            errors.push(e);

            // continues trying to commit other tasks, since cannot recover at this
            // point might as well try to deliver a partial install
        } else {
            task_progress.set(Progress::complete());
        }

        if cancel.get() {
            break;
        }
    }

    let canceled = cancel.get();
    if errors.is_empty() && !canceled {
        // ensure general status updates to complete
        status.modify(clmv!(|a| {
            if let SetupStatus::ValidateUninstall(s) = a.value_mut() {
                s.task_progress = VarEq(const_var(Progress::complete()));
            }
        }));
        Ok(UninstallConfig { tasks, cfg })
    } else {
        Err(SetupError::task_errors(errors, SetupErrorState::Canceled))
    }
}
