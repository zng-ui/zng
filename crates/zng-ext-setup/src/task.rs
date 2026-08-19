//! Install and uninstall tasks.
//!
//! See [`SetupTask`] docs for a description of the steps an install or uninstall task runs.

mod extract_tar;
pub use extract_tar::{ExtractTar, ExtractTarConfig};

mod create_shortcut;
#[cfg(any(windows, target_os = "linux"))]
pub use create_shortcut::{CreateShortcut, CreateShortcutConfig};

mod register_uninstaller;
#[cfg(windows)]
pub use register_uninstaller::{RegisterUninstaller, RegisterUninstallerConfig};

use zng_task::Progress;
use zng_txt::Txt;
use zng_var::{Var, impl_from_and_into_var};

use std::{any::Any, borrow::Cow, error::Error, fmt, io, ops, path::PathBuf, pin::Pin, sync::Arc};

use zng_ext_config::{ConfigValue, RawConfigValue};

/// Unique name for an install or uninstall task.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct TaskTypeId(pub Txt);
impl ops::Deref for TaskTypeId {
    type Target = Txt;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl_from_and_into_var! {
    fn from(id: &'static str) -> TaskTypeId {
        TaskTypeId(id.into())
    }
    fn from(id: Txt) -> TaskTypeId {
        TaskTypeId(id)
    }
}
impl fmt::Display for TaskTypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Represents an install and uninstall task implementation.
///
/// Setup tasks runs in steps, steps do not necessarily run on the same process, communication
/// between steps is done using serialized data. The steps are implemented as associated functions,
/// not methods, the task type is not instantiated.
///
/// Each step runs for all tasks on the setup list, before moving to the next step.
///
/// # Install Steps
///
/// 1 - If the user did not cancel, [`SetupTask::prepare_install`] is called.
/// 2.a - If did not cancel, [`SetupTask::install`] is called, the user cannot cancel once this starts.
/// 2.b - If did cancel, [`SetupTask::cancel_install`] is called.
///
/// Note that steps 1 and 2 might not run on the same process. A case where this happens is a self-updater
/// that starts preparing to install the update while it is still running.
///
/// # Uninstall Steps
///
/// 1 - [`SetupTask::Install`] data is deserialized from the install log.
/// 2 - [`SetupTask::validate_uninstall`] is called.
/// 3 - If did not cancel, [`SetupTask::uninstall`] is called.
///
/// # Register
///
/// Custom setup task types must be registered with [`SETUP.register_task_type`] otherwise install and uninstall
/// will fail with [`SetupTaskError::UnknownType`].
///
/// [`SETUP.register_task_type`]: crate::SETUP::register_task_type
pub trait SetupTask: Sized {
    /// Install config type.
    type InstallConfig: Any + Send;
    /// Prepared install data type.
    type PrepareInstall: ConfigValue;
    /// Installed data type.
    type Install: ConfigValue;

    /// Unique ID for the task type.
    fn task_type_id() -> TaskTypeId;

    /// Run all expensive install operations that can run without affecting the system or previous installs.
    ///
    /// This step **must not** cause any change that affects existing install, even if reversible, it must only
    /// run all potentially expensive tasks in such a way that the final *commit* can happen quickly.
    ///
    /// The user may cancel the install at any time, if possible monitor the [`cancel`] var and return
    /// early on cancel. Implement cancellation cleanup on [`cancel_install`].
    ///
    /// [`cancel_install`]: Self::cancel_install
    /// [`cancel`]: PrepareInstallArgs::cancel
    fn prepare_install(
        args: PrepareInstallArgs<Self>,
    ) -> impl Future<Output = Result<Self::PrepareInstall, SetupTaskError>> + Send + 'static;

    /// Commit prepared install changes.
    ///
    /// The user cannot cancel installation when this step is running. Progress indicators will only show *indeterminate*
    /// with the expectation this step will finish quickly.
    ///
    /// Install must not fail at the first error encountered, a best attempt to apply all install steps must be made,
    /// errors can be aggregated on the [`InstallTaskError::error`]. The [`InstallTaskError::clean_data`] must include uninstall instructions
    /// for all successful steps, best attempt of partial steps and any data from the previous version that was not replaced in case
    /// it is installing an update.
    ///
    /// [`prepare_install`]: Self::prepare_install
    fn install(args: InstallArgs<Self>) -> impl Future<Output = Result<Self::Install, InstallTaskError<Self::Install>>> + Send + 'static;

