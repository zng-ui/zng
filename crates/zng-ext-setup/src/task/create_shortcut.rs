#![cfg(any(windows, target_os = "linux"))]

use crate::task::InstallTaskError;
use crate::task::{escape_arg, path_utf8};
use std::fmt::Write as _;
use std::{io, path::PathBuf};

use super::SetupTaskError;

/// Setup task that creates a shortcut to an executable file.
pub enum CreateShortcut {}

// https://docs.rs/mslnk/0.1.8/mslnk/

impl super::SetupTask for CreateShortcut {
    type InstallConfig = CreateShortcutConfig;

    type PrepareInstall = PrepareInstallData;

    type Install = InstallData;

    fn task_type_id() -> super::TaskTypeId {
        "zng-setup/CreateShortcut".into()
    }

    #[cfg(windows)]
    async fn prepare_install(args: super::PrepareInstallArgs<Self>) -> Result<Self::PrepareInstall, SetupTaskError> {
        let c = args.config;

        let working_dir = path_utf8(c.working_dir)?;
        let icon = path_utf8(c.icon)?;

        let mut args = String::new();
        let mut sep = "";
        for arg in &c.args {
            write!(&mut args, "{sep}{}", escape_arg(arg)).unwrap();
            sep = " ";
        }

        Ok(PrepareInstallData {
            link_file: c.link_file.with_added_extension("lnk"),
            target_file: c.target_file,
            working_dir,
            arguments: args,
            app_id: c.app_id,
            name: c.name,
            icon,
        })
    }

    #[cfg(target_os = "linux")]
    async fn prepare_install(args: super::PrepareInstallArgs<Self>) -> Result<Self::PrepareInstall, SetupTaskError> {
        let c = args.config;

        let mut desktop = "[Desktop Entry]\nVersion=1.0\nType=Application\n".to_owned();

        write!(&mut desktop, "Exec={}", escape_arg(&path_utf8(c.target_file)?)).unwrap();
        for arg in c.args {
            write!(&mut desktop, " {}", escape_arg(&arg)).unwrap();
        }
        writeln!(&mut desktop).unwrap();

        if !c.working_dir.as_os_str().is_empty() {
            writeln!(&mut desktop, "Path={}", path_utf8(c.working_dir)?).unwrap();
        }

        if !c.icon.as_os_str().is_empty() {
            writeln!(&mut desktop, "Icon={}", path_utf8(c.icon)?).unwrap();
        }

        if c.name.is_empty() {
            match c.link_file.file_name() {
                Some(n) => writeln!(&mut desktop, "Name={}", n.to_string_lossy()).unwrap(),
                None => {
                    return Err(SetupTaskError::io(
                        c.link_file,
                        io::Error::new(io::ErrorKind::InvalidData, "missing name"),
                    ));
                }
            }
        } else {
            let name = c
                .name
                .replace("\\", r"\\")
                .replace("\n", r"\n")
                .replace("\t", r"\t")
                .replace("\r", r"\r");
            writeln!(&mut desktop, "Name={name}").unwrap();
        }

        Ok(PrepareInstallData {
            link_file: c.link_file,
            desktop,
        })
    }

    #[cfg(windows)]
    async fn install(args: super::InstallArgs<Self>) -> Result<Self::Install, InstallTaskError<Self::Install>> {
        let data = InstallData {
            link_file: args.data.link_file.clone(),
        };

        // must run in clean thread to avoid COM issues
        let (sx, rx) = zng_task::channel::rendezvous();
        let r = std::thread::spawn(move || {
            let r = windows_install(args.data);
            sx.send_blocking(()).unwrap();
            r
        });
        let _ = rx.recv().await;
        let r = match r.join() {
            Ok(r) => r,
            Err(p) => std::panic::resume_unwind(p),
        };
        if let Err(e) = r {
            return Err(InstallTaskError {
                error: SetupTaskError::other(e),
                // link_file probably does not exist, but `Some` here indicates that the failed uninstall can
                // be cleanup and `uninstall` will not error if the link file does not exist.
                clean_data: Some(data),
            });
        }
        Ok(data)
    }

