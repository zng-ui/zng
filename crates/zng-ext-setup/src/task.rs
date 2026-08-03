//! Install and uninstall tasks.
//!
//! See [`SetupTaskImpl`] docs for a description of the steps an install or uninstall task runs.

mod extract_tar;
pub use extract_tar::{ExtractTar, ExtractTarConfig};
use zng_task::Progress;
use zng_txt::Txt;
use zng_var::Var;

use std::{any::Any, error::Error, fmt, path::PathBuf};

use zng_ext_config::{ConfigValue, RawConfigValue};

/// Task result.
pub type Result<T> = std::result::Result<T, SetupTaskError>;

/// Unique name for an install or uninstall task.
pub type TaskTypeId = Txt;

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
/// 1 - [`SetupTaskImpl::Config`] default instance is generated and might be editable by user.
/// 2 - If the user did not cancel, [`SetupTaskImpl::prepare_install`] is called.
/// 3.a - If did not cancel, [`SetupTaskImpl::install`] is called, the user cannot cancel once this starts.
/// 3.b - If did cancel, [`SetupTaskImpl::cancel_install`] is called.
///
/// Note that steps 2 and 3 might not run on the same process. A case where this happens is a self-updater
/// that starts preparing to install the update while it is still running.
///
/// # Uninstall Steps
///
/// 1 - [`SetupTaskImpl::Config`] and [`SetupTaskImpl::Install`] data is deserialized from the install log.
/// 2 - [`SetupTaskImpl::validate_uninstall`] is called.
/// 3 - If did not cancel, [`SetupTaskImpl::uninstall`] is called.
pub trait SetupTaskImpl: Sized {
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
    fn prepare_install(args: PrepareInstallArgs<Self>) -> impl Future<Output = Result<Self::PrepareInstall>> + Send + 'static;

    /// Commit prepared install changes.
    ///
    /// The user cannot cancel installation when this step is running. Progress indicators will only show *indeterminate*
    /// with the expectation this step will finish quickly.
    ///
    /// [`prepare_install`]: Self::prepare_install
    fn install(args: InstallArgs<Self>) -> impl Future<Output = Result<Self::Install>> + Send + 'static;

    /// Cancel prepared install changes.
    ///
    /// This is called if the user requested cancel during or after [`prepare_install`] and before [`install`].
    ///
    /// This step must find and cleanup all prepared changes, such as temporary files. The cancel logic must be resilient to
    /// partial changes as [`prepare_install`] might return early due to user cancel or an error.
    ///
    /// [`prepare_install`]: Self::prepare_install
    /// [`install`]: Self::install
    fn cancel_install(args: CancelInstallArgs<Self>) -> impl Future<Output = Result<()>> + Send + 'static;

    /// Validate the install state for uninstall.
    ///
    /// This step **must not** make any changes to the file system, not even creating temp files. This step
    /// allows tasks to validate the install state before [`uninstall`] makes irreversible changes.
    ///
    /// This step is not expected to take long, but if it does check the [`cancel`] flag to avoid unnecessary work.
    /// If the uninstall is canceled when another task is preparing after this one the returned data is just dropped.
    ///
    /// This step returns a validation error or the correct install data.
    ///
    /// [`uninstall`]: Self::uninstall
    /// [`cancel`]: ValidateUninstallArgs::cancel
    fn validate_uninstall(args: ValidateUninstallArgs<Self>) -> impl Future<Output = Result<Self::Install>> + Send + 'static;

    /// Uninstall.
    ///
    /// The user cannot cancel uninstallation when this step is running.
    fn uninstall(args: UninstallArgs<Self>) -> impl Future<Output = Result<()>> + Send + 'static;
}

/// Arguments for [`SetupTaskImpl::prepare_install`]
#[non_exhaustive]
pub struct PrepareInstallArgs<T: SetupTaskImpl> {
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

/// Arguments for [`SetupTaskImpl::install`].
#[non_exhaustive]
pub struct InstallArgs<T: SetupTaskImpl> {
    /// Data generated by [`SetupTaskImpl::prepare_install`].
    pub data: T::PrepareInstall,

    /// Progress indicator for the task cancellation. Starts as [`Progress::indeterminate`] by default.
    pub progress: Var<Progress>,
}

/// Arguments for [`SetupTaskImpl::cancel_install`].
#[non_exhaustive]
pub struct CancelInstallArgs<T: SetupTaskImpl> {
    /// Data generated by [`SetupTaskImpl::prepare_install`].
    ///
    /// Data may be partial if it was returned because user requested cancel.
    pub data: T::PrepareInstall,

    /// Progress indicator for the task cancellation. Starts as [`Progress::indeterminate`] by default.
    pub progress: Var<Progress>,
}

/// Arguments for [`SetupTaskImpl::validate_uninstall`].
#[non_exhaustive]
pub struct ValidateUninstallArgs<T: SetupTaskImpl> {
    /// Data generated by [`SetupTaskImpl::install`].
    pub data: T::Install,