    /// Cancel prepared install changes.
    ///
    /// This is called if the user requested cancel during or after [`prepare_install`] and before [`install`].
    ///
    /// This step must find and cleanup all prepared changes, such as temporary files. The cancel logic must be resilient to
    /// partial changes as [`prepare_install`] might return early due to user cancel or an error.
    ///
    /// [`prepare_install`]: Self::prepare_install
    /// [`install`]: Self::install
    fn cancel_install(args: CancelInstallArgs<Self>) -> impl Future<Output = Result<(), SetupTaskError>> + Send + 'static;

    /// Validate the install state for uninstall.
    ///
    /// This step **must not** make any changes to the file system, not even creating temp files. This step
    /// allows tasks to validate the install state before [`uninstall`] makes irreversible changes.
    ///
    /// This step is not expected to take long, but if it does check the [`cancel`] flag to avoid unnecessary work.
    /// If the uninstall is canceled when another task is preparing after this one the returned data is just dropped.
    ///
    /// This step returns a validation error or the corrected install data.
    ///
    /// [`uninstall`]: Self::uninstall
    /// [`cancel`]: ValidateUninstallArgs::cancel
    fn validate_uninstall(
        args: ValidateUninstallArgs<Self>,
    ) -> impl Future<Output = Result<Self::Install, SetupTaskError>> + Send + 'static;

    /// Uninstall.
    ///
    /// The user cannot cancel uninstallation when this step is running.
    ///
    /// Uninstall must not fail in case a step is already completed, for example, if the task must remove a file
    /// and it is not found, that is not an error. Task runners can retry partially run uninstall with an install data clone.
    ///
    /// Uninstall must not fail at the first error encountered, a best attempt to apply all uninstall steps must be made,
    /// errors can be aggregated on the [`SetupTaskError`].
    fn uninstall(args: UninstallArgs<Self>) -> impl Future<Output = Result<(), SetupTaskError>> + Send + 'static;
}

/// Arguments for [`SetupTask::prepare_install`]
#[non_exhaustive]
pub struct PrepareInstallArgs<T: SetupTask> {
    /// Config for the new installation.
    pub config: T::InstallConfig,

    /// Data from the previous installation that is being replaced with this one.
    ///
    /// This is set if is installing over a previous installation and the same task is present
    /// on the new installation.
    pub update: Option<T::Install>,

    /// Progress indicator for the task. Starts as [`Progress::indeterminate`] by default.
    pub progress: Var<Progress>,
    /// Read-only var that is `true` if the user cancels the installation.
    ///
    /// If possible check this flag often and return immediately on cancel. The *prepare install*
    /// step is not expected to cleanup on cancel, just return immediately.
    pub cancel: Var<bool>,
}

/// Arguments for [`SetupTask::install`].
#[non_exhaustive]
pub struct InstallArgs<T: SetupTask> {
    /// Data generated by [`SetupTask::prepare_install`].
    pub data: T::PrepareInstall,

    /// Progress indicator for the task cancellation. Starts as [`Progress::indeterminate`] by default.
    pub progress: Var<Progress>,
}

/// Arguments for [`SetupTask::cancel_install`].
#[non_exhaustive]
pub struct CancelInstallArgs<T: SetupTask> {
    /// Data generated by [`SetupTask::prepare_install`].
    ///
    /// Data may be partial if it was returned because user requested cancel.
    pub data: T::PrepareInstall,

    /// Progress indicator for the task cancellation. Starts as [`Progress::indeterminate`] by default.
    pub progress: Var<Progress>,
}

/// Arguments for [`SetupTask::validate_uninstall`].
#[non_exhaustive]
pub struct ValidateUninstallArgs<T: SetupTask> {
    /// Data generated by [`SetupTask::install`].
    pub data: T::Install,

    /// Progress indicator for the task uninstall. Starts as [`Progress::indeterminate`] by default.
    pub progress: Var<Progress>,
    /// Read-only var that is `true` if the user cancels uninstallation.
    ///
    /// If possible check this flag often and return immediately on cancel.
    pub cancel: Var<bool>,
}

/// Arguments for [`SetupTask::uninstall`].
#[non_exhaustive]
pub struct UninstallArgs<T: SetupTask> {
    /// Data generated by [`SetupTask::install`].
    pub data: T::Install,
    /// Progress indicator for the task uninstall. Continues from [`ValidateUninstallArgs::progress`].
    pub progress: Var<Progress>,
}

/// Represents a [`SetupTask`] step error.
///
/// Some tasks may continue after an error in a best attempt to at least complete most of the work,
/// this can cause multiple errors to aggregate. In these cases the [`Error::source`] is the first
/// error.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SetupTaskError {
    /// Task data is in an unexpected format.
    CorruptedTaskData(Arc<dyn Error + Send + Sync>),
    /// Task type is not [registered].
    ///
    /// [registered]: crate::SETUP::register_task_type
    UnknownType(TaskTypeId),
    /// IO errors associated with a file or directory path.
    Io(Vec<(PathBuf, Arc<std::io::Error>)>),
    /// Other errors.
    Other(Vec<Arc<dyn Error + Send + Sync>>),
}
impl SetupTaskError {
    /// New `Io` error with a single entry.
    pub fn io(related_path: PathBuf, error: std::io::Error) -> Self {
        Self::Io(vec![(related_path, Arc::new(error))])
    }