    #[cfg(target_os = "linux")]
    async fn install(args: super::InstallArgs<Self>) -> Result<Self::Install, InstallTaskError<Self::Install>> {
        fn write(link_file: PathBuf, desktop: String) -> io::Result<()> {
            use std::io::Write as _;
            use std::os::unix::fs::PermissionsExt as _;

            let mut f = fs::File::create(link_file)?;
            f.write_all(desktop.as_bytes())?;

            let mut perms = f.metadata()?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(link_file, perms)?;

            Ok(())
        }

        let data = InstallData {
            link_file: args.data.link_file.clone(),
        };

        let link_file = args.data.link_file.clone();

        if let Err(e) = zng_task::wait(move || write(args.data.link_file, args.data.desktop)).await {
            return Err(InstallTaskError {
                error: SetupTaskError::io(args.data.link_file, e),
                // link_file probably does not exist, but `Some` here indicates that the failed uninstall can
                // be cleanup and `uninstall` will not error if the link file does not exist.
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
        if let Err(e) = zng_task::fs::remove_file(&args.data.link_file).await
            && !matches!(e.kind(), io::ErrorKind::NotFound)
        {
            return Err(SetupTaskError::io(args.data.link_file, e));
        }
        Ok(())
    }
}

/// Config for [`CreateShortcut`].
pub struct CreateShortcutConfig {
    /// Path to the shortcut, without extension.
    pub link_file: PathBuf,
    /// Path to the shortcut target executable file.
    pub target_file: PathBuf,
    /// Path to the initial `current_dir` of the executable.
    pub working_dir: PathBuf,
    /// Arguments to pass the executable.
    pub args: Vec<String>,

    /// Globally unique app ID.
    ///
    /// This is only used on Windows to set the AUMID on the shortcut, this **must** be
    /// the same value as `zng::env::About::windows_aumid` on the app executable. Note that
    /// if the app does not have a shortcut on the Start Menu that defines the ID notifications
    /// and other shell integrations will not work properly.
    pub app_id: String,

    /// Optional display name of the shortcut.
    ///
    /// Empty is the `link_file` file name.
    pub name: String,
    /// Optional icon for the shortcut.
    ///
    /// Empty is the `target_file` icon 0 in Windows, and no icon in Linux.
    pub icon: PathBuf,
}

#[cfg(target_os = "linux")]
#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrepareInstallData {
    link_file: PathBuf,
    desktop: String,
}

#[cfg(windows)]
#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrepareInstallData {
    link_file: PathBuf,
    target_file: PathBuf,
    working_dir: String,
    arguments: String,
    app_id: String,
    name: String,
    icon: String,
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstallData {
    link_file: PathBuf,
}

/// Must run in own thread to avoid COM issues
#[cfg(windows)]
fn windows_install(d: PrepareInstallData) -> windows::core::Result<()> {
    use std::path::Path;

    use windows::{
        Win32::{
            Storage::EnhancedStorage::PKEY_AppUserModel_ID,
            System::{
                Com::{
                    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile,
                    StructuredStorage::VariantToPropVariant,
                },
                Variant::VARIANT,
            },
            UI::Shell::{IShellLinkW, PropertiesSystem::IPropertyStore, ShellLink},
        },
        core::{Interface as _, PCWSTR},
    };

    fn wide(s: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;

        s.as_ref().encode_wide().chain(std::iter::once(0)).collect()
    }

    unsafe {
        let _ok = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
        debug_assert!(_ok, "expected to run in a new thread");
        struct ComDeinit;
        impl Drop for ComDeinit {
            fn drop(&mut self) {
                unsafe {
                    CoUninitialize();
                }
            }
        }
        let _com_deinit = ComDeinit;

        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;

        let target = wide(&d.target_file);
        link.SetPath(PCWSTR(target.as_ptr()))?;

        if !d.working_dir.is_empty() {
            let value = wide(&d.working_dir);
            link.SetWorkingDirectory(PCWSTR(value.as_ptr()))?;
        }

        if !d.arguments.is_empty() {
            let value = wide(&d.arguments);
            link.SetArguments(PCWSTR(value.as_ptr()))?;
        }

        if !d.name.is_empty() {
            let value = wide(&d.name);
            link.SetDescription(PCWSTR(value.as_ptr()))?;
        }

        if !d.icon.is_empty() {
            let value = wide(&d.icon);
            link.SetIconLocation(PCWSTR(value.as_ptr()), 0)?;
        }

        // System.AppUserModel.ID
        if !d.app_id.is_empty() {
            let store: IPropertyStore = link.cast()?;

            let variant = VARIANT::from(d.app_id.as_str());
            let value = VariantToPropVariant(&variant)?;

            store.SetValue(&PKEY_AppUserModel_ID, &value)?;
            store.Commit()?;
        }

        let persist: IPersistFile = link.cast()?;
        let output = wide(Path::new(&d.link_file));
        persist.Save(PCWSTR(output.as_ptr()), true)?;
    }
    todo!()
}
