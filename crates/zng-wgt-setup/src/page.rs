//! Common setup pages.

mod welcome;
pub use welcome::WelcomePage;

mod install_dir;
pub use install_dir::InstallDirPage;

mod eula;
pub use eula::{EulaPage, EulaTxt};

mod status;
pub use status::{StatusPage, StatusTaskInfoFnArgs};

mod finish;
pub use finish::{FinishAction, FinishPage};

use zng_ext_l10n::l10n;
use zng_ext_setup::task::{SetupTask, TaskTypeId};
use zng_wgt::prelude::*;

use crate::{SETUP_OP_VAR, SetupOp};

/// Represents display info about a setup task component of a setup operation.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct TaskInfo {
    /// The id is [`SetupTask::task_type_id`] and task instance name.
    pub id: (TaskTypeId, Txt),

    /// Short description of the task.
    pub info: VarEq<Txt>,
}
impl TaskInfo {
    /// New with short description.
    pub fn new(task_type_id: impl Into<TaskTypeId>, task_instance_name: impl Into<Txt>, info: impl IntoVar<Txt>) -> Self {
        Self {
            id: (task_type_id.into(), task_instance_name.into()),
            info: VarEq(info.into_var()),
        }
    }

    /// Get display info for common setup task types.
    pub fn try_from_type(task_type_id: impl Into<TaskTypeId>, task_instance_name: impl Into<Txt>) -> Option<Self> {
        Self::try_from_ty_impl((task_type_id.into(), task_instance_name.into()))
    }
    fn try_from_ty_impl(id: (TaskTypeId, Txt)) -> Option<Self> {
        if id.0 == zng_ext_setup::task::ExtractTar::task_type_id() || id.0 == zng_ext_setup::task::CopyCurrentExe::task_type_id() {
            let info = SETUP_OP_VAR.flat_map(|op| match op {
                SetupOp::Install | SetupOp::Update | SetupOp::Repair => l10n!("status/default-info.extract-files", "Extract files"),
                SetupOp::Uninstall => l10n!("status/default-info.remove-files", "Remove files"),
            });
            return Some(TaskInfo { id, info: VarEq(info) });
        }

        #[cfg(any(windows, target_os = "linux"))]
        if id.0 == zng_ext_setup::task::CreateShortcut::task_type_id() {
            let info = SETUP_OP_VAR.flat_map(|op| match op {
                SetupOp::Install | SetupOp::Update | SetupOp::Repair => l10n!("status/default-info.create-shortcut", "Create shortcut"),
                SetupOp::Uninstall => l10n!("status/default-info.remove-shortcut", "Remove shortcut"),
            });
            return Some(TaskInfo { id, info: VarEq(info) });
        }

        #[cfg(windows)]
        if id.0 == zng_ext_setup::task::RegisterUninstaller::task_type_id() {
            let info = SETUP_OP_VAR.flat_map(|op| match op {
                SetupOp::Install | SetupOp::Update | SetupOp::Repair => {
                    l10n!("status/default-info.register-uninstaller", "Register uninstaller")
                }
                SetupOp::Uninstall => l10n!("status/default-info.unregister-uninstaller", "Unregister uninstaller"),
            });
            return Some(TaskInfo { id, info: VarEq(info) });
        }

        None
    }
}
