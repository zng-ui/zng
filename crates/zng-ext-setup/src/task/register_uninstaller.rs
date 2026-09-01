#![cfg(windows)]

use std::fmt::Write as _;
use std::sync::Arc;
use std::{fmt, path::PathBuf};

use zng_unit::ByteLength;

use crate::task::{InstallTaskError, SetupTaskError, escape_arg, path_utf8};

/// Setup task that register an uninstaller for the app on Windows.
pub enum RegisterUninstaller {}

/// Config for [`RegisterUninstaller`].
pub struct RegisterUninstallerConfig {
    // TODO(breaking) non_exhaustive
    /// Globally unique ID of the app.
    ///
    /// This must be the same ID as previous versions if the installer is updating.
    ///
    /// This does not need to be the same ID used to identify the app in shell services like notifications,
    /// but it is strongly recommended the app uses an unified ID, in the reverse-DNS style. Only use a different ID
    /// if the app was already deployed with another setup builder that generated a GUID.
    ///
    /// # Default
    ///
    /// If this is empty the [`zng::env::About::app_id`] value is used. Note that this is only valid
    /// if the setup app is defined on the same app executable.
    ///
    /// [`zng::env::About::app_id`]: zng_env::About::app_id
    pub app_id: String, // TODO(breaking) Txt

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
    ///
    /// # Default
    ///
    /// If this is empty the [`zng::env::About::app`] value is used. Note that this is only valid
    /// if the setup app is defined on the same app executable.
    ///
    /// [`zng::env::About::app`]: zng_env::About::app
    pub app: String,
    /// App display version.
    ///
    /// # Default
    ///
    /// If this is empty the [`zng::env::About::version`] value is used. Note that this is only valid
    /// if the setup app is defined on the same app executable.
    ///
    /// [`zng::env::About::version`]: zng_env::About::version
    pub version: String,
    /// Path to the executable that defines the app icon.
    ///
    /// # Default
    ///
    /// If this is empty the `uninstaller` executable is used.
    pub icon: PathBuf,
    /// App install location.
    pub install_location: PathBuf,
    /// Estimated sum total of the files installed.
    ///
    /// # Default
    ///
    /// If this is `0` no estimated size is registered. If this is grater than `u32::MAX` no
    /// estimated size is registered too, due to limitation on Windows API.
    pub estimated_size: ByteLength,
}
impl RegisterUninstallerConfig {
    /// New with minimal required info.
    ///
    /// Note that `app_id` and other metadata are copied from the `zng::env::about()`. This
    /// is only valid if the setup app is implemented on the same app executable.
    pub fn new(uninstaller: PathBuf, args: Vec<String>, silent_args: Vec<String>, install_location: PathBuf) -> Self {
        Self {
            uninstaller,
            args,
            silent_args,
            install_location,
            app_id: String::new(),
            app: String::new(),
            version: String::new(),
            icon: PathBuf::new(),
            estimated_size: ByteLength(0),
        }
    }
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrepareInstallData {
    update: Option<InstallData>,

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

        // validate app_id, required, no leading/trailing spaces, no '\'
        let app_id = if d.app_id.is_empty() {
            zng_env::about().app_id.to_string()
        } else {
            d.app_id
        };
        let app_id_trim = app_id.trim();
        if app_id_trim.is_empty() || app_id_trim.len() != app_id.len() || app_id.contains('\\') {
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

        let display_name = if d.app.is_empty() {
            zng_env::about().app.to_string()
        } else {
            d.app
        };
        let display_version = if d.version.is_empty() {
            zng_env::about().version.to_string()
        } else {
            d.version.to_string()
        };
        let icon = if d.icon.as_os_str().is_empty() {
            uninstaller.clone()
        } else {
            path_utf8(d.icon)?
        };

        fn cmd(mut exe: String, args: Vec<String>) -> String {
            for arg in args {
                write!(&mut exe, " {}", escape_arg(&arg)).unwrap();
            }
            exe
        }
        Ok(PrepareInstallData {
            update: args.update,
            app_id,
            uninstall: cmd(uninstaller.clone(), d.args),
            quiet_uninstall: cmd(uninstaller, d.silent_args),
            display_name,
            display_version,
            icon,
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

        let mut errors = vec![];

        // if changed ID and is updating remove previous key
        if let Some(u) = &args.data.update
            && u.app_id != args.data.app_id
            && let Err(e) = {
                let app_id = u.app_id.clone();
                zng_task::wait(move || unregister(&app_id)).await
            }
        {
            match e {
                SetupTaskError::Other(e) => errors = e,
                _ => unreachable!(),
            }
        }

        let data = InstallData {
            app_id: args.data.app_id.clone(),
        };
        if let Err(e) = zng_task::wait(move || register(&args.data)).await {
            errors.push(Arc::new(e));
            return Err(InstallTaskError {
                error: SetupTaskError::Other(errors),
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
        zng_task::wait(move || unregister(&args.data.app_id)).await
    }
}

fn unregister(app_id: &str) -> Result<(), SetupTaskError> {
    const ERROR_FILE_NOT_FOUND: i32 = 2;
    let key = format!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{app_id}");
    if let Err(e) = windows_registry::LOCAL_MACHINE.remove_tree(&key)
        && e.code().0 != ERROR_FILE_NOT_FOUND
    {
        return Err(SetupTaskError::other(e));
    }
    Ok(())
}