    /// Progress indicator for the task uninstall. Starts as [`Progress::indeterminate`] by default.
    pub progress: Var<Progress>,
    /// Read-only var that is `true` if the user cancels uninstallation.
    ///
    /// If possible check this flag often and return immediately on cancel.
    pub cancel: Var<bool>,
}

/// Arguments for [`SetupTaskImpl::uninstall`].
#[non_exhaustive]
pub struct UninstallArgs<T: SetupTaskImpl> {
    /// Data generated by [`SetupTaskImpl::install`].
    pub data: T::Install,
    /// Progress indicator for the task uninstall. Continues from [`PrepareUninstallArgs::progress`].
    pub progress: Var<Progress>,
}

/// Represents a [`SetupTaskImpl`] error.
///
/// Some tasks may continue after an error in a best attempt to at least complete most of the work,
/// this can cause multiple errors to aggregate. In these cases the [`Error::source`] is the first
/// error.
#[non_exhaustive]
pub enum SetupTaskError {
    /// Task data is in an unexpected format.
    CorruptedTaskData(Box<dyn Error>),
    /// IO errors associated with a file or directory path.
    Io(Vec<(PathBuf, std::io::Error)>),
    /// Other errors.
    Other(Vec<Box<dyn Error>>),
}
impl fmt::Debug for SetupTaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CorruptedTaskData(arg0) => f.debug_tuple("CorruptedTaskData").field(arg0).finish(),
            Self::Io(arg0) => f.debug_tuple("Io").field(arg0).finish(),
            Self::Other(arg0) => f.debug_tuple("Other").field(arg0).finish(),
        }
    }
}
impl fmt::Display for SetupTaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SetupTaskError::CorruptedTaskData(e) => write!(f, "corrupted task data, {e}"),
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
            Self::Io(e) => Some(&e.first()?.1),
            SetupTaskError::Other(e) => Some(&**e.first()?),
        }
    }
}

type BoxFutResult<T> = Box<dyn Future<Output = Result<T>> + Send + 'static>;

fn value_de<T: ConfigValue>(raw: RawConfigValue) -> Result<T> {
    match raw.deserialize() {
        Ok(r) => Ok(r),
        Err(e) => Err(SetupTaskError::CorruptedTaskData(Box::new(e))),
    }
}

/// Represents an instance of a [`SetupTaskImpl`].
pub struct SetupTask {
    instance_id: Txt,
    task_type_id: fn() -> TaskTypeId,
    prepare_install: fn(Box<dyn Any + Send>, Option<RawConfigValue>, Var<Progress>, Var<bool>) -> BoxFutResult<RawConfigValue>,
    install: fn(RawConfigValue, Var<Progress>) -> BoxFutResult<RawConfigValue>,
    cancel_install: fn(RawConfigValue, Var<Progress>) -> BoxFutResult<()>,
    validate_uninstall: fn(RawConfigValue, Var<Progress>, Var<bool>) -> BoxFutResult<RawConfigValue>,
    uninstall: fn(RawConfigValue, Var<Progress>) -> BoxFutResult<()>,
}
impl SetupTask {
    /// New task instance.
    ///
    /// The same task type may be used multiple times during install, the `instance_id` must
    /// differentiate so that uninstall applies on the correct order. It can be an empty string.
    pub fn new<T: SetupTaskImpl>(instance_id: impl Into<Txt>) -> Self {
        Self {
            instance_id: instance_id.into(),
            task_type_id: T::task_type_id,
            prepare_install: Self::raw_prepare_install::<T>,
            install: Self::raw_install::<T>,
            cancel_install: Self::raw_cancel_install::<T>,
            validate_uninstall: Self::raw_validate_uninstall::<T>,
            uninstall: Self::raw_uninstall::<T>,
        }
    }
    fn raw_prepare_install<T: SetupTaskImpl>(
        config: Box<dyn Any + Send>,
        update: Option<RawConfigValue>,
        progress: Var<Progress>,
        cancel: Var<bool>,
    ) -> BoxFutResult<RawConfigValue> {
        Box::new(async move {
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
    fn raw_install<T: SetupTaskImpl>(data: RawConfigValue, progress: Var<Progress>) -> BoxFutResult<RawConfigValue> {
        Box::new(async move {
            let args = InstallArgs {
                data: value_de(data)?,
                progress,
            };
            let r = T::install(args).await?;
            Ok(RawConfigValue::serialize(r).unwrap())
        })
    }
    fn raw_cancel_install<T: SetupTaskImpl>(data: RawConfigValue, progress: Var<Progress>) -> BoxFutResult<()> {
        Box::new(async move {
            let args = CancelInstallArgs {
                data: value_de(data)?,
                progress,
            };
            T::cancel_install(args).await
        })
    }
    fn raw_validate_uninstall<T: SetupTaskImpl>(
        data: RawConfigValue,
        progress: Var<Progress>,
        cancel: Var<bool>,
    ) -> BoxFutResult<RawConfigValue> {
        Box::new(async move {
            let args = ValidateUninstallArgs {
                data: value_de(data)?,
                progress,
                cancel,
            };
            let r = T::validate_uninstall(args).await?;
            Ok(RawConfigValue::serialize(r).unwrap())
        })
    }
    fn raw_uninstall<T: SetupTaskImpl>(data: RawConfigValue, progress: Var<Progress>) -> BoxFutResult<()> {
        Box::new(async move {
            let args = UninstallArgs {
                data: value_de(data)?,
                progress,
            };
            T::uninstall(args).await
        })
    }

    /// Setup task instance ID.
    ///
    /// Identifies the same task across install/uninstall processes.
    pub fn instance_id(&self) -> Txt {
        self.instance_id.clone()
    }

    /// Setup task type ID.
    pub fn task_type_id(&self) -> Txt {
        (self.task_type_id)()
    }

    pub async fn prepare_install(&self, config: Box<dyn Any + Send>) -> Result<RawConfigValue> {
        todo!()
    }
}

///
pub struct TaskList {}