    /// New `Other` error with a single entry.
    pub fn other(error: impl Error + Send + Sync + 'static) -> Self {
        Self::Other(vec![Arc::new(error)])
    }
}
/// Inner errors only compare `Arc` pointer.
impl PartialEq for SetupTaskError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::CorruptedTaskData(a), Self::CorruptedTaskData(b)) => Arc::ptr_eq(a, b),
            (Self::UnknownType(a), Self::UnknownType(b)) => a == b,
            (Self::Io(a), Self::Io(b)) => a.len() == b.len() && a.iter().zip(b).all(|(a, b)| Arc::ptr_eq(&a.1, &b.1) && a.0 == b.0),
            (Self::Other(a), Self::Other(b)) => a.len() == b.len() && a.iter().zip(b).all(|(a, b)| Arc::ptr_eq(a, b)),
            _ => false,
        }
    }
}
impl fmt::Display for SetupTaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SetupTaskError::CorruptedTaskData(e) => write!(f, "corrupted task data, {e}"),
            SetupTaskError::UnknownType(t) => write!(f, "unknown task type {t}"),
            SetupTaskError::Io(e) => {
                let tab = if e.len() > 1 { "   " } else { "" };
                let mut sep = "";
                if e.len() > 1 {
                    write!(f, "{} io errors:", e.len())?;
                    sep = "\n";
                }
                for (p, e) in e.iter() {
                    write!(f, "{sep}{tab}{e}\n{tab}   related path: {}", p.display())?;
                    sep = "\n";
                }
                if e.is_empty() { write!(f, "unknown io error") } else { Ok(()) }
            }
            SetupTaskError::Other(e) => {
                let tab = if e.len() > 1 { "   " } else { "" };
                let mut sep = "";
                if e.len() > 1 {
                    write!(f, "{} errors:", e.len())?;
                    sep = "\n";
                }
                for e in e.iter() {
                    write!(f, "{sep}{tab}{e}")?;
                }
                if e.is_empty() { write!(f, "unknown error") } else { Ok(()) }
            }
        }
    }
}
impl Error for SetupTaskError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SetupTaskError::CorruptedTaskData(e) => Some(&**e),
            Self::UnknownType(_) => None,
            Self::Io(e) => Some(&e.first()?.1),
            SetupTaskError::Other(e) => Some(&**e.first()?),
        }
    }
}

/// Error in a [`SetupTask::install`] task run.
pub struct InstallTaskError<I> {
    /// The error.
    pub error: SetupTaskError,
    /// Cleanup [`SetupTask::Install`] data.
    ///
    /// This must contain data to uninstall the partial committed changes, if there where any. Task runners may
    /// use this to attempt a [`SetupTask::uninstall`] to cleanup the corrupted install.
    ///
    /// In case the install is an [update], this must also contain all data from the previous install that has not
    /// been invalidated by the failed install.
    ///
    /// If this is `None` the task runner will show that the task corrupted the install and the changes made
    /// cannot even be uninstalled. It will also assume that the [`SetupTask::PrepareInstall`] data was not
    /// fully cleaned before the error.
    ///
    /// If this is `Some` the task runner will assume the [`SetupTask::PrepareInstall`] is fully cleaned. The
    /// task must attempt to run a *cancel* on the partial prepared data that has not committed yet, if the
    /// error was encountered before any changes where actually committed and all prepared changes where successfully
    /// canceled this must be set to `Some` value that represents an *empty install* that the uninstall task will
    /// recognize and immediately return success for.
    ///
    /// [update]: PrepareInstallArgs::update
    pub clean_data: Option<I>,
}
impl<I> fmt::Debug for InstallTaskError<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InstallTaskError")
            .field("error", &self.error)
            .field("clean_data.is_some()", &self.clean_data.is_some())
            .finish()
    }
}
impl<I> fmt::Display for InstallTaskError<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl<I> std::error::Error for InstallTaskError<I> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

type BoxFutResult<T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'static>>;

fn value_de<T: ConfigValue>(raw: RawConfigValue) -> Result<T, SetupTaskError> {
    match raw.deserialize() {
        Ok(r) => Ok(r),
        Err(e) => Err(SetupTaskError::CorruptedTaskData(Arc::new(e))),
    }
}

