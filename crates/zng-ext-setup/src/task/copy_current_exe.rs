use std::path::PathBuf;

use crate::task::{SetupTask, SetupTaskError};

/// Setup task that copies [`std::env::current_exe`] to a location.
///
/// The current exe is the setup executable, this task is useful when the setup executable
/// is also the app executable, or when it is needed to uninstall.
pub enum CopyCurrentExe {}
impl SetupTask for CopyCurrentExe {
    type InstallConfig = CopyCurrentExeConfig;

    type PrepareInstall = PrepareInstallData;

    type Install = InstallData;

    fn task_type_id() -> super::TaskTypeId {
        "zng-setup/CopyCurrentExe".into()
    }

    async fn prepare_install(args: super::PrepareInstallArgs<Self>) -> Result<Self::PrepareInstall, SetupTaskError> {
        let exe = match std::env::current_exe() {
            Ok(p) => p,
            Err(e) => return Err(SetupTaskError::io(PathBuf::new(), e)),
        };
        let temp_exe = args.config.destination_exe.with_added_extension(".cce-tmp");

        let target_dir = temp_exe.parent().unwrap();
        if let Err(e) = zng_task::fs::create_dir_all(target_dir).await {
            return Err(SetupTaskError::io(target_dir.to_path_buf(), e));
        }

        if let Err(e) = zng_task::fs::copy(&exe, &temp_exe).await {
            return Err(SetupTaskError::io(temp_exe, e));
        }

        Ok(PrepareInstallData {
            temp_exe,
            destination_exe: args.config.destination_exe,
        })
    }

    async fn install(args: super::InstallArgs<Self>) -> Result<Self::Install, super::InstallTaskError<Self::Install>> {
        if let Err(e) = zng_task::fs::rename(&args.data.temp_exe, &args.data.destination_exe).await {
            let _ = zng_task::fs::remove_file(&args.data.temp_exe).await;
            return Err(super::InstallTaskError {
                error: SetupTaskError::io(args.data.temp_exe, e),
                clean_data: None,
            });
        }
        Ok(InstallData {
            destination_exe: args.data.destination_exe,
        })
    }

    async fn cancel_install(args: super::CancelInstallArgs<Self>) -> Result<(), SetupTaskError> {
        if let Err(e) = zng_task::fs::remove_file(&args.data.temp_exe).await
            && !matches!(e.kind(), std::io::ErrorKind::NotFound)
        {
            return Err(SetupTaskError::io(args.data.temp_exe, e));
        }
        Ok(())
    }

    async fn validate_uninstall(args: super::ValidateUninstallArgs<Self>) -> Result<Self::Install, SetupTaskError> {
        Ok(args.data)
    }

    async fn uninstall(args: super::UninstallArgs<Self>) -> Result<(), SetupTaskError> {
        if let Err(e) = zng_task::fs::remove_file(&args.data.destination_exe).await
            && !matches!(e.kind(), std::io::ErrorKind::NotFound)
        {
            return Err(SetupTaskError::io(args.data.destination_exe, e));
        }
        Ok(())
    }
}

/// Config for [`CopyCurrentExe`].
#[non_exhaustive]
pub struct CopyCurrentExeConfig {
    /// Current exe is copied to this path.
    pub destination_exe: PathBuf,
}
impl CopyCurrentExeConfig {
    /// New with destination path.
    pub fn new(destination_exe: PathBuf) -> Self {
        Self { destination_exe }
    }
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrepareInstallData {
    temp_exe: PathBuf,
    destination_exe: PathBuf,
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstallData {
    destination_exe: PathBuf,
}
