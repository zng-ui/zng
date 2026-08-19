#![cfg(windows)]

use std::fmt::Write as _;
use std::{fmt, path::PathBuf};

use zng_unit::ByteLength;

use crate::task::{InstallTaskError, SetupTaskError, escape_arg, path_utf8};

/// Setup task that register an uninstaller for the app on Windows.
pub enum RegisterUninstaller {}

/// Config for [`RegisterUninstaller`].
pub struct RegisterUninstallerConfig {
    /// Globally unique ID of the app.
    ///
    /// This must be the same ID as previous versions if the installer is updating.
    ///
    /// This does not need to be the same ID used to identify the app in shell services like notifications,
    /// but it is strongly recommended the app uses an unified ID, in the reverse-DNS style. Only use a different ID
    /// if the app was already deployed with another setup builder that generated a GUID.
    pub app_id: String,

    /// Uninstaller executable.
    pub uninstaller: PathBuf,
    /// Arguments for the uninstaller that can show a GUI.
    pub args: Vec<String>,
    /// Arguments for the uninstaller to run without showing any GUI.
    ///
    /// Note that this replaces `args` when running in silent mode. Any common args
    /// between GUI and silent must be duplicated here.
    pub silent_args: Vec<String>,

    /// App display name.
    pub app: String,
    /// App display version.
    pub version: String,
    /// Path to the executable that defines the app icon.
    pub icon: PathBuf,
    /// App install location.
    pub install_location: PathBuf,
    /// Estimated sum total of the files installed.
    pub estimated_size: ByteLength,
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrepareInstallData {
    app_id: String,
    uninstall: String,
    quiet_uninstall: String,

    display_name: String,
    display_version: String,

    icon: String,
    install_location: String,
    // KB
    estimated_size: u32,
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstallData {
    app_id: String,
}

impl super::SetupTask for RegisterUninstaller {
    type InstallConfig = RegisterUninstallerConfig;

    type PrepareInstall = PrepareInstallData;

    type Install = InstallData;

    fn task_type_id() -> super::TaskTypeId {
        "zng-setup/RegisterUninstaller".into()
    }

    async fn prepare_install(args: super::PrepareInstallArgs<Self>) -> Result<Self::PrepareInstall, SetupTaskError> {
        let d = args.config;

        if let Some(u) = args.update {
            // !!: TODO remove previous if ID changed?
            let _ = u;
        }

        // validate app_id, required, no leading/trailing spaces, no '\'
        let app_id = d.app_id.trim();
        if app_id.is_empty() || app_id.len() != d.app_id.len() || app_id.contains('\\') {
            #[derive(Debug)]
            struct InvalidAppId;
            impl fmt::Display for InvalidAppId {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    write!(f, "invalid app_id")
                }
            }
            impl std::error::Error for InvalidAppId {}
            return Err(SetupTaskError::other(InvalidAppId));
        }

        let uninstaller = format!("{}", escape_arg(&path_utf8(d.uninstaller)?));

        if d.app.is_empty() {
            #[derive(Debug)]
            struct AppNameRequired;
            impl fmt::Display for AppNameRequired {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    write!(f, "app display name required")
                }
            }
            impl std::error::Error for AppNameRequired {}
            return Err(SetupTaskError::other(AppNameRequired));
        }

        fn cmd(mut exe: String, args: Vec<String>) -> String {
            for arg in args {
                write!(&mut exe, " {}", escape_arg(&arg)).unwrap();
            }
            exe
        }
        Ok(PrepareInstallData {
            app_id: d.app_id,
            uninstall: cmd(uninstaller.clone(), d.args),
            quiet_uninstall: cmd(uninstaller, d.silent_args),
            display_name: d.app,
            display_version: d.version,
            icon: path_utf8(d.icon)?,
            install_location: path_utf8(d.install_location)?,
            estimated_size: {
                let kb = d.estimated_size.kilos();
                if kb > u32::MAX as f64 { 0 } else { kb as u32 }
            },
        })
    }

    async fn install(args: super::InstallArgs<Self>) -> Result<Self::Install, InstallTaskError<Self::Install>> {
        fn register(d: &PrepareInstallData) -> windows_registry::Result<()> {
            let key =
                windows_registry::LOCAL_MACHINE.create(format!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{}", d.app_id))?;
            key.set_string("UninstallString", &d.uninstall)?;
            key.set_string("QuietUninstallString", &d.quiet_uninstall)?;
            key.set_string("DisplayName", &d.display_name)?;
            if !d.display_version.is_empty() {
                key.set_string("DisplayVersion", &d.display_version)?;
            }
            if d.estimated_size > 0 {
                key.set_u32("EstimatedSize", d.estimated_size)?;
            }

            // modify/repair not supported
            key.set_u32("NoModify", 1)?;
            key.set_u32("NoRepair", 1)?;

            Ok(())
        }

        let data = InstallData {
            app_id: args.data.app_id.clone(),
        };
        if let Err(e) = register(&args.data) {
            return Err(InstallTaskError {
                error: SetupTaskError::other(e),
                // key might not actually exist, but this indicates that
                // the failed install can be uninstalled and `uninstall` does not error
                // if the key is not found.
                clean_data: Some(data),
            });
        }
        Ok(data)
    }

    async fn cancel_install(_: super::CancelInstallArgs<Self>) -> Result<(), SetupTaskError> {
        Ok(())
    }

    async fn validate_uninstall(args: super::ValidateUninstallArgs<Self>) -> Result<Self::Install, SetupTaskError> {
        Ok(args.data)
    }

    async fn uninstall(args: super::UninstallArgs<Self>) -> Result<(), SetupTaskError> {
        const ERROR_FILE_NOT_FOUND: i32 = 2;
        let key = format!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{}", args.data.app_id);
        if let Err(e) = windows_registry::LOCAL_MACHINE.remove_tree(&key)
            && e.code().0 != ERROR_FILE_NOT_FOUND
        {
            return Err(SetupTaskError::other(e));
        }
        Ok(())
    }
}