#[derive(Clone)]
pub(crate) struct SetupTaskType {
    pub task_type_id: fn() -> TaskTypeId,
    #[allow(clippy::type_complexity)]
    pub prepare_install:
        fn(Box<dyn Any + Send>, Option<RawConfigValue>, Var<Progress>, Var<bool>) -> BoxFutResult<RawConfigValue, SetupTaskError>,
    pub install: fn(RawConfigValue, Var<Progress>) -> BoxFutResult<RawConfigValue, InstallTaskError<RawConfigValue>>,
    pub cancel_install: fn(RawConfigValue, Var<Progress>) -> BoxFutResult<(), SetupTaskError>,
    pub validate_uninstall: fn(RawConfigValue, Var<Progress>, Var<bool>) -> BoxFutResult<RawConfigValue, SetupTaskError>,
    pub uninstall: fn(RawConfigValue, Var<Progress>) -> BoxFutResult<(), SetupTaskError>,
}
impl SetupTaskType {
    /// New task instance.
    pub fn new<T: SetupTask>() -> Self {
        Self {
            task_type_id: T::task_type_id,
            prepare_install: Self::raw_prepare_install::<T>,
            install: Self::raw_install::<T>,
            cancel_install: Self::raw_cancel_install::<T>,
            validate_uninstall: Self::raw_validate_uninstall::<T>,
            uninstall: Self::raw_uninstall::<T>,
        }
    }
    fn raw_prepare_install<T: SetupTask>(
        config: Box<dyn Any + Send>,
        update: Option<RawConfigValue>,
        progress: Var<Progress>,
        cancel: Var<bool>,
    ) -> BoxFutResult<RawConfigValue, SetupTaskError> {
        Box::pin(async move {
            let args = PrepareInstallArgs {
                config: *config.downcast().unwrap(),
                update: match update {
                    Some(d) => value_de(d)?,
                    None => None,
                },
                progress,
                cancel,
            };
            let r = T::prepare_install(args).await?;
            Ok(RawConfigValue::serialize(r).unwrap())
        })
    }
    fn raw_install<T: SetupTask>(
        data: RawConfigValue,
        progress: Var<Progress>,
    ) -> BoxFutResult<RawConfigValue, InstallTaskError<RawConfigValue>> {
        Box::pin(async move {
            let args = InstallArgs {
                data: match value_de(data) {
                    Ok(d) => d,
                    Err(e) => {
                        return Err(InstallTaskError {
                            error: e,
                            // can't cancel either without the data
                            clean_data: None,
                        });
                    }
                },
                progress,
            };
            match T::install(args).await {
                Ok(r) => Ok(RawConfigValue::serialize(r).unwrap()),
                Err(e) => Err(InstallTaskError {
                    error: e.error,
                    clean_data: e.clean_data.map(|d| RawConfigValue::serialize(d).unwrap()),
                }),
            }
        })
    }
    fn raw_cancel_install<T: SetupTask>(data: RawConfigValue, progress: Var<Progress>) -> BoxFutResult<(), SetupTaskError> {
        Box::pin(async move {
            let args = CancelInstallArgs {
                data: value_de(data)?,
                progress,
            };
            T::cancel_install(args).await
        })
    }
    fn raw_validate_uninstall<T: SetupTask>(
        data: RawConfigValue,
        progress: Var<Progress>,
        cancel: Var<bool>,
    ) -> BoxFutResult<RawConfigValue, SetupTaskError> {
        Box::pin(async move {
            let args = ValidateUninstallArgs {
                data: value_de(data)?,
                progress,
                cancel,
            };
            let r = T::validate_uninstall(args).await?;
            Ok(RawConfigValue::serialize(r).unwrap())
        })
    }
    fn raw_uninstall<T: SetupTask>(data: RawConfigValue, progress: Var<Progress>) -> BoxFutResult<(), SetupTaskError> {
        Box::pin(async move {
            let args = UninstallArgs {
                data: value_de(data)?,
                progress,
            };
            T::uninstall(args).await
        })
    }
}

#[allow(unused)]
pub(crate) fn path_utf8(p: PathBuf) -> Result<String, SetupTaskError> {
    match p.to_str() {
        Some(s) => Ok(if cfg!(windows) {
            s.replace('/', "\\")
        } else {
            s.replace('\\', "/")
        }),
        None => Err(SetupTaskError::io(
            p,
            io::Error::new(io::ErrorKind::InvalidData, "path must be utf-8"),
        )),
    }
}
#[allow(unused)]
pub(crate) fn escape_arg(arg: &str) -> Cow<'_, str> {
    #[cfg(windows)]
    {
        shell_escape::windows::escape(Cow::Borrowed(arg))
    }
    #[cfg(not(windows))]
    {
        shell_escape::unix::escape(Cow::Borrowed(arg))
    }
}
